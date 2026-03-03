//! SecureContext — in-memory vault for sensitive field values.
//!
//! Extracts sensitive values from pipeline data at the start, replaces them
//! with unforgeable placeholders (`<<NB:nonce:field>>`), and resolves
//! placeholders back to real values only for trusted stages.

use std::collections::HashMap;

use serde_json::Value;

const REDACTED: &str = "[REDACTED]";

/// Holds sensitive field values extracted from pipeline data.
///
/// Placeholders use the format `<<NB:{nonce}:{field}>>` where `nonce` is a
/// random 16-hex-char token generated per pipeline run, making placeholders
/// unforgeable by external processes.
pub struct SecureContext {
    nonce: String,
    secrets: HashMap<String, Value>,
}

impl SecureContext {
    /// Extract sensitive fields from `data`, replacing them with placeholders.
    ///
    /// `data` is mutated in place — sensitive values are swapped for placeholder
    /// strings. The original values are stored in the returned `SecureContext`.
    ///
    /// Returns `None` if `sensitive_fields` is empty or none of the fields
    /// exist in `data`.
    pub fn extract(data: &mut Value, sensitive_fields: &[String]) -> Option<Self> {
        if sensitive_fields.is_empty() {
            return None;
        }

        let nonce = random_nonce();
        let mut secrets = HashMap::new();

        if let Value::Object(map) = data {
            for field in sensitive_fields {
                if let Some(real_value) = map.remove(field.as_str()) {
                    let placeholder = format!("<<NB:{nonce}:{field}>>");
                    secrets.insert(field.clone(), real_value);
                    map.insert(field.clone(), Value::String(placeholder));
                }
            }
        }

        if secrets.is_empty() {
            None
        } else {
            Some(Self { nonce, secrets })
        }
    }

    /// Deep-walk a `Value` tree, replacing placeholders with real values.
    ///
    /// - Exact match: a string that is exactly `<<NB:nonce:field>>` is replaced
    ///   with the original `Value` (preserving type — numbers, bools, objects).
    /// - Substring match: a string containing one or more placeholders gets
    ///   string replacement (result is always a string).
    pub fn resolve(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => self.resolve_string(s),
            Value::Array(arr) => Value::Array(arr.iter().map(|v| self.resolve(v)).collect()),
            Value::Object(map) => {
                let resolved: serde_json::Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect();
                Value::Object(resolved)
            }
            other => other.clone(),
        }
    }

    /// Deep-walk a `Value` tree, replacing any occurrence of a secret value
    /// with `[REDACTED]`. Used to scrub pipeline results (e.g. HTTP response
    /// bodies that echo back secrets) before returning to the agent.
    pub fn redact(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => self.redact_string(s),
            Value::Array(arr) => Value::Array(arr.iter().map(|v| self.redact(v)).collect()),
            Value::Object(map) => {
                let scrubbed: serde_json::Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.redact(v)))
                    .collect();
                Value::Object(scrubbed)
            }
            other => {
                // Check non-string values for exact match (e.g. secret number 42)
                for (_, real_value) in &self.secrets {
                    if other == real_value {
                        return Value::String(REDACTED.to_string());
                    }
                }
                other.clone()
            }
        }
    }

    /// Returns the placeholder string for a field, if it's in this context.
    pub fn placeholder_for(&self, field: &str) -> Option<String> {
        if self.secrets.contains_key(field) {
            Some(format!("<<NB:{}:{field}>>", self.nonce))
        } else {
            None
        }
    }

    fn redact_string(&self, s: &str) -> Value {
        let mut result = s.to_string();
        for (_, real_value) in &self.secrets {
            let needle = match real_value {
                Value::String(secret) if !secret.is_empty() => secret.clone(),
                _ => continue,
            };
            if result.contains(&needle) {
                result = result.replace(&needle, REDACTED);
            }
        }
        Value::String(result)
    }

    fn resolve_string(&self, s: &str) -> Value {
        // Check for exact match first (preserves original Value type)
        for (field, real_value) in &self.secrets {
            let placeholder = format!("<<NB:{}:{field}>>", self.nonce);
            if s == placeholder {
                return real_value.clone();
            }
        }

        // Substring replacement (always returns String)
        let mut result = s.to_string();
        for (field, real_value) in &self.secrets {
            let placeholder = format!("<<NB:{}:{field}>>", self.nonce);
            if result.contains(&placeholder) {
                let replacement = match real_value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        Value::String(result)
    }
}

/// Generate a 16-hex-char random nonce.
fn random_nonce() -> String {
    let mut buf = [0u8; 8];
    getrandom::fill(&mut buf).expect("getrandom failed");
    hex_encode(&buf)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_replaces_sensitive_fields_with_placeholders() {
        let mut data = json!({
            "username": "alice",
            "password": "s3cret",
            "api_key": "key123"
        });

        let ctx =
            SecureContext::extract(&mut data, &["password".into(), "api_key".into()]).unwrap();

        // Data now has placeholders
        assert_eq!(data["username"], "alice");
        assert!(data["password"].as_str().unwrap().starts_with("<<NB:"));
        assert!(data["api_key"].as_str().unwrap().starts_with("<<NB:"));

        // Secrets are stored
        assert_eq!(ctx.secrets["password"], "s3cret");
        assert_eq!(ctx.secrets["api_key"], "key123");
    }

    #[test]
    fn resolve_restores_exact_match_preserving_type() {
        let mut data = json!({
            "count": 42,
            "active": true,
            "password": "s3cret"
        });

        let ctx = SecureContext::extract(
            &mut data,
            &["count".into(), "active".into(), "password".into()],
        )
        .unwrap();

        // Resolve each placeholder — types should be preserved
        let resolved = ctx.resolve(&data);
        assert_eq!(resolved["count"], 42);
        assert_eq!(resolved["active"], true);
        assert_eq!(resolved["password"], "s3cret");
    }

    #[test]
    fn resolve_handles_substring_replacement() {
        let mut data = json!({"token": "bearer-xyz"});
        let ctx = SecureContext::extract(&mut data, &["token".into()]).unwrap();

        let placeholder = ctx.placeholder_for("token").unwrap();
        let template = Value::String(format!("Authorization: Bearer {placeholder}"));

        let resolved = ctx.resolve(&template);
        assert_eq!(resolved, "Authorization: Bearer bearer-xyz");
    }

    #[test]
    fn resolve_walks_nested_structures() {
        let mut data = json!({"secret": "abc"});
        let ctx = SecureContext::extract(&mut data, &["secret".into()]).unwrap();

        let placeholder = ctx.placeholder_for("secret").unwrap();
        let nested = json!({
            "headers": {"auth": placeholder},
            "list": [placeholder, "plain"]
        });

        let resolved = ctx.resolve(&nested);
        assert_eq!(resolved["headers"]["auth"], "abc");
        assert_eq!(resolved["list"][0], "abc");
        assert_eq!(resolved["list"][1], "plain");
    }

    #[test]
    fn nonce_is_random_across_instances() {
        let mut d1 = json!({"x": "a"});
        let mut d2 = json!({"x": "a"});
        let c1 = SecureContext::extract(&mut d1, &["x".into()]).unwrap();
        let c2 = SecureContext::extract(&mut d2, &["x".into()]).unwrap();

        assert_ne!(c1.nonce, c2.nonce);
        // Placeholders differ
        assert_ne!(d1["x"], d2["x"]);
    }

    #[test]
    fn extract_returns_none_for_empty_fields() {
        let mut data = json!({"name": "alice"});
        assert!(SecureContext::extract(&mut data, &[]).is_none());
    }

    #[test]
    fn extract_returns_none_when_no_fields_match() {
        let mut data = json!({"name": "alice"});
        assert!(SecureContext::extract(&mut data, &["nonexistent".into()]).is_none());
    }

    #[test]
    fn placeholder_for_returns_none_for_unknown_field() {
        let mut data = json!({"password": "s3cret"});
        let ctx = SecureContext::extract(&mut data, &["password".into()]).unwrap();
        assert!(ctx.placeholder_for("unknown").is_none());
    }

    #[test]
    fn redact_scrubs_secret_values_from_output() {
        let mut data = json!({"api_key": "key-abc-123", "token": "tok-xyz"});
        let ctx =
            SecureContext::extract(&mut data, &["api_key".into(), "token".into()]).unwrap();

        // Simulate an API response that echoes back the secrets
        let api_response = json!({
            "status": 200,
            "body": {
                "received_key": "key-abc-123",
                "auth": "Bearer tok-xyz",
                "message": "Logged in with key-abc-123"
            }
        });

        let scrubbed = ctx.redact(&api_response);
        assert_eq!(scrubbed["status"], 200);
        assert_eq!(scrubbed["body"]["received_key"], "[REDACTED]");
        assert_eq!(scrubbed["body"]["auth"], "Bearer [REDACTED]");
        assert_eq!(scrubbed["body"]["message"], "Logged in with [REDACTED]");
    }

    #[test]
    fn redact_handles_nested_arrays() {
        let mut data = json!({"secret": "xyz"});
        let ctx = SecureContext::extract(&mut data, &["secret".into()]).unwrap();

        let output = json!({"list": ["safe", "xyz", "also xyz here"]});
        let scrubbed = ctx.redact(&output);
        assert_eq!(scrubbed["list"][0], "safe");
        assert_eq!(scrubbed["list"][1], "[REDACTED]");
        assert_eq!(scrubbed["list"][2], "also [REDACTED] here");
    }

    #[test]
    fn redact_ignores_non_secret_values() {
        let mut data = json!({"password": "hunter2"});
        let ctx = SecureContext::extract(&mut data, &["password".into()]).unwrap();

        let output = json!({"user": "alice", "status": "ok"});
        let scrubbed = ctx.redact(&output);
        assert_eq!(scrubbed, output); // unchanged
    }
}
