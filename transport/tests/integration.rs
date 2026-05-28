//! Integration tests for the transport crate.
//!
//! These tests verify TCP transport behavior: framing round-trips over the wire,
//! malformed line handling, and bidirectional message flow.

use std::time::Duration;

use protocol::{
    EventEnvelope, Message, OvenState, OvenStatusUpdatedPayload, SetTargetTemperaturePayload,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use transport::framing::{parse_json_line, serialize_envelope};
use transport::task::run_transport_tasks;

/// Helper: creates a sample envelope for testing.
fn sample_envelope() -> EventEnvelope {
    EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        }),
    )
}

fn sample_envelope_status() -> EventEnvelope {
    EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 200.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        }),
    )
}

// ── Framing over TCP ─────────────────────────────────────────────────────────

#[tokio::test]
async fn envelope_roundtrip_over_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let env = sample_envelope();

    // Spawn a simple echo server
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read_half, write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);
        let mut writer = write_half;

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        writer.write_all(line.as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.flush().await.unwrap();
    });

    // Client sends envelope and reads it back
    let stream = TcpStream::connect(addr).await.unwrap();
    let (read_half, write_half) = tokio::io::split(stream);
    let mut writer = write_half;
    let mut reader = BufReader::new(read_half);

    let json = serialize_envelope(&env);
    writer.write_all(json.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();

    let mut response = String::new();
    reader.read_line(&mut response).await.unwrap();

    let parsed = parse_json_line(&response);
    assert!(parsed.is_some());
    assert_eq!(parsed.unwrap(), env);

    server.await.unwrap();
}

// ── Transport task round-trip via channels ───────────────────────────────────

#[tokio::test]
async fn server_client_roundtrip_via_tasks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server side: outbound_rx (receives nothing from empty channel), inbound_tx (sends to test)
    let (_, server_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (server_in_tx, mut server_in_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);

    // Client side: outbound from test, inbound to nowhere
    let (client_out_tx, client_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (client_in_tx, _) = mpsc::channel::<Vec<EventEnvelope>>(64);

    // Spawn server transport task
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        run_transport_tasks(
            stream,
            server_out_rx,
            server_in_tx,
            "rpi-controller".to_string(),
            "pc-app".to_string(),
        )
        .await;
    });

    // Give server a moment to start accepting
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Spawn client transport task
    let client_stream = TcpStream::connect(addr).await.unwrap();
    let client_handle = tokio::spawn(async move {
        run_transport_tasks(
            client_stream,
            client_out_rx,
            client_in_tx,
            "pc-app".to_string(),
            "rpi-controller".to_string(),
        )
        .await;
    });

    // Give client a moment to connect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client sends an envelope
    let env = sample_envelope();
    client_out_tx.send(vec![env.clone()]).await.unwrap();

    // Server should receive it
    let result = tokio::time::timeout(Duration::from_secs(2), server_in_rx.recv()).await;
    assert!(result.is_ok(), "Server should receive envelope from client");
    let batch = result.unwrap().unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0], env);

    // Clean shutdown
    drop(client_out_tx);

    let _ = tokio::time::timeout(Duration::from_secs(2), client_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

// ── Malformed line handling ──────────────────────────────────────────────────

#[tokio::test]
async fn malformed_line_does_not_crash_reader() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (_, server_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (server_in_tx, mut server_in_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);

    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        run_transport_tasks(
            stream,
            server_out_rx,
            server_in_tx,
            "server".to_string(),
            "client".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client connects manually and writes garbage + valid JSON
    let mut stream = TcpStream::connect(addr).await.unwrap();

    // Send a malformed line
    stream.write_all(b"this is not json\n").await.unwrap();

    // Send a valid envelope
    let env = sample_envelope();
    let json = serialize_envelope(&env);
    stream.write_all(json.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();

    // Server should receive only the valid envelope
    let result = tokio::time::timeout(Duration::from_secs(2), server_in_rx.recv()).await;
    assert!(result.is_ok(), "Server should receive valid envelope after malformed line");
    let batch = result.unwrap().unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0], env);

    drop(stream);
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

// ── Multiple envelopes in batch ──────────────────────────────────────────────

#[tokio::test]
async fn multiple_envelopes_in_batch() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (_, server_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (server_in_tx, mut server_in_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);

    let (client_out_tx, client_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (client_in_tx, _) = mpsc::channel::<Vec<EventEnvelope>>(64);

    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        run_transport_tasks(
            stream,
            server_out_rx,
            server_in_tx,
            "server".to_string(),
            "client".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client_stream = TcpStream::connect(addr).await.unwrap();
    let client_handle = tokio::spawn(async move {
        run_transport_tasks(
            client_stream,
            client_out_rx,
            client_in_tx,
            "client".to_string(),
            "server".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send a batch with two envelopes
    let env1 = sample_envelope();
    let env2 = sample_envelope_status();
    client_out_tx
        .send(vec![env1.clone(), env2.clone()])
        .await
        .unwrap();

    // The reader sends each envelope individually, so we receive two batches of 1.
    let result1 = tokio::time::timeout(Duration::from_secs(2), server_in_rx.recv()).await;
    assert!(result1.is_ok());
    let batch1 = result1.unwrap().unwrap();
    assert_eq!(batch1.len(), 1);
    assert_eq!(batch1[0], env1);

    let result2 = tokio::time::timeout(Duration::from_secs(2), server_in_rx.recv()).await;
    assert!(result2.is_ok());
    let batch2 = result2.unwrap().unwrap();
    assert_eq!(batch2.len(), 1);
    assert_eq!(batch2[0], env2);

    drop(client_out_tx);

    let _ = tokio::time::timeout(Duration::from_secs(2), client_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

// ── Client retry on connection failure ───────────────────────────────────────
//
// Strategy: bind a listener to get a free port, then DROP it so the client
// gets ConnectionRefused. After a short delay, bind again on the SAME port
// and accept. The client must retry and eventually succeed.
//
// Because the retry delay is 2s, this test takes ~3s total (1 failed attempt
// + sleep + successful retry).

#[tokio::test]
async fn client_retries_on_connection_refused_then_succeeds() {
    // Step 1: Reserve a port, then release it so client's first connect fails.
    let tmp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = tmp_listener.local_addr().unwrap();
    drop(tmp_listener);

    // Channels for the client side.
    let (client_out_tx, client_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (client_in_tx, _) = mpsc::channel::<Vec<EventEnvelope>>(64);

    // Spawn the client connect loop in the background.
    // It will fail on the first attempt, log a retry, sleep 2s, then try again.
    let client_task = tokio::spawn(async move {
        transport::client::connect(
            addr,
            client_out_rx,
            client_in_tx,
            "pc-app".to_string(),
            "rpi-controller".to_string(),
        )
        .await;
    });

    // Step 2: Wait long enough for the first attempt to fail (near-instant)
    // but NOT long enough for the retry delay (2s) to elapse.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // The client task should still be running (sleeping before retry).
    assert!(!client_task.is_finished(), "client should still be retrying");

    // Step 3: Start the real server on the same port.
    // Retry bind a few times in case the OS hasn't fully released the port.
    let server_listener = {
        let mut listener = None;
        for _ in 0..10 {
            match TcpListener::bind(addr).await {
                Ok(l) => {
                    listener = Some(l);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
        listener.expect("should be able to rebind the port after dropping")
    };

    let (_, server_out_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);
    let (server_in_tx, mut server_in_rx) = mpsc::channel::<Vec<EventEnvelope>>(64);

    let server_task = tokio::spawn(async move {
        let (stream, _peer) = server_listener.accept().await.unwrap();
        run_transport_tasks(
            stream,
            server_out_rx,
            server_in_tx,
            "rpi-controller".to_string(),
            "pc-app".to_string(),
        )
        .await;
    });

    // Step 4: Wait for the retry delay to elapse and client to connect (2s + margin).
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Step 5: Send an envelope through the now-connected client.
    let env = sample_envelope();
    client_out_tx.send(vec![env.clone()]).await.unwrap();

    // Server should receive it — proves the retry succeeded and connection is live.
    let result = tokio::time::timeout(Duration::from_secs(2), server_in_rx.recv()).await;
    assert!(result.is_ok(), "Server should receive envelope after client retry");
    let batch = result.unwrap().unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0], env);

    // Clean shutdown.
    drop(client_out_tx);
    drop(server_in_rx);
    let _ = tokio::time::timeout(Duration::from_secs(2), client_task).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}
