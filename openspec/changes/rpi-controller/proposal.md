# Proposal: rpi-controller

## Intent

Build the Raspberry Pi physical controller crate that validates PC commands, maintains oven state, simulates temperature sensing, applies basic threshold heating control, and reports state back via the shared protocol. It acts as the authority of the physical world.

## Scope

### In Scope
- Async Tokio runtime with `HashMap<String, OvenState>` as oven store
- Command validation (oven exists, range valid, no emergency stop)
- Simulated oven detection (CLI flag `--simulate N`)
- Simulated temperature updates every 2s with noise
- Threshold heating control (hysteresis 5°C, no PID)
- Serial port abstraction (trait-based for testability)
- Event emission: OvenDetected, CommandAccepted, CommandRejected, OvenStatusUpdated, FaultRaised
- Emergency stop handling

### Out of Scope
- Real GPIO / rppal integration
- PID control algorithm
- Persistence to disk
- Authentication / multiple PC connections
- Web UI or HTTP API

## Capabilities

### New Capabilities
- `oven-simulation`: CLI-based simulated oven creation and temperature evolution
- `command-validation`: Pure function validation of incoming protocol commands
- `heating-control`: Threshold-based on/off heating logic with hysteresis
- `serial-bridge`: Trait-based serial port abstraction for production and test modes

### Modified Capabilities
- None (protocol crate is consumed as-is)

## Approach

Async Tokio runtime. Single `OvenStore` (`HashMap<String, OvenState>`) shared via `tokio::sync::RwLock`. Main loop uses `tokio::select!` to multiplex:
1. Commands from serial channel (deserialized `EventEnvelope`)
2. Temperature ticker (every 2s)
3. Optional CLI REPL for manual oven insertion

Command handlers are pure functions: take `&OvenState` + command → `Result<(), FaultCode>`. On success, mutate store and emit `CommandAccepted` + `OvenStatusUpdated`. On failure, emit `CommandRejected` with correlation ID.

Temperature simulation: if heating, drift up toward target with rand noise; if disabled, drift down to room temp. Heating decision: `enabled && current < target - 5.0`.

Serial abstraction behind a `SerialPort` trait so tests inject a mock channel.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `rpi-controller/src/main.rs` | New | Entry point, Tokio runtime setup, CLI args |
| `rpi-controller/src/state.rs` | New | `OvenState`, `OvenStore` |
| `rpi-controller/src/validation.rs` | New | Command validation functions |
| `rpi-controller/src/control.rs` | New | Heating threshold logic |
| `rpi-controller/src/simulation.rs` | New | Mock temperature, mock oven detection |
| `rpi-controller/src/serial.rs` | New | `SerialPort` trait + implementation |
| `rpi-controller/src/handlers.rs` | New | Map `Message` → side effects + replies |
| `rpi-controller/Cargo.toml` | Modified | Add `rand`, keep `serialport` optional/mockable |
| `protocol/` | Consumed | Re-export types, no changes |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Serial mocking overhead | Med | Abstract `SerialPort` as trait early; tests use channels |
| Simulated temp not credible | Low | Add ±5°C rand jitter; drift rates configurable |
| Emergency stop state machine bugs | Med | Prioritize TDD tests for EmergencyStop + recovery |
| HashMap contention | Low | 1-4 ovens typical; `RwLock` sufficient |

## Rollback Plan

Delete `rpi-controller/src/*.rs` except `main.rs`, restore `main.rs` to `println!("Hello from rpi-controller!")`. No other crates affected.

## Dependencies

- `protocol` crate (completed)
- `tokio`, `serialport`, `rand`

## Success Criteria

- [ ] Receives `SetTargetTemperature` and responds `CommandAccepted` or `CommandRejected`
- [ ] Emits `OvenDetected` for each simulated oven at startup
- [ ] Emits `OvenStatusUpdated` every 2s with plausible temperature
- [ ] Rejects commands for non-existent ovens with `FaultCode::OvenNotFound`
- [ ] `EmergencyStop` disables all outputs and blocks further commands
- [ ] All command handlers have TDD unit tests
- [ ] Serial trait can be mocked for integration tests
