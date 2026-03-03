//! Pipeline events — emitted by stages for UI feedback.

/// Severity level for toast notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}

/// Events emitted during pipeline execution.
#[derive(Debug, Clone)]
pub enum PipeEvent {
    /// Display a toast notification to the user.
    Toast {
        message: String,
        severity: Severity,
    },
    /// A pipeline stage has started executing.
    StageStarted {
        name: String,
    },
    /// A pipeline stage completed successfully.
    StageCompleted {
        name: String,
    },
    /// A pipeline stage failed.
    StageFailed {
        name: String,
        error: String,
    },
}
