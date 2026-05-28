# Tasks: rpi-controller (v1 minimal)

## Overview

Implementation of the `rpi-controller` crate — an async Tokio headless process that manages simulated ovens, validates commands, applies 5 °C hysteresis heating control, and bridges events via in-memory `MockTransport`. No serial port, no CLI, no `RealTransport`.

**Change**: `rpi-controller` | **Project**: `sistemarust` | **Generated**: 2026-05-21

---

## Review Workload Forecast

| Metric | Value |
|--------|-------|
| **Estimated total changed lines** | ~750 |
| **400-line budget risk** | Low |
| **Chained PRs recommended** | No |
| **Decision needed before apply** | No |

**Rationale**: 9 modules (~600 net-new) + ~150 lines tests. Scope trimmed to mock-only transport and no CLI. Single PR is appropriate; phases are independent and reviewable in sequence.

---

## Task List

### Phase 1 — Scaffold & Core Types

#### T-01 · `Cargo.toml` setup
**What**: Add `rand` crate; add `async-trait`. No `serialport` dependency.
**Why**: Required dependencies identified in design.md.
**Files**: `rpi-controller/Cargo.toml`
**Changed lines**: ~4
**Acceptance criteria**:
- [x] `cargo build` compiles cleanly
- [x] `rand` and `async-trait` are available

---

#### T-02 · `state.rs` — Oven entity + OvenStore
**What**: Define `Oven` struct with all fields from spec + `OvenStore = Arc<RwLock<HashMap<String, Oven>>>`. Derive `Default` for `Oven`. Define `SimulationConfig` constants (`HEATING_DRIFT`, `COOLING_DRIFT`, `ROOM_TEMP`, `NOISE_AMPLITUDE`).
**Why**: Core domain type; all other modules depend on it.
**Files**: `rpi-controller/src/state.rs` (new)
**Changed lines**: ~65
**Acceptance criteria**:
- [x] `Oven` has all fields from design.md interface contract
- [x] `OvenStore` is `Arc<RwLock<HashMap<String, Oven>>>`
- [x] `Default` impl produces a safe disabled oven at room temp
- [x] Unit tests for `OvenStore` insert/remove/get

---

#### T-03 · `validation.rs` — Pure command validators
**What**: Implement pure `fn` validators: `validate_set_target(oven, target)`, `validate_set_enabled(oven)`, `validate_oven_exists(oven)`. Return `Result<(), FaultCode>`. Also `validate_temperature_range(target, max_celsius)`.
**Why**: All command handlers must call validators; pure functions are trivially unit-testable.
**Files**: `rpi-controller/src/validation.rs` (new)
**Changed lines**: ~70
**Acceptance criteria**:
- [x] All scenarios from `CMD-VAL-*` spec are covered by tests
- [x] `validate_set_target` rejects negative and `> max_celsius`
- [x] `validate_set_enabled` returns `Ok(())` for non-faulted ovens
- [x] Emergency-stop guard function `is_emergency_stop_active()` exists for handlers

---

### Phase 2 — Domain Logic

#### T-04 · `control.rs` — Hysteresis heating decision
**What**: Implement `heating_decision(current, target, heating_prev) -> bool` per ADR 002.
Rules:
- `current < target - 5.0` → `true`
- `current >= target` → `false`
- `(target - 5.0) .. target` → maintain `heating_prev`
**Why**: Core control loop logic; must be TDD'd against boundary values.
**Files**: `rpi-controller/src/control.rs` (new)
**Changed lines**: ~45
**Acceptance criteria**:
- [x] All hysteresis scenarios from `HEAT-CTRL-*` spec pass
- [x] Boundary tests: exactly `target - 5.0`, exactly `target`, `target - 4.9`
- [x] Unit tests for all 3 rules with explicit comments matching ADR 002 table

---

#### T-05 · `simulation.rs` — Temperature update
**What**: Implement `update_temperature(oven, dt) -> Oven`. Heating: `current += HEATING_DRIFT * dt + noise`. Cooling: `current -= COOLING_DRIFT * dt + noise` but floor at `ROOM_TEMP`. Safety check: if `current > max_celsius` → set state `Faulted`.
**Why**: Core simulation; must respect room temp floor and safety limits.
**Files**: `rpi-controller/src/simulation.rs` (new)
**Changed lines**: ~55
**Acceptance criteria**:
- [x] `TEMP-SIM-*` scenarios from spec are covered
- [x] Temperature never falls below `ROOM_TEMP`
- [x] Temperature exceeding `max_celsius` sets `state = Faulted`
- [x] Tests use deterministic noise (seeded RNG or fixed delta)

---

### Phase 3 — MockTransport + Event Handlers

#### T-06 · `mock_transport.rs` — In-memory transport
**What**: Define `#[async_trait] trait Transport` with `read()` → `Result<Option<EventEnvelope>>` and `write(envelope)`. Implement `MockTransport` using `tokio::sync::mpsc` channels for tests. No `RealTransport`.
**Why**: Mockable transport for integration tests; no serial port wiring.
**Files**: `rpi-controller/src/mock_transport.rs` (new)
**Changed lines**: ~50
**Acceptance criteria**:
- [x] `Transport: Send + Sync` (required for `Arc<dyn Transport>`)
- [x] `MockTransport` can be created with `channel()` pair
- [x] Unit tests for `MockTransport` send/receive round-trip

---

#### T-07 · `handlers.rs` — Command dispatch + reply emission
**What**: Implement handlers for each `Message` variant. Each handler: validates → mutates store → returns `Vec<EventEnvelope>` (replies + side-effects). Export `handle(message, &OvenStore, emergency_active)`.
**Why**: Central orchestration of command processing; must emit correct events per spec.
**Files**: `rpi-controller/src/handlers.rs` (new)
**Changed lines**: ~150
**Acceptance criteria**:
- [x] `SetTargetTemperature` → `CommandAccepted` or `CommandRejected`
- [x] `SetOvenEnabled` → `CommandAccepted` or `CommandRejected`
- [x] `EmergencyStop` → disables all ovens, sets `EmergencyStopped`, returns `OvenStatusUpdated` per oven
- [x] `RequestStatus` → returns `OvenStatusUpdated` per oven (or single)
- [x] All handlers respect emergency-stop guard (reject non-emergency commands)
- [x] Unit tests with in-memory `OvenStore` for each handler
- [x] Handler tests cover both happy path and rejection cases

---

### Phase 4 — App Orchestration + Entry Point

#### T-08 · `app.rs` — App struct + `tokio::select!` event loop
**What**: Define `App` struct holding `Arc<RwLock<OvenStore>>`, `Arc<dyn Transport>`, `emergency_active: bool`. Implement `App::new(...)` and `App::run()`. The `run()` loop uses `tokio::select!` over:
1. `transport.read()` → `handlers::handle()` → `transport.write()` replies
2. `tokio::time::interval(2s)` → `simulation::update_temperature()` for all ovens → `control::heating_decision()` → emit `OvenStatusUpdated` per oven
**Why**: Core event loop; ties all modules together. No CLI/REPL branch.
**Files**: `rpi-controller/src/app.rs` (new)
**Changed lines**: ~110
**Acceptance criteria**:
- [x] Both branches of `select!` are wired
- [x] Temperature ticker runs every 2 seconds
- [x] Safety fault detection emits `FaultRaised` then sets oven to `Faulted`
- [x] `App::new(...)` accepts `impl Transport + Send + Sync + 'static`
- [x] `App::run()` returns `Result<()>` and handles shutdown gracefully

---

#### T-09 · `main.rs` — Simple runtime bootstrap
**What**: Parse `--simulate N` CLI arg (create N simulated ovens on startup). Build `Tokio` runtime, construct `MockTransport`, create `App`, call `app.run()`. No `--port` flag.
**Why**: Entry point; replaces placeholder `println!`.
**Files**: `rpi-controller/src/main.rs`
**Changed lines**: ~30
**Acceptance criteria**:
- [x] `--simulate N` creates N `Oven` entries and emits `OvenDetected` for each
- [x] Without `--simulate`, no ovens exist at startup
- [x] `cargo run -- --simulate 2` starts with 2 simulated ovens
- [x] Runtime is single-threaded (`#[tokio::main(flavor = "current_thread")]`) (NOTE: renamed from `single_threaded`)

---

### Phase 5 — Testing

#### T-10 · `validation.rs` tests
**What**: Add comprehensive `#[cfg(test)]` module covering all `CMD-VAL-*` scenarios from spec.
**Why**: TDD contract from spec.
**Files**: `rpi-controller/src/validation.rs`
**Changed lines**: ~50 (tests)
**Acceptance criteria**:
- [x] Tests for: oven not found, invalid temp (negative), invalid temp (> max), emergency stop active

---

#### T-11 · `control.rs` tests
**What**: Add comprehensive `#[cfg(test)]` module covering all `HEAT-CTRL-*` scenarios and ADR 002 boundary table.
**Why**: TDD contract from spec.
**Files**: `rpi-controller/src/control.rs`
**Changed lines**: ~50 (tests)
**Acceptance criteria**:
- [x] Test exactly `target - 5.0` → `true`
- [x] Test exactly `target` → `false`
- [x] Test in-band `target - 4.0` maintains previous state
- [x] Test `enabled = false` → always `false`

---

#### T-12 · `simulation.rs` tests
**What**: Add `#[cfg(test)]` module covering `TEMP-SIM-*` and safety fault.
**Files**: `rpi-controller/src/simulation.rs`
**Changed lines**: ~50 (tests)
**Acceptance criteria**:
- [x] Heating raises temperature
- [x] Cooling never goes below `ROOM_TEMP` (20.0)
- [x] Exceeding `max_celsius` sets state to `Faulted`

---

#### T-13 · Integration tests
**What**: Create `rpi-controller/tests/integration.rs` using `MockTransport`. Test full event loop: create oven, send `SetTargetTemperature`, receive `OvenStatusUpdated`, verify hysteresis behavior over multiple ticks.
**Why**: TDD contract from spec; validates end-to-end wiring.
**Files**: `rpi-controller/tests/integration.rs` (new)
**Changed lines**: ~100
**Acceptance criteria**:
- [x] `MockTransport` round-trips `EventEnvelope` correctly
- [x] Full scenario: create oven → set target → verify `heating` becomes true after tick
- [x] Emergency stop test: estop → verify all subsequent commands rejected

---

## Implementation Order

```
T-01  Cargo.toml
T-02  state.rs        ← all other modules depend on Oven/OvenStore
T-03  validation.rs   ← pure functions, no deps on state (but handlers need types)
T-04  control.rs      ← depends only on state types
T-05  simulation.rs   ← depends on state types
T-06  mock_transport.rs ← Transport trait + MockTransport only
T-07  handlers.rs     ← depends on validation, state, protocol
T-08  app.rs          ← depends on all above
T-09  main.rs         ← depends on app, mock_transport
T-10  validation tests
T-11  control tests
T-12  simulation tests
T-13  integration tests
```

---

## Modules in v1 Minimal

| Module | Role |
|--------|------|
| `state.rs` | `Oven` entity + `OvenStore` |
| `validation.rs` | Pure command validators |
| `control.rs` | Hysteresis heating decision |
| `simulation.rs` | Temperature update + safety fault |
| `mock_transport.rs` | `Transport` trait + `MockTransport` |
| `handlers.rs` | Command dispatch + event emission |
| `app.rs` | `tokio::select!` event loop |
| `main.rs` | Entry point + `--simulate N` |

---

## Acceptance Criteria Summary

- [x] All 13 tasks completed
- [x] `cargo test` passes (unit + integration)
- [x] `cargo build` succeeds with mock transport only
- [x] All `spec.md` requirements (`OVEN-SIM-*`, `CMD-VAL-*`, `HEAT-CTRL-*`, `TEMP-SIM-*`, `STATUS-*`, `FAULT-*`, `ESTOP-*`) are covered by passing tests
- [x] `app.rs` `select!` loop handles both event sources concurrently
- [x] Hysteresis implementation matches ADR 002 exactly

(End of file — total lines: ~829)
