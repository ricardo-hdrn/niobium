// Hub WebSocket client plugin.
//
// Connects to mcp-discuss hub via WebSocket, receives real-time events,
// and converts them into generic Pill events on the Niobium event bus.

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio_tungstenite::tungstenite;
use tracing::{error, info, warn};

use crate::core::event_bus::EventBus;
use crate::core::events::{Event, Pill};
use crate::core::plugin::Plugin;

/// Configuration for connecting to the mcp-discuss hub.
#[derive(Debug, Clone)]
pub struct HubConfig {
    /// WebSocket URL (e.g. "ws://localhost:8787/ws" or "wss://hub.example.com/ws")
    pub url: String,
    /// API key for authentication
    pub api_key: String,
}

impl HubConfig {
    /// Load from environment variables. Returns None if not configured.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("NIOBIUM_HUB_URL").ok()?;
        let api_key = std::env::var("NIOBIUM_HUB_API_KEY").ok()?;
        Some(Self { url, api_key })
    }
}

pub struct HubClientPlugin {
    config: HubConfig,
}

impl HubClientPlugin {
    pub fn new(config: HubConfig) -> Self {
        Self { config }
    }
}

/// Max reconnect delay (60 seconds).
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Base backoff delay (1 second).
const BASE_BACKOFF: Duration = Duration::from_secs(1);

#[async_trait::async_trait]
impl Plugin for HubClientPlugin {
    fn name(&self) -> &str {
        "hub-client"
    }

    async fn start(&self, bus: EventBus) -> anyhow::Result<()> {
        let mut attempt: u32 = 0;

        loop {
            match connect_and_listen(&self.config, &bus).await {
                Ok(()) => {
                    info!("hub-client: disconnected cleanly");
                    break;
                }
                Err(e) => {
                    attempt += 1;
                    let delay = backoff_delay(attempt);

                    warn!(
                        attempt,
                        delay_ms = delay.as_millis(),
                        "hub-client: connection failed: {e} — reconnecting"
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = wait_for_shutdown(&bus) => {
                            info!("hub-client: shutdown during reconnect backoff");
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// ── Hub-specific wire types (private to this plugin) ─────────────────

/// Raw hub WS message — parsed then converted to Pill.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
enum HubMessage {
    #[serde(rename = "update_event")]
    UpdateEvent {
        subject_id: String,
        #[allow(dead_code)]
        event_id: i64,
        source_kind: String,
        source_id: String,
        summary: String,
        #[allow(dead_code)]
        payload_ref: Option<String>,
        created_at: String,
    },

    #[serde(rename = "actionable_update")]
    ActionableUpdate {
        subject_id: String,
        actionable_id: String,
        update_id: i64,
        source_kind: String,
        source_id: String,
        summary: String,
        created_at: String,
        output_type: Option<String>,
        options: Option<Vec<String>>,
        response_url: Option<String>,
        content: Option<Value>,
    },

    #[serde(rename = "actionable_state")]
    ActionableState {
        subject_id: String,
        actionable_id: String,
        old_state: String,
        new_state: String,
    },

    #[serde(rename = "subject_status")]
    SubjectStatus {
        subject_id: String,
        old_status: String,
        new_status: String,
    },
}

impl HubMessage {
    fn into_pill(self) -> Pill {
        match self {
            HubMessage::UpdateEvent {
                subject_id,
                source_kind,
                source_id,
                summary,
                created_at,
                ..
            } => Pill {
                source: "hub".into(),
                summary,
                created_at,
                output_type: None,
                options: None,
                content: None,
                response_url: None,
                meta: Some(serde_json::json!({
                    "event_type": "update_event",
                    "subject_id": subject_id,
                    "source_kind": source_kind,
                    "source_id": source_id,
                })),
            },

            HubMessage::ActionableUpdate {
                subject_id,
                actionable_id,
                update_id,
                source_kind,
                source_id,
                summary,
                created_at,
                output_type,
                options,
                response_url,
                content,
            } => Pill {
                source: "hub".into(),
                summary,
                created_at,
                output_type,
                options,
                content,
                response_url,
                meta: Some(serde_json::json!({
                    "event_type": "actionable_update",
                    "subject_id": subject_id,
                    "actionable_id": actionable_id,
                    "update_id": update_id,
                    "source_kind": source_kind,
                    "source_id": source_id,
                })),
            },

            HubMessage::ActionableState {
                subject_id,
                actionable_id,
                old_state,
                new_state,
            } => Pill {
                source: "hub".into(),
                summary: format!("{old_state} → {new_state}"),
                created_at: String::new(),
                output_type: None,
                options: None,
                content: None,
                response_url: None,
                meta: Some(serde_json::json!({
                    "event_type": "actionable_state",
                    "subject_id": subject_id,
                    "actionable_id": actionable_id,
                    "old_state": old_state,
                    "new_state": new_state,
                })),
            },

            HubMessage::SubjectStatus {
                subject_id,
                old_status,
                new_status,
            } => Pill {
                source: "hub".into(),
                summary: format!("{old_status} → {new_status}"),
                created_at: String::new(),
                output_type: None,
                options: None,
                content: None,
                response_url: None,
                meta: Some(serde_json::json!({
                    "event_type": "subject_status",
                    "subject_id": subject_id,
                    "old_status": old_status,
                    "new_status": new_status,
                })),
            },
        }
    }
}

// ── Connection logic ─────────────────────────────────────────────────

async fn connect_and_listen(config: &HubConfig, bus: &EventBus) -> anyhow::Result<()> {
    let ws_url = &config.url;

    let uri: http::Uri = ws_url.parse()?;
    let host = uri.host().unwrap_or("localhost");
    let request = http::Request::builder()
        .uri(ws_url)
        .header("Host", host)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        .body(())?;

    let (ws_stream, _response) = tokio_tungstenite::connect_async(request).await?;

    info!("hub-client: connected to {ws_url}");

    let (_write, mut read) = ws_stream.split();
    let mut shutdown_rx = bus.subscribe();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        match serde_json::from_str::<HubMessage>(&text) {
                            Ok(hub_msg) => {
                                bus.emit(Event::Pill(hub_msg.into_pill()));
                            }
                            Err(e) => {
                                warn!("hub-client: failed to parse message: {e}");
                            }
                        }
                    }
                    Some(Ok(tungstenite::Message::Ping(_))) => {}
                    Some(Ok(tungstenite::Message::Close(_))) => {
                        info!("hub-client: server closed connection");
                        return Err(anyhow::anyhow!("server closed connection"));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        error!("hub-client: WebSocket error: {e}");
                        return Err(e.into());
                    }
                    None => {
                        return Err(anyhow::anyhow!("WebSocket stream ended"));
                    }
                }
            }
            event = shutdown_rx.recv() => {
                if matches!(event, Ok(Event::Shutdown)) {
                    return Ok(());
                }
            }
        }
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    let exp = BASE_BACKOFF.saturating_mul(1 << attempt.min(6));
    let capped = exp.min(MAX_BACKOFF);
    let jitter_ms = (capped.as_millis() as u64) / 4;
    let jitter = Duration::from_millis(fastrand_u64() % jitter_ms.max(1));
    capped + jitter
}

fn fastrand_u64() -> u64 {
    let mut buf = [0u8; 8];
    let _ = getrandom::fill(&mut buf);
    u64::from_le_bytes(buf)
}

async fn wait_for_shutdown(bus: &EventBus) {
    let mut rx = bus.subscribe();
    loop {
        if let Ok(Event::Shutdown) = rx.recv().await {
            return;
        }
    }
}
