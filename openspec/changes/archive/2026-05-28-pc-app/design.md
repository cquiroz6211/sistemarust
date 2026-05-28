# Design: pc-app — Bevy ECS Read Model & Command Author

## Technical Approach

`pc-app` v1 mirrors the `rpi-controller` Bevy ECS headless pattern: `MinimalPlugins` + `ScheduleRunnerPlugin` at 50ms `FixedUpdate`, with `Update` for protocol I/O and command authoring. No visual UI, no real transport. The PC acts as a read model consumer and command author — it displays and requests; the Raspberry validates and decides.

Data flows through two in-memory protocol queues (`InboundProtocolQueue` / `OutboundProtocolQueue`) that mirror the RPi pattern. Inbound envelopes are drained, deserialized, dispatched as internal ECS events, and routed to state-update systems. Outbound commands are authored from intent, wrapped in `EventEnvelope`, and pushed to the outbound queue.

## Architecture Decisions

### Decision: MinimalPlugins + ScheduleRunnerPlugin (headless)

**Choice**: `MinimalPlugins` with `ScheduleRunnerPlugin::run_loop(50ms)` + single `PcAppPlugin`.
**Alternatives**: `DefaultPlugins` (too heavy, needs window/GPU), bare `App::update()` loop (no real-time tick).
**Rationale**: Matches `rpi-controller` exactly — proven headless pattern. `MinimalPlugins` avoids GPU/window bootstrap. `ScheduleRunnerPlugin` gives deterministic `FixedUpdate` ticks for state transitions. Tests use `App::update()` to step deterministically.

### Decision: Shared protocol types, local components

**Choice**: Reuse `protocol::OvenState` for `OvenStatus` component; define local `FaultState` and `LastCommandResult` components (not in protocol crate).
**Alternatives**: Define duplicate PC enums; extend `protocol` crate with PC types.
**Rationale**: `protocol` is shared and complete (85 tests). Adding PC-internal types there couples the crate. Local components are small, pure data, no logic — exactly the ECS pattern.

### Decision: Separate Update and FixedUpdate schedules

**Choice**: `Update` for protocol ingestion (`ingest_inbound_protocol`) and command authoring (`author_*` systems). `FixedUpdate` for state mutation (`apply_*` systems).
**Alternatives**: All in `Update`; all in `FixedUpdate`.
**Rationale**: Protocol I/O is event-driven (Update). State mutations are deterministic tick-driven (FixedUpdate). This separation prevents non-deterministic ordering between ingestion and state application, matching the RPi pattern where `Update` handles routing and `FixedUpdate` handles domain logic.

### Decision: OvenIndex + per-entity lookup

**Choice**: `OvenIndex(HashMap<String, Entity>)` Resource for `oven_id` → `Entity` lookup. `apply_oven_detected` checks index: if entity exists, update components; if not, spawn new entity with all components.
**Alternatives**: Query all entities for `OvenId` match; use entity IDs as protocol keys.
**Rationale**: HashMap lookup is O(1) per event. Querying all entities scales poorly. Protocol uses string `oven_id`, not ECS `Entity` — index is the natural bridge.

### Decision: FaultState persistence semantics

**Choice**: `FaultState` component is set on `FaultRaised` and NEVER cleared by `OvenStatusUpdated`. Only explicit `FaultCleared` (future) or entity removal clears it. Global faults (`oven_id: None`) stored in a `GlobalFault` Resource.
**Alternatives**: Clear fault on every status update; correlate fault with status state.
**Rationale**: Spec explicitly requires fault survives status update. Faults are independent from operational state — a fault can exist while oven is idle. Global faults must not touch any oven entity.

### Decision: Command feedback via LastCommandResult component

**Choice**: `LastCommandResult` component on each oven entity stores the last accepted/rejected result. No pending-command queue in v1.
**Alternatives**: Pending command tracking with correlation IDs; event-based feedback.
**Rationale**: Spec says "record last accepted/rejected result." Correlation tracking adds complexity (pending queue, timeout) not needed for headless v1. UI layer (v1.1) can add correlation if needed.

## Data Flow

### Inbound Protocol Events

```
InboundProtocolQueue (Vec<EventEnvelope>)
        │
        ▼
┌─────────────────────────┐
│ ingest_inbound_protocol │  ← Update schedule
│ - drain queue           │
│ - deserialize each      │
│ - emit internal event   │
└─────────┬───────────────┘
          │ OvenDiscovered / OvenStatusReceived / FaultReceived / CommandAcceptedReceived / CommandRejectedReceived
          ▼
┌──────────────────────────┐
│ apply_oven_detected      │  ← FixedUpdate
│ apply_oven_status_updated│
│ apply_fault_raised       │
│ record_command_result    │
└─────────┬────────────────┘
          │ Entity updates (components)
          ▼
    Oven entities (ECS)
```

### Outbound Command Authoring

```
[External trigger: UI keypress, API, tests]
        │
        ▼
┌──────────────────────────┐
│ author_set_target_temp   │  ← Update schedule
│ author_set_oven_enabled  │  reads intent/parameters
│ author_request_status    │  constructs EventEnvelope
│ author_emergency_stop    │  pushes to OutboundProtocolQueue
└─────────┬────────────────┘
          │
          ▼
OutboundProtocolQueue (Vec<EventEnvelope>)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `pc-app/src/main.rs` | Modify | Bevy app bootstrap: `MinimalPlugins` + `ScheduleRunnerPlugin` + `PcAppPlugin` |
| `pc-app/src/components.rs` | Create | Oven ECS components: `OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`, `FaultState`, `LastCommandResult` |
| `pc-app/src/resources.rs` | Create | `InboundProtocolQueue`, `OutboundProtocolQueue`, `OvenIndex`, `GlobalFault` |
| `pc-app/src/events.rs` | Create | Internal events: `OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, `CommandRejectedReceived` |
| `pc-app/src/systems/ingest.rs` | Create | `ingest_inbound_protocol` — drain queue, deserialize, dispatch internal events |
| `pc-app/src/systems/state.rs` | Create | `apply_oven_detected`, `apply_oven_status_updated`, `apply_fault_raised`, `record_command_result` |
| `pc-app/src/systems/commands.rs` | Create | `author_set_target_temperature_command`, `author_set_oven_enabled_command`, `author_request_status_command`, `author_emergency_stop_command` |
| `pc-app/src/plugins/mod.rs` | Create | Plugin module re-export |
| `pc-app/src/plugins/pc_app.rs` | Create | `PcAppPlugin` — registers events, resources, systems in correct schedule order |
| `pc-app/src/systems/mod.rs` | Create | Systems module re-export |
| `pc-app/Cargo.toml` | Modify | Remove `serialport` and `tokio` from v1 (future transport adapters) |

**Totals**: 1 new (components.rs, resources.rs, events.rs, systems/*.rs, plugins/*.rs, systems/mod.rs, plugins/mod.rs = 9 new files), 2 modified (main.rs, Cargo.toml), 0 deleted.

## Interfaces / Contracts

### Components (from `components.rs`)

```rust
use bevy::prelude::Component;
use protocol::{FaultCode, OvenState, Severity};

#[derive(Component, Debug, Clone)]
pub struct OvenId(pub String);

#[derive(Component, Debug, Clone)]
pub struct SensorRef(pub String);

#[derive(Component, Debug, Clone)]
pub struct OutputRef(pub String);

#[derive(Component, Debug, Clone, Default)]
pub struct CurrentTemperature(pub f64);

#[derive(Component, Debug, Clone, Default)]
pub struct TargetTemperature(pub f64);

#[derive(Component, Debug, Clone, Default)]
pub struct MaxTemperature(pub f64);

#[derive(Component, Debug, Clone, Default)]
pub struct Enabled(pub bool);

#[derive(Component, Debug, Clone, Default)]
pub struct Heating(pub bool);

#[derive(Component, Debug, Clone)]
pub struct OvenStatus(pub OvenState); // Reuses protocol::OvenState

#[derive(Component, Debug, Clone, Default)]
pub struct FaultState(pub Option<FaultInfo>);

#[derive(Debug, Clone)]
pub struct FaultInfo {
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

#[derive(Component, Debug, Clone, Default)]
pub struct LastCommandResult(pub CommandResult);

#[derive(Debug, Clone)]
pub enum CommandResult {
    Accepted { accepted_type: String, message: String },
    Rejected { rejected_type: String, reason: FaultCode, message: String },
}
```

### Resources (from `resources.rs`)

```rust
use bevy::prelude::{Entity, Resource};
use protocol::EventEnvelope;
use std::collections::HashMap;

#[derive(Resource, Debug, Default)]
pub struct InboundProtocolQueue(pub Vec<EventEnvelope>);

#[derive(Resource, Debug, Default)]
pub struct OutboundProtocolQueue(pub Vec<EventEnvelope>);

#[derive(Resource, Debug, Default)]
pub struct OvenIndex(pub HashMap<String, Entity>);

#[derive(Resource, Debug, Default)]
pub struct GlobalFault(pub Option<FaultInfo>);
```

### Internal Events (from `events.rs`)

```rust
use bevy::prelude::Event;
use protocol::{FaultCode, OvenState, Severity};

#[derive(Event, Debug, Clone)]
pub struct OvenDiscovered {
    pub oven_id: String,
    pub sensor_ref: String,
    pub output_ref: String,
    pub max_celsius: f64,
}

#[derive(Event, Debug, Clone)]
pub struct OvenStatusReceived {
    pub oven_id: String,
    pub current_celsius: f64,
    pub target_celsius: f64,
    pub enabled: bool,
    pub heating: bool,
    pub output_level: Option<f64>,
    pub state: OvenState,
}

#[derive(Event, Debug, Clone)]
pub struct FaultReceived {
    pub oven_id: Option<String>,
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

#[derive(Event, Debug, Clone)]
pub struct CommandAcceptedReceived {
    pub accepted_type: String,
    pub oven_id: Option<String>,
    pub message: String,
}

#[derive(Event, Debug, Clone)]
pub struct CommandRejectedReceived {
    pub rejected_type: String,
    pub oven_id: Option<String>,
    pub reason: FaultCode,
    pub message: String,
}
```

### Plugin Registration (from `plugins/pc_app.rs`)

```rust
use bevy::prelude::*;

pub struct PcAppPlugin;

impl Plugin for PcAppPlugin {
    fn build(&self, app: &mut App) {
        // Events
        app.add_event::<events::OvenDiscovered>();
        app.add_event::<events::OvenStatusReceived>();
        app.add_event::<events::FaultReceived>();
        app.add_event::<events::CommandAcceptedReceived>();
        app.add_event::<events::CommandRejectedReceived>();

        // Resources
        app.insert_resource(resources::InboundProtocolQueue::default());
        app.insert_resource(resources::OutboundProtocolQueue::default());
        app.insert_resource(resources::OvenIndex::default());
        app.insert_resource(resources::GlobalFault::default());

        // Update: I/O and command authoring
        app.add_systems(Update, systems::ingest::ingest_inbound_protocol);
        app.add_systems(Update, systems::commands::author_set_target_temperature_command);
        app.add_systems(Update, systems::commands::author_set_oven_enabled_command);
        app.add_systems(Update, systems::commands::author_request_status_command);
        app.add_systems(Update, systems::commands::author_emergency_stop_command);

        // FixedUpdate: state mutation
        app.add_systems(FixedUpdate, systems::state::apply_oven_detected);
        app.add_systems(FixedUpdate, systems::state::apply_oven_status_updated);
        app.add_systems(FixedUpdate, systems::state::apply_fault_raised);
        app.add_systems(FixedUpdate, systems::state::record_command_result);
    }
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `ingest_inbound_protocol` drains queue correctly | `App::update()` → push envelopes to `InboundProtocolQueue` → assert queue empty |
| Unit | `apply_oven_detected` spawns entity for new oven | Push `OvenDetected` envelope → query for `OvenId` component → assert values match payload |
| Unit | `apply_oven_detected` updates existing entity (no duplicate) | Spawn entity first → push same `OvenDetected` → assert exactly 1 entity |
| Unit | `apply_oven_status_updated` mutates components | Push `OvenStatusUpdated` → query components → assert `CurrentTemperature`, `Enabled`, etc. |
| Unit | `apply_oven_status_updated` ignores unknown oven | No entity exists → push envelope → assert no new entity spawned |
| Unit | `apply_fault_raised` sets `FaultState` | Push `FaultRaised` → query `FaultState` → assert fault_code, severity, message |
| Unit | `FaultState` survives `OvenStatusUpdated` | Set fault → push status update → assert fault still present |
| Unit | Global fault (`oven_id: None`) doesn't touch oven entities | Push global fault → push oven fault → assert oven entity fault ≠ global |
| Unit | `record_command_result` stores accepted result | Push `CommandAccepted` → query `LastCommandResult` → assert accepted_type |
| Unit | `author_set_target_temperature_command` enqueues correct envelope | Trigger command → query `OutboundProtocolQueue` → assert `Message::SetTargetTemperature` with correct payload |
| Unit | `author_request_status_command` handles Single vs All scope | Push both scopes → assert correct `RequestScope` in envelope |
| Unit | Malformed JSON in inbound queue is discarded | Push invalid bytes → `App::update()` → assert no panic, valid envelopes still processed |
| Unit | Command variants in inbound queue are ignored | Push `SetTargetTemperature` in inbound → assert no internal event emitted |
| Integration | Full inbound flow: envelope → queue → entity state | Multi-envelope batch → assert all entities correct |
| Integration | Full outbound flow: command intent → envelope in queue | Author command → assert envelope in `OutboundProtocolQueue` with correct `source`/`target` |

All tests use `bevy::prelude::App` with `MinimalPlugins` and `PcAppPlugin`. No real-time dependencies (no `Time` resource needed for v1 state systems).

## Migration / Rollout

No migration required. `pc-app` is a new crate with a stub `main.rs`. No existing code is affected. Rollback: delete `pc-app/src/*.rs` (except `main.rs` stub), revert `Cargo.toml` dependencies.

## Open Questions

- [ ] Should `OutputLevel` be a separate component or stored within `OvenStatus`/`OvenStatusUpdatedPayload`? Spec doesn't mention it as a component, but the payload includes `output_level: Option<f64>`. Decide whether to track it.
- [ ] Should command authoring systems read from a `CommandQueue` resource (operator intent buffer) or accept parameters directly? The spec describes "systems that construct and enqueue" but doesn't define the trigger mechanism.
