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

    // ── Hub events (WebSocket → UI) ─────────────────────────────────────
    /// A real-time event from the mcp-discuss hub.
    HubEvent(HubEvent),

    /// Hub connection state changed.
    HubConnectionState(HubConnectionState),

    // ── Lifecycle ────────────────────────────────────────────────────────
    /// The runtime is shutting down.
    Shutdown,
}

/// Event types pushed by the mcp-discuss hub over WebSocket.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum HubEvent {
    /// New update event on a subject feed.
    #[serde(rename = "update_event")]
    UpdateEvent {
        subject_id: String,
        event_id: i64,
        source_kind: String,
        source_id: String,
        summary: String,
        payload_ref: Option<String>,
        created_at: String,
    },

    /// New progress update on an actionable.
    #[serde(rename = "actionable_update")]
    ActionableUpdate {
        subject_id: String,
        actionable_id: String,
        update_id: i64,
        source_kind: String,
        source_id: String,
        summary: String,
        created_at: String,
    },

    /// Actionable state changed (proposed → dispatched → done, etc.)
    #[serde(rename = "actionable_state")]
    ActionableState {
        subject_id: String,
        actionable_id: String,
        old_state: String,
        new_state: String,
    },

    /// Subject status changed (open → paused → closed).
    #[serde(rename = "subject_status")]
    SubjectStatus {
        subject_id: String,
        old_status: String,
        new_status: String,
    },
}

/// Hub WebSocket connection state.
#[derive(Debug, Clone)]
pub enum HubConnectionState {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
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
            Event::PipeEvent(_)
            | Event::HubEvent(_)
            | Event::HubConnectionState(_)
            | Event::Shutdown => None,
        }
    }
}
