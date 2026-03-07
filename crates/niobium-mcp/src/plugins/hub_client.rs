// Hub WebSocket client plugin.
//
// Connects to mcp-discuss hub via WebSocket, receives real-time events,
// and emits them on the Niobium event bus for the UI layer.

use std::time::Duration;

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite;
use tracing::{error, info, warn};

use crate::core::event_bus::EventBus;
use crate::core::events::{Event, HubConnectionState, HubEvent};
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
                    // Clean disconnect (shutdown signal)
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

                    bus.emit(Event::HubConnectionState(
                        HubConnectionState::Reconnecting { attempt },
                    ));

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

async fn connect_and_listen(config: &HubConfig, bus: &EventBus) -> anyhow::Result<()> {
    let ws_url = &config.url;

    // Build request with auth header
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
    bus.emit(Event::HubConnectionState(HubConnectionState::Connected));

    let (_write, mut read) = ws_stream.split();

    let mut shutdown_rx = bus.subscribe();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        match serde_json::from_str::<HubEvent>(&text) {
                            Ok(event) => {
                                bus.emit(Event::HubEvent(event));
                            }
                            Err(e) => {
                                warn!("hub-client: failed to parse hub event: {e}");
                            }
                        }
                    }
                    Some(Ok(tungstenite::Message::Ping(_))) => {
                        // tungstenite auto-responds with pong
                    }
                    Some(Ok(tungstenite::Message::Close(_))) => {
                        info!("hub-client: server closed connection");
                        bus.emit(Event::HubConnectionState(HubConnectionState::Disconnected));
                        return Err(anyhow::anyhow!("server closed connection"));
                    }
                    Some(Ok(_)) => {
                        // Ignore binary, pong, frame
                    }
                    Some(Err(e)) => {
                        error!("hub-client: WebSocket error: {e}");
                        bus.emit(Event::HubConnectionState(HubConnectionState::Disconnected));
                        return Err(e.into());
                    }
                    None => {
                        // Stream ended
                        bus.emit(Event::HubConnectionState(HubConnectionState::Disconnected));
                        return Err(anyhow::anyhow!("WebSocket stream ended"));
                    }
                }
            }
            event = shutdown_rx.recv() => {
                if matches!(event, Ok(Event::Shutdown)) {
                    bus.emit(Event::HubConnectionState(HubConnectionState::Disconnected));
                    return Ok(());
                }
            }
        }
    }
}

/// Exponential backoff with jitter, capped at MAX_BACKOFF.
fn backoff_delay(attempt: u32) -> Duration {
    let exp = BASE_BACKOFF.saturating_mul(1 << attempt.min(6));
    let capped = exp.min(MAX_BACKOFF);
    // Add ~25% jitter
    let jitter_ms = (capped.as_millis() as u64) / 4;
    let jitter = Duration::from_millis(fastrand_u64() % jitter_ms.max(1));
    capped + jitter
}

/// Simple u64 from getrandom (no extra dep, already in workspace).
fn fastrand_u64() -> u64 {
    let mut buf = [0u8; 8];
    // Best-effort; if getrandom fails, no jitter
    let _ = getrandom::fill(&mut buf);
    u64::from_le_bytes(buf)
}

/// Wait for a shutdown event on the bus.
async fn wait_for_shutdown(bus: &EventBus) {
    let mut rx = bus.subscribe();
    loop {
        if let Ok(Event::Shutdown) = rx.recv().await {
            return;
        }
    }
}
