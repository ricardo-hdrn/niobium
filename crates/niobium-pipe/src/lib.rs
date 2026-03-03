//! niobium-pipe — secure data pipeline engine for Niobium.
//!
//! Routes sensitive form data directly to HTTP sinks without
//! passing through the LLM. The agent declares WHAT to collect
//! and WHERE to send it; Niobium handles the HOW.
//!
//! Stages are pluggable via the [`StageBuilder`] trait and [`StageRegistry`].
//! Built-in types: `http`, `process`, `transform`, `toast`, `redact`.

pub mod event;
pub mod http;
pub mod parallel;
pub mod pipeline;
pub mod process;
pub mod redact;
pub mod registry;
pub mod secure_context;
pub mod stage;
pub mod template;
pub mod toast;
pub mod transform;

pub use event::{PipeEvent, Severity};
pub use pipeline::{Pipeline, build_pipeline};
pub use redact::{extract_sensitive_fields, redact_sensitive};
pub use registry::{StageBuilder, StageRegistry, default_registry};
pub use secure_context::SecureContext;
pub use stage::Stage;
