# Exploration: Dynamic Simulated Ovens at Runtime

## 1. Recommended Approach

**Add a new protocol command `CreateSimulatedOvens { count }` that rpi-controller handles in `route_command`, spawning new oven entities at runtime and emitting `OvenDetected` + `OvenStatusUpdated` events back to pc-app.**

### Justification

This is a protocol-level change, not a UI-only trick. The user explicitly agreed this should be a real command. The pattern already exists: the protocol supports bidirectional `EventEnvelope` messages, rpi-controller processes inbound commands via `route_command`, and pc-app maintains a read model via `apply_oven_detected`. We follow the same flow:

```
PC: UiIntent → author_create_simulated_ovens_command → OutboundProtocolQueue
    → transport → rpi-controller InboundProtocolQueue
    → route_command match → spawn runtime ovens
    → emit OvenDetected + OvenStatusUpdated → OutboundProtocolQueue
    → transport → PC ingest → apply_oven_detected → spawn PC entities
```

### Alternative considered: `AddSimulatedOvens`
- Rejected: `CreateSimulatedOvens` is more explicit about what it does (creation, not addition). The verb "create" signals this is a resource-allocation command.

## 2. Protocol Changes

### 2.1 New Payload (`protocol/src/payloads.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateSimulatedOvensPayload {
    pub count: u32,
}
```

### 2.2 New Message Variant (`protocol/src/message.rs`)

Add to the "Comandos (PC → Raspberry Pi)" section of `Message` enum:

```rust
CreateSimulatedOvens(CreateSimulatedOvensPayload),
```

This makes it the 5th command variant. The serde `tag = "type"` external tagging will serialize as `{"type":"CreateSimulatedOvens","payload":{"count":3}}`.

### 2.3 Re-export (`protocol/src/lib.rs`)

Add `CreateSimulatedOvensPayload` to the re-exports.

### 2.4 Response Events (existing, reused)

No new event types needed. The response flow reuses:
- `CommandAccepted` — with `accepted_type: "CreateSimulatedOvens"` and `message: "Created N ovens: oven-5, oven-6, oven-7"`
- `OvenDetected` — one per new oven (existing event type)
- `OvenStatusUpdated` — one per new oven (existing event type, emitted by periodic_status)

### 2.5 FaultCode extension (optional, for rejection)

Add `MaxOvensReached` to `FaultCode` enum for when the count exceeds the limit:

```rust
pub enum FaultCode {
    OvenNotFound,
    SensorUnavailable,
    InvalidTemperature,
    OutputUnavailable,
    SafetyLimitExceeded,
    EmergencyStopActive,
    MaxOvensReached,  // NEW
}
```

## 3. rpi-controller Changes

### 3.1 New Resource: `MaxSimulatedOvens` (`rpi-controller/src/resources.rs`)

```rust
#[derive(Debug, Clone, Resource)]
pub struct MaxSimulatedOvens(pub usize);

impl Default for MaxSimulatedOvens {
    fn default() -> Self {
        Self(100)  // Presentation/demo safe default
    }
}
```

### 3.2 Runtime Oven Counter (`rpi-controller/src/resources.rs`)

Track the highest oven index used so far, to avoid ID collisions with startup ovens:

```rust
#[derive(Debug, Clone, Resource)]
pub struct NextOvenIndex(pub usize);

impl Default for NextOvenIndex {
    fn default() -> Self {
        Self(0)  // Will be set to simulate_count by build_app
    }
}
```

### 3.3 `build_app` modification (`rpi-controller/src/bevy_app.rs`)

Set `NextOvenIndex` to `simulate_count` so runtime ovens start numbering after startup ovens:

```rust
app.insert_resource(crate::resources::NextOvenIndex(simulate_count));
```

### 3.4 Extract `spawn_single_oven` helper (`rpi-controller/src/systems/startup.rs`)

Extract the oven entity creation loop body into a reusable function:

```rust
pub fn spawn_single_oven(
    commands: &mut Commands,
    oven_index: &mut ResMut<OvenIndex>,
    next_index: &mut ResMut<NextOvenIndex>,
    config: &SimulationConfig,
) -> (String, Entity) {
    let idx = next_index.0;
    next_index.0 += 1;
    let oven_id = format!("oven-{}", idx);
    let sensor_ref = format!("temp-{}", idx);
    let output_ref = format!("relay-{}", idx);
    
    let entity = commands.spawn((
        OvenId(oven_id.clone()),
        SensorRef(sensor_ref.clone()),
        OutputRef(output_ref.clone()),
        CurrentTemperature(config.room_temp),
        TargetTemperature(config.room_temp),
        MaxTemperature(300.0),
        Enabled(false),
        Heating(false),
        OvenStatus(protocol::OvenState::Disabled),
    )).id();
    
    oven_index.0.insert(oven_id.clone(), entity);
    (oven_id, entity)
}
```

Refactor `spawn_ovens` to call this helper in a loop.

### 3.5 New command handler in `route_command` (`rpi-controller/src/systems/update.rs`)

Add a new match arm:

```rust
Message::CreateSimulatedOvens(payload) => {
    handle_create_simulated_ovens(
        envelope,
        payload,
        &oven_index,
        &mut commands,
        &mut next_index,
        &config,
        &mut max_ovens,
        &mut ev_accepted,
        &mut ev_rejected,
        &mut ev_oven_detected,
        &mut outbound,
    );
}
```

Handler logic:
1. Check `EmergencyStopActive` — reject with `EmergencyStopActive` fault
2. Check `MaxSimulatedOvens` — reject with `MaxOvensReached` if current + count > max
3. For each count: call `spawn_single_oven`, collect oven_ids
4. Emit `OvenDetectedEvent` + outbound `Message::OvenDetected` for each new oven
5. Emit `CommandAccepted` with summary message listing created IDs
6. Emit `StatusPublishEvent(None)` to trigger status publication for new ovens

### 3.6 Plugin registration (`rpi-controller/src/plugins/oven_controller.rs`)

Add `MaxSimulatedOvens` and `NextOvenIndex` to resource insertions.

### 3.7 `command_type_name` helper (`rpi-controller/src/systems/update.rs`)

Add `"CreateSimulatedOvens"` to the match.

## 4. pc-app Changes

### 4.1 New UI Intent field (`pc-app/src/resources.rs`)

```rust
pub struct UiIntent {
    // ... existing fields ...
    pub create_simulated_ovens: Option<u32>,
}
```

### 4.2 New authoring function (`pc-app/src/systems/commands.rs`)

```rust
pub fn author_create_simulated_ovens_command(
    count: u32,
    outbound: &mut OutboundProtocolQueue,
) {
    let envelope = EventEnvelope::new(
        SOURCE, TARGET,
        Message::CreateSimulatedOvens(CreateSimulatedOvensPayload { count }),
    );
    outbound.0.push(envelope);
}
```

### 4.3 UI Panel — "Add Simulated Ovens" (`pc-app/src/ui/panels.rs`)

Add a section to the header panel or central panel (below oven cards) with:
- A numeric input or +/- buttons for count (default 1, range 1-20)
- An "Add Ovens" button
- Disabled state when disconnected or emergency stop active

This is a low-frequency control, so it doesn't need per-oven placement. The header panel is the right place since it's the global controls area.

### 4.4 UI Dispatch (`pc-app/src/systems/ui/dispatch.rs`)

```rust
if let Some(count) = intent.create_simulated_ovens.take() {
    author_create_simulated_ovens_command(count, &mut outbound);
}
```

### 4.5 Log Capture (`pc-app/src/systems/ui/log_capture.rs`)

Add a case in the outbound capture match:

```rust
Message::CreateSimulatedOvens(p) => Some(LogEntry {
    direction: LogDirection::Out,
    message_type: "CreateSimulatedOvens".into(),
    summary: format!("Requesting {} simulated ovens", p.count),
}),
```

## 5. Test Plan

### 5.1 Protocol tests (`protocol/tests/roundtrip.rs` + `protocol/src/payloads.rs`)

- Round-trip serialization of `CreateSimulatedOvensPayload`
- Round-trip of `Message::CreateSimulatedOvens(...)` envelope
- JSON field names are snake_case (`count`)
- Missing `count` field causes deserialization error
- Unknown variant rejection test for `RemoveOven` should still pass (unrelated)

### 5.2 rpi-controller tests (`rpi-controller/tests/integration_tests.rs`)

- **Valid creation**: Push `CreateSimulatedOvens { count: 3 }`, verify 3 new oven entities spawned with IDs "oven-3", "oven-4", "oven-5" (assuming 3 startup ovens)
- **ID continuity**: New ovens continue numbering from startup count, no duplicates
- **Max overflow**: Push `CreateSimulatedOvens { count: 999 }` when max is 100 and 95 exist → reject with `MaxOvensReached`
- **Emergency stop**: Push `CreateSimulatedOvens` when emergency stop active → reject with `EmergencyStopActive`
- **Zero count**: Push `CreateSimulatedOvens { count: 0 }` → accepted but no ovens spawned (no-op)
- **OvenDetected events**: Verify N `OvenDetected` envelopes emitted to outbound queue
- **Oven entity state**: Verify new ovens have correct initial state (room_temp, disabled, etc.)
- **OvenIndex consistency**: Verify `OvenIndex` contains all new oven IDs → entity mappings

### 5.3 pc-app tests (`pc-app/tests/`)

- **Command authoring**: `author_create_simulated_ovens_command(5, ...)` pushes correct envelope to outbound queue
- **UI integration**: Setting `UiIntent.create_simulated_ovens = Some(3)` triggers command dispatch

### 5.4 Integration tests (transport layer)

- Full flow: pc-app sends `CreateSimulatedOvens` → transport → rpi-controller receives → spawns ovens → sends `OvenDetected` → transport → pc-app receives → spawns PC entities

## 6. Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Duplicate oven IDs** | High | Use `NextOvenIndex` resource that persists and increments. Both startup and runtime use the same counter. |
| **No max guard → memory exhaustion** | High | `MaxSimulatedOvens` resource (default 100). Rejected with `FaultCode::MaxOvensReached`. |
| **Simulation command leaks to production** | Medium | New `FaultCode::MaxOvensReached` is only relevant for simulation. The `CreateSimulatedOvens` command name is explicit. No hardware path exists — this command only spawns ECS entities. |
| **Status update spam with many ovens** | Medium | `periodic_status` already publishes all ovens. With 100 ovens at 1s interval, that's 100 messages/sec — acceptable for localhost TCP. No change needed for v1. |
| **Emergency stop doesn't affect new ovens** | Low | `derive_oven_status` in FixedUpdate already checks `EmergencyStopActive` resource — applies to ALL entities. No change needed. |
| **PC read model doesn't update** | Low | `apply_oven_detected` already handles `OvenDetected` events — it spawns/updates entities. New ovens from protocol will flow through the existing path. |
| **Protocol version bump needed** | Low | Adding a variant to a tagged serde enum is backward-compatible — old pc-app will silently ignore the unknown variant via `_ => {}`. No version bump needed. |

## 7. Recommended Change Name and SDD Scope

**Change name**: `2026-06-04-dynamic-simulated-ovens`

**New capability**: `dynamic-simulated-ovens` — Runtime creation of simulated ovens via protocol command, controlled from PC UI.

**Modified capabilities**: None. Existing specs (`pc-command-authoring`, `pc-oven-read-model`, `pc-protocol-ingestion`, `protocol-message-serialization`, `protocol-versioning`) are extended but not fundamentally changed. The existing `Message` enum extension pattern is already established.

**Scope**:
- `protocol`: Add `CreateSimulatedOvensPayload`, `Message::CreateSimulatedOvens`, optional `FaultCode::MaxOvensReached`
- `rpi-controller`: Add `MaxSimulatedOvens` + `NextOvenIndex` resources, extract `spawn_single_oven`, add command handler in `route_command`
- `pc-app`: Add UI control (count input + button in header), authoring function, intent field, dispatch, log capture

## 8. First-Slice Scope

**Phase 1: Protocol + rpi-controller (no UI)**

1. Add `CreateSimulatedOvensPayload` to `protocol/src/payloads.rs`
2. Add `Message::CreateSimulatedOvens` variant to `protocol/src/message.rs`
3. Re-export in `protocol/src/lib.rs`
4. Add `MaxSimulatedOvens` and `NextOvenIndex` resources to `rpi-controller/src/resources.rs`
5. Register resources in `rpi-controller/src/plugins/oven_controller.rs`
6. Set `NextOvenIndex` in `rpi-controller/src/bevy_app.rs`
7. Extract `spawn_single_oven` helper in `rpi-controller/src/systems/startup.rs`
8. Add command handler in `rpi-controller/src/systems/update.rs`
9. Update `command_type_name` helper
10. Add protocol round-trip tests + rpi-controller integration tests
11. Test via CLI: push command directly to `InboundProtocolQueue`

This first slice is testable end-to-end without any UI changes. The command can be tested by manually pushing an envelope to the inbound queue or by writing a simple test harness.

**Phase 2: pc-app UI**

1. Add `CreateSimulatedOvensPayload` import + authoring function to `pc-app/src/systems/commands.rs`
2. Add `create_simulated_ovens` field to `UiIntent` in `pc-app/src/resources.rs`
3. Add UI control to header panel in `pc-app/src/ui/panels.rs`
4. Add dispatch handling in `pc-app/src/systems/ui/dispatch.rs`
5. Add log capture case in `pc-app/src/systems/ui/log_capture.rs`
6. Integration test

**Phase 3: PC read model verification**

1. Verify `apply_oven_detected` correctly handles `OvenDetected` events for runtime-created ovens
2. Verify status updates flow correctly for runtime-created ovens
3. Integration test with full PC ↔ RPi transport
