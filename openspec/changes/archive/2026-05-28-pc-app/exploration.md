## Exploration: pc-app

### Current State

`protocol` is complete (85 tests, archived 2026-05-14) and exposes the shared `EventEnvelope`, `Message` enum (9 variants: 4 commands + 5 events), all payload structs, and domain enums (`OvenState`, `FaultCode`, `RequestScope`, `Severity`). The envelope supports `new()` and `reply_to()` for correlation_id inversion. JSON serialization uses `#[serde(tag = "type", content = "payload")]` with PascalCase variant names and snake_case field names.

`rpi-controller-bevy-headless` is complete (38 tests, archived 2026-05-21). It implements the proven Bevy ECS headless pattern: `MinimalPlugins` + `ScheduleRunnerPlugin` with `FixedUpdate` at 50ms. Ovens are Bevy entities with 9 components (`OvenId`, `SensorRef`, `OutputRef`, `CurrentTemperature`, `TargetTemperature`, `MaxTemperature`, `Enabled`, `Heating`, `OvenStatus`). Protocol bridge uses `InboundProtocolQueue` / `OutboundProtocolQueue` Resources. Internal event bus uses `add_event::<T>()` for `CommandReceivedEvent`, `OvenDetectedEvent`, `FaultDetectedEvent`, `StatusPublishEvent`. Thermal control uses 5°C hysteresis. Testability via `App::update()` stepping and `TestRng`.

`pc-app` currently only has `pc-app/src/main.rs` with `println!("Hello ")`, but its `Cargo.toml` already depends on `protocol`, `bevy = "0.15"`, `serialport`, and `tokio`. The next useful v1 is not transport or UI polish; it is a PC-side ECS read model that consumes RPi events and can author protocol commands through mock queues.

### Affected Areas

- `pc-app/src/main.rs` — Replace stub entry point with Bevy app bootstrap or move bootstrap into library modules.
- `pc-app/Cargo.toml` — Already has protocol/Bevy dependencies; `serialport` and `tokio` are future adapters (transport deferred).
- `protocol/src/{envelope,message,payloads,types}.rs` — Defines all PC inbound events and outbound commands to consume.
- `rpi-controller/src/components.rs` — Source pattern for oven ECS components.
- `rpi-controller/src/resources.rs` — Source pattern for `InboundProtocolQueue`, `OutboundProtocolQueue`, `OvenIndex`.
- `rpi-controller/src/events.rs` — Source pattern for internal event bus.
- `rpi-controller/src/systems/{startup,update,fixed_update}.rs` — Source pattern for schedule separation and deterministic tests.
- `openspec/changes/pc-app/` — New SDD change folder for proposal/spec/design/tasks after exploration.

### Approaches

1. **Headless/testable read model first** — Build `pc-app` as a Bevy ECS app without visual UI in v1: resources for inbound/outbound queues, components for local oven read state, internal events for received RPi facts and operator command intents, plus tests that push protocol envelopes and assert ECS state/commands.
   - Pros: mirrors the proven `rpi-controller` architecture; fast deterministic tests; validates protocol consumption before UI complexity; keeps PC as read model + command author, not physical authority.
   - Cons: no visual oven representation yet; operator-facing value is delayed; later UI systems must be added on top of the read model.
   - Effort: Low/Medium

2. **Minimal Bevy UI immediately** — Build the ECS read model and a simple Bevy UI in the same v1: show one card/list row per oven, current/target temperature, enabled/heating/status/fault indicators, and maybe buttons or keybindings that enqueue commands.
   - Pros: demonstrates the product loop visually; proves `OvenDetected` → screen and status updates early; aligns with PC's role as visual center.
   - Cons: UI systems run per `Update` frame and add more moving parts; tests become harder unless UI is thin over a separately tested read model; visual polish can distract from protocol correctness.
   - Effort: Medium

3. **Transport-first PC app** — Implement serial/TCP adapter first and wire it directly to `protocol` queues, with only logs or a tiny state dump.
   - Pros: moves toward real PC-RPi integration; validates serialization boundaries beyond in-memory queues.
   - Cons: violates the current project sequence; transport is explicitly future in both protocol and rpi archive reports; harder to test without a stable PC domain model; risks coupling IO to UI/domain too early.
   - Effort: Medium/High

### Recommendation

Use approach 1 for v1, with a deliberately thin path for approach 2 next. The PC should start as a testable ECS read model and command-authoring app: consume `OvenDetected`, `OvenStatusUpdated`, `CommandAccepted`, `CommandRejected`, and `FaultRaised` from `InboundProtocolQueue`; maintain oven entities keyed by `OvenId`; and enqueue `SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, and `EmergencyStop` into `OutboundProtocolQueue`. This matches the already-verified RPi pattern and keeps the real invariant intact: the PC displays and requests, the Raspberry validates and decides. Bevy UI should be deferred to v1.1 or kept as a minimal optional layer only after the read model has tests.

For oven modeling, use one Bevy entity per detected oven with small components: `OvenId`, `SensorRef`, `OutputRef`, `MaxTemperature`, `CurrentTemperature`, `TargetTemperature`, `Enabled`, `Heating`, `OvenStatus`, and `FaultState`. Add `OvenIndex(HashMap<String, Entity>)` as the bridge from protocol IDs to ECS entities. `FaultState` should hold no fault or the last/active fault (`FaultCode`, `Severity`, `message`) because `FaultRaised` can arrive independently from status. Keep `OvenStatus` as the protocol enum (`OvenState`) rather than inventing a duplicate PC enum.

Protocol bridge should mirror the controller but reverse semantics: PC `InboundProtocolQueue` receives RPi events; PC `OutboundProtocolQueue` contains commands to send to RPi. Internal events should include `ProtocolEventReceived`, `OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, `CommandRejectedReceived`, and command intent events such as `SetTargetTemperatureRequested`, `SetOvenEnabledRequested`, `RequestStatusRequested`, `EmergencyStopRequested`. Systems should be: `ingest_inbound_protocol`, `apply_oven_detected`, `apply_oven_status_updated`, `apply_fault_raised`, `record_command_result`, `author_set_target_temperature_command`, `author_set_oven_enabled_command`, `author_request_status_command`, and `author_emergency_stop_command`. If UI is added later, it should only read components and emit command intent events.

Defer UI polish, real serial transport, auth, advanced history, multi-client/session semantics, persistence, charts, and transport reconnection/heartbeat. Context7 Bevy docs support this split: `Update` is appropriate for UI/input and regular app logic, `FixedUpdate` is for consistent timing, and `App::update()` can step schedules in tests; PC v1 likely does not need `FixedUpdate` except maybe a future polling/timeout concern.

### Risks

- If v1 includes UI too early, visual code can hide whether protocol ingestion and command correlation are correct.
- If `FaultRaised` and `OvenStatusUpdated` arrive out of order, `FaultState` and `OvenStatus` rules must be explicit to avoid clearing faults accidentally.
- `CommandAccepted`/`CommandRejected` correlation needs a small pending-command model if the UI must show command status; otherwise v1 should only record last results.
- `pc-app` currently depends on `serialport` and `tokio`, but transport is deferred; unused dependencies may confuse scope unless the proposal calls them future adapters.

### Ready for Proposal

Yes — the orchestrator should propose a v1 `pc-app` change focused on a headless/testable Bevy ECS read model plus mock protocol queues, with Bevy UI explicitly deferred or limited to a very thin optional display after the core ECS behavior is specified and tested.
