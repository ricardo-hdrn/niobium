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

    // ── Pipeline events ─────────────────────────────────────────────────
    /// A pipeline event (toast, stage progress) for UI forwarding.
    PipeEvent(niobium_pipe::PipeEvent),

    // ── Lifecycle ────────────────────────────────────────────────────────
    /// The runtime is shutting down.
    Shutdown,
}

impl Event {
    /// Get the request ID if this event is part of a request/response pair.
    pub fn request_id(&self) -> Option<RequestId> {
        match self {
            Event::ShowForm { request_id, .. }
            | Event::ShowConfirmation { request_id, .. }
            | Event::ShowOutput { request_id, .. }
            | Event::FormSubmitted { request_id, .. }
            | Event::FormCancelled { request_id }
            | Event::Confirmed { request_id, .. }
            | Event::OutputDismissed { request_id } => Some(*request_id),
            Event::PipeEvent(_) | Event::Shutdown => None,
        }
    }
}
