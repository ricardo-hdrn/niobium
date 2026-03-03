use std::fmt;

/// Domain errors for Niobium MCP server.
#[derive(Debug)]
pub enum NiobiumError {
    /// Form submission timed out
    FormTimeout { timeout_secs: u64 },
    /// User cancelled the form
    FormCancelled,
    /// Invalid JSON Schema provided
    InvalidSchema(String),
    /// SQLite storage error
    StorageError(String),
}

impl fmt::Display for NiobiumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormTimeout { timeout_secs } => {
                write!(f, "Form timed out after {timeout_secs}s")
            }
            Self::FormCancelled => write!(f, "User cancelled the form"),
            Self::InvalidSchema(msg) => write!(f, "Invalid schema: {msg}"),
            Self::StorageError(msg) => write!(f, "Storage error: {msg}"),
        }
    }
}

impl std::error::Error for NiobiumError {}

impl From<rusqlite::Error> for NiobiumError {
    fn from(e: rusqlite::Error) -> Self {
        Self::StorageError(e.to_string())
    }
}
