# Proposal: PC-RPi Transport Link

## Intent

Connect the two independent Bevy ECS applications (`pc-app` and `rpi-controller`) via a TCP localhost transport layer so students can observe the full distributed data flow: oven detection, command authoring, status updates, and fault reporting — all visible across two terminal windows.

## Scope

### In Scope
- New `transport` crate with TCP localhost adapter around `InboundProtocolQueue`/`OutboundProtocolQueue`
- JSON line framing over TCP using existing `EventEnvelope` serde (no new serialization format)
- CLI mode flag (`--transport tcp` or similar) to run as server/listener or client/connector
- Observable console logs showing messages flowing between processes
- Integration demo proving: oven detection → command → status → fault

### Out of Scope
- UI changes
- Real GPIO or serial I/O
- Authentication, encryption, or TLS
- Reconnect logic, backoff, or multi-client support
- Production-grade network hardening

## Capabilities

### New Capabilities
- `transport-tcp-localhost`: TCP transport adapter bridging protocol queues between two local processes

### Modified Capabilities
- None (no existing spec requirements change; transport is orthogonal to protocol/message types)

## Approach

Create a `transport` crate depending on `protocol` and `tokio`. It exposes a `TransportPlugin` (Bevy `Plugin`) that spawns a background async task:

1. **Server side** (`rpi-controller`): listens on `127.0.0.1:PORT`, accepts one client connection. Drains `OutboundProtocolQueue` → serializes to JSON lines → writes to socket. Reads socket → deserializes → pushes to `InboundProtocolQueue`.
2. **Client side** (`pc-app`): connects to `127.0.0.1:PORT`. Same bidirectional queue bridge.
3. **Framing**: newline-delimited JSON (one `EventEnvelope` per line). Simple, observable, debuggable with `nc` or `telnet`.
4. **Observability**: both sides log every envelope (source, type, correlation_id) to stderr with `[TX]`/`[RX]` prefixes so students see the message flow in real time.

Both processes run on localhost — one terminal for RPi simulation, one for PC read model.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `Cargo.toml` (workspace) | Modified | Add `transport` crate to workspace |
| `transport/` | New | New crate: TCP adapter, framing, Bevy plugin |
| `rpi-controller/src/main.rs` | Modified | Add `--transport` flag, register transport plugin |
| `pc-app/src/main.rs` | Modified | Add `--transport` flag, register transport plugin |
| `protocol/` | Unchanged | No spec changes; reuses existing `EventEnvelope` serde |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Blocking Bevy main loop with I/O | Medium | Use `tokio::task::spawn_blocking` or async task + channel; never block systems |
| Connection race during demo setup | Low | Simple: start server first (RPi), then client (PC); log connection state clearly |
| Complexity creep (reconnect, auth) | Medium | Enforce scope via code review; v1 exits cleanly on disconnect |

## Rollback Plan

Remove `transport` crate from workspace Cargo.toml, revert main.rs changes in both apps. All existing tests (154) remain unaffected since transport is opt-in via CLI flag.

## Dependencies

- Existing `protocol` crate (EventEnvelope, Message types, serde)
- `tokio` (already a dependency of Bevy or easily added)

## Success Criteria

- [ ] Two terminal windows run simultaneously: `rpi-controller --transport tcp --server` and `pc-app --transport tcp --client`
- [ ] `OvenDetected` emitted by RPi appears in PC logs within 1 second
- [ ] PC sends `SetTargetTemperature` → RPi responds with `CommandAccepted` → visible in both terminals
- [ ] RPi sends `OvenStatusUpdated` after command → PC read model updates
- [ ] `cargo test --workspace` still passes (154 tests)
