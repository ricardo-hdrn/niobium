//! MCP server definition — tool handlers and protocol integration.

use std::collections::HashMap;
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
    /// Node types (18):
    /// - `{"type": "markdown", "content": "# Hello"}` — rendered markdown
    /// - `{"type": "text", "content": "plain text"}` — plain text block
    /// - `{"type": "divider"}` — horizontal line
    /// - `{"type": "spacer", "props": {"size": "lg"}}` — vertical spacing
    /// - `{"type": "input", "key": "field_name", "field": {JSON Schema field}}` — form input
    /// - `{"type": "section", "title": "Group", "children": [...]}` — titled panel with nested nodes
    /// - `{"type": "stat", "props": {"label": "Tests", "value": "42", "variant": "success"}}` — metric display
    /// - `{"type": "progress", "props": {"value": 0.7, "label": "Migration"}}` — progress bar
    /// - `{"type": "badge", "content": "PASSED", "props": {"variant": "success"}}` — inline badge
    /// - `{"type": "image", "content": "https://...", "props": {"alt": "diagram"}}` — image
    /// - `{"type": "alert", "props": {"variant": "warning"}, "children": [...]}` — alert box
    /// - `{"type": "card", "children": [...]}` — card panel
    /// - `{"type": "collapse", "title": "Details", "props": {"expanded": true}, "children": [...]}` — collapsible
    /// - `{"type": "hero", "children": [...]}` — hero banner
    /// - `{"type": "row", "children": [...]}` — horizontal layout (children are col nodes)
    /// - `{"type": "col", "children": [...]}` — column within a row
    /// - `{"type": "tabs", "children": [...]}` — tab container (children are tab nodes)
    /// - `{"type": "tab", "title": "Overview", "children": [...]}` — tab panel
    #[schemars(
        description = "Array of page nodes. 18 types: markdown, text, divider, spacer, input, section, \
        stat, progress, badge, image, alert, card, collapse, hero, row, col, tabs, tab"
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MdToPageInput {
    /// Markdown content to render as a native page.
    ///
    /// Headings become titled section panels:
    /// - `# Title` → top-level section
    /// - `## Subtitle` → nested section
    ///
    /// Horizontal rules (`---`) become dividers.
    /// Fenced code blocks with language `niobium` are parsed as inline JSON nodes
    /// (input fields, sections, etc.) — enabling interactive forms within markdown.
    ///
    /// HTML comments `<!-- nb:xxx -->` embed rich components:
    /// - Self-closing: `<!-- nb:stat label="Tests" value="42" -->`, `<!-- nb:progress value="0.7" -->`, `<!-- nb:gap lg -->`
    /// - Inline content: `<!-- nb:badge variant="success" -->PASSED<!-- nb:end -->`, `<!-- nb:image -->url<!-- nb:end -->`
    /// - Containers: `<!-- nb:alert variant="warning" -->...<!-- nb:end -->`, `<!-- nb:card -->`, `<!-- nb:collapse title="Details" expanded -->`, `<!-- nb:hero -->`, `<!-- nb:row -->`, `<!-- nb:tabs -->`
    /// - Row columns: `<!-- nb:col -->` inside `<!-- nb:row -->`
    /// - Tab panels: `<!-- nb:tab title="Overview" -->` inside `<!-- nb:tabs -->`
    ///
    /// Everything else renders as rich markdown content inside the current section.
    #[schemars(
        description = "Markdown text. # headings become titled panels, --- become dividers, \
        ```niobium blocks embed interactive JSON components, \
        <!-- nb:xxx --> HTML comments embed rich components (stat, progress, badge, image, alert, card, collapse, hero, row/col, tabs/tab, gap)"
    )]
    pub markdown: String,

    /// Page title
    #[schemars(description = "Title for the page window")]
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

// ── nb: comment parsing helpers ───────────────────────────────────────────

/// Parse `key="value"` pairs and bare flags from an nb: comment attribute string.
///
/// Returns (props map, optional title extracted from props).
/// Numeric strings are parsed as numbers. Bare words become `true`.
fn parse_nb_attrs(attr_str: &str) -> (HashMap<String, Value>, Option<String>) {
    let mut props: HashMap<String, Value> = HashMap::new();
    let mut rest = attr_str.trim();

    while !rest.is_empty() {
        // Skip whitespace
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        // Try key="value"
        if let Some(eq_pos) = rest.find('=') {
            let before_eq = &rest[..eq_pos];
            // Only treat as key=value if there's no space before the '='
            if !before_eq.contains(' ') {
                let key = before_eq.trim();
                let after_eq = &rest[eq_pos + 1..];
                if let Some(quoted) = after_eq.strip_prefix('"')
                    && let Some(end_quote) = quoted.find('"')
                {
                    let val_str = &quoted[..end_quote];
                    let value = try_parse_number(val_str);
                    props.insert(key.to_string(), value);
                    rest = &quoted[end_quote + 1..];
                    continue;
                }
            }
        }

        // Bare flag (word without '=' before next space)
        let end = rest.find(' ').unwrap_or(rest.len());
        let flag = &rest[..end];
        if !flag.is_empty() {
            props.insert(flag.to_string(), Value::Bool(true));
        }
        rest = &rest[end..];
    }

    let title = props.remove("title").and_then(|v| match v {
        Value::String(s) => Some(s),
        _ => None,
    });

    (props, title)
}

/// Try to parse a string as a number, fall back to string.
fn try_parse_number(s: &str) -> Value {
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = s.parse::<f64>()
        && let Some(n) = serde_json::Number::from_f64(f)
    {
        return Value::Number(n);
    }
    Value::String(s.to_string())
}

/// Parse an `<!-- nb:xxx ... -->` comment from a line.
///
/// Returns `Some((tag, attr_str, rest_of_line))` where:
/// - tag is e.g. "alert", "stat", "end", "col", "tab"
/// - attr_str is the attributes inside the comment
/// - rest_of_line is any text after `-->` (used for inline badge/image content)
fn parse_nb_comment(line: &str) -> Option<(&str, &str, &str)> {
    let trimmed = line.trim();
    let after_open = trimmed.strip_prefix("<!--")?;
    // Find the closing -->
    let close_pos = after_open.find("-->")?;
    let inner = after_open[..close_pos].trim();
    let rest_after = after_open[close_pos + 3..].trim();
    let body = inner.strip_prefix("nb:")?;
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    // Split into tag and rest
    let (tag, attrs) = match body.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&body[..pos], body[pos..].trim()),
        None => (body, ""),
    };
    Some((tag, attrs, rest_after))
}

/// Which nb: tags are self-closing (no children, no nb:end needed).
fn is_self_closing_nb(tag: &str) -> bool {
    matches!(tag, "stat" | "progress" | "gap")
}

/// Which nb: tags need content between them and nb:end (inline content, not children).
fn is_inline_content_nb(tag: &str) -> bool {
    matches!(tag, "badge" | "image")
}

/// Which nb: tags act as sibling separators within their parent container.
fn is_sibling_separator_nb(tag: &str) -> bool {
    matches!(tag, "col" | "tab")
}

// ── nb: container stack frame ────────────────────────────────────────────

/// A frame on the nb: container stack, tracking an open container element.
struct NbFrame {
    /// The nb: tag name (e.g. "alert", "card", "row", "tabs")
    #[allow(dead_code)]
    tag: String,
    /// The JSON node type to emit (usually same as tag)
    node_type: String,
    /// Parsed props (variant, etc.)
    props: HashMap<String, Value>,
    /// Optional title
    title: Option<String>,
    /// Accumulated children nodes
    children: Vec<Value>,
    /// For sibling-separator parents (row/tabs): the current sibling tag and its children
    #[allow(clippy::type_complexity)]
    current_sibling: Option<(String, HashMap<String, Value>, Option<String>, Vec<Value>)>,
}

impl NbFrame {
    fn new(
        tag: &str,
        node_type: &str,
        props: HashMap<String, Value>,
        title: Option<String>,
    ) -> Self {
        Self {
            tag: tag.to_string(),
            node_type: node_type.to_string(),
            props,
            title,
            children: Vec::new(),
            current_sibling: None,
        }
    }

    /// Flush the current sibling (col/tab) into children.
    fn flush_sibling(&mut self) {
        if let Some((sib_tag, sib_props, sib_title, sib_children)) = self.current_sibling.take()
            && (!sib_children.is_empty() || sib_title.is_some())
        {
            let mut node = serde_json::json!({
                "type": sib_tag,
                "children": sib_children,
            });
            if let Some(t) = sib_title {
                node["title"] = Value::String(t);
            }
            if !sib_props.is_empty() {
                node["props"] = serde_json::to_value(&sib_props).unwrap_or_default();
            }
            self.children.push(node);
        }
    }

    /// Build the final JSON node for this container.
    fn into_node(mut self) -> Value {
        self.flush_sibling();
        let mut node = serde_json::json!({
            "type": self.node_type,
            "children": self.children,
        });
        if let Some(t) = self.title {
            node["title"] = Value::String(t);
        }
        if !self.props.is_empty() {
            node["props"] = serde_json::to_value(&self.props).unwrap_or_default();
        }
        node
    }

    /// Push a node into the current sibling if one is open, otherwise into children.
    fn push(&mut self, node: Value) {
        if let Some((_, _, _, ref mut sib_children)) = self.current_sibling {
            sib_children.push(node);
        } else {
            self.children.push(node);
        }
    }
}

/// Parse markdown into a page node tree for show_page.
///
/// Supported syntax:
/// - `# Heading` → section with title (top-level)
/// - `## Heading` → section with title (nested inside current h1 section)
/// - `---` / `***` / `___` → divider
/// - ` ```niobium ` fenced blocks → inline JSON nodes
/// - `<!-- nb:xxx ... -->` HTML comments → rich component nodes
/// - Everything else → markdown content node
fn md_to_page_nodes(markdown: &str) -> Value {
    let mut root: Vec<Value> = Vec::new();
    let mut h1_title: Option<String> = None;
    let mut h1_children: Vec<Value> = Vec::new();
    let mut h2_title: Option<String> = None;
    let mut h2_children: Vec<Value> = Vec::new();
    let mut buf = String::new();

    // Stack of open nb: containers
    let mut nb_stack: Vec<NbFrame> = Vec::new();
    // For inline-content nb: tags (badge, image): accumulates text until nb:end
    let mut inline_nb: Option<(String, HashMap<String, Value>)> = None;
    let mut inline_buf = String::new();

    fn flush_buf(buf: &mut String) -> Option<Value> {
        let trimmed = buf.trim();
        if trimmed.is_empty() {
            buf.clear();
            return None;
        }
        let node = serde_json::json!({"type": "markdown", "content": trimmed});
        buf.clear();
        Some(node)
    }

    /// Push a node into the deepest open context (nb stack, h2, h1, or root).
    fn push_node(
        node: Value,
        nb_stack: &mut [NbFrame],
        h2_title: &Option<String>,
        h2_children: &mut Vec<Value>,
        h1_title: &Option<String>,
        h1_children: &mut Vec<Value>,
        root: &mut Vec<Value>,
    ) {
        if let Some(frame) = nb_stack.last_mut() {
            frame.push(node);
        } else if h2_title.is_some() {
            h2_children.push(node);
        } else if h1_title.is_some() {
            h1_children.push(node);
        } else {
            root.push(node);
        }
    }

    /// Close h2 section, pushing it into h1 or root.
    fn close_h2(
        h2_title: &mut Option<String>,
        h2_children: &mut Vec<Value>,
        buf: &mut String,
        h1_title: &Option<String>,
        h1_children: &mut Vec<Value>,
        root: &mut Vec<Value>,
    ) {
        if let Some(node) = flush_buf(buf) {
            if h2_title.is_some() {
                h2_children.push(node);
            } else if h1_title.is_some() {
                h1_children.push(node);
            } else {
                root.push(node);
            }
        }
        if let Some(title) = h2_title.take() {
            let section = serde_json::json!({
                "type": "section",
                "title": title,
                "children": std::mem::take(h2_children),
            });
            if h1_title.is_some() {
                h1_children.push(section);
            } else {
                root.push(section);
            }
        }
    }

    fn is_hr(line: &str) -> bool {
        line.len() >= 3
            && (line.starts_with("---") || line.starts_with("***") || line.starts_with("___"))
            && line
                .chars()
                .all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
    }

    // Track fenced code blocks: None = not in a block, Some(true) = niobium block, Some(false) = regular code
    let mut in_fence: Option<bool> = None;
    let mut nb_buf = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // If we're accumulating inline content for badge/image, collect until nb:end
        if inline_nb.is_some() {
            if let Some(("end", _, _)) = parse_nb_comment(trimmed) {
                let (tag, props) = inline_nb.take().unwrap();
                let content = inline_buf.trim().to_string();
                inline_buf.clear();
                let mut node = serde_json::json!({
                    "type": tag,
                    "content": content,
                });
                if !props.is_empty() {
                    node["props"] = serde_json::to_value(&props).unwrap_or_default();
                }
                // Flush buf before pushing
                if let Some(md_node) = flush_buf(&mut buf) {
                    push_node(
                        md_node,
                        &mut nb_stack,
                        &h2_title,
                        &mut h2_children,
                        &h1_title,
                        &mut h1_children,
                        &mut root,
                    );
                }
                push_node(
                    node,
                    &mut nb_stack,
                    &h2_title,
                    &mut h2_children,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
            } else {
                if !inline_buf.is_empty() {
                    inline_buf.push('\n');
                }
                inline_buf.push_str(trimmed);
            }
            continue;
        }

        // Fenced code block handling
        if trimmed.starts_with("```") {
            match in_fence {
                None => {
                    // Opening fence
                    let lang = trimmed.trim_start_matches('`').trim();
                    if lang == "niobium" || lang.starts_with("niobium ") {
                        // Flush markdown before the niobium block
                        if let Some(node) = flush_buf(&mut buf) {
                            push_node(
                                node,
                                &mut nb_stack,
                                &h2_title,
                                &mut h2_children,
                                &h1_title,
                                &mut h1_children,
                                &mut root,
                            );
                        }
                        in_fence = Some(true);
                        nb_buf.clear();
                    } else {
                        // Regular code block — pass through as markdown
                        in_fence = Some(false);
                        buf.push_str(line);
                        buf.push('\n');
                    }
                    continue;
                }
                Some(true) => {
                    // Closing niobium fence — parse JSON and inject node(s)
                    in_fence = None;
                    let json_str = nb_buf.trim();
                    if !json_str.is_empty()
                        && let Ok(val) = serde_json::from_str::<Value>(json_str)
                    {
                        // Support both single node and array of nodes
                        if let Some(arr) = val.as_array() {
                            for node in arr {
                                push_node(
                                    node.clone(),
                                    &mut nb_stack,
                                    &h2_title,
                                    &mut h2_children,
                                    &h1_title,
                                    &mut h1_children,
                                    &mut root,
                                );
                            }
                        } else {
                            push_node(
                                val,
                                &mut nb_stack,
                                &h2_title,
                                &mut h2_children,
                                &h1_title,
                                &mut h1_children,
                                &mut root,
                            );
                        }
                    }
                    nb_buf.clear();
                    continue;
                }
                Some(false) => {
                    // Closing regular code fence — pass through
                    in_fence = None;
                    buf.push_str(line);
                    buf.push('\n');
                    continue;
                }
            }
        }

        // Inside a fenced block
        if let Some(is_nb) = in_fence {
            if is_nb {
                nb_buf.push_str(line);
                nb_buf.push('\n');
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
            continue;
        }

        // ── nb: HTML comment handling ────────────────────────────────────
        if let Some((tag, attr_str, rest_of_line)) = parse_nb_comment(trimmed) {
            // Flush pending markdown buffer before any nb: directive
            if let Some(md_node) = flush_buf(&mut buf) {
                push_node(
                    md_node,
                    &mut nb_stack,
                    &h2_title,
                    &mut h2_children,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
            }

            if tag == "end" {
                // Close the innermost nb: container
                if let Some(frame) = nb_stack.pop() {
                    let node = frame.into_node();
                    push_node(
                        node,
                        &mut nb_stack,
                        &h2_title,
                        &mut h2_children,
                        &h1_title,
                        &mut h1_children,
                        &mut root,
                    );
                }
                continue;
            }

            if tag == "gap" {
                // <!-- nb:gap lg --> → spacer with size prop
                let size = if attr_str.is_empty() {
                    "md".to_string()
                } else {
                    // First bare word is the size
                    attr_str
                        .split_whitespace()
                        .next()
                        .unwrap_or("md")
                        .to_string()
                };
                let node = serde_json::json!({
                    "type": "spacer",
                    "props": { "size": size },
                });
                push_node(
                    node,
                    &mut nb_stack,
                    &h2_title,
                    &mut h2_children,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
                continue;
            }

            let (props, title) = parse_nb_attrs(attr_str);

            if is_self_closing_nb(tag) {
                // Self-closing: stat, progress
                let mut node = serde_json::json!({ "type": tag });
                if !props.is_empty() {
                    node["props"] = serde_json::to_value(&props).unwrap_or_default();
                }
                push_node(
                    node,
                    &mut nb_stack,
                    &h2_title,
                    &mut h2_children,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
                continue;
            }

            if is_inline_content_nb(tag) {
                // Inline content: badge, image
                // Check if content + nb:end are on the same line
                if let Some(end_pos) = rest_of_line.find("<!-- nb:end -->") {
                    let content = rest_of_line[..end_pos].trim().to_string();
                    let mut node = serde_json::json!({
                        "type": tag,
                        "content": content,
                    });
                    if !props.is_empty() {
                        node["props"] = serde_json::to_value(&props).unwrap_or_default();
                    }
                    push_node(
                        node,
                        &mut nb_stack,
                        &h2_title,
                        &mut h2_children,
                        &h1_title,
                        &mut h1_children,
                        &mut root,
                    );
                } else {
                    // Multi-line: collect text until nb:end
                    inline_nb = Some((tag.to_string(), props));
                    inline_buf.clear();
                    if !rest_of_line.is_empty() {
                        inline_buf.push_str(rest_of_line);
                    }
                }
                continue;
            }

            if is_sibling_separator_nb(tag) {
                // col/tab — flush previous sibling in the parent frame, start new one
                if let Some(frame) = nb_stack.last_mut() {
                    // Flush any pending markdown into the current sibling
                    if let Some(md_node) = flush_buf(&mut buf) {
                        frame.push(md_node);
                    }
                    frame.flush_sibling();
                    frame.current_sibling = Some((tag.to_string(), props, title, Vec::new()));
                }
                continue;
            }

            // Container tags: alert, card, collapse, hero, row, tabs, etc.
            let frame = NbFrame::new(tag, tag, props, title);
            nb_stack.push(frame);
            continue;
        }

        // Skip nb: handling when inside an nb: container — headings/hr still parsed
        // but nodes are pushed into the nb: stack frame instead of h1/h2/root.

        if is_hr(trimmed) {
            if let Some(node) = flush_buf(&mut buf) {
                push_node(
                    node,
                    &mut nb_stack,
                    &h2_title,
                    &mut h2_children,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
            }
            let divider = serde_json::json!({"type": "divider"});
            push_node(
                divider,
                &mut nb_stack,
                &h2_title,
                &mut h2_children,
                &h1_title,
                &mut h1_children,
                &mut root,
            );
            continue;
        }

        // Only process headings when NOT inside an nb: container
        if nb_stack.is_empty() {
            // H1 heading — close everything and start fresh section
            if let Some(title) = trimmed.strip_prefix("# ") {
                close_h2(
                    &mut h2_title,
                    &mut h2_children,
                    &mut buf,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
                if let Some(node) = flush_buf(&mut buf) {
                    if h1_title.is_some() {
                        h1_children.push(node);
                    } else {
                        root.push(node);
                    }
                }
                if let Some(t) = h1_title.take() {
                    root.push(serde_json::json!({
                        "type": "section",
                        "title": t,
                        "children": std::mem::take(&mut h1_children),
                    }));
                }
                h1_title = Some(title.to_string());
                continue;
            }

            // H2 heading — close current h2 and start new one
            if let Some(title) = trimmed.strip_prefix("## ") {
                close_h2(
                    &mut h2_title,
                    &mut h2_children,
                    &mut buf,
                    &h1_title,
                    &mut h1_children,
                    &mut root,
                );
                h2_title = Some(title.to_string());
                continue;
            }
        }

        buf.push_str(line);
        buf.push('\n');
    }

    // Flush any remaining markdown buffer into the nb stack if open
    if let Some(md_node) = flush_buf(&mut buf) {
        push_node(
            md_node,
            &mut nb_stack,
            &h2_title,
            &mut h2_children,
            &h1_title,
            &mut h1_children,
            &mut root,
        );
    }

    // Close any unclosed nb: containers (graceful recovery)
    while let Some(frame) = nb_stack.pop() {
        let node = frame.into_node();
        push_node(
            node,
            &mut nb_stack,
            &h2_title,
            &mut h2_children,
            &h1_title,
            &mut h1_children,
            &mut root,
        );
    }

    // Close remaining h2/h1
    close_h2(
        &mut h2_title,
        &mut h2_children,
        &mut buf,
        &h1_title,
        &mut h1_children,
        &mut root,
    );
    if let Some(node) = flush_buf(&mut buf) {
        if h1_title.is_some() {
            h1_children.push(node);
        } else {
            root.push(node);
        }
    }
    if let Some(t) = h1_title.take() {
        root.push(serde_json::json!({
            "type": "section",
            "title": t,
            "children": h1_children,
        }));
    }

    Value::Array(root)
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
        description = "Render markdown as a native paneled page. Headings become titled section panels \
        (# = top-level, ## = nested), horizontal rules (---) become dividers, everything else renders \
        as rich markdown content. Embed components via ```niobium JSON blocks or <!-- nb:xxx --> HTML comments: \
        self-closing (stat, progress, gap), inline-content (badge, image), and containers \
        (alert, card, collapse, hero, row/col, tabs/tab). \
        If the page has inputs, returns collected values. Otherwise returns {dismissed: true} when closed."
    )]
    async fn md_to_page(
        &self,
        Parameters(input): Parameters<MdToPageInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let children = md_to_page_nodes(&input.markdown);
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
                    children,
                    title,
                    prefill: None,
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
                 Use md_to_page to render markdown as a structured paneled document \
                 (headings become titled panels, --- become dividers). \
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_to_page_basic() {
        let md = "# Introduction\nHello world\n\n## Details\nSome details here\n\n---\n\nMore text\n\n# Conclusion\nDone";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();

        assert_eq!(arr.len(), 2, "two top-level sections");
        assert_eq!(arr[0]["title"], "Introduction");
        assert_eq!(arr[1]["title"], "Conclusion");

        // Introduction has: markdown("Hello world"), section(Details)
        let h1_children = arr[0]["children"].as_array().unwrap();
        assert_eq!(h1_children[0]["type"], "markdown");
        assert!(
            h1_children[0]["content"]
                .as_str()
                .unwrap()
                .contains("Hello")
        );
        assert_eq!(h1_children[1]["type"], "section");
        assert_eq!(h1_children[1]["title"], "Details");

        // Details section has: markdown, divider, markdown
        let h2_children = h1_children[1]["children"].as_array().unwrap();
        assert_eq!(h2_children[0]["type"], "markdown");
        assert_eq!(h2_children[1]["type"], "divider");
        assert_eq!(h2_children[2]["type"], "markdown");
        assert!(
            h2_children[2]["content"]
                .as_str()
                .unwrap()
                .contains("More text")
        );
    }

    #[test]
    fn test_md_to_page_no_headings() {
        let md = "Just some text\n\n---\n\nMore text";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();

        assert_eq!(arr[0]["type"], "markdown");
        assert_eq!(arr[1]["type"], "divider");
        assert_eq!(arr[2]["type"], "markdown");
    }

    #[test]
    fn test_md_to_page_multiple_h1() {
        let md = "# First\nContent 1\n# Second\nContent 2";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "First");
        assert_eq!(arr[1]["title"], "Second");
    }

    #[test]
    fn test_md_to_page_niobium_blocks() {
        let md = "# Review\n\nRate this:\n\n```niobium\n{\"type\": \"input\", \"key\": \"rating\", \"field\": {\"type\": \"string\", \"enum\": [\"Good\", \"Bad\"]}}\n```\n\nThanks!";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        let children = arr[0]["children"].as_array().unwrap();

        // markdown("Rate this:"), input node, markdown("Thanks!")
        assert_eq!(children[0]["type"], "markdown");
        assert!(children[0]["content"].as_str().unwrap().contains("Rate"));
        assert_eq!(children[1]["type"], "input");
        assert_eq!(children[1]["key"], "rating");
        assert_eq!(children[2]["type"], "markdown");
        assert!(children[2]["content"].as_str().unwrap().contains("Thanks"));
    }

    #[test]
    fn test_md_to_page_regular_code_blocks_pass_through() {
        let md = "Some code:\n\n```python\ndef foo():\n    pass\n```\n\nDone.";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();

        // Everything is one markdown node (code block passes through)
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "markdown");
        let content = arr[0]["content"].as_str().unwrap();
        assert!(content.contains("```python"));
        assert!(content.contains("def foo"));
    }

    // ── nb: comment tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_nb_attrs_basic() {
        let (props, title) = parse_nb_attrs(r#"label="Tests" value="42" variant="success""#);
        assert_eq!(props.get("label").unwrap(), "Tests");
        assert_eq!(props.get("value").unwrap(), 42); // numeric strings become numbers
        assert_eq!(props.get("variant").unwrap(), "success");
        assert!(title.is_none());
    }

    #[test]
    fn test_parse_nb_attrs_with_title() {
        let (props, title) = parse_nb_attrs(r#"title="My Card" variant="info""#);
        assert_eq!(title.unwrap(), "My Card");
        assert_eq!(props.get("variant").unwrap(), "info");
    }

    #[test]
    fn test_parse_nb_attrs_bare_flag() {
        let (props, _title) = parse_nb_attrs(r#"title="Details" expanded"#);
        assert_eq!(props.get("expanded").unwrap(), true);
    }

    #[test]
    fn test_parse_nb_attrs_numeric() {
        let (props, _) = parse_nb_attrs(r#"value="0.7" height="300""#);
        assert_eq!(props.get("value").unwrap(), 0.7);
        assert_eq!(props.get("height").unwrap(), 300);
    }

    #[test]
    fn test_md_to_page_nb_alert() {
        let md = "<!-- nb:alert variant=\"warning\" -->\nWatch out!\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "alert");
        assert_eq!(arr[0]["props"]["variant"], "warning");
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["type"], "markdown");
    }

    #[test]
    fn test_md_to_page_nb_stat() {
        let md = "<!-- nb:stat label=\"Tests\" value=\"42\" variant=\"success\" -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "stat");
        assert_eq!(arr[0]["props"]["label"], "Tests");
        assert_eq!(arr[0]["props"]["value"], 42); // numeric strings parsed as numbers
    }

    #[test]
    fn test_md_to_page_nb_row_with_cols() {
        let md = "<!-- nb:row -->\n<!-- nb:col -->\nLeft content\n<!-- nb:col -->\nRight content\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "row");
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_md_to_page_nb_collapse() {
        let md = "<!-- nb:collapse title=\"Details\" expanded -->\nHidden content\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "collapse");
        assert_eq!(arr[0]["title"], "Details");
        assert_eq!(arr[0]["props"]["expanded"], true);
    }

    #[test]
    fn test_md_to_page_nb_badge() {
        let md = "<!-- nb:badge variant=\"success\" -->PASSED<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "badge");
        assert_eq!(arr[0]["content"], "PASSED");
        assert_eq!(arr[0]["props"]["variant"], "success");
    }

    #[test]
    fn test_md_to_page_nb_image() {
        let md = "<!-- nb:image alt=\"diagram\" height=\"300\" -->https://example.com/img.png<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "image");
        assert_eq!(arr[0]["content"], "https://example.com/img.png");
        assert_eq!(arr[0]["props"]["alt"], "diagram");
        assert_eq!(arr[0]["props"]["height"], 300);
    }

    #[test]
    fn test_md_to_page_nb_gap() {
        let md = "Hello\n<!-- nb:gap lg -->\nWorld";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["type"], "markdown");
        assert_eq!(arr[1]["type"], "spacer");
        assert_eq!(arr[1]["props"]["size"], "lg");
        assert_eq!(arr[2]["type"], "markdown");
    }

    #[test]
    fn test_md_to_page_nb_progress() {
        let md = "<!-- nb:progress value=\"0.7\" label=\"Migration\" detail=\"7/10\" -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "progress");
        assert_eq!(arr[0]["props"]["value"], 0.7);
        assert_eq!(arr[0]["props"]["label"], "Migration");
        assert_eq!(arr[0]["props"]["detail"], "7/10");
    }

    #[test]
    fn test_md_to_page_nb_tabs() {
        let md = "<!-- nb:tabs -->\n<!-- nb:tab title=\"Overview\" -->\nOverview content\n<!-- nb:tab title=\"Details\" -->\nDetail content\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "tabs");
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["type"], "tab");
        assert_eq!(children[0]["title"], "Overview");
        assert_eq!(children[1]["type"], "tab");
        assert_eq!(children[1]["title"], "Details");
    }

    #[test]
    fn test_md_to_page_nb_card_with_title() {
        let md = "<!-- nb:card title=\"My Card\" -->\nCard content\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "card");
        assert_eq!(arr[0]["title"], "My Card");
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["type"], "markdown");
    }

    #[test]
    fn test_md_to_page_nb_nested_in_section() {
        let md = "# Report\n<!-- nb:stat label=\"Score\" value=\"95\" -->\nSummary text";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "section");
        assert_eq!(arr[0]["title"], "Report");
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["type"], "stat");
        assert_eq!(children[1]["type"], "markdown");
    }

    #[test]
    fn test_md_to_page_nb_hero() {
        let md = "<!-- nb:hero -->\n# Welcome\nThis is a hero section.\n<!-- nb:end -->";
        let nodes = md_to_page_nodes(md);
        let arr = nodes.as_array().unwrap();
        assert_eq!(arr[0]["type"], "hero");
        let children = arr[0]["children"].as_array().unwrap();
        // Inside hero, # heading is treated as markdown (not a section)
        assert_eq!(children[0]["type"], "markdown");
    }
}
