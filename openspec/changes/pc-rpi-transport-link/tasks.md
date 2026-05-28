# Tasks: PC-RPi Transport Link

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 700–900 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Foundation) → PR 2 (Core + Systems) → PR 3 (CLI + Integration) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

Decision needed before apply: ~~Yes~~ Resolved — maintainer approved `size:exception` for single PR.
Chained PRs recommended: ~~Yes~~ Overridden — single PR per maintainer decision.
Chain strategy: ~~pending~~ `size-exception`
400-line budget risk: ~~High~~ Accepted.

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Crate scaffolding + framing module + unit tests | PR 1 | Base branch; tests/docs included; no Bevy deps yet |
| 2 | TransportPlugin + tokio tasks + Bevy systems + channel wiring | PR 2 | Feature branch chain; depends on PR 1; includes unit tests |
| 3 | CLI flags (--listen/--connect) + main.rs integration + integration demo | PR 3 | Feature branch chain; depends on PR 2; includes end-to-end tests |

## Phase 1: Foundation — Crate + Framing

- [x] 1.1 Add `transport` crate to workspace `Cargo.toml` members list
- [x] 1.2 Create `transport/Cargo.toml` with deps: `protocol` (path), `tokio` (rt,net,sync,macros), `serde_json`
- [x] 1.3 Create `transport/src/lib.rs` — crate root with `TransportConfig`, `TransportPlugin` stub structs, module declarations
- [x] 1.4 Create `transport/src/framing.rs` — `serialize_envelope()`, `parse_json_line()` with `serde_json`
- [x] 1.5 Write unit tests for framing: valid envelope round-trip, empty line returns None, malformed JSON returns None, multiple lines
- [x] 1.6 Verify: `cargo test --package transport` passes

## Phase 2: Core — TransportPlugin + Tokio Tasks

- [x] 2.1 Create `transport/src/logging.rs` — `log_tx()` and `log_rx()` helpers with `[TX]`/`[RX]` prefixes using `eprintln!`
- [x] 2.2 Create `transport/src/task.rs` — `spawn_transport_tasks()` function that takes `TcpStream` + channel handles, returns task handles; spawns reader half (read → deserialize → send to inbound channel) and writer half (recv from outbound channel → serialize → write)
- [x] 2.3 Create `transport/src/server.rs` — `serve()` function: `TcpListener::bind`, `accept`, on connection call `spawn_transport_tasks()`, returns `ServerHandle` with shutdown method
- [x] 2.4 Create `transport/src/client.rs` — `connect()` function: `TcpStream::connect` with retry loop (2s interval, 30 max attempts), on success call `spawn_transport_tasks()`, returns `ClientHandle`
- [x] 2.5 Complete `transport/src/lib.rs` — wire `TransportPlugin::build()`: create channels, spawn tasks based on `is_server` flag, register `flush_outbound` and `flush_inbound` systems
- [x] 2.6 Create `transport/src/systems.rs` — `OutboundSender`, `InboundReceiver`, `TokioRuntime` channel Resources (bridge systems defined in each app, not transport crate — see deviations)
- [x] 2.7 Write unit tests: framing round-trip, logging type names, parse edge cases
- [x] 2.8 Verify: `cargo test --package transport` passes (9 unit + 4 integration = 13 tests)

## Phase 3: Wiring — CLI + App Integration

- [x] 3.1 Add `transport` dependency to `pc-app/Cargo.toml`
- [x] 3.2 Add `transport` dependency to `rpi-controller/Cargo.toml`
- [x] 3.3 Modify `pc-app/src/main.rs` — parse `--connect <addr>` and `--demo` flags; when present, create `TransportConfig::client(addr)`, register `TransportPlugin` after `PcAppPlugin`, add bridge systems + demo system
- [x] 3.4 Modify `rpi-controller/src/main.rs` — parse `--listen <addr>` flag; when present, create `TransportConfig::server(addr)`, register `TransportPlugin` after `OvenControllerPlugin`, add bridge systems
- [x] 3.5 Wire `TransportPlugin` to be added AFTER protocol queue resources are registered (respect dependency order in `App::new()` builder)
- [x] 3.6 Verify: both apps compile with and without transport flags (backward compatibility)

## Phase 4: Integration + Demo

- [x] 4.1 Write integration test: start server + client in same test process (mock TCP via `127.0.0.1:0`), send `EventEnvelope` from client → assert server receives it
- [x] 4.2 Write integration test: send malformed JSON line from client → assert server logs error and continues processing valid messages
- [x] 4.3 Write integration test: client connects before server → assert retry loop works (log message present, eventual connection succeeds) — test: `client_retries_on_connection_refused_then_succeeds`
- [x] 4.4 Manual demo validation: run `cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000` in terminal 1, `cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo` in terminal 2; verify `[TX]`/`[RX]` logs show full flow — automated via Start-Process with log capture; full bidirectional flow confirmed: OvenDetected, SetTargetTemperature, CommandAccepted, OvenStatusUpdated, RequestStatus
- [x] 4.5 Verify: `cargo test --workspace` passes (all existing tests + new transport tests = 167 total)

## Implementation Order

1. **Phase 1** first — framing is pure logic with no dependencies, easy to test in isolation
2. **Phase 2** second — core transport logic depends on framing; systems depend on Phase 1 types
3. **Phase 3** third — app integration depends on the plugin being stable
4. **Phase 4** last — integration tests verify the whole chain end-to-end

This order ensures each phase builds on a working foundation. If you hit a wall in Phase 2, you can still test framing (Phase 1) independently.
