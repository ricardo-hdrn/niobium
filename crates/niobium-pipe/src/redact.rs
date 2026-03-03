//! Redaction — strips sensitive values from pipeline data.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::event::PipeEvent;
use crate::registry::StageBuilder;
use crate::stage::Stage;

const REDACTED: &str = "[REDACTED]";

/// A stage that replaces specified field paths with `[REDACTED]`.
pub struct RedactStage {
    name: String,
    fields: Vec<String>,
}

impl RedactStage {
    pub fn new(name: String, fields: Vec<String>) -> Self {
        Self { name, fields }
    }
}

#[async_trait]
impl Stage for RedactStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        mut input: Value,
        _events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        for path in &self.fields {
            redact_path(&mut input, path);
        }
        Ok(input)
    }
}

/// Walk a dot-notation path and replace the leaf value with `[REDACTED]`.
fn redact_path(value: &mut Value, path: &str) {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for (i, segment) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            // Last segment — redact if it exists
            if let Value::Object(map) = current {
                if map.contains_key(*segment) {
                    map.insert(segment.to_string(), Value::String(REDACTED.to_string()));
                }
            }
        } else {
            // Intermediate segment — descend
            match current.get_mut(*segment) {
                Some(next) => current = next,
                None => return, // path doesn't exist, skip
            }
        }
    }
}

/// Builder for redact stages.
pub struct RedactStageBuilder;

impl StageBuilder for RedactStageBuilder {
    fn stage_type(&self) -> &str {
        "redact"
    }

    fn build(&self, def: &Value) -> Result<Box<dyn Stage>> {
        let name = def
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("redact")
            .to_string();

        let fields = def
            .get("fields")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Box::new(RedactStage::new(name, fields)))
    }
}

/// Extract the names of top-level sensitive fields from a JSON Schema.
///
/// **All fields are sensitive by default.** Only fields explicitly marked
/// `"x-sensitive": false` are excluded. Used to identify which fields should
/// be protected by [`SecureContext`] during pipeline execution.
pub fn extract_sensitive_fields(schema: &Value) -> Vec<String> {
    let properties = match schema.get("properties") {
        Some(Value::Object(props)) => props,
        _ => return vec![],
    };

    properties
        .iter()
        .filter(|(_, field_schema)| {
            // Sensitive by default — only false opts out
            field_schema
                .get("x-sensitive")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Scan a JSON Schema and redact sensitive fields from the data object.
///
/// **All fields are sensitive by default.** Only fields explicitly marked
/// `"x-sensitive": false` are left untouched. Used to strip form values
/// before returning data to the agent.
pub fn redact_sensitive(schema: &Value, data: &mut Value) {
    let properties = match schema.get("properties") {
        Some(Value::Object(props)) => props,
        _ => return,
    };

    let data_obj = match data {
        Value::Object(map) => map,
        _ => return,
    };

    for (field_name, field_schema) in properties {
        // Sensitive by default — only explicit false opts out
        let is_sensitive = field_schema
            .get("x-sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if is_sensitive && data_obj.contains_key(field_name) {
            data_obj.insert(field_name.clone(), Value::String(REDACTED.to_string()));
        }

        // Recurse into nested objects
        if field_schema.get("type").and_then(|t| t.as_str()) == Some("object") {
            if let Some(nested_data) = data_obj.get_mut(field_name) {
                redact_sensitive(field_schema, nested_data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn redact_stage_top_level() {
        let stage = RedactStage::new("redact".into(), vec!["password".into(), "api_key".into()]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"username": "alice", "password": "s3cret", "api_key": "key123"});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["username"], "alice");
        assert_eq!(result["password"], "[REDACTED]");
        assert_eq!(result["api_key"], "[REDACTED]");
    }

    #[tokio::test]
    async fn redact_stage_nested_path() {
        let stage = RedactStage::new("redact".into(), vec!["db.password".into()]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"db": {"host": "localhost", "password": "s3cret"}});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["db"]["host"], "localhost");
        assert_eq!(result["db"]["password"], "[REDACTED]");
    }

    #[tokio::test]
    async fn redact_stage_missing_field_noop() {
        let stage = RedactStage::new("redact".into(), vec!["nonexistent".into()]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"name": "alice"});

        let result = stage.execute(input.clone(), &tx).await.unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn redact_sensitive_defaults_to_redacting_all() {
        let schema = json!({
            "type": "object",
            "properties": {
                "username": {"type": "string"},
                "password": {"type": "string"},
                "visible": {"type": "string", "x-sensitive": false}
            }
        });
        let mut data = json!({
            "username": "alice",
            "password": "s3cret",
            "visible": "hello"
        });

        redact_sensitive(&schema, &mut data);
        // Default: all fields redacted
        assert_eq!(data["username"], "[REDACTED]");
        assert_eq!(data["password"], "[REDACTED]");
        // Only x-sensitive: false is visible
        assert_eq!(data["visible"], "hello");
    }

    #[test]
    fn extract_sensitive_fields_defaults_all_sensitive() {
        let schema = json!({
            "type": "object",
            "properties": {
                "username": {"type": "string"},
                "password": {"type": "string"},
                "api_key": {"type": "string"},
                "visible": {"type": "string", "x-sensitive": false}
            }
        });
        let mut fields = extract_sensitive_fields(&schema);
        fields.sort();
        // All fields except visible (x-sensitive: false)
        assert_eq!(fields, vec!["api_key", "password", "username"]);
    }

    #[test]
    fn extract_sensitive_fields_all_when_no_annotations() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        });
        let mut fields = extract_sensitive_fields(&schema);
        fields.sort();
        assert_eq!(fields, vec!["email", "name"]);
    }

    #[test]
    fn extract_sensitive_fields_empty_for_empty_schema() {
        let schema = json!({"type": "object"});
        assert!(extract_sensitive_fields(&schema).is_empty());
    }

    #[test]
    fn redact_sensitive_nested_object() {
        // Parent object must be x-sensitive: false to allow recursion into children
        let schema = json!({
            "type": "object",
            "properties": {
                "db": {
                    "type": "object",
                    "x-sensitive": false,
                    "properties": {
                        "host": {"type": "string", "x-sensitive": false},
                        "password": {"type": "string"}
                    }
                }
            }
        });
        let mut data = json!({"db": {"host": "localhost", "password": "s3cret"}});

        redact_sensitive(&schema, &mut data);
        assert_eq!(data["db"]["host"], "localhost");
        assert_eq!(data["db"]["password"], "[REDACTED]");
    }
}
