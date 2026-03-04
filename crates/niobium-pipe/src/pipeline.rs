//! Pipeline runner — orchestrates a sequence of stages.

use std::sync::Arc;

use anyhow::{Result, bail};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::info;

use crate::event::PipeEvent;
use crate::parallel::ParallelStage;
use crate::redact::RedactStage;
use crate::registry::StageRegistry;
use crate::secure_context::SecureContext;
use crate::stage::Stage;

/// A pipeline of stages executed in sequence.
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
    trusted: Vec<bool>,
    sensitive_fields: Vec<String>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.stages.iter().map(|s| s.name()).collect();
        f.debug_struct("Pipeline")
            .field("stages", &names)
            .field("sensitive_fields", &self.sensitive_fields)
            .finish()
    }
}

impl Pipeline {
    pub fn new(
        stages: Vec<Box<dyn Stage>>,
        trusted: Vec<bool>,
        sensitive_fields: Vec<String>,
    ) -> Self {
        Self {
            stages,
            trusted,
            sensitive_fields,
        }
    }

    /// Run all stages in order, accumulating results under `context["pipe"][stage_name]`.
    ///
    /// If sensitive fields are configured, a [`SecureContext`] is created to
    /// protect secrets. Trusted stages receive resolved data (real values),
    /// while untrusted stages only see placeholders.
    pub async fn run(
        &self,
        mut initial_input: Value,
        events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        // Extract sensitive values into SecureContext (if any)
        let secure_ctx = SecureContext::extract(&mut initial_input, &self.sensitive_fields);

        let mut context = initial_input;
        let mut results = serde_json::Map::new();

        for (i, stage) in self.stages.iter().enumerate() {
            let name = stage.name().to_string();
            let trusted = self.trusted.get(i).copied().unwrap_or(true);
            info!(stage = %name, trusted, "starting pipeline stage");

            let _ = events.send(PipeEvent::StageStarted { name: name.clone() });

            // Trusted stages get resolved context (real secrets).
            // Untrusted stages get context as-is (with placeholders).
            let stage_input = match (&secure_ctx, trusted) {
                (Some(ctx), true) => ctx.resolve(&context),
                _ => context.clone(),
            };

            match stage.execute(stage_input, events).await {
                Ok(output) => {
                    let _ = events.send(PipeEvent::StageCompleted { name: name.clone() });

                    // Store result for inter-stage references: ${pipe.<name>.<path>}
                    results.insert(name.clone(), output.clone());

                    // Update context so next stage can reference previous results
                    if let Value::Object(ref mut ctx) = context {
                        let pipe = ctx
                            .entry("pipe")
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                        if let Value::Object(pipe_map) = pipe {
                            pipe_map.insert(name, output);
                        }
                    }
                }
                Err(err) => {
                    let error_msg = err.to_string();
                    let _ = events.send(PipeEvent::StageFailed {
                        name: name.clone(),
                        error: error_msg.clone(),
                    });
                    bail!("pipeline failed at stage '{name}': {error_msg}");
                }
            }
        }

        let mut output = Value::Object(results);

        // Scrub any secret values that leaked into stage outputs
        // (e.g. an API echoing back credentials in its response body)
        if let Some(ref ctx) = secure_ctx {
            output = ctx.redact(&output);
        }

        Ok(output)
    }
}

/// Build a pipeline from an `x-sink` (single object) or `x-pipe` (array) definition.
///
/// Uses the provided [`StageRegistry`] to look up builders by type.
/// Backwards-compatible: `x-sink` (object with `url`) still works as before.
///
/// `sensitive_fields` lists schema fields marked `x-sensitive: true`. When
/// non-empty, the pipeline will create a [`SecureContext`] at runtime to
/// protect these values from untrusted stages.
pub fn build_pipeline(
    sink_def: &Value,
    registry: &StageRegistry,
    sensitive_fields: Vec<String>,
) -> Result<Pipeline> {
    match sink_def {
        // Single x-sink: build via registry + optional RedactStage
        Value::Object(_) => {
            let built = registry.build_stage(sink_def)?;
            let mut stages: Vec<Box<dyn Stage>> = vec![built.stage];
            let mut trusted: Vec<bool> = vec![built.trusted];

            // Check for response.redact fields (x-sink shorthand for redaction)
            if let Some(redact_fields) = sink_def
                .get("response")
                .and_then(|r| r.get("redact"))
                .and_then(|r| r.as_array())
            {
                let fields: Vec<String> = redact_fields
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !fields.is_empty() {
                    stages.push(Box::new(RedactStage::new("redact".into(), fields)));
                    trusted.push(true); // RedactStage is built-in, always trusted
                }
            }

            Ok(Pipeline::new(stages, trusted, sensitive_fields))
        }

        // x-pipe: array of typed stage definitions
        Value::Array(arr) => {
            let mut stages: Vec<Box<dyn Stage>> = Vec::with_capacity(arr.len());
            let mut trusted: Vec<bool> = Vec::with_capacity(arr.len());
            for (i, stage_def) in arr.iter().enumerate() {
                if is_parallel_stage(stage_def) {
                    let parallel = build_parallel_stage(stage_def, registry, &sensitive_fields)
                        .map_err(|e| anyhow::anyhow!("invalid parallel stage at index {i}: {e}"))?;
                    stages.push(Box::new(parallel));
                    trusted.push(true); // structural stage, always trusted
                } else {
                    let built = registry
                        .build_stage(stage_def)
                        .map_err(|e| anyhow::anyhow!("invalid stage config at index {i}: {e}"))?;
                    stages.push(built.stage);
                    trusted.push(built.trusted);
                }
            }
            Ok(Pipeline::new(stages, trusted, sensitive_fields))
        }

        other => bail!("x-sink/x-pipe must be an object or array, got {}", other),
    }
}

/// Check whether a stage definition describes a parallel stage.
fn is_parallel_stage(def: &Value) -> bool {
    def.get("type").and_then(|v| v.as_str()) == Some("parallel") || def.get("branches").is_some()
}

/// Build a [`ParallelStage`] from a JSON definition.
///
/// Validates:
/// - `name` field is required
/// - `branches` must be a non-empty object
/// - Branch names must not contain dots (breaks template path resolution)
/// - Each branch value must be an array of stage definitions
fn build_parallel_stage(
    def: &Value,
    registry: &StageRegistry,
    sensitive_fields: &[String],
) -> Result<ParallelStage> {
    let name = def
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("parallel stage requires a 'name' field"))?
        .to_string();

    let branches_obj = def
        .get("branches")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("parallel stage requires a 'branches' object"))?;

    if branches_obj.is_empty() {
        bail!("parallel stage 'branches' must not be empty");
    }

    let mut branches = Vec::with_capacity(branches_obj.len());

    for (branch_name, branch_def) in branches_obj {
        if branch_name.contains('.') {
            bail!(
                "branch name '{branch_name}' must not contain dots (breaks template path resolution)"
            );
        }

        let branch_stages = branch_def
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("branch '{branch_name}' must be an array of stages"))?;

        let pipeline = build_pipeline(
            &Value::Array(branch_stages.clone()),
            registry,
            sensitive_fields.to_vec(),
        )?;

        branches.push((branch_name.clone(), Arc::new(pipeline)));
    }

    Ok(ParallelStage::new(name, branches))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::default_registry;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn single_stage_pipeline() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/data"))
            .and(body_json(json!({"value": "test"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;

        let sink = json!({
            "name": "send_data",
            "url": format!("{}/api/data", server.uri()),
            "body": {"value": "${value}"}
        });

        let registry = default_registry();
        let pipeline = build_pipeline(&sink, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline.run(json!({"value": "test"}), &tx).await.unwrap();

        assert_eq!(result["send_data"]["status"], 200);
        assert_eq!(result["send_data"]["body"]["ok"], true);
    }

    #[tokio::test]
    async fn multi_stage_with_inter_stage_refs() {
        let server = MockServer::start().await;

        // Stage 1: authenticate → returns token
        Mock::given(method("POST"))
            .and(path("/auth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "abc123"})))
            .mount(&server)
            .await;

        // Stage 2: use token from stage 1 (body contains the token)
        Mock::given(method("POST"))
            .and(path("/api/submit"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 42})))
            .mount(&server)
            .await;

        let pipe = json!([
            {
                "name": "auth",
                "url": format!("{}/auth", server.uri()),
                "body": {"user": "${username}"}
            },
            {
                "name": "submit",
                "url": format!("{}/api/submit", server.uri()),
                "body": {"token": "${pipe.auth.body.token}", "data": "${payload}"}
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"username": "alice", "payload": "important"});
        let result = pipeline.run(input, &tx).await.unwrap();

        assert_eq!(result["auth"]["status"], 200);
        assert_eq!(result["submit"]["status"], 201);
    }

    #[tokio::test]
    async fn early_abort_on_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fail"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error": "boom"})))
            .mount(&server)
            .await;

        let sink = json!({
            "name": "will_fail",
            "url": format!("{}/fail", server.uri()),
            "expect": {"status": 200}
        });

        let registry = default_registry();
        let pipeline = build_pipeline(&sink, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let err = pipeline.run(json!({}), &tx).await.unwrap_err();
        assert!(err.to_string().contains("will_fail"));
    }

    #[tokio::test]
    async fn event_emission_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ok"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;

        let sink = json!({
            "name": "test_stage",
            "url": format!("{}/ok", server.uri()),
        });

        let registry = default_registry();
        let pipeline = build_pipeline(&sink, &registry, vec![]).unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        pipeline.run(json!({}), &tx).await.unwrap();

        let e1 = rx.try_recv().unwrap();
        assert!(matches!(e1, PipeEvent::StageStarted { ref name } if name == "test_stage"));

        let e2 = rx.try_recv().unwrap();
        assert!(matches!(e2, PipeEvent::StageCompleted { ref name } if name == "test_stage"));
    }

    #[tokio::test]
    async fn mixed_stage_types_in_pipe() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "secret"})))
            .mount(&server)
            .await;

        let pipe = json!([
            {
                "type": "http",
                "name": "login",
                "url": format!("{}/api/login", server.uri()),
                "body": {"user": "${username}"}
            },
            {
                "type": "transform",
                "name": "extract",
                "expr": {"token": "$.pipe.login.body.token"}
            },
            {
                "type": "toast",
                "name": "notify",
                "message": "Logged in!",
                "severity": "success"
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let input = json!({"username": "alice"});
        let result = pipeline.run(input, &tx).await.unwrap();

        // HTTP stage result
        assert_eq!(result["login"]["status"], 200);

        // Transform extracted the token
        assert_eq!(result["extract"]["token"], "secret");

        // Toast passes through the context (its input, which includes pipe results)
        assert!(result.get("notify").is_some());

        // Collect all events
        let mut events = vec![];
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        // Should have: StageStarted/Completed for each stage + Toast event
        let toast_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PipeEvent::Toast { .. }))
            .collect();
        assert_eq!(toast_events.len(), 1);
    }

    #[tokio::test]
    async fn secure_context_protects_process_stage() {
        // Pipeline: process (untrusted, sees placeholders) that echoes stdin via cat
        // The echoed output should contain placeholders, NOT real secrets
        let pipe = json!([
            {
                "type": "process",
                "name": "echo",
                "command": "cat",
                "timeout": 5
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec!["password".into()]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"username": "alice", "password": "s3cret"});
        let result = pipeline.run(input, &tx).await.unwrap();

        // The process saw placeholders, so the echoed output has the placeholder
        let echoed = &result["echo"];
        assert_eq!(echoed["username"], "alice");

        // Password should be a placeholder, not the real value
        let pw = echoed["password"].as_str().unwrap();
        assert!(pw.starts_with("<<NB:"), "expected placeholder, got: {pw}");
        assert!(pw.contains("password"));
        assert!(!pw.contains("s3cret"), "real secret leaked to process!");
    }

    #[tokio::test]
    async fn secure_context_resolves_for_trusted_http_stage() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/data"))
            .and(body_json(
                json!({"username": "alice", "password": "s3cret"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;

        let sink = json!({
            "name": "send",
            "url": format!("{}/api/data", server.uri()),
            "body": {"username": "${username}", "password": "${password}"}
        });

        let registry = default_registry();
        let pipeline = build_pipeline(&sink, &registry, vec!["password".into()]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"username": "alice", "password": "s3cret"});
        let result = pipeline.run(input, &tx).await.unwrap();

        // HTTP stage (trusted) received real values — mock matched the exact body
        assert_eq!(result["send"]["status"], 200);
        assert_eq!(result["send"]["body"]["ok"], true);
    }

    #[tokio::test]
    async fn no_sensitive_fields_means_no_protection() {
        // With empty sensitive_fields, process stage sees everything
        let pipe = json!([
            {
                "type": "process",
                "name": "echo",
                "command": "cat",
                "timeout": 5
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let input = json!({"password": "s3cret"});
        let result = pipeline.run(input, &tx).await.unwrap();

        // No SecureContext → process sees real value
        assert_eq!(result["echo"]["password"], "s3cret");
    }

    // ── Declarative secure pipeline tests ────────────────────────────────

    /// Describes a field expectation in a stage's output.
    enum Expect {
        /// Field value must equal this exact string/value.
        Exact(Value),
        /// Field value must be a placeholder (starts with `<<NB:`) or `[REDACTED]`.
        Placeholder,
        /// Field must not exist in the output.
        Absent,
        /// Field value must be `[REDACTED]` (secret leaked into output, then scrubbed).
        Redacted,
    }

    /// A declarative description of one stage's expected output.
    struct StageExpect {
        stage_name: &'static str,
        fields: Vec<(&'static str, Expect)>,
    }

    /// A full declarative test case for SecureContext pipeline behavior.
    struct SecureTestCase {
        name: &'static str,
        /// JSON input data (form submission).
        input: Value,
        /// Fields to protect (passed to build_pipeline).
        sensitive: Vec<&'static str>,
        /// Pipeline stage definitions.
        pipeline: Value,
        /// Expected output per stage.
        expects: Vec<StageExpect>,
    }

    /// Run a declarative test case and assert all expectations.
    async fn run_secure_test(case: SecureTestCase) {
        let registry = default_registry();
        let sensitive: Vec<String> = case.sensitive.iter().map(|s| s.to_string()).collect();
        let pipeline = build_pipeline(&case.pipeline, &registry, sensitive)
            .unwrap_or_else(|e| panic!("[{}] build failed: {e}", case.name));

        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline
            .run(case.input, &tx)
            .await
            .unwrap_or_else(|e| panic!("[{}] run failed: {e}", case.name));

        for stage_expect in &case.expects {
            let stage_output = &result[stage_expect.stage_name];
            for (field, expect) in &stage_expect.fields {
                match expect {
                    Expect::Exact(expected) => {
                        assert_eq!(
                            &stage_output[field], expected,
                            "[{}] {}.{field} mismatch",
                            case.name, stage_expect.stage_name,
                        );
                    }
                    Expect::Placeholder => {
                        let val = stage_output[field].as_str().unwrap_or_else(|| {
                            panic!(
                                "[{}] {}.{field} not a string: {}",
                                case.name, stage_expect.stage_name, stage_output[field]
                            )
                        });
                        assert!(
                            val.starts_with("<<NB:") && val.ends_with(">>"),
                            "[{}] {}.{field} expected placeholder, got: {val}",
                            case.name,
                            stage_expect.stage_name,
                        );
                        assert!(
                            !val.contains("s3cret") && !val.contains("key999"),
                            "[{}] {}.{field} placeholder contains real secret: {val}",
                            case.name,
                            stage_expect.stage_name,
                        );
                    }
                    Expect::Redacted => {
                        assert_eq!(
                            stage_output[field], "[REDACTED]",
                            "[{}] {}.{field} expected [REDACTED], got: {}",
                            case.name, stage_expect.stage_name, stage_output[field],
                        );
                    }
                    Expect::Absent => {
                        assert!(
                            stage_output.get(field).is_none() || stage_output[field].is_null(),
                            "[{}] {}.{field} should be absent, got: {}",
                            case.name,
                            stage_expect.stage_name,
                            stage_output[field],
                        );
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn secure_pipeline_declarative_cases() {
        // Case 1: Process (untrusted) sees only placeholders
        run_secure_test(SecureTestCase {
            name: "process_sees_placeholders",
            input: json!({"user": "alice", "password": "s3cret", "api_key": "key999"}),
            sensitive: vec!["password", "api_key"],
            pipeline: json!([{
                "type": "process",
                "name": "echo",
                "command": "cat",
                "timeout": 5
            }]),
            expects: vec![StageExpect {
                stage_name: "echo",
                fields: vec![
                    ("user", Expect::Exact(json!("alice"))),
                    ("password", Expect::Placeholder),
                    ("api_key", Expect::Placeholder),
                ],
            }],
        })
        .await;

        // Case 2: Transform (trusted) processes real values internally,
        // but the output returned to the agent has secrets scrubbed.
        run_secure_test(SecureTestCase {
            name: "transform_output_scrubbed",
            input: json!({"token": "s3cret"}),
            sensitive: vec!["token"],
            pipeline: json!([{
                "type": "transform",
                "name": "extract",
                "expr": {"got_token": "$.token"}
            }]),
            expects: vec![StageExpect {
                stage_name: "extract",
                fields: vec![("got_token", Expect::Redacted)],
            }],
        })
        .await;

        // Case 3: No sensitive fields — process sees everything
        run_secure_test(SecureTestCase {
            name: "no_sensitive_no_protection",
            input: json!({"password": "s3cret"}),
            sensitive: vec![],
            pipeline: json!([{
                "type": "process",
                "name": "echo",
                "command": "cat",
                "timeout": 5
            }]),
            expects: vec![StageExpect {
                stage_name: "echo",
                fields: vec![("password", Expect::Exact(json!("s3cret")))],
            }],
        })
        .await;

        // Case 4: Mixed pipeline — http (trusted) then process (untrusted)
        // Process should see placeholders in the original fields,
        // but can see http results (those aren't sensitive).
        run_secure_test(SecureTestCase {
            name: "mixed_trusted_untrusted",
            input: json!({"user": "alice", "password": "s3cret"}),
            sensitive: vec!["password"],
            pipeline: json!([
                {
                    "type": "transform",
                    "name": "prep",
                    "expr": {"greeting": "$.user"}
                },
                {
                    "type": "process",
                    "name": "echo",
                    "command": "cat",
                    "timeout": 5
                }
            ]),
            expects: vec![
                StageExpect {
                    stage_name: "prep",
                    fields: vec![
                        // Transform (trusted) can read real user value
                        ("greeting", Expect::Exact(json!("alice"))),
                    ],
                },
                StageExpect {
                    stage_name: "echo",
                    fields: vec![
                        // Process (untrusted) sees non-sensitive fields fine
                        ("user", Expect::Exact(json!("alice"))),
                        // But sensitive field is a placeholder
                        ("password", Expect::Placeholder),
                    ],
                },
            ],
        })
        .await;

        // Case 5: Sensitive field not present in input — no-op
        run_secure_test(SecureTestCase {
            name: "missing_sensitive_field_noop",
            input: json!({"user": "alice"}),
            sensitive: vec!["password"],
            pipeline: json!([{
                "type": "process",
                "name": "echo",
                "command": "cat",
                "timeout": 5
            }]),
            expects: vec![StageExpect {
                stage_name: "echo",
                fields: vec![
                    ("user", Expect::Exact(json!("alice"))),
                    ("password", Expect::Absent),
                ],
            }],
        })
        .await;
    }

    // ── Realistic agent scenario ─────────────────────────────────────────
    //
    // Simulates the full server.rs flow when an agent calls show_form with:
    //
    //   show_form({
    //     title: "Deploy to Production",
    //     schema: { ... with x-sensitive annotations ... },
    //     x-pipe: [
    //       http  → POST /api/auth  (trusted: receives real password)
    //       transform → extract token from auth response
    //       process → call external deploy CLI (untrusted: sees placeholder)
    //     ]
    //   })
    //
    // The test verifies the full chain:
    //   1. extract_sensitive_fields reads the schema
    //   2. build_pipeline wires trust flags from the registry
    //   3. pipeline.run creates SecureContext, resolves per trust level
    //   4. redact_sensitive strips secrets from the agent-facing response

    #[tokio::test]
    async fn agent_deploy_scenario() {
        use crate::redact::{extract_sensitive_fields, redact_sensitive};

        // ── Schema: what the agent declared ──────────────────────────────
        // All fields sensitive by default. Only "app" is marked public.
        let schema = json!({
            "type": "object",
            "properties": {
                "app":      { "type": "string", "title": "Application",    "x-sensitive": false },
                "username": { "type": "string", "title": "Deploy User" },
                "password": { "type": "string", "title": "Deploy Password" }
            },
            "required": ["app", "username", "password"]
        });

        // ── Form data: what the user submitted ──────────────────────────
        let form_data = json!({
            "app": "web-frontend",
            "username": "deployer",
            "password": "hunter2"
        });

        // ── Mock API server ─────────────────────────────────────────────
        let server = MockServer::start().await;

        // POST /api/auth — expects real credentials (trusted HTTP stage)
        Mock::given(method("POST"))
            .and(path("/api/auth"))
            .and(body_json(json!({
                "username": "deployer",
                "password": "hunter2"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "token": "jwt-abc-123", "expires": 3600 })),
            )
            .expect(1)
            .mount(&server)
            .await;

        // ── Pipeline: what the agent declared as x-pipe ─────────────────
        let pipe_def = json!([
            {
                "name": "auth",
                "url": format!("{}/api/auth", server.uri()),
                "method": "POST",
                "body": {
                    "username": "${username}",
                    "password": "${password}"
                },
                "expect": { "status": 200 }
            },
            {
                "name": "extract_token",
                "type": "transform",
                "expr": {
                    "token": "$.pipe.auth.body.token"
                }
            },
            {
                "name": "deploy_cli",
                "type": "process",
                "command": "cat",
                "timeout": 5
            }
        ]);

        // ── Execute: mirror server.rs flow ──────────────────────────────

        // Step 1: extract sensitive fields from schema (as server.rs does)
        let sensitive_fields = extract_sensitive_fields(&schema);
        // "app" opted out; "username" and "password" are sensitive by default
        assert!(sensitive_fields.contains(&"username".to_string()));
        assert!(sensitive_fields.contains(&"password".to_string()));
        assert!(!sensitive_fields.contains(&"app".to_string()));

        // Step 2: build pipeline with trust flags
        let registry = default_registry();
        let pipeline = build_pipeline(&pipe_def, &registry, sensitive_fields).unwrap();

        // Step 3: run pipeline
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline.run(form_data.clone(), &tx).await.unwrap();

        // ── Verify: auth stage (HTTP, trusted) received real credentials ─
        // The mock's body_json matcher enforces the exact body — if the HTTP
        // stage had sent placeholders instead of real values, the mock would
        // have returned a 404 and the expect:200 would have failed.
        assert_eq!(result["auth"]["status"], 200);
        assert_eq!(result["auth"]["body"]["token"], "jwt-abc-123");

        // ── Verify: transform (trusted) extracted the real token ─────────
        assert_eq!(result["extract_token"]["token"], "jwt-abc-123");

        // ── Verify: process (untrusted) never saw real secrets ───────────
        let deploy_output = &result["deploy_cli"];

        // "app" is x-sensitive:false → process sees it in the clear
        assert_eq!(deploy_output["app"], "web-frontend");

        // "username" and "password" are sensitive → process only sees placeholders
        let username_val = deploy_output["username"].as_str().unwrap();
        assert!(
            username_val.starts_with("<<NB:") && username_val.ends_with(">>"),
            "process saw real username: {username_val}"
        );
        assert!(!username_val.contains("deployer"));

        let password_val = deploy_output["password"].as_str().unwrap();
        assert!(
            password_val.starts_with("<<NB:") && password_val.ends_with(">>"),
            "process saw real password: {password_val}"
        );
        assert!(!password_val.contains("hunter2"));

        // Process CAN see inter-stage results (they're in pipe.* context, not sensitive)
        assert!(
            deploy_output.get("pipe").is_some(),
            "process should see pipe context"
        );

        // ── Verify: redaction for agent response ─────────────────────────
        // server.rs calls redact_sensitive on the form data before returning
        let mut agent_data = form_data;
        redact_sensitive(&schema, &mut agent_data);

        // Agent sees "app" (x-sensitive:false) but not the rest
        assert_eq!(agent_data["app"], "web-frontend");
        assert_eq!(agent_data["username"], "[REDACTED]");
        assert_eq!(agent_data["password"], "[REDACTED]");
    }
}
