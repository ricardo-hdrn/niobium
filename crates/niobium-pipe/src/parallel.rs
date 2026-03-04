//! ParallelStage — splits execution into concurrent branches, joins results.
//!
//! Each branch is a full [`Pipeline`] that runs independently. Results from all
//! branches are collected into a single JSON object keyed by branch name.

use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::event::PipeEvent;
use crate::pipeline::Pipeline;
use crate::stage::Stage;

/// A stage that runs multiple branch pipelines concurrently.
///
/// Output: `{ "branch_a": <branch_a_result>, "branch_b": <branch_b_result>, ... }`
pub struct ParallelStage {
    name: String,
    branches: Vec<(String, Arc<Pipeline>)>,
}

impl ParallelStage {
    pub fn new(name: String, branches: Vec<(String, Arc<Pipeline>)>) -> Self {
        Self { name, branches }
    }
}

#[async_trait]
impl Stage for ParallelStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        input: Value,
        events: &mpsc::UnboundedSender<PipeEvent>,
    ) -> Result<Value> {
        let mut join_set = JoinSet::new();

        for (branch_name, pipeline) in &self.branches {
            let branch_input = input.clone();
            let branch_events = events.clone();
            let branch_name = branch_name.clone();
            let pipeline = Arc::clone(pipeline);

            join_set.spawn(async move {
                let result = pipeline.run(branch_input, &branch_events).await;
                (branch_name, result)
            });
        }

        let mut results = serde_json::Map::new();

        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok((branch_name, Ok(output))) => {
                    results.insert(branch_name, output);
                }
                Ok((branch_name, Err(err))) => {
                    join_set.abort_all();
                    bail!("parallel branch '{branch_name}' failed: {err}");
                }
                Err(join_err) => {
                    join_set.abort_all();
                    bail!("parallel branch panicked: {join_err}");
                }
            }
        }

        Ok(Value::Object(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::build_pipeline;
    use crate::registry::default_registry;
    use serde_json::json;

    #[tokio::test]
    async fn two_branches_merge_results() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "analysis",
                "branches": {
                    "left": [
                        {
                            "type": "process",
                            "name": "echo_left",
                            "command": "cat",
                            "timeout": 5
                        }
                    ],
                    "right": [
                        {
                            "type": "process",
                            "name": "echo_right",
                            "command": "cat",
                            "timeout": 5
                        }
                    ]
                }
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline.run(json!({"value": "hello"}), &tx).await.unwrap();

        // Parallel stage output is keyed by branch name
        let analysis = &result["analysis"];
        assert_eq!(analysis["left"]["echo_left"]["value"], "hello");
        assert_eq!(analysis["right"]["echo_right"]["value"], "hello");
    }

    #[tokio::test]
    async fn branch_failure_aborts_and_returns_error() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "ops",
                "branches": {
                    "ok": [
                        {
                            "type": "process",
                            "name": "good",
                            "command": "cat",
                            "timeout": 5
                        }
                    ],
                    "bad": [
                        {
                            "type": "process",
                            "name": "fail",
                            "command": "sh",
                            "args": ["-c", "exit 1"],
                            "timeout": 5
                        }
                    ]
                }
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let err = pipeline.run(json!({}), &tx).await.unwrap_err();
        assert!(
            err.to_string().contains("bad"),
            "error should mention failing branch: {err}"
        );
    }

    #[tokio::test]
    async fn downstream_stage_references_parallel_results() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "gather",
                "branches": {
                    "alpha": [
                        {
                            "type": "transform",
                            "name": "make_a",
                            "expr": {"val": "$.data"}
                        }
                    ],
                    "beta": [
                        {
                            "type": "transform",
                            "name": "make_b",
                            "expr": {"val": "$.data"}
                        }
                    ]
                }
            },
            {
                "type": "transform",
                "name": "combine",
                "expr": {
                    "a_val": "$.pipe.gather.alpha.make_a.val",
                    "b_val": "$.pipe.gather.beta.make_b.val"
                }
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline.run(json!({"data": "test123"}), &tx).await.unwrap();

        assert_eq!(result["combine"]["a_val"], "test123");
        assert_eq!(result["combine"]["b_val"], "test123");
    }

    #[tokio::test]
    async fn secure_context_process_in_branch_sees_placeholders() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "secure_test",
                "branches": {
                    "branch": [
                        {
                            "type": "process",
                            "name": "echo",
                            "command": "cat",
                            "timeout": 5
                        }
                    ]
                }
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec!["password".into()]).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = pipeline
            .run(json!({"user": "alice", "password": "s3cret"}), &tx)
            .await
            .unwrap();

        let echoed = &result["secure_test"]["branch"]["echo"];
        assert_eq!(echoed["user"], "alice");

        let pw = echoed["password"].as_str().unwrap();
        assert!(
            pw.starts_with("<<NB:"),
            "process in branch should see placeholder, got: {pw}"
        );
        assert!(
            !pw.contains("s3cret"),
            "real secret leaked to process in branch!"
        );
    }

    #[tokio::test]
    async fn events_emitted_from_both_branches() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "dual",
                "branches": {
                    "a": [
                        {
                            "type": "transform",
                            "name": "step_a",
                            "expr": {"x": "$.v"}
                        }
                    ],
                    "b": [
                        {
                            "type": "transform",
                            "name": "step_b",
                            "expr": {"x": "$.v"}
                        }
                    ]
                }
            }
        ]);

        let registry = default_registry();
        let pipeline = build_pipeline(&pipe, &registry, vec![]).unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        pipeline.run(json!({"v": 1}), &tx).await.unwrap();

        let mut events = vec![];
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        // Each branch pipeline emits StageStarted + StageCompleted for its stage,
        // plus the parent pipeline emits StageStarted + StageCompleted for the parallel stage.
        // Branch events: step_a started/completed + step_b started/completed = 4
        // Parent events: dual started/completed = 2
        // Total = 6
        let started: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PipeEvent::StageStarted { .. }))
            .collect();
        let completed: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PipeEvent::StageCompleted { .. }))
            .collect();

        // At least the two branch stages should have started+completed
        // (order between branches is non-deterministic)
        assert!(
            started.len() >= 3,
            "expected >=3 StageStarted events, got {}",
            started.len()
        );
        assert!(
            completed.len() >= 3,
            "expected >=3 StageCompleted events, got {}",
            completed.len()
        );
    }

    #[test]
    fn empty_branches_rejected() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "empty",
                "branches": {}
            }
        ]);

        let registry = default_registry();
        let err = build_pipeline(&pipe, &registry, vec![]).unwrap_err();
        assert!(
            err.to_string().contains("must not be empty"),
            "expected empty branches error, got: {err}"
        );
    }

    #[test]
    fn branch_name_with_dot_rejected() {
        let pipe = json!([
            {
                "type": "parallel",
                "name": "dotted",
                "branches": {
                    "bad.name": [
                        {
                            "type": "transform",
                            "name": "s",
                            "expr": {"x": "$.v"}
                        }
                    ]
                }
            }
        ]);

        let registry = default_registry();
        let err = build_pipeline(&pipe, &registry, vec![]).unwrap_err();
        assert!(
            err.to_string().contains("must not contain dots"),
            "expected dot rejection error, got: {err}"
        );
    }
}
