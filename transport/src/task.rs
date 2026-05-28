//! Reader and writer tokio tasks for TCP transport.
//!
//! `run_transport_tasks` splits a `TcpStream` into read/write halves and spawns
//! two async tasks. The writer drains the outbound channel and writes JSON lines
//! to the socket. The reader reads JSON lines from the socket and sends
//! deserialized envelopes through the inbound channel.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use protocol::EventEnvelope;

use crate::framing::{parse_json_line, serialize_envelope};
use crate::logging::{log_rx, log_tx};

/// Spawns reader and writer tasks on the given TCP stream.
///
/// Returns when EITHER task finishes (disconnect or error).
/// The caller should drop channel handles to signal shutdown.
pub async fn run_transport_tasks(
    stream: TcpStream,
    outbound_rx: mpsc::Receiver<Vec<EventEnvelope>>,
    inbound_tx: mpsc::Sender<Vec<EventEnvelope>>,
    _local_label: String,
    _remote_label: String,
) {
    let peer_addr = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let (read_half, write_half) = tokio::io::split(stream);

    let reader = tokio::spawn(read_task(read_half, inbound_tx));
    let writer = tokio::spawn(write_task(write_half, outbound_rx));

    // Wait for either task to finish — on disconnect, the other will fail soon too.
    tokio::select! {
        r = reader => {
            match r {
                Ok(()) => eprintln!("[TRANSPORT] Reader finished for peer {}", peer_addr),
                Err(e) => eprintln!("[TRANSPORT] Reader error for peer {}: {}", peer_addr, e),
            }
        }
        r = writer => {
            match r {
                Ok(()) => eprintln!("[TRANSPORT] Writer finished for peer {}", peer_addr),
                Err(e) => eprintln!("[TRANSPORT] Writer error for peer {}: {}", peer_addr, e),
            }
        }
    }
}

/// Reader half: reads JSON lines from TCP → deserializes → sends to inbound channel.
async fn read_task(
    read_half: tokio::io::ReadHalf<TcpStream>,
    inbound_tx: mpsc::Sender<Vec<EventEnvelope>>,
) {
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF — peer disconnected
                break;
            }
            Ok(_) => {
                if let Some(envelope) = parse_json_line(&line) {
                    log_rx(&envelope);
                    // Send batch of 1 — simple and preserves ordering
                    if inbound_tx.send(vec![envelope]).await.is_err() {
                        // Channel closed — Bevy side shut down
                        break;
                    }
                }
                // Malformed lines are already logged by parse_json_line; continue
            }
            Err(e) => {
                eprintln!("[TRANSPORT] Socket read error: {}", e);
                break;
            }
        }
    }
}

/// Writer half: receives envelope batches from outbound channel → serializes → writes to TCP.
async fn write_task(
    write_half: tokio::io::WriteHalf<TcpStream>,
    mut outbound_rx: mpsc::Receiver<Vec<EventEnvelope>>,
) {
    let mut writer = write_half;

    while let Some(batch) = outbound_rx.recv().await {
        for envelope in &batch {
            log_tx(envelope);
            let json = serialize_envelope(envelope);
            if let Err(e) = writer.write_all(json.as_bytes()).await {
                eprintln!("[TRANSPORT] Socket write error: {}", e);
                return;
            }
            if let Err(e) = writer.write_all(b"\n").await {
                eprintln!("[TRANSPORT] Socket write error: {}", e);
                return;
            }
        }
        if let Err(e) = writer.flush().await {
            eprintln!("[TRANSPORT] Socket flush error: {}", e);
            return;
        }
    }
}
