# Archive Report — `rpi-controller-bevy-headless`

**Change**: `rpi-controller-bevy-headless`
**Archived**: 2026-05-21
**Status**: PASS ✅
**Artifact store**: `hybrid` (openspec + Engram)

---

## Executive Summary

El change `rpi-controller-bevy-headless` reemplaza el enfoque Tokio-first del dominio `rpi-controller` por una aplicación Bevy ECS headless. El dominio de hornos se modela ahora como entidades ECS con componentes por horno (`OvenId`, `CurrentTemperature`, `TargetTemperature`, etc.), recursos globales (`EmergencyStopActive`, colas de protocolo), y sistemas en schedules explícitos (`Startup`, `Update`, `FixedUpdate`). La verificación pasó con 38 tests (10 unit + 28 integración) en `cargo test -p rpi-controller` y `cargo test --workspace`.

---

## What Was Achieved

### Scope delivered

| Requirement Group | Result |
|---|---|
| Headless runtime (`MinimalPlugins` + `ScheduleRunnerPlugin`) | ✅ |
| Oven entity model (9 ECS components) | ✅ |
| Simulated oven detection (`Startup` spawning) | ✅ |
| Protocol command processing (4 commands) | ✅ |
| Thermal control + 5°C hysteresis (`FixedUpdate` 50 ms) | ✅ |
| Fault and safety (`FaultRaised`, faulted oven behavior) | ✅ |
| Status publication (on-change + periodic) | ✅ |
| Testability (deterministic `App::update()` tests) | ✅ |

### Tests

- `cargo test -p rpi-controller` ✅
- `cargo test --workspace` ✅
- **38 tests total**: 10 unit + 28 integration

---

## Key Architectural Decisions

### 1. ECS as the domain model

El problema es inherentemente ECS: múltiples hornos independientes, estado por entidad, procesamiento periódico, transiciones de estado. ECS reemplaza el `HashMap<String, OvenState>` + `RwLock` + `select!` del enfoque Tokio-first con iteración zero-cost y schedules explícitos.

### 2. Headless Bevy runtime

`MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(FixedInterval(50ms))` elimina renderer, UI, y ventana. El dominio corre en un proceso puro sin hardware. I/O real puede agregarse como future adapter que popula `InboundProtocolQueue` / drena `OutboundProtocolQueue` sin tocar lógica de dominio.

### 3. Hysteresis de 5°C (ADR 002)

Banda muerta: `current < target − 5 → heating = true`; `current ≥ target → heating = false`; entre ambos valores se mantiene estado anterior. Protege el relay de ciclos rápidos de encendido/apagado.

### 4. Protocol bridge via queues

`InboundProtocolQueue` (Vec<EventEnvelope>) y `OutboundProtocolQueue` son Resources. El protocolo consume y produce envelopes; ECS procesa comandos y emite eventos. `EventEnvelope::reply_to()` preserva `correlation_id` y invierte `source`/`target` automáticamente.

### 5. Determinism via TestRng

`TestRng(Option<StdRng>)` Resource permite tests deterministas con semilla fija. En runtime normal `None` usa `rand::thread_rng()` para simulación con ruido.

### 6. Internal event bus

`add_event::<T>()` para eventos internos ECS (`CommandReceivedEvent`, `OvenDetectedEvent`, `FaultDetectedEvent`, `StatusPublishEvent`). Desacopla ingestión, procesamiento, y emisión.

---

## Files Changed

### Created (9 files)

| File | Purpose |
|---|---|
| `rpi-controller/src/components.rs` | 9 Bevy components + `OvenState` enum |
| `rpi-controller/src/resources.rs` | 8 resources (`EmergencyStopActive`, queues, config, index, rng) |
| `rpi-controller/src/events.rs` | 4 internal ECS event types |
| `rpi-controller/src/systems/startup.rs` | Oven spawning + `OvenDetected` emission |
| `rpi-controller/src/systems/update.rs` | Command ingestion, routing, response emission |
| `rpi-controller/src/systems/fixed_update.rs` | Thermal drift, hysteresis, status, fault detection |
| `rpi-controller/src/plugins/oven_controller.rs` | Plugin bundling all systems |
| `rpi-controller/src/bevy_app.rs` | `build_app()` helper |
| `rpi-controller/tests/integration_tests.rs` | 28 integration tests |

### Deleted (7 files)

`app.rs`, `handlers.rs`, `state.rs`, `control.rs`, `simulation.rs`, `validation.rs`, `mock_transport.rs`

### Modified (2 files)

`rpi-controller/src/lib.rs` (new module tree), `rpi-controller/src/main.rs` (Bevy bootstrap, drop `#[tokio::main]`)

---

## Lineage / Traceability

| Artifact | Engram ID | Saved to openspec |
|---|---|---|
| Proposal | #1502 | `openspec/changes/rpi-controller-bevy-headless/proposal.md` |
| Spec | #1503 | `openspec/changes/rpi-controller-bevy-headless/spec.md` |
| Design | #1505/#1506 | `openspec/changes/rpi-controller-bevy-headless/design.md` |
| Tasks | #1508 | `openspec/changes/rpi-controller-bevy-headless/tasks.md` |
| Apply progress | #1511 | — (in-memory progress) |
| Verify report | #1516 | `openspec/changes/rpi-controller-bevy-headless/verify-report.md` |

---

## Test Status

| Command | Result |
|---|---|
| `cargo test -p rpi-controller` | ✅ Pass |
| `cargo test --workspace` | ✅ Pass |

### Compliance per requirement group

| Group | Status |
|---|---|
| `headless-runtime` | ✅ Compliant |
| `oven-entity-model` | ✅ Compliant |
| `simulated-oven-detection` | ✅ Compliant |
| `protocol-command-processing` | ✅ Compliant |
| `thermal-control-fixed-update` | ✅ Compliant |
| `fault-and-safety` | ✅ Compliant |
| `status-publication` | ✅ Compliant |
| `testability` | ✅ Compliant |

---

## Next Recommended Steps

1. **I/O adapter (future)**: cuando se necesite `serialport` real o GPIO real, agregar un adaptador que popule `InboundProtocolQueue` y drene `OutboundProtocolQueue` sin modificar Bevy App ni sistemas ECS.

2. **PID como etapa posterior**: ADR 002 permite mantener hysteresis como output stage sobre PID. Cuando se implemente PID, la decisión de potencia continua se traduce a on/off via la misma capa de histéresis.

3. **Multi-client support (future)**: actualmente un solo cliente PC. Si se necesita múltiples, se requiere resource de sesión o actor por conexión en el mismo Bevy App.

4. **Persist the state (future)**: el estado del dominio vive en ECS. Si se requiere persistencia entre arranques, un sistema puede serializar el estado a archivo en `Update` schedule periódicamente.

---

## Archive Contents

```
openspec/changes/archive/2026-05-21-rpi-controller-bevy-headless/
├── proposal.md
├── spec.md
├── design.md
├── tasks.md
└── verify-report.md
```

### Source of truth updated

`openspec/specs/rpi-controller/spec.md` — created from delta spec (no previous main spec existed for this domain).

---

## SDD Cycle Complete

El change ha sido planificado (proposal), especificado (delta spec), diseñado (design), descompuesto en tareas (tasks), implementado (apply), verificado (verify-report → PASS), y archivado. Listo para el próximo change.