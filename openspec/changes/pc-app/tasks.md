# Tasks: pc-app — Bevy ECS Read Model & Command Author

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~600–750 (net additions ~550–650) |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (foundation) → PR 2 (core systems) → PR 3 (tests) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Cargo.toml + ECS foundation (components, resources, events, modules) | PR 1 | Base: feature/pc-app; tests/docs included |
| 2 | Core implementation (plugin, systems, main bootstrap) | PR 2 | Base: PR 1 branch; depends on PR 1 |
| 3 | Comprehensive tests for all spec scenarios | PR 3 | Base: PR 2 branch; verifies all scenarios |

## Phase 1: Foundation — Types, Modules, Resources

- [ ] 1.1 Clean `pc-app/Cargo.toml`: remove `serialport` and `tokio` from v1 dependencies (keep `protocol` and `bevy`)
- [ ] 1.2 Create `pc-app/src/components.rs`: define all 11 ECS components (`OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`, `FaultState`, `LastCommandResult`) + `FaultInfo` struct + `CommandResult` enum with `#[derive(Component)]`
- [ ] 1.3 Create `pc-app/src/resources.rs`: define `InboundProtocolQueue`, `OutboundProtocolQueue`, `OvenIndex(HashMap<String, Entity>)`, `GlobalFault` with `#[derive(Resource)]`
- [ ] 1.4 Create `pc-app/src/events.rs`: define 5 internal event types (`OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, `CommandRejectedReceived`) with `#[derive(Event)]`
- [ ] 1.5 Create `pc-app/src/systems/mod.rs`: module re-export for `ingest`, `state`, `commands`
- [ ] 1.6 Create `pc-app/src/plugins/mod.rs`: module re-export for `pc_app`

## Phase 2: Core — Plugin, Systems, Bootstrap

- [ ] 2.1 Create `pc-app/src/systems/ingest.rs`: `ingest_inbound_protocol` system — drains `InboundProtocolQueue`, deserializes each `EventEnvelope`, emits correct internal event variant, silently drops malformed JSON and unknown message types, ignores command variants in inbound queue
- [ ] 2.2 Create `pc-app/src/systems/state.rs`: implement 4 systems — `apply_oven_detected` (spawn/update entity via `OvenIndex`), `apply_oven_status_updated` (mutate components, ignore unknown ovens), `apply_fault_raised` (set `FaultState`, persist across status updates, handle `oven_id: None` as `GlobalFault`), `record_command_result` (store `LastCommandResult` per oven)
- [ ] 2.3 Create `pc-app/src/systems/commands.rs`: implement 4 authoring systems — `author_set_target_temperature_command`, `author_set_oven_enabled_command`, `author_request_status_command` (Single + All scope), `author_emergency_stop_command` — each constructs `EventEnvelope` with correct `Message` variant and pushes to `OutboundProtocolQueue`
- [ ] 2.4 Create `pc-app/src/plugins/pc_app.rs`: `PcAppPlugin` — registers all 5 event types, 4 resources, 8 systems in correct schedule order (`Update` for ingest + commands, `FixedUpdate` for state mutation)
- [ ] 2.5 Modify `pc-app/src/main.rs`: replace stub with Bevy app bootstrap — `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(50ms)` + `PcAppPlugin`

## Phase 3: Testing — Spec Scenario Coverage

- [ ] 3.1 Write tests for `ingest_inbound_protocol`: queue drained on tick, empty queue no-op, malformed JSON discarded, unknown message variant dropped, command variant in inbound ignored
- [ ] 3.2 Write tests for `apply_oven_detected`: first detection spawns entity with correct components, redetection updates MaxTemperature without duplicate, entity exists in `OvenIndex`
- [ ] 3.3 Write tests for `apply_oven_status_updated`: known oven gets updated components, unknown oven silently ignored, fault state survives status update
- [ ] 3.4 Write tests for `apply_fault_raised`: fault recorded on entity, global fault (`oven_id: None`) stored in `GlobalFault` resource without touching oven entities
- [ ] 3.5 Write tests for `record_command_result`: accepted command stores `accepted_type` + `message`, rejected command stores `rejected_type` + `reason` + `message`
- [ ] 3.6 Write tests for command authoring: `SetTargetTemperature` envelope correct, `SetOvenEnabled` envelope correct (both true/false), `RequestStatus` envelope for Single and All scope, `EmergencyStop` envelope with reason
- [ ] 3.7 Write integration test: full inbound flow (envelope → queue → entity state), full outbound flow (command intent → envelope in outbound queue)

## Phase 4: Cleanup & Verification

- [ ] 4.1 Verify `cargo build --package pc-app` succeeds with zero warnings
- [ ] 4.2 Verify `cargo test --package pc-app` passes all tests
- [ ] 4.3 Confirm no UI code, no real transport code, no `serialport` or `tokio` usage in v1

## Dependency Graph

```
1.1 (Cargo.toml) → 2.5 (main.rs depends on clean deps)
1.2 (components) → 2.2 (state systems need components)
1.3 (resources) → 2.1, 2.3 (systems need queues), 2.4 (plugin registers resources)
1.4 (events) → 2.1 (ingest emits events), 2.4 (plugin registers events)
1.5, 1.6 (module structure) → 2.4 (plugin imports modules)
2.1 (ingest) + 2.2 (state) + 2.3 (commands) → 2.4 (plugin registers all systems)
2.4 (plugin) → 2.5 (main.rs registers plugin)
Phase 2 → Phase 3 (tests verify all systems)
Phase 3 → Phase 4 (cleanup and verify)
```

## Acceptance Criteria Per Task

| Task | Criteria |
|------|----------|
| 1.1 | `cargo build -p pc-app` resolves; no `serialport` or `tokio` in deps |
| 1.2 | All 11 components compile with `#[derive(Component)]`; `FaultInfo` + `CommandResult` defined |
| 1.3 | All 4 resources compile; `OvenIndex` wraps `HashMap<String, Entity>` |
| 1.4 | All 5 internal event types compile with `#[derive(Event)]` |
| 2.1 | `ingest_inbound_protocol` drains queue, deserializes, dispatches 5 variants, drops malformed/unknown/command |
| 2.2 | `apply_oven_detected` spawns/updates via index; `apply_oven_status_updated` mutates components; `apply_fault_raised` persists fault; `record_command_result` stores accepted/rejected |
| 2.3 | Each author system produces correct `EventEnvelope` in `OutboundProtocolQueue` |
| 2.4 | `PcAppPlugin` registers events, resources, systems in correct schedule order |
| 2.5 | App bootstraps with `MinimalPlugins` + `ScheduleRunnerPlugin` at 50ms |
| 3.1–3.7 | All spec scenarios covered: 17+ deterministic `App::update()` tests pass |
| 4.1–4.3 | `cargo build` clean, `cargo test` passes, no UI/transport code present |
