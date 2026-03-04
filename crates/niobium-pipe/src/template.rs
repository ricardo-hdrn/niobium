//! Template engine — resolves `${...}` references against a JSON context.
//!
//! Patterns:
//! - `${field_name}` — top-level form field
//! - `${pipe.step_name.body.path}` — value from a previous pipe stage
//! - `${env:VAR_NAME}` — environment variable

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Resolve all `${...}` references in a template string against a context.
pub fn render_string(template: &str, context: &Value) -> Result<String> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut ref_name = String::new();
            let mut found_close = false;
            for ch in chars.by_ref() {
                if ch == '}' {
                    found_close = true;
                    break;
                }
                ref_name.push(ch);
            }
            if !found_close {
                bail!("unclosed template reference: ${{{ref_name}");
            }
            let resolved = resolve_ref(&ref_name, context)?;
            result.push_str(&resolved);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Recursively walk a JSON value, applying `render_string` to all string leaves.
pub fn render_value(template: &Value, context: &Value) -> Result<Value> {
    match template {
        Value::String(s) => {
            if s.contains("${") {
                let rendered = render_string(s, context)?;
                Ok(Value::String(rendered))
            } else {
                Ok(template.clone())
            }
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(k.clone(), render_value(v, context)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(arr) => {
            let out: Result<Vec<Value>> = arr.iter().map(|v| render_value(v, context)).collect();
            Ok(Value::Array(out?))
        }
        // Numbers, bools, nulls pass through unchanged
        other => Ok(other.clone()),
    }
}

fn resolve_ref(ref_name: &str, context: &Value) -> Result<String> {
    // Environment variable: ${env:VAR_NAME}
    if let Some(var_name) = ref_name.strip_prefix("env:") {
        return std::env::var(var_name)
            .with_context(|| format!("environment variable '{var_name}' not set"));
    }

    // Dot-notation path walking: ${a.b.c} → context["a"]["b"]["c"]
    let mut current = context;
    for segment in ref_name.split('.') {
        current = current.get(segment).with_context(|| {
            format!("template ref '${{{ref_name}}}': key '{segment}' not found")
        })?;
    }

    match current {
        Value::String(s) => Ok(s.clone()),
        Value::Null => Ok(String::new()),
        other => Ok(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_ref() {
        let ctx = json!({"name": "alice", "age": 30});
        assert_eq!(render_string("Hello ${name}", &ctx).unwrap(), "Hello alice");
    }

    #[test]
    fn nested_path() {
        let ctx = json!({"pipe": {"verify": {"body": {"token": "abc123"}}}});
        assert_eq!(
            render_string("Bearer ${pipe.verify.body.token}", &ctx).unwrap(),
            "Bearer abc123"
        );
    }

    #[test]
    fn env_var() {
        unsafe { std::env::set_var("NB_TEST_VAR", "secret42") };
        let ctx = json!({});
        assert_eq!(
            render_string("key=${env:NB_TEST_VAR}", &ctx).unwrap(),
            "key=secret42"
        );
        unsafe { std::env::remove_var("NB_TEST_VAR") };
    }

    #[test]
    fn missing_key_error() {
        let ctx = json!({"name": "alice"});
        let err = render_string("${missing}", &ctx).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn mixed_refs() {
        let ctx = json!({"user": "admin", "pass": "s3cret", "host": "db.local"});
        assert_eq!(
            render_string("postgres://${user}:${pass}@${host}", &ctx).unwrap(),
            "postgres://admin:s3cret@db.local"
        );
    }

    #[test]
    fn render_value_recursive() {
        let ctx = json!({"name": "alice", "token": "xyz"});
        let template = json!({
            "url": "https://api.example.com/${name}",
            "headers": {"Authorization": "Bearer ${token}"},
            "count": 42
        });
        let rendered = render_value(&template, &ctx).unwrap();
        assert_eq!(rendered["url"], "https://api.example.com/alice");
        assert_eq!(rendered["headers"]["Authorization"], "Bearer xyz");
        assert_eq!(rendered["count"], 42);
    }

    #[test]
    fn non_string_passthrough() {
        let ctx = json!({});
        let template = json!({"enabled": true, "count": 5, "items": [1, 2, 3]});
        let rendered = render_value(&template, &ctx).unwrap();
        assert_eq!(rendered, template);
    }

    #[test]
    fn numeric_value_to_string() {
        let ctx = json!({"port": 5432});
        assert_eq!(render_string("host:${port}", &ctx).unwrap(), "host:5432");
    }

    #[test]
    fn unclosed_ref_error() {
        let ctx = json!({});
        let err = render_string("${unclosed", &ctx).unwrap_err();
        assert!(err.to_string().contains("unclosed"));
    }
}
