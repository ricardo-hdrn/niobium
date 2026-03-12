//! FFI API functions exposed to Dart via flutter_rust_bridge.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use flutter_rust_bridge::DartFnFuture;

/// Initialize logging for the Rust side.
/// Call once from Dart before starting the MCP server.
pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

/// Start the MCP server with Dart callbacks for UI operations.
///
/// - `show_form`: Called when the agent requests a form. Receives JSON string
///   with {schema, title, prefill}. Return JSON string with form data, or null if cancelled.
/// - `show_confirm`: Called when the agent requests confirmation. Receives JSON string
///   with {message, title}. Return true if confirmed.
/// - `show_toast`: Called for toast notifications. Receives JSON string
///   with {message, severity}. Fire-and-forget.
/// - `show_output`: Called to display rich output. Receives JSON string
///   with {content, output_type, title}. Return true when dismissed.
/// - `show_page`: Called when the agent requests a page. Receives JSON string
///   with {children, title, prefill}. Return JSON string with input data, or null if cancelled.
///
/// This function blocks until the MCP server shuts down (stdin closes).
pub async fn start_mcp_server(
    show_form: impl Fn(String) -> DartFnFuture<Option<String>> + Send + Sync + 'static,
    show_confirm: impl Fn(String) -> DartFnFuture<bool> + Send + Sync + 'static,
    show_toast: impl Fn(String) -> DartFnFuture<()> + Send + Sync + 'static,
    show_output: impl Fn(String) -> DartFnFuture<bool> + Send + Sync + 'static,
    on_pill: impl Fn(String) -> DartFnFuture<()> + Send + Sync + 'static,
    show_page: impl Fn(String) -> DartFnFuture<Option<String>> + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let show_form: niobium_mcp::api::ShowFormFn = Arc::new(move |payload: String| {
        Box::pin(show_form(payload)) as Pin<Box<dyn Future<Output = Option<String>> + Send>>
    });

    let show_confirm: niobium_mcp::api::ShowConfirmFn = Arc::new(move |payload: String| {
        Box::pin(show_confirm(payload)) as Pin<Box<dyn Future<Output = bool> + Send>>
    });

    let show_toast: niobium_mcp::api::ShowToastFn = Arc::new(move |payload: String| {
        Box::pin(show_toast(payload)) as Pin<Box<dyn Future<Output = ()> + Send>>
    });

    let show_output: niobium_mcp::api::ShowOutputFn = Arc::new(move |payload: String| {
        Box::pin(show_output(payload)) as Pin<Box<dyn Future<Output = bool> + Send>>
    });

    let on_pill: niobium_mcp::api::OnPillFn = Arc::new(move |payload: String| {
        Box::pin(on_pill(payload)) as Pin<Box<dyn Future<Output = ()> + Send>>
    });

    let show_page: niobium_mcp::api::ShowPageFn = Arc::new(move |payload: String| {
        Box::pin(show_page(payload)) as Pin<Box<dyn Future<Output = Option<String>> + Send>>
    });

    niobium_mcp::api::start_mcp_server_ffi(
        show_form,
        show_confirm,
        show_toast,
        show_output,
        on_pill,
        show_page,
    )
    .await
}

/// Get the version of the Niobium MCP server.
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
