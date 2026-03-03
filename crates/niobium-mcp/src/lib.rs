//! Niobium MCP server — native GUI runtime for CLI AI agents.
//!
//! This library exposes the core MCP server logic, event bus, schema store,
//! and FFI API for flutter_rust_bridge integration.

pub mod config;
pub mod core;
pub mod error;
pub mod plugins;
pub mod schema_store;
pub mod server;

// FFI API for flutter_rust_bridge
pub mod api;
