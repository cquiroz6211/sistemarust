//! Client-side TCP connector for the transport layer.
//!
//! Connects to a server with a retry loop (2-second interval, 30 max attempts).
//! On successful connection, runs the reader/writer tasks.

use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

use protocol::EventEnvelope;

use crate::task::run_transport_tasks;

/// Maximum number of connection retry attempts.
const MAX_RETRY_ATTEMPTS: u32 = 30;

/// Delay between retry attempts.
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Default server address for the client to connect to.
pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:7000";

/// Connects to a TCP server with retry, then runs transport tasks.
///
/// Retries every 2 seconds for up to 30 attempts (60 seconds total).
/// Logs each retry attempt. On success, delegates to `run_transport_tasks`.
pub async fn connect(
    addr: SocketAddr,
    outbound_rx: mpsc::Receiver<Vec<EventEnvelope>>,
    inbound_tx: mpsc::Sender<Vec<EventEnvelope>>,
    local_label: String,
    remote_label: String,
) {
    let mut attempts = 0;

    loop {
        attempts += 1;

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                eprintln!(
                    "[CLIENT] {} connected to {} ({})",
                    local_label, addr, remote_label
                );
                run_transport_tasks(stream, outbound_rx, inbound_tx, local_label.clone(), remote_label.clone())
                    .await;
                eprintln!("[CLIENT] {} disconnected from {}", local_label, addr);
                return;
            }
            Err(e) => {
                if attempts >= MAX_RETRY_ATTEMPTS {
                    eprintln!(
                        "[CLIENT] {} failed to connect to {} after {} attempts: {}. Giving up.",
                        local_label, addr, attempts, e
                    );
                    return;
                }
                eprintln!(
                    "[CLIENT] {} connection attempt {}/{} to {} failed: {}. Retrying in {}s...",
                    local_label,
                    attempts,
                    MAX_RETRY_ATTEMPTS,
                    addr,
                    e,
                    RETRY_DELAY.as_secs()
                );
                sleep(RETRY_DELAY).await;
            }
        }
    }
}
