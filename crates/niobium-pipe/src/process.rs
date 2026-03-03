//! ProcessStage — spawns an external process, pipes JSON via stdin/stdout.

use std::collections::HashMap;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::debug;

use crate::event::PipeEvent;
use crate::registry::StageBuilder;
use crate::stage::Stage;
use crate::template;

/// Configuration for a process stage.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessStageConfig {
    /// Unique name for this stage.
    pub name: String,

    /// The command to execute (e.g., "python3").
    pub command: String,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Timeout in seconds (default: 30).
    pub timeout: Option<u64>,

    /// Extra environment variables (values support `${...}` templates).
    pub env: Option<HashMap<String, String>>,
}

pub struct ProcessStage {
    config: ProcessStageConfig,
}

impl ProcessStage {
    pub fn new(config: ProcessStageConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Stage for ProcessStage {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn execute(
        &self,
        input: Value,
        _events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        // Render command and args via template engine
        let command = template::render_string(&self.config.command, &input)?;
        let mut rendered_args = Vec::with_capacity(self.config.args.len());
        for arg in &self.config.args {
            rendered_args.push(template::render_string(arg, &input)?);
        }

        debug!(
            stage = self.config.name,
            command = %command,
            "executing process stage"
        );

        // Build command with minimal environment
        let mut cmd = tokio::process::Command::new(&command);
        cmd.args(&rendered_args);

        // Clear inherited env — subprocess gets a minimal environment
        cmd.env_clear();
        cmd.env("NIOBIUM_PIPE", "1");

        // Add PATH so the command can be found
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        // Render and add configured env vars
        if let Some(ref env_map) = self.config.env {
            for (k, v) in env_map {
                let rendered = template::render_string(v, &input)?;
                cmd.env(k, rendered);
            }
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!(
                "stage '{}': failed to spawn '{}': {e}",
                self.config.name,
                command
            )
        })?;

        // Write input JSON to stdin, then close it
        let input_bytes = serde_json::to_vec(&input)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&input_bytes).await?;
            // stdin is dropped here, closing it
        }

        let timeout_secs = self.config.timeout.unwrap_or(30);
        let timeout_dur = std::time::Duration::from_secs(timeout_secs);

        // Spawn wait_with_output on a task so we can abort it on timeout
        let handle = tokio::spawn(async move { child.wait_with_output().await });

        let output = match tokio::time::timeout(timeout_dur, handle).await {
            Ok(Ok(Ok(output))) => output,
            Ok(Ok(Err(e))) => {
                bail!("stage '{}': process I/O error: {e}", self.config.name);
            }
            Ok(Err(e)) => {
                bail!("stage '{}': task join error: {e}", self.config.name);
            }
            Err(_) => {
                // Timeout — abort the task (which drops the child, closing pipes)
                bail!(
                    "stage '{}': process timed out after {timeout_secs}s",
                    self.config.name,
                );
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            bail!(
                "stage '{}': process exited with code {code}: {stderr}",
                self.config.name,
            );
        }

        // Parse stdout as JSON
        if output.stdout.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_slice(&output.stdout).map_err(|e| {
            let raw = String::from_utf8_lossy(&output.stdout);
            anyhow::anyhow!(
                "stage '{}': failed to parse stdout as JSON: {e}\nstdout: {raw}",
                self.config.name,
            )
        })
    }
}

/// Builder for process stages.
pub struct ProcessStageBuilder;

impl StageBuilder for ProcessStageBuilder {
    fn stage_type(&self) -> &str {
        "process"
    }

    fn is_trusted(&self) -> bool {
        false
    }

    fn build(&self, def: &Value) -> Result<Box<dyn Stage>> {
        let config: ProcessStageConfig = serde_json::from_value(def.clone())?;
        Ok(Box::new(ProcessStage::new(config)))
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
    async fn echo_script_passthrough() {
        // `cat` reads stdin and writes to stdout — perfect echo test
        let config = ProcessStageConfig {
            name: "echo".into(),
            command: "cat".into(),
            args: vec![],
            timeout: Some(5),
            env: None,
        };

        let stage = ProcessStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({"hello": "world", "count": 42});

        let result = stage.execute(input.clone(), &tx).await.unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn nonzero_exit_fails() {
        let config = ProcessStageConfig {
            name: "fail".into(),
            command: "sh".into(),
            args: vec!["-c".into(), "echo 'oops' >&2; exit 1".into()],
            timeout: Some(5),
            env: None,
        };

        let stage = ProcessStage::new(config);
        let (tx, _rx) = make_events();

        let err = stage.execute(json!({}), &tx).await.unwrap_err();
        assert!(err.to_string().contains("exited with code 1"));
        assert!(err.to_string().contains("oops"));
    }

    #[tokio::test]
    async fn timeout_kills_process() {
        let config = ProcessStageConfig {
            name: "slow".into(),
            command: "sleep".into(),
            args: vec!["60".into()],
            timeout: Some(1),
            env: None,
        };

        let stage = ProcessStage::new(config);
        let (tx, _rx) = make_events();

        let err = stage.execute(json!({}), &tx).await.unwrap_err();
        assert!(err.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn env_vars_passed() {
        let config = ProcessStageConfig {
            name: "env_test".into(),
            command: "sh".into(),
            args: vec!["-c".into(), r#"echo "{\"token\": \"$MY_TOKEN\"}" "#.into()],
            timeout: Some(5),
            env: Some(HashMap::from([("MY_TOKEN".into(), "${secret}".into())])),
        };

        let stage = ProcessStage::new(config);
        let (tx, _rx) = make_events();
        let input = json!({"secret": "abc123"});

        let result = stage.execute(input, &tx).await.unwrap();
        assert_eq!(result["token"], "abc123");
    }

    #[tokio::test]
    async fn empty_stdout_returns_null() {
        let config = ProcessStageConfig {
            name: "empty".into(),
            command: "true".into(),
            args: vec![],
            timeout: Some(5),
            env: None,
        };

        let stage = ProcessStage::new(config);
        let (tx, _rx) = make_events();

        let result = stage.execute(json!({}), &tx).await.unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn builder_creates_stage() {
        let builder = ProcessStageBuilder;
        assert_eq!(builder.stage_type(), "process");

        let def = json!({
            "name": "test",
            "command": "cat",
        });
        let stage = builder.build(&def).unwrap();
        assert_eq!(stage.name(), "test");
    }
}
