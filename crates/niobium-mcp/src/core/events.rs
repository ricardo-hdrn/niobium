// Typed events for the Niobium event bus.
//
// Every interaction flows through the bus as an event:
//   Agent → MCP server emits ToolCalled → UI engine renders → User interacts → UI emits result

use serde_json::Value;
use uuid::Uuid;

/// Unique identifier for a request/response pair.
pub type RequestId = Uuid;

/// All events that flow through the Niobium event bus.
#[derive(Debug, Clone)]
pub enum Event {
    // ── UI requests (MCP server → UI engine) ────────────────────────────
    /// Show a form and await user submission.
    ShowForm {
        request_id: RequestId,
        schema: Value,
        title: String,
        prefill: Option<Value>,
        width: Option<u32>,
        height: Option<u32>,
        density: Option<String>,
        animate: Option<bool>,
        accent: Option<String>,
    },

    /// Show a confirmation dialog and await user response.
    ShowConfirmation {
        request_id: RequestId,
        message: String,
        title: String,
        width: Option<u32>,
        height: Option<u32>,
        accent: Option<String>,
    },

    /// Show a page with mixed content and input nodes.
    ShowPage {
        request_id: RequestId,
        children: Value,
        title: String,
        prefill: Option<Value>,
        width: Option<u32>,
        height: Option<u32>,
        density: Option<String>,
        animate: Option<bool>,
        accent: Option<String>,
    },

    /// Show rich output content (markdown, JSON, table, diff) and await dismissal.
    ShowOutput {
        request_id: RequestId,
        content: String,
        output_type: String,
        title: String,
        width: Option<u32>,
        height: Option<u32>,
        accent: Option<String>,
    },

    // ── UI responses (UI engine → MCP server) ───────────────────────────
    /// User submitted form data.
    FormSubmitted { request_id: RequestId, data: Value },

    /// User cancelled a form.
    FormCancelled { request_id: RequestId },

    /// User responded to a confirmation dialog.
    Confirmed { request_id: RequestId, value: bool },

    /// User dismissed an output display.
    OutputDismissed { request_id: RequestId },

    /// User submitted page data (page had input nodes).
    PageSubmitted { request_id: RequestId, data: Value },

    /// User dismissed a content-only page (no input nodes).
    PageDismissed { request_id: RequestId },

    /// User cancelled a page.
    PageCancelled { request_id: RequestId },

    // ── Pipeline events ─────────────────────────────────────────────────
    /// A pipeline event (toast, stage progress) for UI forwarding.
    PipeEvent(niobium_pipe::PipeEvent),

    // ── Pill SPI ────────────────────────────────────────────────────────
    /// A pill pushed into the feed by any source plugin.
    Pill(Pill),

    // ── Lifecycle ────────────────────────────────────────────────────────
    /// The runtime is shutting down.
    Shutdown,
}

/// A pill — generic unit in the activity feed.
///
/// Any plugin can produce pills (hub WS client, voice, local watchers, etc.).
/// Niobium renders them in the feed and routes taps to the appropriate component.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pill {
    /// Source plugin identifier (e.g. "hub", "voice", "watcher").
    pub source: String,
    /// Human-readable summary.
    pub summary: String,
    /// When this pill was created (ISO 8601).
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    /// Output type hint for rendering ("decision", "form", "markdown", "table", etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_type: Option<String>,
    /// Decision options (when output_type = "decision").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Rich content (form schema, table data, markdown text, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    /// URL to sink the user's response to (remote routing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_url: Option<String>,
    /// Source-specific metadata (IDs, refs, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

fn default_timestamp() -> String {
    chrono_now()
}

fn chrono_now() -> String {
    // Simple ISO 8601 without chrono dep — good enough for display
    String::new()
}

impl Event {
    /// Get the request ID if this event is part of a request/response pair.
    pub fn request_id(&self) -> Option<RequestId> {
        match self {
            Event::ShowForm { request_id, .. }
            | Event::ShowConfirmation { request_id, .. }
            | Event::ShowOutput { request_id, .. }
            | Event::ShowPage { request_id, .. }
            | Event::FormSubmitted { request_id, .. }
            | Event::FormCancelled { request_id }
            | Event::Confirmed { request_id, .. }
            | Event::OutputDismissed { request_id }
            | Event::PageSubmitted { request_id, .. }
            | Event::PageDismissed { request_id }
            | Event::PageCancelled { request_id } => Some(*request_id),
            Event::PipeEvent(_)
            | Event::Pill(_)
            | Event::Shutdown => None,
        }
    }
}
