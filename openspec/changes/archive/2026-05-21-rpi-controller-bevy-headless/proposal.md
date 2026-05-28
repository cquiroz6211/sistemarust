# Proposal: rpi-controller-bevy-headless

## Intent

Rebuild the Raspberry Pi controller domain as a **Bevy headless ECS application**, replacing the Tokio-first approach from `rpi-controller`. Bevy becomes the runtime orchestrator for oven entities, state, and periodic control — the same paradigm as the PC app, but without renderer. The Tokio/mock-transport direction is **superseded** by this change.

## Scope

### In Scope
- `App::new()` + `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(...)` headless runtime
- `Startup` schedule: spawn N simulated `Oven` entities, emit `OvenDetected` events
- `Update` schedule: ingest inbound protocol queue, validate commands, mutate ECS state, enqueue outbound protocol messages
- `FixedUpdate` schedule (50 ms ticks): simulate temperature drift, apply hysteresis (ADR 002: 5 °C band), detect faults, publish `OvenStatusUpdated`
- ECS components: `OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`
- ECS resources: `EmergencyStopActive`, `SimulationConfig`, `InboundProtocolQueue`, `OutboundProtocolQueue`, `ControllerConfig`
- Internal events via `add_event::<T>()`: `CommandReceivedEvent`, `OvenDetectedEvent`, `FaultDetectedEvent`, `StatusPublishEvent`
- Commands processed: `SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`
- Events emitted: `OvenDetected`, `CommandAccepted`, `CommandRejected`, `OvenStatusUpdated`, `FaultRaised`
- Mock I/O queues (no `serialport` real, no GPIO real)
- Tests using `App::update()` stepping for deterministic verification

### Out of Scope
- Real GPIO (`rppal`)
- Real serial port (`serialport`)
- Tokio as runtime of the domain
- PID control
- Local UI on Raspberry
- Persistence
- Multiple simultaneous PC clients

## Capabilities

### New Capabilities
- `bevy-headless-oven-controller`: Bevy ECS headless app managing oven domain lifecycle
- `fixed-tick-thermal-simulation`: Temperature drift + hysteresis on `FixedUpdate` ticks
- `ecs-command-routing`: Event-driven command ingestion, validation, and response via ECS events

### Modified Capabilities
- None (protocol crate consumed as-is; `rpi-controller` old proposal is superseded, not modified)

## Approach

**Why ECS fits the problem better than Tokio+HashMap:**

The domain is inherently ECS-shaped: multiple independent ovens, each with per-entity state, periodic processing, and event-driven state transitions. A `HashMap<String, OvenState>` behind a lock is a manual simulation of what ECS gives for free — zero-cost per-entity iteration, schedule-driven systems, and explicit data flow via events.

**Why Bevy headless reduces complexity vs Tokio-first:**

The Tokio-first design required: manual `select!` multiplexing of serial commands, temperature ticker, and REPL; a shared `OvenStore` with `RwLock`; and a custom `Transport` trait for mocking. The Bevy approach replaces all of that with: `Startup`/`Update`/`FixedUpdate` schedules that are already wired to timers; `Query<(&mut TargetTemperature, &OvenId)>` iteration; and `EventWriter`/`EventReader` for internal decoupled communication. No `RwLock`, no `select!`, no custom trait for I/O.

**I/O real (serial, GPIO) as future adapter:**

Async I/O and hardware access are not abandoned — they can return as a `Resources`-backed adapter that populates the inbound queue and drains the outbound queue. Bevy's `SystemState` allows swapping this adapter without touching domain logic.

**Context7 findings applied:**
- `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(...)` → headless without renderer
- `Time<Fixed>::from_seconds(...)` + `FixedUpdate` → fixed 50 ms ticks for thermal simulation
- `add_event::<T>()` + `EventWriter/EventReader` → internal event bus for command processing
- `App::update()` / stepping → deterministic tests without hardware

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `rpi-controller/src/` | Replaced | Full rewrite: entities, components, systems, schedules |
| `rpi-controller/src/main.rs` | Modified | CLI + Bevy app bootstrap; `MinimalPlugins`, `ScheduleRunnerPlugin` |
| `rpi-controller/src/entities.rs` | New | `Oven` entity + components |
| `rpi-controller/src/components.rs` | New | All component definitions |
| `rpi-controller/src/resources.rs` | New | `EmergencyStopActive`, queues, config |
| `rpi-controller/src/systems/startup.rs` | New | Oven spawning + `OvenDetected` emission |
| `rpi-controller/src/systems/update.rs` | New | Command ingestion, validation, response |
| `rpi-controller/src/systems/fixed_update.rs` | New | Thermal simulation + hysteresis + fault detection |
| `rpi-controller/src/simulation.rs` | Replaced | ECS-native simulation logic |
| `rpi-controller/src/control.rs` | Replaced | Hysteresis logic as Bevy system |
| `rpi-controller/src/validation.rs` | Replaced | Command validation as Bevy system |
| `rpi-controller/src/handlers.rs` | Removed | Replaced by ECS event systems |
| `rpi-controller/src/state.rs` | Removed | Replaced by ECS components |
| `rpi-controller/src/serial.rs` | Removed | Replaced by mock queues as resources |
| `rpi-controller/src/app.rs` | Removed | Replaced by Bevy schedules |
| `protocol/` | Consumed | Re-export types, no changes |
| `openspec/changes/rpi-controller/` | Superseded | This change supersedes the Tokio-first proposal |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| ECS learning curve (non-game devs) | Med | Keep ECS patterns shallow; use clear component naming; document schedule intent |
| Over-modeling with events | Med | Reserve `add_event` for cross-system communication; state lives in components, not events |
| Future I/O integration complexity | Low | Keep inbound/outbound queues as Resources; I/O adapter is a thin wrapper, not domain logic |
| Bevy tick rate calibration | Low | Make tick interval configurable; ADR 002 hysteresis already reduces sensitivity |

## Rollback Plan

1. Delete `openspec/changes/rpi-controller-bevy-headless/`
2. `rpi-controller/src/` restore from `git checkout HEAD~1 -- rpi-controller/src/` (Tokio-first code)
3. No other crates affected; protocol is consumed as-is

## Dependencies

- `protocol` crate (completed)
- `bevy` (headless: `bevy` with `MinimalPlugins` — no `bevy_ui`, `bevy_render`)
- `serde`, `uuid`, `chrono` (already in protocol)

## Success Criteria

- [ ] `rpi-controller` starts headless with `--simulate N` flag
- [ ] `Startup` spawns N `Oven` entities and emits `OvenDetected` for each
- [ ] `SetTargetTemperature` updates component state and emits `CommandAccepted` or `CommandRejected`
- [ ] `SetOvenEnabled` updates `Enabled` component and emits `CommandAccepted` or `CommandRejected`
- [ ] `RequestStatus` emits `OvenStatusUpdated` per matching oven
- [ ] `EmergencyStop` sets `EmergencyStopActive` resource, all ovens → `EmergencyStopped`, emits `CommandAccepted`
- [ ] `FixedUpdate` (50 ms) drifts temperature, applies 5 °C hysteresis, emits `OvenStatusUpdated` periodically
- [ ] `FaultRaised` emitted when `current_celsius > max_celsius`
- [ ] All domain logic testable via `App::update()` stepping without hardware
- [ ] No `tokio` runtime as orchestrator of domain logic