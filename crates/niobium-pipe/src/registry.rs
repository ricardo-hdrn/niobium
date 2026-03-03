//! Stage registry — maps type names to stage builders.

use std::collections::HashMap;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::stage::Stage;

/// Builds a [`Stage`] from a JSON definition.
pub trait StageBuilder: Send + Sync {
    /// The type name this builder handles (e.g., "http", "process", "transform", "toast").
    fn stage_type(&self) -> &str;

    /// Build a Stage from a JSON stage definition.
    fn build(&self, def: &Value) -> Result<Box<dyn Stage>>;

    /// Whether stages built by this builder are trusted with sensitive data.
    ///
    /// Trusted stages (default) receive resolved secrets during pipeline execution.
    /// Untrusted stages (e.g., external processes) only see placeholders.
    fn is_trusted(&self) -> bool {
        true
    }
}

/// A built stage paired with its trust level from the builder.
pub struct BuiltStage {
    pub stage: Box<dyn Stage>,
    pub trusted: bool,
}

/// Registry that maps stage type names to their builders.
pub struct StageRegistry {
    builders: HashMap<String, Box<dyn StageBuilder>>,
}

impl StageRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    /// Register a builder. Replaces any existing builder for the same type.
    pub fn register(&mut self, builder: Box<dyn StageBuilder>) {
        let name = builder.stage_type().to_string();
        self.builders.insert(name, builder);
    }

    /// Look up the builder for a stage type and build a stage from the definition.
    ///
    /// Returns a [`BuiltStage`] containing the stage and its trust level.
    pub fn build_stage(&self, def: &Value) -> Result<BuiltStage> {
        let stage_type = infer_stage_type(def)?;
        let builder = self
            .builders
            .get(&stage_type)
            .ok_or_else(|| anyhow::anyhow!("unknown stage type: '{stage_type}'"))?;
        let stage = builder.build(def)?;
        Ok(BuiltStage {
            trusted: builder.is_trusted(),
            stage,
        })
    }
}

impl Default for StageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default registry with all built-in stage types.
pub fn default_registry() -> StageRegistry {
    let mut reg = StageRegistry::new();
    reg.register(Box::new(crate::http::HttpStageBuilder));
    reg.register(Box::new(crate::redact::RedactStageBuilder));
    reg.register(Box::new(crate::process::ProcessStageBuilder));
    reg.register(Box::new(crate::transform::TransformStageBuilder));
    reg.register(Box::new(crate::toast::ToastStageBuilder));
    reg
}

/// Infer the stage type from a stage definition.
///
/// Explicit `"type"` field takes priority. Otherwise, infer from fields:
/// - `url` present → `"http"`
/// - `command` present → `"process"`
/// - `expr` present → `"transform"`
/// - `message` present (without `url`) → `"toast"`
/// - `fields` present → `"redact"`
fn infer_stage_type(def: &Value) -> Result<String> {
    // Explicit type field
    if let Some(t) = def.get("type").and_then(|v| v.as_str()) {
        return Ok(t.to_string());
    }

    // Infer from fields
    if def.get("url").is_some() {
        return Ok("http".to_string());
    }
    if def.get("command").is_some() {
        return Ok("process".to_string());
    }
    if def.get("expr").is_some() {
        return Ok("transform".to_string());
    }
    if def.get("message").is_some() {
        return Ok("toast".to_string());
    }
    if def.get("fields").is_some() {
        return Ok("redact".to_string());
    }
    if def.get("branches").is_some() {
        return Ok("parallel".to_string());
    }

    bail!(
        "cannot infer stage type: no 'type' field and no recognizable fields in definition: {}",
        def
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infer_explicit_type() {
        let def = json!({"type": "http", "name": "s1", "url": "http://example.com"});
        assert_eq!(infer_stage_type(&def).unwrap(), "http");
    }

    #[test]
    fn infer_http_from_url() {
        let def = json!({"name": "s1", "url": "http://example.com"});
        assert_eq!(infer_stage_type(&def).unwrap(), "http");
    }

    #[test]
    fn infer_process_from_command() {
        let def = json!({"name": "s1", "command": "python3", "args": ["script.py"]});
        assert_eq!(infer_stage_type(&def).unwrap(), "process");
    }

    #[test]
    fn infer_transform_from_expr() {
        let def = json!({"name": "s1", "expr": {"token": "$.pipe.auth.body.token"}});
        assert_eq!(infer_stage_type(&def).unwrap(), "transform");
    }

    #[test]
    fn infer_toast_from_message() {
        let def = json!({"name": "s1", "message": "Done!"});
        assert_eq!(infer_stage_type(&def).unwrap(), "toast");
    }

    #[test]
    fn infer_fails_for_unknown() {
        let def = json!({"name": "s1"});
        assert!(infer_stage_type(&def).is_err());
    }

    #[test]
    fn default_registry_has_all_types() {
        let reg = default_registry();
        assert!(reg.builders.contains_key("http"));
        assert!(reg.builders.contains_key("redact"));
        assert!(reg.builders.contains_key("process"));
        assert!(reg.builders.contains_key("transform"));
        assert!(reg.builders.contains_key("toast"));
    }
}
