//! Server-side TCP listener for the transport layer.
//!
//! Binds a `TcpListener`, accepts one client connection, and runs the
//! reader/writer tasks. After a client disconnects, the server loops
//! and waits for a new connection (useful for demo scenarios).

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::sync::mpsc;

use protocol::EventEnvelope;

use crate::task::run_transport_tasks;

/// Default bind address for the server.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:7000";

/// Runs the TCP server loop: bind → accept → run transport tasks → repeat.
///
/// This function runs indefinitely until the process is terminated or channels
/// are dropped.
pub async fn serve(
    addr: SocketAddr,
    outbound_rx: mpsc::Receiver<Vec<EventEnvelope>>,
    inbound_tx: mpsc::Sender<Vec<EventEnvelope>>,
    local_label: String,
    remote_label: String,
) {
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => {
            eprintln!("[SERVER] Listening on {}", addr);
            l
        }
        Err(e) => {
            eprintln!("[SERVER] Failed to bind {}: {}", addr, e);
            return;
        }
    };

    // Re-create channels for each accepted connection.
    // The original outbound_rx/inbound_tx are moved into the first connection.
    // For simplicity, v1 only accepts one connection and exits on disconnect.
    let mut outbound_rx = Some(outbound_rx);
    let mut inbound_tx = Some(inbound_tx);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[SERVER] Accept error: {}", e);
                continue;
            }
        };

        eprintln!(
            "[SERVER] {} accepted connection from {} ({})",
            local_label, peer_addr, remote_label
        );

        let out_rx = outbound_rx.take();
        let in_tx = inbound_tx.take();

        match (out_rx, in_tx) {
            (Some(out_rx), Some(in_tx)) => {
                let local = local_label.clone();
        let remote = remote_label.clone();
        run_transport_tasks(stream, out_rx, in_tx, local, remote).await;
            }
            _ => {
                eprintln!("[SERVER] Channel handles already consumed, closing connection");
                break;
            }
        }

        eprintln!("[SERVER] Client {} disconnected", peer_addr);
        // v1: exit loop after first disconnect (channels consumed)
        break;
    }

    eprintln!("[SERVER] {} server task exiting", local_label);
}
