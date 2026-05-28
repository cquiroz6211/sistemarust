//! transport — TCP localhost adapter for bridging Bevy protocol queues.
//!
//! This crate provides a `TransportPlugin` (Bevy `Plugin`) that spawns background
//! async tokio tasks for TCP communication. Messages are newline-delimited JSON
//! using the existing `EventEnvelope` serde format.
//!
//! # Architecture
//!
//! ```text
//! Bevy ECS (domain)
//!     ↕ flush_outbound / flush_inbound (bridge systems in each app)
//! OutboundSender / InboundReceiver (channel Resources)
//!     ↕ tokio::sync::mpsc (bounded)
//! tokio tasks (reader + writer)
//!     ↕ TCP localhost
//! ```
//!
//! The transport crate never mutates the Bevy `World` directly — all communication
//! passes through bounded channels.

pub mod client;
pub mod framing;
pub mod logging;
pub mod server;
pub mod systems;
pub mod task;

use std::net::SocketAddr;

use bevy::prelude::*;

use systems::TokioRuntime;

// Re-export channel resource types for consumers.
pub use systems::{InboundReceiver, OutboundSender};

/// Default channel capacity for the bounded mpsc channels.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 64;

/// Configuration for the TCP transport.
pub struct TransportConfig {
    /// Socket address to connect to (client) or listen on (server).
    pub address: SocketAddr,
    /// Whether this process is the server/listener.
    pub is_server: bool,
    /// Bounded channel capacity (messages per batch).
    pub channel_capacity: usize,
    /// Human-readable label for this process (e.g., "rpi-controller").
    pub local_label: String,
    /// Human-readable label for the remote process (e.g., "pc-app").
    pub remote_label: String,
}

impl TransportConfig {
    /// Creates a server config that listens on the given address.
    pub fn server(address: SocketAddr) -> Self {
        Self {
            address,
            is_server: true,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            local_label: "rpi-controller".to_string(),
            remote_label: "pc-app".to_string(),
        }
    }

    /// Creates a client config that connects to the given address.
    pub fn client(address: SocketAddr) -> Self {
        Self {
            address,
            is_server: false,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            local_label: "pc-app".to_string(),
            remote_label: "rpi-controller".to_string(),
        }
    }
}

/// Bevy plugin that spawns async transport tasks.
///
/// Must be added AFTER protocol queue resources are registered.
/// Does NOT register bridge systems — each app adds its own bridge systems
/// that connect `OutboundProtocolQueue`/`InboundProtocolQueue` to the channel resources.
pub struct TransportPlugin {
    pub config: TransportConfig,
}

impl Plugin for TransportPlugin {
    fn build(&self, app: &mut App) {
        // Create a dedicated tokio runtime for the transport tasks.
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for transport");

        // Create bounded channels: outbound (Bevy→Network) and inbound (Network→Bevy).
        let (out_tx, out_rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);
        let (in_tx, in_rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);

        let addr = self.config.address;
        let local_label = self.config.local_label.clone();
        let remote_label = self.config.remote_label.clone();

        if self.config.is_server {
            rt.spawn(server::serve(addr, out_rx, in_tx, local_label, remote_label));
        } else {
            rt.spawn(client::connect(addr, out_rx, in_tx, local_label, remote_label));
        }

        // Keep the runtime alive as a Resource — it will be dropped when the app shuts down.
        app.insert_resource(TokioRuntime(rt));

        // Channel handles for bridge systems in each app.
        app.insert_resource(OutboundSender(out_tx));
        app.insert_resource(InboundReceiver(std::sync::Mutex::new(in_rx)));

        eprintln!(
            "[TRANSPORT] {} mode enabled — {}",
            if self.config.is_server { "Server" } else { "Client" },
            addr,
        );
    }
}
