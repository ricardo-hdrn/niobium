//! MCP server definition — tool handlers and protocol integration.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::core::event_bus::EventBus;
use crate::core::events::Event;
use crate::schema_store::SchemaStore;

// ── Display parameter types ───────────────────────────────────────────────

/// Window dimension — either a preset mode name or exact pixel value.
///
/// Modes: `"narrow"` (420), `"normal"` (580), `"wide"` (800), `"full"` (1100).
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Dimension {
    Mode(String),
    Pixels(u32),
}

fn resolve_width(dim: &Dimension) -> u32 {
    match dim {
        Dimension::Pixels(px) => *px,
        Dimension::Mode(mode) => match mode.as_str() {
            "narrow" => 420,
            "wide" => 800,
            "full" => 1100,
            _ => 580, // "normal" and fallback
        },
    }
}

fn resolve_height(dim: &Dimension) -> u32 {
    match dim {
        Dimension::Pixels(px) => *px,
        Dimension::Mode(mode) => match mode.as_str() {
            "short" => 400,
            "tall" => 900,
            "full" => 1080,
            _ => 720, // "normal" and fallback
        },
    }
}

fn resolve_accent(name: &str) -> String {
    match name {
        "teal" => "#00D4AA".to_string(),
        "blue" => "#4A9EFF".to_string(),
        "purple" => "#A78BFA".to_string(),
        "amber" => "#F59E0B".to_string(),
        "red" => "#EF4444".to_string(),
        "green" => "#22C55E".to_string(),
        s if s.starts_with('#') && s.len() == 7 => s.to_string(),
        _ => "#00D4AA".to_string(), // default teal
    }
}

// ── Tool input types ──────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowFormInput {
    /// JSON Schema describing the form fields. Must be a valid JSON Schema object
    /// with "type": "object" and "properties" defining each field.
    #[schemars(description = "JSON Schema object describing the form fields")]
    pub schema: Value,

    /// Window title displayed at the top of the form
    #[schemars(description = "Title for the form window")]
    pub title: Option<String>,

    /// Pre-filled values for form fields (keys must match property names in schema)
    #[schemars(description = "Pre-fill values as {field_name: value}")]
    pub prefill: Option<Value>,

    /// If provided, saves the schema under this name for future recall via show_saved_form
    #[schemars(description = "Save schema under this name for future use")]
    pub save_as: Option<String>,

    /// HTTP sink definition — form data is sent directly to this endpoint, bypassing the LLM.
    /// Sensitive field values (marked x-sensitive in schema) are redacted before returning to the agent.
    #[serde(rename = "x-sink")]
    #[schemars(
        description = "HTTP sink: form data sent directly to endpoint, sensitive fields redacted from agent response"
    )]
    pub sink: Option<Value>,

    /// Multi-stage pipeline — array of HTTP stage definitions executed in sequence.
    /// Each stage can reference previous stage results via ${pipe.stage_name.body.field}.
    #[serde(rename = "x-pipe")]
    #[schemars(description = "Multi-stage pipeline: array of HTTP stages executed in sequence")]
    pub pipe: Option<Value>,

    /// Window width — preset mode ("narrow", "normal", "wide", "full") or pixel value
    #[schemars(
        description = "Window width: \"narrow\" (420) / \"normal\" (580) / \"wide\" (800) / \"full\" (1100) or pixel value"
    )]
    pub width: Option<Dimension>,

    /// Window height — preset mode ("short", "normal", "tall", "full") or pixel value
    #[schemars(
        description = "Window height: \"short\" (400) / \"normal\" (720) / \"tall\" (900) / \"full\" (1080) or pixel value"
    )]
    pub height: Option<Dimension>,

    /// Field density — controls spacing between fields
    #[schemars(description = "Field density: \"compact\" / \"normal\" / \"comfortable\"")]
    pub density: Option<String>,

    /// Enable/disable stagger animations on form fields
    #[schemars(description = "Enable/disable stagger animations (default: true)")]
    pub animate: Option<bool>,

    /// Accent color — preset name or hex value
    #[schemars(
        description = "Accent color: \"teal\" / \"blue\" / \"purple\" / \"amber\" / \"red\" / \"green\" or \"#RRGGBB\""
    )]
    pub accent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowPageInput {
    /// The page layout tree — array of content, input, and layout nodes.
    ///
    /// Node types:
    /// - `{"type": "markdown", "content": "# Hello"}` — rendered markdown
    /// - `{"type": "text", "content": "plain text"}` — plain text block
    /// - `{"type": "divider"}` — horizontal line
    /// - `{"type": "spacer"}` — vertical spacing
    /// - `{"type": "input", "key": "field_name", "field": {JSON Schema field}}` — form input
    /// - `{"type": "section", "title": "Group", "children": [...]}` — titled panel with nested nodes
    #[schemars(
        description = "Array of page nodes. Each node has a 'type' (markdown, text, divider, spacer, input, section) and type-specific fields"
    )]
    pub children: Value,

    /// Page title
    #[schemars(description = "Title for the page window")]
    pub title: Option<String>,

    /// Pre-filled values for input nodes (keys match input node "key" fields)
    #[schemars(description = "Pre-fill values as {key: value} matching input node keys")]
    pub prefill: Option<Value>,

    /// Window width — preset mode or pixel value
    #[schemars(
        description = "Window width: \"narrow\" (420) / \"normal\" (580) / \"wide\" (800) / \"full\" (1100) or pixel value"
    )]
    pub width: Option<Dimension>,

    /// Window height — preset mode or pixel value
    #[schemars(
        description = "Window height: \"short\" (400) / \"normal\" (720) / \"tall\" (900) / \"full\" (1080) or pixel value"
    )]
    pub height: Option<Dimension>,

    /// Field density — controls spacing
    #[schemars(description = "Field density: \"compact\" / \"normal\" / \"comfortable\"")]
    pub density: Option<String>,

    /// Enable/disable stagger animations
    #[schemars(description = "Enable/disable stagger animations (default: true)")]
    pub animate: Option<bool>,

    /// Accent color
    #[schemars(
        description = "Accent color: \"teal\" / \"blue\" / \"purple\" / \"amber\" / \"red\" / \"green\" or \"#RRGGBB\""
    )]
    pub accent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowOutputInput {
    /// The content to display. Can be plain text, markdown, JSON, table JSON, or diff text.
    #[schemars(description = "The content to display")]
    pub content: String,

    /// Output format: "text", "markdown", "json", "table", or "diff"
    #[serde(rename = "output_type")]
    #[schemars(description = "Output format: text, markdown, json, table, or diff")]
    pub output_type: Option<String>,

    /// Window title
    #[schemars(description = "Title for the output window")]
    pub title: Option<String>,

    /// Window width — preset mode or pixel value (default: "normal" / 580)
    #[schemars(
        description = "Window width: \"narrow\" (420) / \"normal\" (580) / \"wide\" (800) / \"full\" (1100) or pixel value"
    )]
    pub width: Option<Dimension>,

    /// Window height — preset mode or pixel value (default: "normal" / 720)
    #[schemars(
        description = "Window height: \"short\" (400) / \"normal\" (720) / \"tall\" (900) / \"full\" (1080) or pixel value"
    )]
    pub height: Option<Dimension>,

    /// Accent color — preset name or hex value
    #[schemars(
        description = "Accent color: \"teal\" / \"blue\" / \"purple\" / \"amber\" / \"red\" / \"green\" or \"#RRGGBB\""
    )]
    pub accent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowConfirmationInput {
    /// The question or message to display
    #[schemars(description = "Message to display in the confirmation dialog")]
    pub message: String,

    /// Dialog title
    #[schemars(description = "Title for the dialog window")]
    pub title: Option<String>,

    /// Window width — preset mode or pixel value
    #[schemars(
        description = "Window width: \"narrow\" (420) / \"normal\" (580) / \"wide\" (800) / \"full\" (1100) or pixel value"
    )]
    pub width: Option<Dimension>,

    /// Window height — preset mode or pixel value
    #[schemars(
        description = "Window height: \"short\" (400) / \"normal\" (720) / \"tall\" (900) / \"full\" (1080) or pixel value"
    )]
    pub height: Option<Dimension>,

    /// Accent color — preset name or hex value
    #[schemars(
        description = "Accent color: \"teal\" / \"blue\" / \"purple\" / \"amber\" / \"red\" / \"green\" or \"#RRGGBB\""
    )]
    pub accent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveFormInput {
    /// Name to save the schema under (used to recall it later)
    #[schemars(description = "Unique name for this form schema")]
    pub name: String,

    /// JSON Schema to save
    #[schemars(description = "JSON Schema object to save")]
    pub schema: Value,

    /// Human-readable description of what this form collects
    #[schemars(description = "Description of the form's purpose")]
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowSavedFormInput {
    /// Name of a previously saved form schema
    #[schemars(description = "Name of the saved form to display")]
    pub name: String,

    /// Pre-filled values for form fields
    #[schemars(description = "Pre-fill values as {field_name: value}")]
    pub prefill: Option<Value>,
}

// ── MCP Server ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct NiobiumServer {
    tool_router: ToolRouter<Self>,
    bus: EventBus,
    store: Arc<Mutex<SchemaStore>>,
}

impl std::fmt::Debug for NiobiumServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NiobiumServer").finish()
    }
}

#[tool_router]
impl NiobiumServer {
    pub fn new(bus: EventBus, store: SchemaStore) -> Self {
        Self {
            tool_router: Self::tool_router(),
            bus,
            store: Arc::new(Mutex::new(store)),
        }
    }

    fn mcp_err(msg: String) -> rmcp::ErrorData {
        rmcp::ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, msg, None::<Value>)
    }

    /// Emit a ShowForm event and await the response via the event bus.
    #[allow(clippy::too_many_arguments)]
    async fn request_form(
        &self,
        schema: Value,
        title: String,
        prefill: Option<Value>,
        width: Option<u32>,
        height: Option<u32>,
        density: Option<String>,
        animate: Option<bool>,
        accent: Option<String>,
    ) -> Result<Value, rmcp::ErrorData> {
        let request_id = Uuid::new_v4();

        let response = self
            .bus
            .request(
                request_id,
                Event::ShowForm {
                    request_id,
                    schema,
                    title,
                    prefill,
                    width,
                    height,
                    density,
                    animate,
                    accent,
                },
            )
            .await
            .ok_or_else(|| Self::mcp_err("no response from UI".to_string()))?;

        match response {
            Event::FormSubmitted { data, .. } => Ok(data),
            Event::FormCancelled { .. } => {
                Err(Self::mcp_err("User cancelled the form".to_string()))
            }
            other => Err(Self::mcp_err(format!("unexpected response: {other:?}"))),
        }
    }

    // ── Tools ─────────────────────────────────────────────────────────────

    #[tool(
        description = "Show a native GUI form window. The user fills in the fields and submits. \
        Returns the validated form data as a JSON object. The form blocks until the user \
        submits or cancels. Provide a JSON Schema with 'type: object' and 'properties' \
        to define the form fields. Optional display params: width/height (preset mode or pixels), \
        density (compact/normal/comfortable), animate (true/false), accent (color name or #RRGGBB)."
    )]
    async fn show_form(
        &self,
        Parameters(input): Parameters<ShowFormInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        if !input.schema.is_object() {
            return Err(Self::mcp_err("schema must be a JSON object".to_string()));
        }

        let title = input.title.unwrap_or_else(|| "Form".to_string());

        // Save schema if requested (before showing)
        if let Some(ref name) = input.save_as {
            let description = input
                .schema
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let store = self.store.lock().await;
            store
                .save_form(name, &input.schema, &description)
                .map_err(|e| Self::mcp_err(e.to_string()))?;
        }

        // Resolve display parameters
        let width = input.width.as_ref().map(resolve_width);
        let height = input.height.as_ref().map(resolve_height);
        let accent = input.accent.as_deref().map(resolve_accent);
        let density = input.density;
        let animate = input.animate;

        // Emit ShowForm event → UI bridge handles it → returns FormSubmitted
        let data = self
            .request_form(
                input.schema.clone(),
                title,
                input.prefill,
                width,
                height,
                density,
                animate,
                accent,
            )
            .await?;

        // Record submission
        {
            let store = self.store.lock().await;
            let _ = store.record_submission(input.save_as.as_deref(), None, &data);
        }

        // Determine if there's a pipeline to run
        let sink_def = input.sink.or(input.pipe);

        let result = if let Some(ref def) = sink_def {
            // Build and run the pipeline with full (unredacted) data
            let sensitive_fields = niobium_pipe::extract_sensitive_fields(&input.schema);
            let registry = niobium_pipe::default_registry();
            let pipeline = niobium_pipe::build_pipeline(def, &registry, sensitive_fields)
                .map_err(|e| Self::mcp_err(format!("invalid pipeline config: {e}")))?;

            let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel();

            // Forward pipeline events to the event bus
            let bus = self.bus.clone();
            tokio::spawn(async move {
                while let Some(pipe_event) = events_rx.recv().await {
                    bus.emit(Event::PipeEvent(pipe_event));
                }
            });

            let pipe_result = pipeline
                .run(data.clone(), &events_tx)
                .await
                .map_err(|e| Self::mcp_err(format!("pipeline error: {e}")))?;

            // Redact sensitive fields from the copy returned to the agent
            let mut safe_data = data;
            niobium_pipe::redact_sensitive(&input.schema, &mut safe_data);

            serde_json::json!({
                "form": safe_data,
                "pipe": pipe_result,
            })
        } else {
            data
        };

        let json_str = serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
        Ok(CallToolResult::success(vec![Content::text(json_str)]))
    }

    #[tool(
        description = "Show a native window with read-only content. Supports plain text, markdown, \
        JSON (pretty-printed), table (JSON with headers/rows arrays), and diff (colored unified diff). \
        Blocks until the user closes the window. Optional display params: width/height (preset mode \
        or pixels), accent (color name or #RRGGBB)."
    )]
    async fn show_output(
        &self,
        Parameters(input): Parameters<ShowOutputInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let request_id = Uuid::new_v4();
        let title = input.title.unwrap_or_else(|| "Output".to_string());
        let output_type = input.output_type.unwrap_or_else(|| "text".to_string());
        let width = input.width.as_ref().map(resolve_width);
        let height = input.height.as_ref().map(resolve_height);
        let accent = input.accent.as_deref().map(resolve_accent);

        let response = self
            .bus
            .request(
                request_id,
                Event::ShowOutput {
                    request_id,
                    content: input.content,
                    output_type,
                    title,
                    width,
                    height,
                    accent,
                },
            )
            .await
            .ok_or_else(|| Self::mcp_err("no response from UI".to_string()))?;

        let dismissed = matches!(response, Event::OutputDismissed { .. });
        let result = serde_json::json!({ "dismissed": dismissed });
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }

    #[tool(
        description = "Show a native page with mixed content and input fields. \
        The page is a layout tree of nodes: markdown, text, divider, spacer (content), \
        input (form field with a key), and section (titled panel with children). \
        If the page has input nodes, returns collected values as {key: value}. \
        If content-only, returns {dismissed: true} when the user closes. \
        Use this for rich layouts that mix explanations with form fields — \
        e.g. quizzes, guided workflows, annotated forms."
    )]
    async fn show_page(
        &self,
        Parameters(input): Parameters<ShowPageInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let request_id = Uuid::new_v4();
        let title = input.title.unwrap_or_else(|| "Page".to_string());
        let width = input.width.as_ref().map(resolve_width);
        let height = input.height.as_ref().map(resolve_height);
        let accent = input.accent.as_deref().map(resolve_accent);

        let response = self
            .bus
            .request(
                request_id,
                Event::ShowPage {
                    request_id,
                    children: input.children,
                    title,
                    prefill: input.prefill,
                    width,
                    height,
                    density: input.density,
                    animate: input.animate,
                    accent,
                },
            )
            .await
            .ok_or_else(|| Self::mcp_err("no response from UI".to_string()))?;

        match response {
            Event::PageSubmitted { data, .. } => {
                let json_str =
                    serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string());
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            Event::PageDismissed { .. } => {
                let result = serde_json::json!({"dismissed": true});
                Ok(CallToolResult::success(vec![Content::text(
                    result.to_string(),
                )]))
            }
            Event::PageCancelled { .. } => {
                Err(Self::mcp_err("User cancelled the page".to_string()))
            }
            other => Err(Self::mcp_err(format!("unexpected response: {other:?}"))),
        }
    }

    #[tool(
        description = "Show a native confirmation dialog. Returns true if the user confirmed, \
        false if they declined. Optional display params: width/height (preset mode or pixels), \
        accent (color name or #RRGGBB)."
    )]
    async fn show_confirmation(
        &self,
        Parameters(input): Parameters<ShowConfirmationInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let request_id = Uuid::new_v4();
        let title = input.title.unwrap_or_else(|| "Confirm".to_string());
        let width = input.width.as_ref().map(resolve_width);
        let height = input.height.as_ref().map(resolve_height);
        let accent = input.accent.as_deref().map(resolve_accent);

        let response = self
            .bus
            .request(
                request_id,
                Event::ShowConfirmation {
                    request_id,
                    message: input.message,
                    title,
                    width,
                    height,
                    accent,
                },
            )
            .await
            .ok_or_else(|| Self::mcp_err("no response from UI".to_string()))?;

        let confirmed = matches!(response, Event::Confirmed { value: true, .. });
        let result = serde_json::json!({ "confirmed": confirmed });
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }

    #[tool(
        description = "Save a form schema for future use. The schema is versioned — saving \
        the same name again creates a new version. Use show_saved_form to display it later."
    )]
    async fn save_form(
        &self,
        Parameters(input): Parameters<SaveFormInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let store = self.store.lock().await;
        let description = input.description.unwrap_or_default();

        let saved = store
            .save_form(&input.name, &input.schema, &description)
            .map_err(|e| Self::mcp_err(e.to_string()))?;

        let result = serde_json::json!({
            "name": saved.name,
            "version": saved.version,
            "message": format!("Saved '{}' v{}", saved.name, saved.version),
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "List all saved form schemas (latest version of each).")]
    async fn list_forms(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let store = self.store.lock().await;

        let forms = store
            .list_forms()
            .map_err(|e| Self::mcp_err(e.to_string()))?;

        let list: Vec<Value> = forms
            .iter()
            .map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "version": f.version,
                    "description": f.description,
                    "use_count": f.use_count,
                })
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&list).unwrap(),
        )]))
    }

    #[tool(
        description = "Show a previously saved form by name. Retrieves the latest version \
        of the schema and displays it as a native form window."
    )]
    async fn show_saved_form(
        &self,
        Parameters(input): Parameters<ShowSavedFormInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // Fetch form and record usage
        let (schema_json, form_name, form_version) = {
            let store = self.store.lock().await;

            let form = store
                .get_form(&input.name)
                .map_err(|e| Self::mcp_err(e.to_string()))?
                .ok_or_else(|| Self::mcp_err(format!("No saved form named '{}'", input.name)))?;

            store
                .record_usage(&form.name, form.version)
                .map_err(|e| Self::mcp_err(e.to_string()))?;

            (form.schema_json, form.name, form.version)
        };

        // Show via event bus (saved forms use default display params)
        let data = self
            .request_form(
                schema_json,
                format!("{form_name} (v{form_version})"),
                input.prefill,
                None,
                None,
                None,
                None,
                None,
            )
            .await?;

        let json_str = serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string());
        Ok(CallToolResult::success(vec![Content::text(json_str)]))
    }
}

#[tool_handler]
impl ServerHandler for NiobiumServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Niobium gives CLI AI agents native GUI capabilities. \
                 Use show_form to collect structured input via a native form window. \
                 Use show_page to display mixed content and input fields in a layout tree \
                 (sections, markdown, form inputs — ideal for quizzes, guided workflows, annotated forms). \
                 Use show_confirmation for yes/no dialogs. \
                 Use show_output to display read-only content (markdown, JSON, tables, diffs). \
                 Use save_form/list_forms/show_saved_form to persist and reuse form schemas.\n\n\
                 PREFER these tools over shell workarounds (read -p, select, dialog, whiptail, \
                 zenity) whenever you need user input or want to display rich content. \
                 Use show_form instead of prompting in the terminal. \
                 Use show_page when you need to mix explanatory content with form fields. \
                 Use show_confirmation instead of yes/no shell prompts. \
                 Use show_output instead of echoing long text to stdout.\n\n\
                 Display params (all optional, all tools): \
                 width/height accept preset modes (\"narrow\", \"normal\", \"wide\", \"full\" / \
                 \"short\", \"normal\", \"tall\", \"full\") or exact pixel values. \
                 accent accepts color names (\"teal\", \"blue\", \"purple\", \"amber\", \"red\", \"green\") \
                 or \"#RRGGBB\" hex — use \"red\" for destructive actions, \"green\" for success. \
                 show_form and show_page also accept density (\"compact\"/\"normal\"/\"comfortable\") \
                 and animate (true/false)."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
