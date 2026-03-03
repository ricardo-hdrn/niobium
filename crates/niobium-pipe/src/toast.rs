//! ToastStage — emits a UI toast event. Passes input through unchanged (pure side-effect).

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::debug;

use crate::event::{PipeEvent, Severity};
use crate::registry::StageBuilder;
use crate::stage::Stage;
use crate::template;

/// Configuration for a toast stage.
#[derive(Debug, Clone, Deserialize)]
pub struct ToastStageConfig {
    /// Unique name for this stage.
    pub name: String,

    /// Message to display (supports `${...}` templates).
    pub message: String,

    /// Severity level: "info" | "success" | "warning" | "error" (default: "info").
    pub severity: Option<String>,
}

pub struct ToastStage {
    config: ToastStageConfig,
}

impl ToastStage {
    pub fn new(config: ToastStageConfig) -> Self {
        Self { config }
    }
}

fn parse_severity(s: Option<&str>) -> Severity {
    match s {
        Some("success") => Severity::Success,
        Some("warning") => Severity::Warning,
        Some("error") => Severity::Error,
        _ => Severity::Info,
    }
}

#[async_trait]
impl Stage for ToastStage {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn execute(
        &self,
        input: Value,
        events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        debug!(stage = self.config.name, "executing toast stage");

        // Render message via template engine
        let message = template::render_string(&self.config.message, &input)?;
        let severity = parse_severity(self.config.severity.as_deref());

        let _ = events.send(PipeEvent::Toast { message, severity });

        // Passthrough — return input unchanged
        Ok(input)
    }
}

/// Builder for toast stages.
pub struct ToastStageBuilder;

impl StageBuilder for ToastStageBuilder {
    fn stage_type(&self) -> &str {
        "toast"
    }

    fn build(&self, def: &Value) -> Result<Box<dyn Stage>> {
        let config: ToastStageConfig = serde_json::from_value(def.clone())?;
        Ok(Box::new(ToastStage::new(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_events() -> (mpsc::UnboundedSender<PipeEvent>, mpsc::UnboundedReceiver<PipeEvent>) {
        mpsc::unbounded_channel()
    }

    #[tokio::test]
    async fn emits_toast_event() {
        let config = ToastStageConfig {
            name: "notify".into(),
            message: "Hello!".into(),
            severity: Some("success".into()),
        };

        let stage = ToastStage::new(config);
        let (tx, mut rx) = make_events();
        let input = json!({"data": "value"});

        let result = stage.execute(input.clone(), &tx).await.unwrap();

        // Passthrough: result equals input
        assert_eq!(result, input);

        // Toast event emitted
        let event = rx.try_recv().unwrap();
        match event {
            PipeEvent::Toast { message, severity } => {
                assert_eq!(message, "Hello!");
                assert_eq!(severity, Severity::Success);
            }
            other => panic!("expected Toast event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn template_in_message() {
        let config = ToastStageConfig {
            name: "notify".into(),
            message: "Created user ${username}".into(),
            severity: None,
        };

        let stage = ToastStage::new(config);
        let (tx, mut rx) = make_events();

        stage
            .execute(json!({"username": "alice"}), &tx)
            .await
            .unwrap();

        let event = rx.try_recv().unwrap();
        match event {
            PipeEvent::Toast { message, severity } => {
                assert_eq!(message, "Created user alice");
                assert_eq!(severity, Severity::Info); // default
            }
            other => panic!("expected Toast event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn default_severity_is_info() {
        let config = ToastStageConfig {
            name: "t".into(),
            message: "msg".into(),
            severity: None,
        };

        let stage = ToastStage::new(config);
        let (tx, mut rx) = make_events();
        stage.execute(json!({}), &tx).await.unwrap();

        match rx.try_recv().unwrap() {
            PipeEvent::Toast { severity, .. } => assert_eq!(severity, Severity::Info),
            other => panic!("expected Toast, got {other:?}"),
        }
    }

    #[test]
    fn builder_creates_stage() {
        let builder = ToastStageBuilder;
        assert_eq!(builder.stage_type(), "toast");

        let def = json!({
            "name": "test",
            "message": "hello"
        });
        let stage = builder.build(&def).unwrap();
        assert_eq!(stage.name(), "test");
    }
}
