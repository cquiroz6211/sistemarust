# SDD Delta Spec — `rpi-controller-bevy-headless`

**Change**: `rpi-controller-bevy-headless`
**Phase**: Spec
**Base approved**: proposal, PRD, protocol crate (archived), ADR 001, ADR 002
**Persisted**: `openspec/changes/rpi-controller-bevy-headless/spec.md`
**Save topic**: `sdd/rpi-controller-bevy-headless/spec`

---

## Contract: TDD Protocol

Every requirement in this spec MUST have a corresponding unit or integration test.
Tests use `App::update()` stepping — no hardware, no `tokio` runtime as domain orchestrator.
Tests are deterministic and repeatable.

---

## 1. headless-runtime

### REQ-HR-001

**Requirement**: The application MUST start as a headless Bevy ECS app using `App::new()` with `MinimalPlugins` and `ScheduleRunnerPlugin::run_loop(...)`.

**Acceptance criteria**:
- `rpi-controller` binary starts without renderer, window, or UI.
- `--simulate N` flag creates N simulated ovens at startup.
- The `Startup` schedule runs once on boot.
- The `Update` schedule runs every frame tick.
- The `FixedUpdate` schedule runs at a fixed 50 ms tick interval.

**TDD Contract**:
- `#[test] fn app_headless_starts_without_panic()` — constructs `App` headless, runs one frame, does not panic.
- `#[test] fn app_update_schedule_runs()` — verifies `Update` runs at least one cycle via `App::update()`.
- `#[test] fn app_fixed_update_schedule_runs()` — verifies `FixedUpdate` ticks at least once via `App::update()` with sufficient delta time.

---

## 2. oven-entity-model

### REQ-EM-001

**Requirement**: Each simulated oven MUST be represented as a Bevy entity with the following components:

| Component | Type | Note |
|---|---|---|
| `OvenId` | `String` | stable identifier |
| `SensorRef` | `String` | sensor label |
| `OutputRef` | `String` | actuator label |
| `CurrentTemperature` | `f64` | simulated sensor value in °C |
| `TargetTemperature` | `f64` | desired temperature in °C |
| `MaxTemperature` | `f64` | safety limit in °C |
| `Enabled` | `bool` | logical enable |
| `Heating` | `bool` | heating output active |
| `OvenStatus` | `OvenState` | composite state enum |

### REQ-EM-002

**Requirement**: The application MUST provide the following global resources:

| Resource | Type | Note |
|---|---|---|
| `EmergencyStopActive` | `bool` | `true` when emergency stop is active |
| `SimulationConfig` | struct | `heating_drift`, `cooling_drift`, `room_temp`, `noise_amplitude`, `hysteresis` |
| `InboundProtocolQueue` | `Vec<EventEnvelope>` | pending commands to process |
| `OutboundProtocolQueue` | `Vec<EventEnvelope>` | outbound events to publish |
| `ControllerConfig` | struct | `tick_interval_ms`, `status_publish_interval_ms` |

### REQ-EM-003

**Requirement**: All simulation constants MUST be defined in `SimulationConfig` and MUST match ADR 002 values:
- `heating_drift = 4.0` °C per FixedUpdate tick
- `cooling_drift = 1.0` °C per FixedUpdate tick
- `room_temp = 20.0` °C (ambient minimum)
- `noise_amplitude = 2.0` °C (± range per tick)
- `hysteresis = 5.0` °C

---

## 3. simulated-oven-detection

### REQ-SD-001

**Requirement**: On `Startup`, a system MUST spawn exactly N `Oven` entities when the `--simulate N` flag is provided.

**Acceptance criteria**:
- Each entity receives a unique `OvenId` of the form `"oven-{n}"`.
- Each entity receives `SensorRef = "temp-{n}"`, `OutputRef = "relay-{n}"`.
- Each entity receives `MaxTemperature = 300.0`.
- Each entity starts with `CurrentTemperature = room_temp`, `TargetTemperature = room_temp`, `Enabled = false`, `Heating = false`, `OvenStatus = OvenState::Disabled`.

### REQ-SD-002

**Requirement**: After spawning each oven, the system MUST emit exactly one `OvenDetected` event via the internal ECS event system and enqueue one `EventEnvelope` in `OutboundProtocolQueue`.

**TDD Contract**:
- `#[test] fn startup_spawns_n_ovens()` — with `N=3`, after `app.update()`, verify exactly 3 `Oven` entities exist.
- `#[test] fn startup_emits_oven_detected_per_oven()` — after `app.update()`, verify exactly N `OvenDetected` envelopes in queue.
- `#[test] fn oven_entity_has_correct_initial_state()` — verify initial component values match spec.

---

## 4. protocol-command-processing

### REQ-PC-001 — SetTargetTemperature

**Requirement**: When `InboundProtocolQueue` contains an `EventEnvelope` with `Message::SetTargetTemperature(payload)`:

1. If `EmergencyStopActive == true` → emit `CommandRejected` with `FaultCode::EmergencyStopActive`.
2. Otherwise, if no entity has `OvenId == payload.oven_id` → emit `CommandRejected` with `FaultCode::OvenNotFound`.
3. Otherwise, if `payload.target_celsius < 0.0` → emit `CommandRejected` with `FaultCode::InvalidTemperature`.
4. Otherwise, if `payload.target_celsius > target entity's MaxTemperature` → emit `CommandRejected` with `FaultCode::SafetyLimitExceeded`.
5. Otherwise → update the entity's `TargetTemperature` component and emit `CommandAccepted`.

**TDD Contract**:
- `#[test] fn set_target_temperature_accepts_valid()` — valid command → `TargetTemperature` updated, `CommandAccepted` emitted.
- `#[test] fn set_target_temperature_rejects_negative()` — negative value → `CommandRejected(InvalidTemperature)`.
- `#[test] fn set_target_temperature_rejects_above_max()` — above `max_celsius` → `CommandRejected(SafetyLimitExceeded)`.
- `#[test] fn set_target_temperature_rejects_nonexistent_oven()` — unknown `oven_id` → `CommandRejected(OvenNotFound)`.
- `#[test] fn set_target_temperature_rejects_when_emergency_stop_active()` — emergency stop active → `CommandRejected(EmergencyStopActive)`.

### REQ-PC-002 — SetOvenEnabled

**Requirement**: When `InboundProtocolQueue` contains `Message::SetOvenEnabled(payload)`:

1. If `EmergencyStopActive == true` → emit `CommandRejected` with `FaultCode::EmergencyStopActive`.
2. Otherwise, if no entity has `OvenId == payload.oven_id` → emit `CommandRejected` with `FaultCode::OvenNotFound`.
3. Otherwise → update the entity's `Enabled` component and emit `CommandAccepted`.

### REQ-PC-003 — RequestStatus

**Requirement**: When `InboundProtocolQueue` contains `Message::RequestStatus(payload)`:

1. If `scope == Single` and `oven_id` matches an entity → emit one `OvenStatusUpdated` for that entity.
2. If `scope == All` → emit one `OvenStatusUpdated` per registered entity.
3. If `scope == Single` and `oven_id` does not match → emit `CommandRejected(OvenNotFound)`.
4. No `CommandAccepted` is emitted for `RequestStatus`; the status events are the response.

### REQ-PC-004 — EmergencyStop

**Requirement**: When `InboundProtocolQueue` contains `Message::EmergencyStop(payload)`:

1. Set `EmergencyStopActive = true` resource.
2. For every `Oven` entity: set `Enabled = false`, `Heating = false`, `OvenStatus = OvenState::EmergencyStopped`.
3. Emit exactly one `CommandAccepted`.

### REQ-PC-005 — CommandAccepted / CommandRejected structure

**Requirement**: All response messages MUST use `EventEnvelope::reply_to()` to preserve `correlation_id` and invert `source`/`target`.

**TDD Contract** (for all commands):
- `#[test] fn command_accepted_has_correct_correlation_id()` — verify `correlation_id` links to original command.
- `#[test] fn command_rejected_has_correct_correlation_id()` — verify `correlation_id` links to original command.
- `#[test] fn outbound_queue_maintains_fifo_order()` — verify outbound order matches processing order.

---

## 5. thermal-control-fixed-update

### REQ-TC-001 — Temperature drift simulation

**Requirement**: On each `FixedUpdate` tick, for every entity where `Enabled == true`:

1. If `Heating == true`: `CurrentTemperature += heating_drift + noise()` where `noise()` is uniform random in `[−noise_amplitude, +noise_amplitude]`.
2. If `Heating == false`: `CurrentTemperature −= cooling_drift + noise()`.
3. `CurrentTemperature` MUST NOT drop below `room_temp`.

### REQ-TC-002 — Hysteresis control (ADR 002)

**Requirement**: After temperature drift, the system MUST apply the hysteresis rule per entity:

```
if current_celsius < (target_celsius − hysteresis):
    Heating = true
else if current_celsius >= target_celsius:
    Heating = false
else:
    // maintain previous Heating state (no change)
```

### REQ-TC-003 — State derivation

**Requirement**: After each `FixedUpdate`, the system MUST update each entity's `OvenStatus` component:

| Condition | Resulting `OvenStatus` |
|---|---|
| `EmergencyStopActive == true` | `EmergencyStopped` |
| `Enabled == false` | `Disabled` |
| `Enabled == true && Heating == false` | `Idle` |
| `Enabled == true && Heating == true` | `Heating` |
| `current_celsius > MaxTemperature` | `Faulted` |

**TDD Contract**:
- `#[test] fn fixed_update_heats_oven_when_below_hysteresis_band()` — target=150, current=140 → after tick(s), `Heating == true`.
- `#[test] fn fixed_update_stops_heating_when_at_target()` — target=150, current=150 (was heating) → `Heating == false`.
- `#[test] fn fixed_update_maintains_heating_in_band()` — target=150, current=147 (was heating) → `Heating` unchanged.
- `#[test] fn fixed_update_never_drops_below_room_temp()` — cooling when at `room_temp` → stays at `room_temp`.
- `#[test] fn fixed_update_derives_correct_oven_status()` — verify state machine mapping.
- `#[test] fn hysteresis_prevents_rapid_oscillation()` — oscillating temperature does NOT flip `Heating` more than once per transition.

---

## 6. fault-and-safety

### REQ-FS-001 — SafetyLimitExceeded fault

**Requirement**: If at any point `CurrentTemperature > MaxTemperature` for an entity:
1. Set that entity's `OvenStatus = OvenState::Faulted`.
2. Set that entity's `Heating = false`.
3. Enqueue one `FaultRaised` event with `FaultCode::SafetyLimitExceeded` and `Severity::High` in `OutboundProtocolQueue`.

### REQ-FS-002 — Faulted oven cannot heat

**Requirement**: An entity with `OvenStatus == Faulted` MUST have `Heating == false` and `Enabled == false`. The system MUST NOT allow a faulted oven to start heating again without external intervention (out of scope for v1; faulted ovens remain faulted).

### REQ-FS-003 — EmergencyStop is final authority

**Requirement**: `EmergencyStop` command MUST be accepted and processed even when there are active faults. No other command is accepted while `EmergencyStopActive == true`.

### REQ-FS-004 — FaultRaised event structure

**Requirement**: `FaultRaised` events MUST include the `oven_id` of the affected entity when applicable.

**TDD Contract**:
- `#[test] fn fault_raised_when_temperature_exceeds_max()` — simulate temperature exceeding `max_celsius` → `FaultRaised` emitted, entity is `Faulted`.
- `#[test] fn faulted_oven_cannot_be_heating()` — verify `Heating == false` for faulted entity.
- `#[test] fn emergency_stop_accepted_during_fault()` — send `EmergencyStop` while fault active → `CommandAccepted` still emitted.
- `#[test] fn no_commands_accepted_during_emergency_stop()` — after emergency stop, all other commands → `CommandRejected(EmergencyStopActive)`.

---

## 7. status-publication

### REQ-SP-001 — Status on change

**Requirement**: Whenever an entity transitions to a new `OvenStatus`, the system MUST enqueue one `OvenStatusUpdated` in `OutboundProtocolQueue`.

### REQ-SP-002 — Periodic status publication

**Requirement**: The system MUST publish `OvenStatusUpdated` for all entities at least once every `status_publish_interval_ms` (default: 1000 ms), even if state has not changed.

### REQ-SP-003 — OvenStatusUpdated payload completeness

**Requirement**: Each `OvenStatusUpdated` event MUST include all fields from `OvenStatusUpdatedPayload`:
- `oven_id`, `current_celsius`, `target_celsius`, `enabled`, `heating`, `output_level` (optional), `state`

**TDD Contract**:
- `#[test] fn status_updated_emitted_on_state_transition()` — trigger a state change → verify `OvenStatusUpdated` in queue.
- `#[test] fn status_updated_includes_all_required_fields()` — verify payload field completeness.
- `#[test] fn periodic_status_published_when_interval_elapsed()` — advance time beyond interval → verify periodic publication.

---

## 8. testability

### REQ-TA-001 — App::update() stepping

**Requirement**: All domain logic MUST be testable using `App::update()` with explicit time control. No test may depend on real hardware, real serial ports, or real GPIO.

### REQ-TA-002 — Deterministic tests

**Requirement**: All tests MUST be deterministic. Random noise in temperature simulation MUST be seeded or replaced by a controlled value in test contexts (e.g., via a `TestRng` resource that wraps the noise function).

### REQ-TA-003 — No tokio runtime as domain orchestrator

**Requirement**: The `rpi-controller` domain logic MUST NOT depend on a `tokio` runtime for its core ECS processing. The `tokio` runtime (if used at all) is restricted to I/O adapter concerns and MUST NOT wrap domain systems.

**TDD Contract**:
- `#[test] fn domain_logic_tested_without_tokio()` — tests construct `App` directly, call `app.update()`, assert on components/queues.
- `#[test] fn simulation_noise_is_deterministic_in_tests()` — verify seeded noise produces reproducible temperature sequences.

---

## Scenario Catalog

### SC-001: Startup with --simulate 3

**Given** the controller starts with `--simulate 3`
**When** the `Startup` schedule runs
**Then** exactly 3 `Oven` entities exist
**And** exactly 3 `OvenDetected` envelopes are in `OutboundProtocolQueue`
**And** all 3 ovens have `OvenStatus = Disabled`, `Enabled = false`, `CurrentTemperature = 20.0`

### SC-002: SetTargetTemperature happy path

**Given** an oven with `oven_id = "oven-1"` and `MaxTemperature = 300.0`
**And** `EmergencyStopActive = false`
**When** `SetTargetTemperature { oven_id: "oven-1", target_celsius: 180.0 }` is enqueued
**Then** the entity's `TargetTemperature` is updated to `180.0`
**And** `CommandAccepted` is enqueued with `correlation_id` pointing to the original command

### SC-003: SetTargetTemperature with negative value

**Given** an oven with `oven_id = "oven-1"`
**And** `EmergencyStopActive = false`
**When** `SetTargetTemperature { oven_id: "oven-1", target_celsius: -10.0 }` is enqueued
**Then** `CommandRejected { reason: InvalidTemperature }` is enqueued
**And** the entity's `TargetTemperature` is unchanged

### SC-004: SetTargetTemperature target > max_celsius

**Given** an oven with `oven_id = "oven-1"`, `max_celsius = 300.0`
**And** `EmergencyStopActive = false`
**When** `SetTargetTemperature { oven_id: "oven-1", target_celsius: 350.0 }` is enqueued
**Then** `CommandRejected { reason: SafetyLimitExceeded }` is enqueued

### SC-005: SetTargetTemperature for nonexistent oven

**Given** `EmergencyStopActive = false`
**When** `SetTargetTemperature { oven_id: "oven-99", target_celsius: 150.0 }` is enqueued
**Then** `CommandRejected { reason: OvenNotFound }` is enqueued

### SC-006: EmergencyStop

**Given** multiple ovens, some in `Heating` state
**When** `EmergencyStop { reason: "Operator command" }` is enqueued
**Then** `EmergencyStopActive = true`
**And** all ovens have `OvenStatus = EmergencyStopped`, `Enabled = false`, `Heating = false`
**And** exactly one `CommandAccepted` is enqueued

### SC-007: RequestStatus single

**Given** oven "oven-1" exists
**When** `RequestStatus { oven_id: Some("oven-1"), scope: Single }` is enqueued
**Then** exactly one `OvenStatusUpdated` for "oven-1" is enqueued

### SC-008: RequestStatus all

**Given** 3 ovens exist
**When** `RequestStatus { oven_id: None, scope: All }` is enqueued
**Then** exactly 3 `OvenStatusUpdated` envelopes are enqueued (one per oven)

### SC-009: Hysteresis band maintenance

**Given** an oven with `TargetTemperature = 150.0`, `CurrentTemperature = 147.0`
**And** `Heating = true` (was previously heating)
**When** one `FixedUpdate` tick runs (without temperature crossing a threshold)
**Then** `Heating` remains `true`
**And** `OvenStatus` remains `Heating`

### SC-010: Hysteresis heating activation

**Given** an oven with `TargetTemperature = 150.0`, `CurrentTemperature = 144.0`
**And** `Heating = false`
**When** `FixedUpdate` runs and `CurrentTemperature` is still below `145.0`
**Then** `Heating` becomes `true`

### SC-011: Hysteresis heating deactivation

**Given** an oven with `TargetTemperature = 150.0`, `CurrentTemperature = 151.0`
**And** `Heating = true`
**When** `FixedUpdate` runs
**Then** `Heating` becomes `false`
**And** `OvenStatus` becomes `Idle`

### SC-012: SafetyLimitExceeded fault

**Given** an oven with `MaxTemperature = 300.0`, `CurrentTemperature = 295.0`, `Enabled = true`, `Heating = true`
**When** `FixedUpdate` runs and `CurrentTemperature` exceeds `300.0`
**Then** `OvenStatus` becomes `Faulted`
**And** `Heating` becomes `false`
**And** `Enabled` becomes `false`
**And** `FaultRaised { fault_code: SafetyLimitExceeded }` is enqueued

### SC-013: Faulted oven cannot restart heating

**Given** an oven is in `Faulted` state
**When** `SetTargetTemperature` is sent for that oven
**Then** `CommandRejected { reason: OvenNotFound }` is NOT emitted (oven exists)
**But** the system does NOT activate heating (fault blocks operation — v1 behavior: faulted ovens remain faulted, no heating possible)

### SC-014: Temperature never below ambient

**Given** an oven with `CurrentTemperature = room_temp = 20.0` and `Heating = false`
**When** many `FixedUpdate` ticks run
**Then** `CurrentTemperature` never drops below `20.0`

### SC-015: All commands rejected during emergency stop

**Given** `EmergencyStopActive = true`
**When** any command other than `EmergencyStop` is enqueued
**Then** `CommandRejected { reason: EmergencyStopActive }` is enqueued
**And** no state changes occur

---

## Summary

| Metric | Value |
|---|---|
| Requirement groups | 8 |
| Individual requirements | 22 |
| TDD contracts (test cases) | 27 |
| Scenarios (Given/When/Then) | 15 |
| Internal event types | 4 (`CommandReceivedEvent`, `OvenDetectedEvent`, `FaultDetectedEvent`, `StatusPublishEvent`) |
| Outbound protocol events | 5 (`OvenDetected`, `CommandAccepted`, `CommandRejected`, `OvenStatusUpdated`, `FaultRaised`) |
| Protocol commands processed | 4 (`SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`) |

---

## References

- Proposal: `openspec/changes/rpi-controller-bevy-headless/proposal.md`
- PRD: `docs/prd-rpi-controller-bevy-headless.md`
- Protocol crate: `protocol/src/` (archived, complete)
- ADR 001 — Protocolo enum: `docs/adr/001-protocolo-eventos-rust-enum.md`
- ADR 002 — Hysteresis: `docs/adr/002-hysteresis-control-termico.md`
- Events doc: `docs/events.md`
- Flow diagram: `docs/diagrama-flujo-protocolo.md`
- Context7 findings: `MinimalPlugins`, `ScheduleRunnerPlugin::run_loop`, `Time<Fixed>::from_seconds`, `add_event::<T>()`, `App::update()`