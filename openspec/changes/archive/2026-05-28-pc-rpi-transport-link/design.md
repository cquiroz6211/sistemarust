# Design: PC-RPi Transport Link — TCP Localhost

## Technical Approach

Create a new `transport` crate that implements a `TransportPlugin` (Bevy `Plugin`). The plugin spawns a single `tokio::task` per process that bridges the `OutboundProtocolQueue` / `InboundProtocolQueue` Resources to a TCP socket. One side runs as server/listener (`rpi-controller`), the other as client/connector (`pc-app`). Messages are newline-delimited JSON (`EventEnvelope` via existing serde). Bevy ECS systems drain/flush the queues — the async task never touches `World` directly.

```
Terminal 1 (rpi-controller):  TcpListener → accept → TcpStream ──┐
                                                                 │
Terminal 2 (pc-app):         TcpStream (connect) ────────────────┤
                                                                 ▼
    OutboundProtocolQueue ──→ [tokio task: serialize → write]    │
    [tokio task: read → deserialize] ← InboundProtocolQueue  ───┘
```

## Architecture Decisions

### Decision: New `transport` crate

**Choice**: Standalone crate `transport/` depending on `protocol` + `tokio`. Both `pc-app` and `rpi-controller` depend on it.

**Alternatives considered**: Module inside `protocol`; module inside each app; separate `platform` crate.

**Rationale**: A standalone crate keeps transport logic isolated and testable. It depends on `protocol` (for `EventEnvelope` serde) but `protocol` doesn't depend on it — clean unidirectional dependency. Each app adds it as an optional plugin.

### Decision: Async tokio task + bounded channels (not blocking I/O in Bevy systems)

**Choice**: `tokio::task::spawn` runs two halves per connection:
- **Writer half**: polls `OutboundProtocolQueue` via a `tokio::sync::mpsc::Receiver<Vec<EventEnvelope>>` — Bevy system sends batches through a bounded `mpsc::Sender`.
- **Reader half**: loops `TcpStream::read_line`-style on JSON lines, deserializes, sends to sender side of channel → Bevy system receives and pushes to `InboundProtocolQueue`.

**Alternatives considered**: Blocking `std::net::TcpStream` in `spawn_blocking`; polling socket in every Bevy tick (too slow); `async-std` instead of `tokio`.

**Rationale**: `tokio::sync::mpsc` with bounded capacity (e.g., 64 messages) provides backpressure. The async task runs on `IoTaskPool` (Bevy's default). Systems read from the channel with `try_recv()` — zero blocking. This is the pattern the spec requires.

### Decision: Server/Client topology (not peer-to-peer)

**Choice**: `rpi-controller` = server (listens), `pc-app` = client (connects). CLI flags: `--listen 127.0.0.1:7000` on RPi side, `--connect 127.0.0.1:7000` on PC side.

**Alternatives considered**: Peer-to-peer (both listen and connect); configurable via `--mode server|client`.

**Rationale**: Matches the domain — RPi is the physical controller (server), PC is the consumer (client). Simpler CLI: each process knows its role. For v1, single connection only, so there's exactly one listener and one connector.

### Decision: JSON line framing over TCP

**Choice**: One `EventEnvelope` per line, terminated by `\n`. Use `serde_json::to_string` for serialization, `serde_json::from_str` for deserialization.

**Alternatives considered**: Length-prefixed binary; CBOR; protobuf.

**Rationale**: The spec requires observability with `nc` or `telnet`. JSON lines are human-readable, debuggable, and `EventEnvelope` already has serde derives. Malformed lines are caught by `serde_json::from_str` error handling — the task logs and continues.

### Decision: No `World` mutation from async tasks

**Choice**: The async task communicates with Bevy via bounded channels only. Bevy systems read channels and mutate resources.

**Alternatives considered**: `Arc<Mutex<World>>` from async; `world.run_schedule()` from tokio task.

**Rationale**: Direct `World` mutation from async tasks is unsafe (Bevy's borrow checker can't protect cross-thread access). Channels provide a clean boundary — the spec explicitly requires this.

### Decision: CLI flags `--listen` and `--connect` (not `--transport tcp --server/--client`)

**Choice**: 
- `rpi-controller`: `--listen 127.0.0.1:7000` enables TCP server mode (replaces/extends `--simulate`)
- `pc-app`: `--connect 127.0.0.1:7000` enables TCP client mode (can combine with `--demo`)

**Alternatives considered**: `--transport tcp --server` / `--transport tcp --client`; `--mode transport=server`.

**Rationale**: More discoverable CLI. `--listen` and `--connect` are self-documenting. Ports are explicit. Matches the proposal's example.

### Decision: TCP keepalive + graceful shutdown (no reconnect)

**Choice**: On disconnect, log and exit cleanly. No reconnect loop in v1. The server keeps running; the client exits.

**Alternatives considered**: Reconnect with exponential backoff; signal-based graceful shutdown.

**Rationale**: Spec says "v1 is single connection, no robust reconnect/backoff." Keep it simple. Students can restart the client process.

## Data Flow

### Full message path (PC → RPi command)

```
PC author function (commands.rs)
    │ constructs EventEnvelope
    ▼
OutboundProtocolQueue (Vec<EventEnvelope>)  ← Bevy Resource
    │ system: flush_outbound (Update schedule)
    ▼
tokio::sync::mpsc::Sender<Vec<EventEnvelope>>  ← bounded channel
    │
    ▼
tokio task (writer half)
    │ serde_json::to_string + "\n"
    ▼
TcpStream::write_all()
    │
    ▼  [TCP localhost wire — visible with `nc -l 7000`]
    │
    ▼
TcpStream::read_line() on RPi side
    │ serde_json::from_str::<EventEnvelope>
    ▼
tokio::sync::mpsc::Sender<Vec<EventEnvelope>>  ← bounded channel
    │
    ▼
Bevy system: flush_inbound (Update schedule)
    │ copies into InboundProtocolQueue
    ▼
ingest_inbound_protocol system (Update)
    │ drains queue, dispatches internal events
    ▼
apply_* systems (FixedUpdate)
    │ mutates entity components
```

### Topology diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    rpi-controller (SERVER)                       │
│                                                                  │
│  TcpListener::bind("127.0.0.1:7000")                            │
│         │ accept()                                              │
│         ▼                                                       │
│  ┌─────────────┐    mpsc::Sender  ┌──────────────────────┐      │
│  │  writer task│◄─────────────────│ OutboundProtocolQueue│      │
│  │ (TCP→JSON)  │                  │  (Bevy Resource)     │      │
│  └──────┬──────┘                  └──────────────────────┘      │
│         │                                                       │
│         │ mpsc::Receiver                                        │
│         ▼                                                       │
│  ┌─────────────┐    mpsc::Sender  ┌──────────────────────┐      │
│  │  reader task│─────────────────►│ InboundProtocolQueue │      │
│  │ (JSON→TCP)  │                  │  (Bevy Resource)     │      │
│  └─────────────┘                  └──────────────────────┘      │
│                                                                  │
│  ECS Systems:                                                    │
│    Update: ingest_commands, route_command, emit_protocol_responses│
│    FixedUpdate: thermal_drift, hysteresis, faults, status       │
└─────────────────────────────────────────────────────────────────┘
                        │ TCP localhost │
                        ▼               ▲
┌─────────────────────────────────────────────────────────────────┐
│                     pc-app (CLIENT)                              │
│                                                                  │
│  TcpStream::connect("127.0.0.1:7000")                           │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐    mpsc::Sender  ┌──────────────────────┐      │
│  │  writer task│◄─────────────────│ OutboundProtocolQueue│      │
│  │ (TCP→JSON)  │                  │  (Bevy Resource)     │      │
│  └──────┬──────┘                  └──────────────────────┘      │
│         │                                                       │
│         │ mpsc::Receiver                                        │
│         ▼                                                       │
│  ┌─────────────┐    mpsc::Sender  ┌──────────────────────┐      │
│  │  reader task│─────────────────►│ InboundProtocolQueue │      │
│  │ (JSON→TCP)  │                  │  (Bevy Resource)     │      │
│  └─────────────┘                  └──────────────────────┘      │
│                                                                  │
│  ECS Systems:                                                    │
│    Update: ingest_inbound_protocol                              │
│    FixedUpdate: apply_oven_detected, apply_oven_status_updated, │
│                 apply_fault_raised, record_command_result        │
└─────────────────────────────────────────────────────────────────┘
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `transport/Cargo.toml` | Create | Crate definition: `transport v0.1.0`, deps: `protocol`, `tokio` (rt + net + sync + macros), `serde_json` |
| `transport/src/lib.rs` | Create | Crate root: `TransportPlugin`, `TransportConfig`, re-exports |
| `transport/src/task.rs` | Create | `spawn_transport_tasks()` — spawns reader + writer tokio tasks, returns channel handles |
| `transport/src/framing.rs` | Create | JSON line framing: `serialize_envelope()`, `deserialize_envelope()`, `parse_json_line()` |
| `transport/src/server.rs` | Create | Server-side TCP listener + task coordination |
| `transport/src/client.rs` | Create | Client-side TCP connector + task coordination |
| `transport/src/systems.rs` | Create | Bevy systems: `flush_outbound`, `flush_inbound` — drain/flush channels into Resources |
| `transport/src/logging.rs` | Create | `[TX]`/`[RX]` log helpers for observability |
| `Cargo.toml` (workspace) | Modify | Add `transport` to workspace members |
| `pc-app/Cargo.toml` | Modify | Add `transport` dependency |
| `rpi-controller/Cargo.toml` | Modify | Add `transport` dependency |
| `pc-app/src/main.rs` | Modify | Parse `--connect` flag, register `TransportPlugin` when present |
| `rpi-controller/src/main.rs` | Modify | Parse `--listen` flag, register `TransportPlugin` when present |
| `pc-app/src/lib.rs` | Modify | Add `transport` module export (if plugin is built into pc-app) |
| `rpi-controller/src/lib.rs` | Modify | Add `transport` module export (if plugin is built into rpi-controller) |

**Totals**: 7 new files in `transport/`, 2 workspace modifications, 4 app modifications.

## Interfaces / Contracts

### TransportPlugin (from `transport/src/lib.rs`)

```rust
use bevy::prelude::*;
use std::net::SocketAddr;

/// Configuration for the TCP transport.
pub struct TransportConfig {
    /// The socket address to connect to (client mode) or listen on (server mode).
    pub address: SocketAddr,
    /// Whether this process is the server/listener.
    pub is_server: bool,
    /// Bounded channel capacity (messages per batch).
    pub channel_capacity: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:7000".parse().unwrap(),
            is_server: false,
            channel_capacity: 64,
        }
    }
}

/// Bevy plugin that spawns async transport tasks.
/// Must be added AFTER the protocol queues are registered as Resources.
pub struct TransportPlugin {
    pub config: TransportConfig,
}

impl Plugin for TransportPlugin {
    fn build(&self, app: &mut App) {
        // Spawn tokio tasks (reader + writer)
        let (out_tx, out_rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);
        let (in_tx, in_rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);

        // ... spawn tasks based on is_server ...

        // Register Bevy systems that drain channels into Resources
        app.add_systems(Update, flush_outbound.pipe(out_rx));
        app.add_systems(Update, flush_inbound.pipe(in_rx));
    }
}
```

### Bevy Systems (from `transport/src/systems.rs`)

```rust
/// Drains OutboundProtocolQueue → sends batch through mpsc channel.
/// Called every Update tick. Non-blocking: uses try_send.
pub fn flush_outbound(
    mut outbound: ResMut<OutboundProtocolQueue>,
    tx: Res<TxOutbound>,  // mpsc::Sender<Vec<EventEnvelope>>
) {
    let batch = std::mem::take(&mut outbound.0);
    if !batch.is_empty() {
        let _ = tx.try_send(batch); // bounded: drops if full (log warning)
    }
}

/// Receives deserialized envelopes from reader task → pushes to InboundProtocolQueue.
/// Called every Update tick. Non-blocking: uses try_recv.
pub fn flush_inbound(
    mut inbound: ResMut<InboundProtocolQueue>,
    rx: Res<TxInbound>,  // mpsc::Receiver<Vec<EventEnvelope>>
) {
    if let Ok(batch) = rx.try_recv() {
        inbound.0.extend(batch);
    }
}
```

### Friming (from `transport/src/framing.rs`)

```rust
use protocol::EventEnvelope;

/// Serializes an envelope to a JSON line (no trailing newline).
pub fn serialize_envelope(env: &EventEnvelope) -> String {
    serde_json::to_string(env).expect("EventEnvelope must serialize")
}

/// Parses a single JSON line into an EventEnvelope.
/// Returns None if the line is empty or malformed.
pub fn parse_json_line(line: &str) -> Option<EventEnvelope> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}
```

### Logging (from `transport/src/logging.rs`)

```rust
/// Log a transmitted envelope with [TX] prefix.
pub fn log_tx(source: &str, target: &str, envelope: &EventEnvelope) {
    eprintln!("[TX] {} → {} | {} | id={}", 
        source, target, 
        std::mem::discriminant(&envelope.payload),
        envelope.event_id
    );
}

/// Log a received envelope with [RX] prefix.
pub fn log_rx(source: &str, target: &str, envelope: &EventEnvelope) {
    eprintln!("[RX] {} → {} | {} | id={}",
        source, target,
        std::mem::discriminant(&envelope.payload),
        envelope.event_id
    );
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `parse_json_line` handles valid JSON | `parse_json_line(&json) → Some(envelope)` — assert fields match |
| Unit | `parse_json_line` handles empty line | `parse_json_line("") → None` |
| Unit | `parse_json_line` handles malformed JSON | `parse_json_line("not json") → None` |
| Unit | `serialize_envelope` round-trips | `parse_json_line(&serialize(&env)) == Some(env)` |
| Unit | `flush_outbound` drains queue | Push envelopes → call system → assert queue empty, channel received batch |
| Unit | `flush_inbound` populates queue | Channel sends batch → call system → assert `InboundProtocolQueue` has items |
| Unit | `flush_outbound` non-blocking on full channel | Fill channel to capacity → call system → assert no panic, batch dropped |
| Integration | Full PC→RPi flow (mocked TCP) | Start server + client in test → send command → assert RPi receives envelope |
| Integration | Full RPi→PC flow (mocked TCP) | RPi emits `OvenDetected` → assert PC receives and processes |
| Integration | Malformed line doesn't crash reader | Server sends garbage → client continues processing valid messages |
| Integration | Client retry on connect failure | Client starts before server → assert retry loop (log error, sleep, retry) |

### Demo validation (manual)

```bash
# Terminal 1
cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000

# Terminal 2  
cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo

# Expected output in Terminal 1 (RPi):
# [RX] pc-app → rpi-controller | SetTargetTemperature | id=...
# [TX] rpi-controller → pc-app | CommandAccepted | id=...
# [TX] rpi-controller → pc-app | OvenStatusUpdated | id=...

# Expected output in Terminal 2 (PC):
# [RX] rpi-controller → pc-app | OvenDetected | id=...
# [TX] pc-app → rpi-controller | SetTargetTemperature | id=...
# [RX] rpi-controller → pc-app | CommandAccepted | id=...
```

## Migration / Rollout

No data migration required. The transport is opt-in via CLI flags — without `--listen` or `--connect`, both apps run exactly as before (in-memory queues only). Rollback: remove `transport` crate from workspace Cargo.toml, revert main.rs changes in both apps. All 154 existing tests remain unaffected.

## Open Questions

- [ ] **Channel capacity**: 64 messages seems reasonable for demo, but should we make it configurable? Could add `--buffer-size N` to CLI.
- [ ] **Connection retry strategy**: Client should retry if server isn't ready. How many retries? Fixed delay (1s) or exponential backoff? Spec says "MAY retry once per frame tick" — that's ~20 retries/sec, seems aggressive. Suggest: retry every 2 seconds with a max of 30 attempts (60s total).
- [ ] **`--demo` flag in pc-app**: What does `--demo` do? The proposal mentions it but doesn't define it. Suggest: `--demo` sends a sequence of commands (SetTargetTemperature, SetOvenEnabled, RequestStatus) automatically to drive the observable flow.
- [ ] **`serde_json` dependency in transport**: `protocol` already depends on `serde_json` (used for serialization tests). Should `transport` add it as a direct dependency, or re-export from `protocol`? Adding it directly is cleaner — transport owns its serialization.
