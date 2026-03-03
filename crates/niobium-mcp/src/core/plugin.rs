// Plugin trait for the Niobium runtime.
//
// All capabilities (MCP tools, UI bridge, env sources) are plugins.
// Plugins subscribe to events and emit events through the bus.

use async_trait::async_trait;

use super::event_bus::EventBus;

/// A Niobium plugin. Plugins are started with a reference to the event bus
/// and run until the bus shuts down.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Human-readable name for logging.
    fn name(&self) -> &str;

    /// Start the plugin. This is called once during runtime startup.
    /// The plugin should subscribe to relevant events and begin its work.
    /// This method should run until shutdown.
    async fn start(&self, bus: EventBus) -> anyhow::Result<()>;
}
