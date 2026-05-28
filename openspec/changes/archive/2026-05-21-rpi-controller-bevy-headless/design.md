# Design: `rpi-controller-bevy-headless`

## Technical Approach

Replace the Tokio/`select!`/RwLock orchestration with a **Bevy headless ECS app** using `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(FixedInterval(50ms))`. The domain maps directly to ECS: each oven is an entity with per-entity components; thermal simulation runs in `FixedUpdate`; command ingestion runs in `Update`. All protocol messages route through `InboundProtocolQueue` / `OutboundProtocolQueue` Resources. No `tokio` runtime wraps domain logic.

---

## 1. Module/File Layout

```
rpi-controller/src/
├── lib.rs                    # crate root — re-exports
├── main.rs                   # CLI + App::run() bootstrap
├── bevy_app.rs               # MinimalPlugins + ScheduleRunnerPlugin setup
├── components.rs             # OvenId, SensorRef, OutputRef, CurrentTemperature,
│                             # TargetTemperature, MaxTemperature, Enabled,
│                             # Heating, OvenStatus
├── resources.rs              # EmergencyStopActive, SimulationConfig,
│                             # InboundProtocolQueue, OutboundProtocolQueue,
│                             # ControllerConfig, TestRng, OvenIndex
├── events.rs                 # Internal ECS event types via add_event::<T>()
├── systems/
│   ├── startup.rs            # spawn_simulated_ovens → OvenDetected
│   ├── update.rs             # ingest_commands, route_command,
│   │                         # emit_protocol_responses
│   └── fixed_update.rs       # thermal_drift, hysteresis_control,
│                             # derive_oven_status, periodic_status,
│                             # fault_detection
└── plugins/
    └── oven_controller.rs     # Plugin bundling all systems + resources
```

**Files deleted**: `app.rs`, `handlers.rs`, `state.rs` (Tokio+HashMap layer), `validation.rs`, `mock_transport.rs`, `simulation.rs`, `control.rs` — replaced by ECS equivalents.

**Files modified**: `lib.rs` (new module tree), `main.rs` (Bevy bootstrap, drop Tokio `#[tokio::main]`).

---

## 2. App / Plugin Composition

```rust
// main.rs
use bevy::prelude::*;
use rpi_controller::plugins::OvenControllerPlugin;
use rpi_controller::resources::SimulationConfig;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let simulate_count = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut app = App::new();
    app.insert_resource(SimulationConfig { /* from spec */ })
       .insert_resource(InboundProtocolQueue::default())
       .insert_resource(OutboundProtocolQueue::default())
       .insert_resource(EmergencyStopActive(false))
       .insert_resource(ControllerConfig {
           tick_interval_ms: 50,
           status_publish_interval_ms: 1000,
       });

    if simulate_count > 0 {
        app.insert_resource(SimulateOvenCount(simulate_count));
    }

    app.add_plugins(MinimalPlugins)
       .add_plugins(ScheduleRunnerPlugin::run_loop(
           bevy::tasks::FixedIntervalDuration::new(std::time::Duration::from_millis(50))
       ))
       .add_plugins(OvenControllerPlugin);  // all domain logic
}
```

**Plugin** `OvenControllerPlugin` registers:
- `components.rs` types via derive
- `events.rs` via `add_event::<T>()`
- `resources.rs` via `insert_resource`
- Systems in correct schedule order (see §4)

---

## 3. Exact ECS Model

### Components

| Component | Type | Note |
|---|---|---|
| `OvenId` | `String` | stable identifier |
| `SensorRef` | `String` | sensor label |
| `OutputRef` | `String` | actuator label |
| `CurrentTemperature` | `f64` | simulated sensor value |
| `TargetTemperature` | `f64` | desired temperature |
| `MaxTemperature` | `f64` | safety limit (300.0) |
| `Enabled` | `bool` | logical enable |
| `Heating` | `bool` | heating output active |
| `OvenStatus` | `OvenState` | composite state enum |

### Resources

| Resource | Type |
|---|---|
| `EmergencyStopActive` | `bool` |
| `SimulationConfig` | `struct { heating_drift: f64, cooling_drift: f64, room_temp: f64, noise_amplitude: f64, hysteresis: f64 }` |
| `InboundProtocolQueue` | `Vec<EventEnvelope>` |
| `OutboundProtocolQueue` | `Vec<EventEnvelope>` |
| `ControllerConfig` | `struct { tick_interval_ms: u64, status_publish_interval_ms: u64 }` |
| `TestRng` | `Option<StdRng>` (seeded RNG opcional para tests deterministas; `None` en runtime usa fallback sin ruido aleatorio efectivo) |
| `OvenIndex` | `HashMap<String, Entity>` |
| `SimulateOvenCount` | `usize` (consumed at Startup to spawn N ovens) |

### Internal ECS Events

| Event | Fields | Purpose |
|---|---|---|
| `CommandReceivedEvent` | `EventEnvelope` | decoupling command ingestion from processing |
| `CommandAcceptedEvent` | `EventEnvelope` | desacoplar aceptación de comando de la publicación final |
| `CommandRejectedEvent` | `EventEnvelope` | desacoplar rechazo de comando de la publicación final |
| `OvenDetectedEvent` | `(String oven_id, String sensor_ref, String output_ref, f64 max_celsius)` | signals new oven for protocol emission |
| `FaultDetectedEvent` | `(String oven_id, FaultCode, Severity, String)` | decoupling fault detection from event emission |
| `StatusPublishEvent` | `Option<String> oven_id` | request status publication |

---

## 4. System Layout by Schedule

### `Startup` (runs once at boot)

```
spawn_simulated_ovens
  reads SimulateOvenCount
  inserts N entities with all 9 components
  emits OvenDetectedEvent per entity
  emits OvenDetected EventEnvelope → OutboundProtocolQueue
  consumes SimulateOvenCount
```

### `Update` (every frame tick)

```
ingest_commands
  drains InboundProtocolQueue
  for each envelope: emits CommandReceivedEvent

route_command
  reads CommandReceivedEvent
  matches Message variant
  SetTargetTemperature → validate + update TargetTemperature + emit CommandAccepted/Rejected
  SetOvenEnabled      → validate + update Enabled + emit CommandAccepted/Rejected
  RequestStatus       → emit OvenStatusUpdated per matching oven (no CommandAccepted)
  EmergencyStop       → set EmergencyStopActive + all ovens → EmergencyStopped + emit CommandAccepted
  all via EventWriter to preserve decoupling

emit_protocol_responses
  drains internal event writers
  builds EventEnvelope::reply_to(...) for CommandAccepted/Rejected
  appends to OutboundProtocolQueue
```

### `FixedUpdate` (every 50 ms)

```
thermal_drift
  Query<(Entity, &mut CurrentTemperature, &TargetTemperature, &Enabled, &Heating)>
  applies heating_drift (4°C) or cooling_drift (1°C) + noise
  CurrentTemperature = CurrentTemperature.max(room_temp)

hysteresis_control
  Query<&mut Heating>
  applies hysteresis rule: below target−5 → true, at/above target → false, else hold

derive_oven_status
  Query<(Entity, &mut OvenStatus, &Enabled, &Heating, &CurrentTemperature, &MaxTemperature)>
  updates OvenStatus per state machine table (spec REQ-TC-003)

fault_detection
  Query<(Entity, &CurrentTemperature, &MaxTemperature)>
  if current > max: set OvenStatus=Faulted, Heating=false, Enabled=false
  emit FaultDetectedEvent → FaultRaised envelope → OutboundProtocolQueue

periodic_status
  Resource<LastStatusPublishTime>
  if interval elapsed: emit StatusPublishEvent(None) → publish all ovens
```

### Execution ordering within `FixedUpdate`

```
1. thermal_drift       (temperature must be updated before hysteresis reads it)
2. hysteresis_control  (reads updated CurrentTemperature, writes Heating)
3. derive_oven_status  (reads Heating + CurrentTemperature, writes OvenStatus)
4. fault_detection     (reads CurrentTemperature, may overwrite OvenStatus)
5. periodic_status     (reads OvenStatus, may emit events)
```

---

## 5. Protocol Bridge Design

### Inbound (PC → RPi)

```
MockTransport → InboundProtocolQueue (Vec<EventEnvelope>)
update() ingest_commands drains queue → CommandReceivedEvent
```

No `serialport` dependency in v1. `InboundProtocolQueue` is a `Vec<EventEnvelope>` Resource; I/O adapter (future `SerialPortAdapter`) writes envelopes into it.

### Outbound (RPi → PC)

```
OutboundProtocolQueue (Vec<EventEnvelope>) → MockTransport (or SerialPortAdapter in future)
```

### Correlation handling

All outbound responses use `EventEnvelope::reply_to(&original)`:

```rust
// In route_command:
let response = inbound.reply_to(Message::CommandAccepted(CommandAcceptedPayload { ... }));
outbound_queue.push(response);
```

`reply_to` automatically: inverts `source`↔`target`, sets `correlation_id = Some(original.event_id)`, preserves `version`.

`EmergencyStop` response: exactly one `CommandAccepted` for all ovens, no per-oven emission.

---

## 6. Simulated Oven Creation & Event Emission

```rust
// startup.rs pseudocode
fn spawn_simulated_ovens(
    mut commands: Commands,
    simulate_count: Res<SimulateOvenCount>,
    mut ev_oven_detected: EventWriter<OvenDetectedEvent>,
    mut ev_status: EventWriter<StatusPublishEvent>,
) {
    for i in 0..simulate_count.0 {
        let oven_id = format!("oven-{}", i);
        commands.spawn((
            OvenId(oven_id.clone()),
            SensorRef(format!("temp-{}", i)),
            OutputRef(format!("relay-{}", i)),
            CurrentTemperature(ROOM_TEMP),
            TargetTemperature(ROOM_TEMP),
            MaxTemperature(300.0),
            Enabled(false),
            Heating(false),
            OvenStatus(OvenState::Disabled),
        ));
        ev_oven_detected.send(OvenDetectedEvent { oven_id });
    }
}

fn emit_oven_detected_on_startup(
    mut ev_oven_detected: EventReader<OvenDetectedEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    for ev in ev_oven_detected.read() {
        outbound.push(EventEnvelope::new(
            "rpi-controller",
            "pc-app",
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: ev.oven_id.clone(),
                sensor_ref: format!("temp-{}", /* lookup */),
                output_ref: format!("relay-{}", /* lookup */),
                max_celsius: 300.0,
            }),
        ));
    }
}
```

---

## 7. Thermal Control Design

### Constants (from `SimulationConfig` resource)

| Constant | Value |
|---|---|
| `heating_drift` | 4.0 °C/tick |
| `cooling_drift` | 1.0 °C/tick |
| `room_temp` | 20.0 °C |
| `noise_amplitude` | 2.0 °C ± |
| `hysteresis` | 5.0 °C |

### Hysteresis rule (ADR 002)

```rust
// fixed_update.rs
fn hysteresis_control(
    config: Res<SimulationConfig>,
    mut query: Query<(&mut Heating, &CurrentTemperature, &TargetTemperature, &Enabled)>,
) {
    for (mut heating, current, target, enabled) in &mut query {
        if !enabled.0 { *heating.0 = false; continue; }
        let lower = target.0 - config.hysteresis;
        *heating.0 = if current.0 < lower {
            true
        } else if current.0 >= target.0 {
            false
        } else {
            heating.0 // maintain previous — dead band
        };
    }
}
```

### Noise in tests

`TestRng` es un `Resource` explícito con forma `Option<StdRng>`. En tests se inyecta `Some(seed)` para determinismo; en runtime normal se usa `None`, manteniendo la API simple sin obligar una semilla global.

### Fault detection

```rust
fn fault_detection(
    mut commands: Commands,
    query: Query<(Entity, &CurrentTemperature, &MaxTemperature)>,
    mut ev_fault: EventWriter<FaultDetectedEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    for (entity, current, max) in &query {
        if current.0 > max.0 {
            commands.entity(entity)
                .insert(OvenStatus(OvenState::Faulted))
                .insert(Enabled(false))
                .insert(Heating(false));
            outbound.push(EventEnvelope::new(
                "rpi-controller", "pc-app",
                Message::FaultRaised(FaultRaisedPayload {
                    oven_id: /* lookup via query */,
                    fault_code: FaultCode::SafetyLimitExceeded,
                    severity: Severity::High,
                    message: format!("{}°C > {}°C limit", current.0, max.0),
                }),
            ));
        }
    }
}
```

---

## 8. Test Strategy

| Layer | What | Approach |
|---|---|---|
| Unit | Hysteresis decision logic | `#[test]` on pure fn `heating_decision(...)` |
| Unit | Component state transitions | `#[test]` on `derive_oven_status` logic |
| Unit | Fault detection | `#[test]` on fault system with ECS query |
| Integration | Full command flow | `App::update()` + drain `OutboundProtocolQueue` |
| Integration | Thermal simulation | `App::update()` + `FixedUpdate` tick + assert temperature |
| Integration | Hysteresis oscillation prevention | Seeded `TestRng`, multiple ticks, assert ≤ 1 `Heating` flip per transition |

### Determinism

```rust
// In tests:
let mut app = App::new();
app.insert_resource(TestRng(Some(StdRng::seed_from_u64(42))));
app.add_plugins(OvenControllerPlugin);
app.update(); // Startup runs
app.update(); // First Update
// FixedUpdate requires Duration > 50ms to tick:
app.update().duration(Duration::from_millis(60));
```

---

## 9. Key Design Decisions

| Decision | Choice | Alternatives | Rationale |
|---|---|---|---|
| Runtime | `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(FixedInterval(50ms))` | `DynamicSchedul_plugins` + manual timer | Minimal (no `bevy_ui`/`bevy_render`), fixed ticks match thermal sim spec |
| State location | ECS components (per-entity) | `HashMap<String, Oven>` in Resource | Zero-cost iteration, system readability, schedule co-location |
| I/O decoupling | `InboundProtocolQueue` + `OutboundProtocolQueue` Resources | Direct `Transport` calls in systems | Future serial/GPIO adapter just populates/drains queues; domain logic unchanged |
| Event bus | `add_event::<T>()` internal ECS events | `Channel<T>` or `post()` | Native to Bevy, testable via `EventReader`, decouples ingest/process/emit |
| RNG determinism | `TestRng(pub Option<StdRng>)` Resource | `rand::thread_rng()` global | Mantiene tests deterministas sin complejizar el bootstrap de runtime |
| Oven lookup | `OvenIndex(HashMap<String, Entity>)` Resource | `Query` O(N) por `oven_id` | El protocolo referencia `oven_id`, no `Entity`; el índice mantiene el bridge protocolo↔ECS limpio |
| Fault handling | `Faulted` oven sets `Enabled=false, Heating=false` | Latched fault, auto-reset | v1: faulted ovens stay faulted; explicit intervention (out of scope) |
| Protocol replies | `EventEnvelope::reply_to()` | Manual envelope construction | Auto-handles `source`/`target` flip + `correlation_id`; no duplication |

---

## 10. Open Questions — RESOLVED

- [x] **TestRng injection point** → `TestRng` será un `Resource` explícito.
- [x] **`OvenStatusUpdated` periodic trigger** → se usará `FixedUpdate` y contador/tiempo dentro de recursos existentes, sin plugin de timers adicional.
- [x] **`SensorRef`/`OutputRef` in `OvenDetectedEvent`** → el evento interno lleva payload completo.
- [x] **Entity lookups by `OvenId`** → se usará `OvenIndex(HashMap<String, Entity>)` como `Resource`.

---

## File Changes Summary

| File | Action | Description |
|---|---|---|
| `rpi-controller/src/lib.rs` | Modify | New module tree: components, resources, events, systems/, plugins/ |
| `rpi-controller/src/main.rs` | Modify | Drop `#[tokio::main]`, add Bevy `App` bootstrap with `MinimalPlugins` + `ScheduleRunnerPlugin` |
| `rpi-controller/src/bevy_app.rs` | Create | `build_app(simulate_count) → App` helper |
| `rpi-controller/src/components.rs` | Create | All 9 component types with `#[derive(Component)]` |
| `rpi-controller/src/resources.rs` | Create | All resource types, incluyendo `OvenIndex` y `TestRng` |
| `rpi-controller/src/events.rs` | Create | Internal event types via `add_event::<T>()` |
| `rpi-controller/src/systems/startup.rs` | Create | Oven spawning + OvenDetectedEvent emission |
| `rpi-controller/src/systems/update.rs` | Create | Ingest, route, respond |
| `rpi-controller/src/systems/fixed_update.rs` | Create | Thermal, hysteresis, status, faults |
| `rpi-controller/src/plugins/oven_controller.rs` | Create | Plugin bundling all systems |
| `rpi-controller/src/app.rs` | Delete | Tokio select! loop |
| `rpi-controller/src/handlers.rs` | Delete | Replace with ECS event systems |
| `rpi-controller/src/state.rs` | Delete | Replace with ECS components |
| `rpi-controller/src/control.rs` | Delete | Hysteresis logic moves to system |
| `rpi-controller/src/simulation.rs` | Delete | Thermal drift moves to system |
| `rpi-controller/src/validation.rs` | Delete | Validation moves to command routing |
| `rpi-controller/src/mock_transport.rs` | Delete | Replaced by queues + direct append |

**Dependencies**: `bevy`, `protocol` crate (unchanged). `rand` (already present via `simulation.rs`).
