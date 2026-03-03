//! TransformStage — in-process JSON transformation using path expressions.

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::debug;

use crate::event::PipeEvent;
use crate::registry::StageBuilder;
use crate::stage::Stage;

/// Configuration for a transform stage.
#[derive(Debug, Clone, Deserialize)]
pub struct TransformStageConfig {
    /// Unique name for this stage.
    pub name: String,

    /// Object mapping output keys to `$.path` expressions.
    pub expr: Value,
}

pub struct TransformStage {
    config: TransformStageConfig,
}

impl TransformStage {
    pub fn new(config: TransformStageConfig) -> Self {
        Self { config }
    }
}

/// Walk a `$.dot.path` expression against a JSON value.
fn resolve_path<'a>(path: &str, context: &'a Value) -> Result<&'a Value> {
    let path = path.strip_prefix("$.").unwrap_or(path);
    let mut current = context;
    for segment in path.split('.') {
        current = current
            .get(segment)
            .ok_or_else(|| anyhow::anyhow!("transform path '$.{path}': key '{segment}' not found"))?;
    }
    Ok(current)
}

#[async_trait]
impl Stage for TransformStage {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn execute(
        &self,
        input: Value,
        _events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        debug!(stage = self.config.name, "executing transform stage");

        let expr_map = match &self.config.expr {
            Value::Object(map) => map,
            _ => bail!(
                "stage '{}': expr must be a JSON object",
                self.config.name
            ),
        };

        let mut output = serde_json::Map::with_capacity(expr_map.len());

        for (key, path_val) in expr_map {
            let path = path_val.as_str().ok_or_else(|| {
                anyhow::anyhow!(
                    "stage '{}': expr value for '{key}' must be a string path",
                    self.config.name
                )
            })?;

            let resolved = resolve_path(path, &input)?;
            output.insert(key.clone(), resolved.clone());
        }

        Ok(Value::Object(output))
    }
}

/// Builder for transform stages.
pub struct TransformStageBuilder;

impl StageBuilder for TransformStageBuilder {
    fn stage_type(&self) -> &str {
        "transform"
    }

    fn build(&self, def: &Value) -> Result<Box<dyn Stage>> {
        let config: TransformStageConfig = serde_json::from_value(def.clone())?;
        Ok(Box::new(TransformStage::new(config)))
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
    async fn simple_extraction() {
        let config = TransformStageConfig {
            name: "extract".into(),
            expr: json!({
                "user": "$.username",
                "tok": "$.pipe.auth.body.token"
            }),
        };

        let stage = TransformStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({
            "username": "alice",
            "pipe": {
                "auth": {
                    "body": {"token": "abc123"}
                }
            }
        });

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["user"], "alice");
        assert_eq!(result["tok"], "abc123");
    }

    #[tokio::test]
    async fn nested_path() {
        let config = TransformStageConfig {
            name: "deep".into(),
            expr: json!({"val": "$.a.b.c"}),
        };

        let stage = TransformStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({"a": {"b": {"c": 42}}});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["val"], 42);
    }

    #[tokio::test]
    async fn missing_key_error() {
        let config = TransformStageConfig {
            name: "bad".into(),
            expr: json!({"x": "$.nonexistent.path"}),
        };

        let stage = TransformStage::new(config);
        let (tx, _rx) = make_events();

        let err = stage.execute(json!({}), &tx).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn preserves_types() {
        let config = TransformStageConfig {
            name: "types".into(),
            expr: json!({
                "s": "$.str_val",
                "n": "$.num_val",
                "b": "$.bool_val",
                "a": "$.arr_val"
            }),
        };

        let stage = TransformStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({
            "str_val": "hello",
            "num_val": 3.14,
            "bool_val": true,
            "arr_val": [1, 2, 3]
        });

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["s"], "hello");
        assert_eq!(result["n"], 3.14);
        assert_eq!(result["b"], true);
        assert_eq!(result["a"], json!([1, 2, 3]));
    }

    #[test]
    fn builder_creates_stage() {
        let builder = TransformStageBuilder;
        assert_eq!(builder.stage_type(), "transform");

        let def = json!({
            "name": "test",
            "expr": {"out": "$.input"}
        });
        let stage = builder.build(&def).unwrap();
        assert_eq!(stage.name(), "test");
    }
}
