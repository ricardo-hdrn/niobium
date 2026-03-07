//! FFI API for flutter_rust_bridge integration.
//!
//! These functions are exposed to Dart via FRB codegen.
//! The Flutter app calls `start_mcp_server()` to run the MCP server
//! in-process, passing Dart callbacks for UI operations.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tracing::info;

use rmcp::ServiceExt;

use crate::config::Config;
use crate::core::event_bus::EventBus;
use crate::core::events::Event;
use crate::core::plugin::Plugin;
use crate::schema_store::SchemaStore;
use crate::server::NiobiumServer;

/// Callback type for showing a form. Receives JSON string, returns JSON string.
pub type ShowFormFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>;

/// Callback type for showing a confirmation. Receives JSON string, returns bool.
pub type ShowConfirmFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

/// Callback type for showing a toast notification. Receives JSON string (fire-and-forget).
pub type ShowToastFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Callback type for showing rich output. Receives JSON string, returns bool (dismissed).
pub type ShowOutputFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

/// Callback type for hub events. Receives JSON string (fire-and-forget).
pub type OnHubEventFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// FFI bridge plugin that calls Dart functions directly instead of HTTP.
struct FfiBridgePlugin {
    show_form: ShowFormFn,
    show_confirm: ShowConfirmFn,
    show_toast: ShowToastFn,
    show_output: ShowOutputFn,
    on_hub_event: OnHubEventFn,
}

#[async_trait::async_trait]
impl Plugin for FfiBridgePlugin {
    fn name(&self) -> &str {
        "ffi-bridge"
    }

    async fn start(&self, bus: EventBus) -> anyhow::Result<()> {
        let mut rx = bus.subscribe();
        info!("ffi-bridge plugin started");

        loop {
            match rx.recv().await {
                Ok(Event::ShowForm {
                    request_id,
                    schema,
                    title,
                    prefill,
                    width,
                    height,
                    density,
                    animate,
                    accent,
                }) => {
                    let mut payload = serde_json::json!({
                        "schema": schema,
                        "title": title,
                        "prefill": prefill,
                    });
                    if let Some(w) = width {
                        payload["width"] = serde_json::json!(w);
                    }
                    if let Some(h) = height {
                        payload["height"] = serde_json::json!(h);
                    }
                    if let Some(d) = density {
                        payload["density"] = serde_json::json!(d);
                    }
                    if let Some(a) = animate {
                        payload["animate"] = serde_json::json!(a);
                    }
                    if let Some(ac) = accent {
                        payload["accent"] = serde_json::json!(ac);
                    }
                    let payload_str = serde_json::to_string(&payload).unwrap();

                    let result = (self.show_form)(payload_str).await;

                    match result {
                        Some(json_str) => {
                            let data: Value =
                                serde_json::from_str(&json_str).unwrap_or(Value::Null);
                            bus.emit(Event::FormSubmitted { request_id, data });
                        }
                        None => {
                            bus.emit(Event::FormCancelled { request_id });
                        }
                    }
                }

                Ok(Event::ShowConfirmation {
                    request_id,
                    message,
                    title,
                    width,
                    height,
                    accent,
                }) => {
                    let mut payload = serde_json::json!({
                        "message": message,
                        "title": title,
                    });
                    if let Some(w) = width {
                        payload["width"] = serde_json::json!(w);
                    }
                    if let Some(h) = height {
                        payload["height"] = serde_json::json!(h);
                    }
                    if let Some(ac) = accent {
                        payload["accent"] = serde_json::json!(ac);
                    }
                    let payload_str = serde_json::to_string(&payload).unwrap();

                    let value = (self.show_confirm)(payload_str).await;
                    bus.emit(Event::Confirmed { request_id, value });
                }

                Ok(Event::PipeEvent(niobium_pipe::PipeEvent::Toast { message, severity })) => {
                    let sev_str = match severity {
                        niobium_pipe::Severity::Info => "info",
                        niobium_pipe::Severity::Success => "success",
                        niobium_pipe::Severity::Warning => "warning",
                        niobium_pipe::Severity::Error => "error",
                    };
                    let payload = serde_json::json!({
                        "message": message,
                        "severity": sev_str,
                    });
                    let payload_str = serde_json::to_string(&payload).unwrap();
                    (self.show_toast)(payload_str).await;
                }

                Ok(Event::ShowOutput {
                    request_id,
                    content,
                    output_type,
                    title,
                    width,
                    height,
                    accent,
                }) => {
                    let mut payload = serde_json::json!({
                        "content": content,
                        "output_type": output_type,
                        "title": title,
                    });
                    if let Some(w) = width {
                        payload["width"] = serde_json::json!(w);
                    }
                    if let Some(h) = height {
                        payload["height"] = serde_json::json!(h);
                    }
                    if let Some(ac) = accent {
                        payload["accent"] = serde_json::json!(ac);
                    }
                    let payload_str = serde_json::to_string(&payload).unwrap();

                    let _ = (self.show_output)(payload_str).await;
                    bus.emit(Event::OutputDismissed { request_id });
                }

                Ok(Event::HubEvent(hub_event)) => {
                    if let Ok(json) = serde_json::to_string(&hub_event) {
                        (self.on_hub_event)(json).await;
                    }
                }

                Ok(Event::Shutdown) => {
                    info!("ffi-bridge: shutting down");
                    break;
                }

                Ok(_) => {}

                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(n, "ffi-bridge: lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }

        Ok(())
    }
}

/// Start the MCP server in-process with FFI callbacks for UI.
///
/// This is the main entry point when running as a library inside Flutter.
/// The MCP server runs on the tokio runtime, communicating with agents
/// over stdio and calling Dart for UI via the provided callbacks.
pub async fn start_mcp_server_ffi(
    show_form: ShowFormFn,
    show_confirm: ShowConfirmFn,
    show_toast: ShowToastFn,
    show_output: ShowOutputFn,
    on_hub_event: OnHubEventFn,
) -> anyhow::Result<()> {
    let config = Config::load();

    // Core: Event bus
    let bus = EventBus::new();

    // Plugin: Schema store
    let store = SchemaStore::open(&config.db_path())?;

    // Plugin: FFI bridge (replaces HTTP bridge)
    let ffi_bridge = FfiBridgePlugin {
        show_form,
        show_confirm,
        show_toast,
        show_output,
        on_hub_event,
    };
    let ffi_bus = bus.clone();
    tokio::spawn(async move {
        if let Err(e) = ffi_bridge.start(ffi_bus).await {
            tracing::error!("ffi-bridge plugin failed: {e}");
        }
    });

    // Plugin: Hub WebSocket client (if configured)
    if let Some(hub_config) = crate::plugins::hub_client::HubConfig::from_env() {
        let hub_plugin = crate::plugins::hub_client::HubClientPlugin::new(hub_config);
        let hub_bus = bus.clone();
        tokio::spawn(async move {
            if let Err(e) = hub_plugin.start(hub_bus).await {
                tracing::error!("hub-client plugin failed: {e}");
            }
        });
        info!("hub-client plugin started (NIOBIUM_HUB_URL configured)");
    }

    // Event bus router
    let router_bus = bus.clone();
    tokio::spawn(async move {
        router_bus.run_router().await;
    });

    // MCP server on stdio
    let server = NiobiumServer::new(bus.clone(), store);
    let service = server.serve(rmcp::transport::stdio()).await?;
    info!("niobium MCP server running on stdio (FFI mode)");

    service.waiting().await?;

    // Shutdown
    bus.emit(Event::Shutdown);
    info!("niobium shutting down");

    Ok(())
}

/// Start the MCP server in headless mode (no UI — forms always cancel).
///
/// Used by the standalone binary when no Flutter app is available,
/// and by E2E tests that only exercise schema storage tools.
pub async fn start_mcp_server_headless() -> anyhow::Result<()> {
    let show_form: ShowFormFn = Arc::new(|_| Box::pin(async { None }));
    let show_confirm: ShowConfirmFn = Arc::new(|_| Box::pin(async { false }));
    let show_toast: ShowToastFn = Arc::new(|_| Box::pin(async {}));
    let show_output: ShowOutputFn = Arc::new(|_| Box::pin(async { true }));
    let on_hub_event: OnHubEventFn = Arc::new(|_| Box::pin(async {}));

    start_mcp_server_ffi(
        show_form,
        show_confirm,
        show_toast,
        show_output,
        on_hub_event,
    )
    .await
}
