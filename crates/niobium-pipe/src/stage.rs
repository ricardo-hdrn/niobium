//! Stage trait — the unit of work in a pipeline.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::event::PipeEvent;

/// A single executable step in a pipeline.
#[async_trait]
pub trait Stage: Send + Sync {
    /// Unique name for this stage (used in context paths like `${pipe.<name>.body.field}`).
    fn name(&self) -> &str;

    /// Execute the stage with the accumulated pipeline context.
    /// Push events to `events` for UI feedback (toasts, progress).
    async fn execute(
        &self,
        input: Value,
        events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value>;
}
