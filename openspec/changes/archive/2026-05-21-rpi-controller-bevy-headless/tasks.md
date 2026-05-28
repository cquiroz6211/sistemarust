# Tasks: `rpi-controller-bevy-headless`

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~900–1100 (net additions ~600–800) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (infra) → PR 2 (core) → PR 3 (tests + cleanup) |

```
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High
```

### Suggested Work Units

| Unit | Goal | Base branch |
|------|------|-------------|
| PR 1 | Cargo.toml + ECS foundation (components, resources, events) | `feature/rpi-controller-bevy-headless` |
| PR 2 | Core (plugin, systems, main bootstrap, bevy_app) | PR 1 branch |
| PR 3 | Integration tests + deletion of old Tokio-first files | PR 2 branch |

---

## Phase 1: Cargo Setup + ECS Foundation (~260 lines)

- [ ] **1.1** Update `rpi-controller/Cargo.toml`: add `bevy` dependency, keep `rand` (remove `tokio` features, keep for `StdRng` compat only)
- [ ] **1.2** Create `rpi-controller/src/components.rs`: define all 9 component types (`OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`) with `#[derive(Component)]`; define `OvenState` enum
- [ ] **1.3** Create `rpi-controller/src/resources.rs`: define all resources (`EmergencyStopActive`, `SimulationConfig`, `InboundProtocolQueue`, `OutboundProtocolQueue`, `ControllerConfig`, `TestRng`, `OvenIndex(HashMap)`, `SimulateOvenCount`)
- [ ] **1.4** Create `rpi-controller/src/events.rs`: define internal ECS event types (`CommandReceivedEvent`, `OvenDetectedEvent`, `FaultDetectedEvent`, `StatusPublishEvent`) with `#[derive(Event)]`

---

## Phase 2: Core Logic — Plugin + Systems (~500 lines)

- [ ] **2.1** Create `rpi-controller/src/plugins/oven_controller.rs`: `OvenControllerPlugin` bundling all systems, resources, event types, and component derive
- [ ] **2.2** Create `rpi-controller/src/systems/startup.rs`: `spawn_simulated_ovens` system — reads `SimulateOvenCount`, spawns N oven entities with all components, emits `OvenDetectedEvent`; `emit_oven_detected_on_startup` drains internal events → `OutboundProtocolQueue` via `EventEnvelope::new`
- [ ] **2.3** Create `rpi-controller/src/systems/update.rs`: `ingest_commands` drains `InboundProtocolQueue` → `CommandReceivedEvent`; `route_command` matches `Message` variants, validates, mutates components, emits `CommandAccepted`/`CommandRejected`; `emit_protocol_responses` drains event writers → `OutboundProtocolQueue`
- [ ] **2.4** Create `rpi-controller/src/systems/fixed_update.rs`: `thermal_drift` applies drift + noise; `hysteresis_control` applies 5 °C band; `derive_oven_status` state machine; `fault_detection` triggers `FaultRaised`; `periodic_status` publishes all ovens on interval
- [ ] **2.5** Create `rpi-controller/src/bevy_app.rs`: `build_app(simulate_count) → App` helper with `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(FixedInterval(50ms))`, inserts all resources, registers `OvenControllerPlugin`
- [ ] **2.6** Modify `rpi-controller/src/main.rs`: drop `#[tokio::main]`, parse `--simulate N`, call `build_app`, run headless

---

## Phase 3: Integration Tests (~200 lines)

- [ ] **3.1** Write tests for startup: `app_headless_starts_without_panic`, `startup_spawns_n_ovens`, `startup_emits_oven_detected_per_oven`, `oven_entity_has_correct_initial_state`
- [ ] **3.2** Write tests for commands: `set_target_temperature_accepts_valid`, `set_target_temperature_rejects_negative`, `set_target_temperature_rejects_above_max`, `set_target_temperature_rejects_nonexistent_oven`, `set_target_temperature_rejects_when_emergency_stop_active`, `command_accepted_has_correct_correlation_id`, `outbound_queue_maintains_fifo_order`
- [ ] **3.3** Write tests for fixed_update: `fixed_update_heats_oven_when_below_hysteresis_band`, `fixed_update_stops_heating_when_at_target`, `fixed_update_maintains_heating_in_band`, `fixed_update_never_drops_below_room_temp`, `fixed_update_derives_correct_oven_status`, `hysteresis_prevents_rapid_oscillation`
- [ ] **3.4** Write tests for faults/emergency: `fault_raised_when_temperature_exceeds_max`, `faulted_oven_cannot_be_heating`, `emergency_stop_accepted_during_fault`, `no_commands_accepted_during_emergency_stop`

---

## Phase 4: Cleanup (~300 lines removed)

- [ ] **4.1** Delete `rpi-controller/src/app.rs` (Tokio select! loop)
- [ ] **4.2** Delete `rpi-controller/src/handlers.rs`, `state.rs`, `control.rs`, `simulation.rs`, `validation.rs`, `mock_transport.rs`
- [ ] **4.3** Update `rpi-controller/src/lib.rs`: new module tree matching the ECS structure
- [ ] **4.4** Verify `cargo build --package rpi-controller` succeeds with zero warnings from old code

---

## Dependency Graph

```
1.1 (Cargo.toml) → 1.2, 1.3, 1.4 (parallel)
1.4 (events) → 2.2 (startup needs OvenDetectedEvent)
1.2 (components) + 1.3 (resources) → 2.1 (plugin imports both)
2.1 (plugin) → 2.5 (bevy_app registers plugin)
2.5 (bevy_app) → 2.6 (main.rs calls bevy_app)
2.6 (main.rs) → Phase 3 (tests use App directly)
Phase 3 → Phase 4 (cleanup after tests pass)
```

---

## Acceptance Criteria Per Task

| Task | Criteria |
|------|----------|
| 1.1 | `cargo build -p rpi-controller` resolves `bevy` |
| 1.2 | All 9 components compile with `#[derive(Component)]`; `OvenState` enum has all 5 variants |
| 1.3 | All 7 resources compile; `OvenIndex` wraps `HashMap<String, Entity>`; `TestRng` wraps `StdRng` |
| 1.4 | All 4 internal event types compile with `#[derive(Event)]` |
| 2.1 | `OvenControllerPlugin` registers all systems in correct schedule order |
| 2.2 | `Startup` spawns N ovens, emits `OvenDetected` internal events, `OutboundProtocolQueue` receives `OvenDetected` envelopes |
| 2.3 | All 4 commands processed; correct `FaultCode` on rejection; `correlation_id` preserved |
| 2.4 | `FixedUpdate` runs 50 ms ticks; hysteresis band respected; fault detection emits `FaultRaised`; periodic status publishes on interval |
| 2.5 | `build_app(N)` returns configured `App` with all resources and plugins |
| 2.6 | Binary accepts `--simulate N`, starts headless, spawns N ovens |
| 3.1–3.4 | All 27 TDD contract tests pass deterministically |
| 4.1–4.3 | Old files deleted; `lib.rs` updated; `cargo build` clean |