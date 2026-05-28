# Proposal: pc-app

## Intent

Build the PC-side Bevy ECS application that consumes Raspberry Pi protocol events and maintains a local read model of oven state. The PC displays and requests; the Raspberry validates and decides. v1 focuses on a headless/testable ECS read model and command-authoring layer — no visual UI, no real serial/TCP transport.

## Scope

### In Scope
- Bevy ECS app with `InboundProtocolQueue` / `OutboundProtocolQueue` Resources mirroring the `rpi-controller` pattern
- Per-oven Bevy entities with components: `OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`, `FaultState`
- `OvenIndex(HashMap<String, Entity>)` bridging protocol IDs to ECS entities
- Systems: `apply_oven_detected`, `apply_oven_status_updated`, `apply_fault_raised`, `record_command_result`
- Command authoring systems: `SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`
- Internal events: `OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, `CommandRejectedReceived`
- Deterministic tests via `App::update()` stepping with mock queues

### Out of Scope
- Visual UI (Bevy UI deferred to v1.1)
- Real serial/TCP transport (future adapter behind traits)
- Auth, multi-client/session semantics, persistence, charts, reconnection/heartbeat

## Capabilities

### New Capabilities
- `pc-oven-read-model`: Local ECS representation of detected ovens, updated by inbound protocol events
- `pc-command-authoring`: Systems that construct and enqueue protocol commands into `OutboundProtocolQueue`
- `pc-protocol-ingestion`: Systems that drain `InboundProtocolQueue`, dispatch internal events, and update ECS state

### Modified Capabilities
- None (protocol crate consumed as-is; no spec-level changes)

## Approach

Mirror the `rpi-controller` Bevy ECS pattern. The app uses `MinimalPlugins` + `ScheduleRunnerPlugin` with `FixedUpdate` at 50ms for deterministic state transitions. Two Resources mediate protocol I/O: `InboundProtocolQueue` (RPi → PC events) and `OutboundProtocolQueue` (PC → RPi commands).

Inbound flow: `ingest_inbound_protocol` drains the queue, then `apply_oven_detected` spawns/updates oven entities, `apply_oven_status_updated` mutates components, `apply_fault_raised` updates `FaultState`, `record_command_result` records last accepted/rejected results.

Outbound flow: each command system reads command intent state, constructs an `EventEnvelope` with the correct `Message` variant, and pushes to `OutboundProtocolQueue`. `correlation_id` inversion is supported by the protocol envelope.

Tests use `App::update()` to step the schedule, push raw JSON envelopes into `InboundProtocolQueue`, and assert component state changes.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `pc-app/src/main.rs` | New | Bevy app bootstrap, plugin registration |
| `pc-app/src/components.rs` | New | Oven ECS components (`OvenId`, `FaultState`, etc.) |
| `pc-app/src/resources.rs` | New | `InboundProtocolQueue`, `OutboundProtocolQueue`, `OvenIndex` |
| `pc-app/src/events.rs` | New | Internal event types |
| `pc-app/src/systems/ingest.rs` | New | Inbound protocol draining and dispatch |
| `pc-app/src/systems/state.rs` | New | `apply_oven_detected`, `apply_oven_status_updated`, `apply_fault_raised`, `record_command_result` |
| `pc-app/src/systems/commands.rs` | New | Command authoring systems |
| `pc-app/Cargo.toml` | Modified | Already has deps; remove unused `tokio` from v1 |
| `protocol/` | Consumed | Types consumed, no changes |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `FaultRaised` / `OvenStatusUpdated` out-of-order clears faults | Med | Explicit rules: `FaultState` persists until acknowledged or oven resets |
| UI developers later break ECS invariants | Low | ECS read model is pure data; UI reads components, emits events — no bidirectional coupling |
| Unused `serialport`/`tokio` deps confuse scope | Low | Proposal explicitly marks them future adapters; `tokio` removed from v1 |
| Command correlation model insufficient for UI feedback | Low | v1 records last accepted/rejected; correlation stored in envelope, not in ECS |

## Rollback Plan

Delete `pc-app/src/*.rs` (except `main.rs` stub), revert `pc-app/Cargo.toml` to original dependencies. No other crates affected.

## Dependencies

- `protocol` crate (completed, consumed as-is)
- `bevy = "0.15"` (already in `pc-app/Cargo.toml`)

## Success Criteria

- [ ] `OvenDetected` envelope spawns/updates oven entity with correct components
- [ ] `OvenStatusUpdated` envelope updates `CurrentTemperature`, `TargetTemperature`, `Enabled`, `Heating`, `OvenStatus`
- [ ] `FaultRaised` envelope updates `FaultState` without clearing on subsequent status updates
- [ ] Command authoring systems produce correctly-typed `EventEnvelope` in `OutboundProtocolQueue`
- [ ] All state-transition systems have deterministic `App::update()` tests
- [ ] No visual UI code in v1 (UI deferred)
