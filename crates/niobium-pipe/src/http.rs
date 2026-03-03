//! HttpStage — executes a single HTTP request with template interpolation.

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::debug;

use crate::event::{PipeEvent, Severity};
use crate::registry::StageBuilder;
use crate::stage::Stage;
use crate::template;

/// Configuration for an HTTP stage, deserialized from an `x-sink` or `x-pipe` element.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpStageConfig {
    /// Unique name for this stage.
    pub name: String,

    /// Target URL (supports `${...}` template references).
    pub url: String,

    /// HTTP method (default: POST).
    #[serde(default = "default_method")]
    pub method: String,

    /// Request headers (values support `${...}` templates).
    pub headers: Option<Value>,

    /// Request body (supports `${...}` templates throughout).
    pub body: Option<Value>,

    /// Expected response status code — abort if mismatched.
    pub expect: Option<ExpectConfig>,

    /// Toast message on success.
    pub on_success: Option<ToastConfig>,

    /// Toast message on error.
    pub on_error: Option<ToastConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpectConfig {
    pub status: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToastConfig {
    pub toast: String,
}

fn default_method() -> String {
    "POST".to_string()
}

pub struct HttpStage {
    config: HttpStageConfig,
    client: reqwest::Client,
}

impl HttpStage {
    pub fn new(config: HttpStageConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(config: HttpStageConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }
}

#[async_trait]
impl Stage for HttpStage {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn execute(
        &self,
        input: Value,
        events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        // Render URL
        let url = template::render_string(&self.config.url, &input)?;
        debug!(stage = self.config.name, %url, "executing HTTP stage");

        // Build request
        let method: reqwest::Method = self
            .config
            .method
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid HTTP method: {}", self.config.method))?;
        let mut req = self.client.request(method, &url);

        // Render and apply headers
        if let Some(ref headers_template) = self.config.headers {
            let rendered = template::render_value(headers_template, &input)?;
            if let Value::Object(map) = rendered {
                for (k, v) in map {
                    if let Value::String(val) = v {
                        req = req.header(&k, &val);
                    }
                }
            }
        }

        // Render and set body
        if let Some(ref body_template) = self.config.body {
            let rendered = template::render_value(body_template, &input)?;
            req = req.json(&rendered);
        }

        // Send request
        let response = req.send().await?;
        let status = response.status().as_u16();

        // Check expected status
        if let Some(ref expect) = self.config.expect
            && status != expect.status
        {
            let error_msg = format!(
                "stage '{}': expected status {}, got {status}",
                self.config.name, expect.status
            );
            if let Some(ref on_error) = self.config.on_error {
                let _ = events.send(PipeEvent::Toast {
                    message: on_error.toast.clone(),
                    severity: Severity::Error,
                });
            }
            bail!(error_msg);
        }

        // Parse response body
        let body: Value = response.json().await.unwrap_or(Value::Null);

        // Emit success toast
        if let Some(ref on_success) = self.config.on_success {
            let _ = events.send(PipeEvent::Toast {
                message: on_success.toast.clone(),
                severity: Severity::Success,
            });
        }

        Ok(serde_json::json!({
            "status": status,
            "body": body,
        }))
    }
}

/// Builder for HTTP stages.
pub struct HttpStageBuilder;

impl StageBuilder for HttpStageBuilder {
    fn stage_type(&self) -> &str {
        "http"
    }

    fn build(&self, def: &Value) -> Result<Box<dyn Stage>> {
        let config: HttpStageConfig = serde_json::from_value(def.clone())?;
        Ok(Box::new(HttpStage::new(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_events() -> (
        mpsc::UnboundedSender<PipeEvent>,
        mpsc::UnboundedReceiver<PipeEvent>,
    ) {
        mpsc::unbounded_channel()
    }

    #[tokio::test]
    async fn success_post() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/users"))
            .and(body_json(json!({"name": "alice"})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 1})))
            .mount(&server)
            .await;

        let config = HttpStageConfig {
            name: "create_user".into(),
            url: format!("{}/api/users", server.uri()),
            method: "POST".into(),
            headers: None,
            body: Some(json!({"name": "${name}"})),
            expect: Some(ExpectConfig { status: 201 }),
            on_success: Some(ToastConfig {
                toast: "User created!".into(),
            }),
            on_error: None,
        };

        let stage = HttpStage::new(config);
        let (tx, mut rx) = make_events();
        let input = json!({"name": "alice"});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["status"], 201);
        assert_eq!(result["body"]["id"], 1);

        let event = rx.try_recv().unwrap();
        assert!(matches!(
            event,
            PipeEvent::Toast {
                severity: Severity::Success,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn expect_mismatch() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/fail"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error": "boom"})))
            .mount(&server)
            .await;

        let config = HttpStageConfig {
            name: "will_fail".into(),
            url: format!("{}/api/fail", server.uri()),
            method: "POST".into(),
            headers: None,
            body: None,
            expect: Some(ExpectConfig { status: 200 }),
            on_success: None,
            on_error: Some(ToastConfig {
                toast: "Request failed!".into(),
            }),
        };

        let stage = HttpStage::new(config);
        let (tx, mut rx) = make_events();

        let err = stage.execute(json!({}), &tx).await.unwrap_err();
        assert!(err.to_string().contains("expected status 200, got 500"));

        let event = rx.try_recv().unwrap();
        assert!(matches!(
            event,
            PipeEvent::Toast {
                severity: Severity::Error,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn template_in_url_and_headers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/42"))
            .and(header("Authorization", "Bearer tok123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name": "bob"})))
            .mount(&server)
            .await;

        let config = HttpStageConfig {
            name: "get_user".into(),
            url: format!("{}/api/users/${{user_id}}", server.uri()),
            method: "GET".into(),
            headers: Some(json!({"Authorization": "Bearer ${token}"})),
            body: None,
            expect: None,
            on_success: None,
            on_error: None,
        };

        let stage = HttpStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({"user_id": "42", "token": "tok123"});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["status"], 200);
        assert_eq!(result["body"]["name"], "bob");
    }
}
