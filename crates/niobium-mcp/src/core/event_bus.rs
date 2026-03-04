// Async event bus using tokio broadcast channels.
//
// The bus is the central nervous system of Niobium.
// Plugins publish and subscribe to typed events.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast, oneshot};
use tracing::debug;

use super::events::{Event, RequestId};

/// Channel capacity for the broadcast bus.
const BUS_CAPACITY: usize = 256;

/// The event bus. Clone-friendly (wraps Arc internals).
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    /// Pending request/response pairs. When a plugin emits a request event
    /// (e.g. ShowForm), it registers a oneshot channel here. When the
    /// response event arrives, the bus completes the oneshot.
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Event>>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Self {
            sender,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Publish an event to all subscribers.
    pub fn emit(&self, event: Event) {
        debug!(?event, "bus: emit");
        // Ignore send error (no active receivers is fine)
        let _ = self.sender.send(event);
    }

    /// Subscribe to the event stream. Returns a receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Emit a request event and wait for a matching response.
    ///
    /// This is the core pattern for blocking MCP tool calls:
    ///   1. Register a oneshot channel keyed by request_id
    ///   2. Emit the request event (e.g. ShowForm)
    ///   3. Await the oneshot — some other plugin (UI bridge) will
    ///      receive the request, do its thing, and emit a response
    ///   4. The bus's routing loop matches the response to the oneshot
    pub async fn request(&self, request_id: RequestId, event: Event) -> Option<Event> {
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(request_id, tx);
        }

        self.emit(event);

        // Await the response
        rx.await.ok()
    }

    /// Route a response event to its pending request.
    /// Called by the bus routing loop when a response event arrives.
    pub async fn resolve(&self, request_id: RequestId, event: Event) {
        let tx = {
            let mut pending = self.pending.lock().await;
            pending.remove(&request_id)
        };

        if let Some(tx) = tx {
            debug!(?request_id, "bus: resolve");
            let _ = tx.send(event);
        }
    }

    /// Check if an event is a response type that the router should resolve.
    fn is_response(event: &Event) -> bool {
        matches!(
            event,
            Event::FormSubmitted { .. }
                | Event::FormCancelled { .. }
                | Event::Confirmed { .. }
                | Event::OutputDismissed { .. }
        )
    }

    /// Start the routing loop that matches response events to pending requests.
    /// Run this as a background task.
    pub async fn run_router(self) {
        let rx = self.subscribe();
        self.run_router_with(rx).await;
    }

    /// Start the routing loop with a pre-created receiver.
    /// Useful when the caller needs to guarantee the subscription exists
    /// before any events are emitted (avoids race conditions in tests).
    pub async fn run_router_with(self, mut rx: broadcast::Receiver<Event>) {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let is_shutdown = matches!(event, Event::Shutdown);

                    // Route response events to their pending requests
                    if Self::is_response(&event)
                        && let Some(id) = event.request_id()
                    {
                        self.resolve(id, event).await;
                    }

                    if is_shutdown {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(n, "bus router lagged — missed events");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_emit_and_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.emit(Event::Shutdown);

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, Event::Shutdown));
    }

    #[tokio::test]
    async fn test_multiple_subscribers_receive_event() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let rid = Uuid::new_v4();
        bus.emit(Event::FormCancelled { request_id: rid });

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.request_id(), Some(rid));
        assert_eq!(e2.request_id(), Some(rid));
    }

    /// Helper: set up a bus with router + responder, all subscribed before
    /// any events flow (avoids race conditions).
    fn setup_bus() -> (
        EventBus,
        broadcast::Receiver<Event>,
        broadcast::Receiver<Event>,
    ) {
        let bus = EventBus::new();
        let router_rx = bus.subscribe();
        let responder_rx = bus.subscribe();
        (bus, router_rx, responder_rx)
    }

    #[tokio::test]
    async fn test_request_response_pairing() {
        let (bus, router_rx, mut responder_rx) = setup_bus();
        let request_id = Uuid::new_v4();

        // Router with pre-created receiver
        let router_bus = bus.clone();
        tokio::spawn(async move { router_bus.run_router_with(router_rx).await });

        // Responder with pre-created receiver
        let responder_bus = bus.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Event::ShowForm { request_id, .. }) = responder_rx.recv().await {
                    responder_bus.emit(Event::FormSubmitted {
                        request_id,
                        data: json!({"name": "Alice"}),
                    });
                    break;
                }
            }
        });

        let response = bus
            .request(
                request_id,
                Event::ShowForm {
                    request_id,
                    schema: json!({"type": "object"}),
                    title: "Test".to_string(),
                    prefill: None,
                    width: None,
                    height: None,
                    density: None,
                    animate: None,
                    accent: None,
                },
            )
            .await;

        let response = response.expect("should get a response");
        match response {
            Event::FormSubmitted { data, .. } => {
                assert_eq!(data, json!({"name": "Alice"}));
            }
            other => panic!("expected FormSubmitted, got {other:?}"),
        }

        bus.emit(Event::Shutdown);
    }

    #[tokio::test]
    async fn test_request_response_confirmation() {
        let (bus, router_rx, mut responder_rx) = setup_bus();
        let request_id = Uuid::new_v4();

        let router_bus = bus.clone();
        tokio::spawn(async move { router_bus.run_router_with(router_rx).await });

        let responder_bus = bus.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Event::ShowConfirmation { request_id, .. }) = responder_rx.recv().await {
                    responder_bus.emit(Event::Confirmed {
                        request_id,
                        value: true,
                    });
                    break;
                }
            }
        });

        let response = bus
            .request(
                request_id,
                Event::ShowConfirmation {
                    request_id,
                    message: "Continue?".to_string(),
                    title: "Confirm".to_string(),
                    width: None,
                    height: None,
                    accent: None,
                },
            )
            .await;

        match response.expect("should get a response") {
            Event::Confirmed { value, .. } => assert!(value),
            other => panic!("expected Confirmed, got {other:?}"),
        }

        bus.emit(Event::Shutdown);
    }

    #[tokio::test]
    async fn test_cancellation_response() {
        let (bus, router_rx, mut responder_rx) = setup_bus();
        let request_id = Uuid::new_v4();

        let router_bus = bus.clone();
        tokio::spawn(async move { router_bus.run_router_with(router_rx).await });

        let responder_bus = bus.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Event::ShowForm { request_id, .. }) = responder_rx.recv().await {
                    responder_bus.emit(Event::FormCancelled { request_id });
                    break;
                }
            }
        });

        let response = bus
            .request(
                request_id,
                Event::ShowForm {
                    request_id,
                    schema: json!({"type": "object"}),
                    title: "Test".to_string(),
                    prefill: None,
                    width: None,
                    height: None,
                    density: None,
                    animate: None,
                    accent: None,
                },
            )
            .await;

        assert!(matches!(response, Some(Event::FormCancelled { .. })));

        bus.emit(Event::Shutdown);
    }

    #[tokio::test]
    async fn test_router_stops_on_shutdown() {
        let bus = EventBus::new();
        let router_rx = bus.subscribe(); // Subscribe BEFORE spawn

        let router_bus = bus.clone();
        let handle = tokio::spawn(async move { router_bus.run_router_with(router_rx).await });

        bus.emit(Event::Shutdown);

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("router should stop within 1s")
            .expect("router task should not panic");
    }

    #[test]
    fn test_is_response_classification() {
        let rid = Uuid::new_v4();

        // Response events
        assert!(EventBus::is_response(&Event::FormSubmitted {
            request_id: rid,
            data: json!({}),
        }));
        assert!(EventBus::is_response(&Event::FormCancelled {
            request_id: rid,
        }));
        assert!(EventBus::is_response(&Event::Confirmed {
            request_id: rid,
            value: true,
        }));
        assert!(EventBus::is_response(&Event::OutputDismissed {
            request_id: rid,
        }));

        // Non-response events
        assert!(!EventBus::is_response(&Event::ShowForm {
            request_id: rid,
            schema: json!({}),
            title: "t".into(),
            prefill: None,
            width: None,
            height: None,
            density: None,
            animate: None,
            accent: None,
        }));
        assert!(!EventBus::is_response(&Event::ShowConfirmation {
            request_id: rid,
            message: "m".into(),
            title: "t".into(),
            width: None,
            height: None,
            accent: None,
        }));
        assert!(!EventBus::is_response(&Event::ShowOutput {
            request_id: rid,
            content: "test".into(),
            output_type: "text".into(),
            title: "t".into(),
            width: None,
            height: None,
            accent: None,
        }));
        assert!(!EventBus::is_response(&Event::Shutdown));
    }
}
