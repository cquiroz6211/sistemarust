# Estado Actual del Sistema — Trazabilidad Consolidada

> **Actualizado**: 2026-05-28
> Proyecto: Sistema de Control de Hornos (Rust + Bevy ECS)
> Crates: `protocol`, `transport`, `rpi-controller`, `pc-app`

## Resumen ejecutivo

El sistema ya tiene un flujo distribuido verificable en local: `rpi-controller` y `pc-app` se conectan por TCP localhost usando el crate `transport`. Bevy ECS sigue siendo el dominio; Tokio queda limitado al adaptador de transporte.

## ¿Qué existe hoy?

| Crate | Rol | Estado | Tests | Change SDD |
|---|---|---|---|---|
| `protocol` | Mensajes compartidos (enum + serde) | ✅ Verificado | 93 tests | `protocol` |
| `transport` | TCP localhost, framing JSON lines, bridge Bevy ↔ Tokio | ✅ Verificado | 14 tests | `pc-rpi-transport-link` |
| `rpi-controller` | Bevy headless: comandos, simulación, histéresis | ✅ Verificado | 38 tests | `rpi-controller-bevy-headless` |
| `pc-app` | Bevy headless: read model, authoring de comandos | ✅ Verificado | 23 tests | `pc-app` |
| UI | Interfaz visual en PC | ❌ No existe | — | `pc-app-ui` (futuro) |
| GPIO | Control físico real en Raspberry | ❌ No existe | — | Futuro |

## Arquitectura actual

### Comunicación

Cada proceso mantiene sus colas ECS internas. El crate `transport` las conecta con tareas Tokio por TCP localhost:

```
┌─────────────────────────────────────────────────────────┐
│                        PC (pc-app)                       │
│                                                          │
│  InboundProtocolQueue  ←  transport::InboundReceiver      │
│  OutboundProtocolQueue →  transport::OutboundSender       │
│                                                          │
│  Systems:                                                 │
│    Update: ingest_inbound_protocol                        │
│    FixedUpdate: apply_oven_detected                       │
│                 apply_oven_status_updated                  │
│                 apply_fault_raised                         │
│                 record_command_result                       │
│                                                          │
│  Functions:                                               │
│    author_set_target_temperature_command                   │
│    author_set_oven_enabled_command                         │
│    author_request_status_command                           │
│    author_emergency_stop_command                           │
└─────────────────────────────────────────────────────────┘
                              ┃
                              ┃  ✅ TCP localhost verificado
                              ┃  JSON line framing + logs [TX]/[RX]
                              ┃
┌─────────────────────────────────────────────────────────┐
│                   Raspberry Pi (rpi-controller)           │
│                                                          │
│  InboundProtocolQueue  ←  transport::InboundReceiver      │
│  OutboundProtocolQueue →  transport::OutboundSender       │
│                                                          │
│  Systems:                                                 │
│    Startup: spawn_simulated_ovens                          │
│    Update: ingest_commands                                 │
│            emit_protocol_responses                         │
│    FixedUpdate: simulate_thermal_drift                     │
│                 check_hysteresis                            │
│                 check_faults                                │
│                 publish_status                              │
└─────────────────────────────────────────────────────────┘
```

### Modelo ECS compartido

| Concepto | `rpi-controller` | `pc-app` |
|---|---|---|
| Runtime | `MinimalPlugins` + `ScheduleRunnerPlugin` | Mismo runtime |
| Schedules | `Startup`, `Update`, `FixedUpdate` (50ms) | `Update`, `FixedUpdate` (50ms) |
| Componentes por horno | 9 (OvenId, SensorRef, OutputRef, CurrentTemp, TargetTemp, MaxTemp, Enabled, Heating, OvenStatus) | 11 (mismos + FaultState + LastCommandResult) |
| Colas | `InboundProtocolQueue`, `OutboundProtocolQueue` | Mismas colas |
| Índice | `OvenIndex(HashMap<String, Entity>)` | Mismo patrón |

## Decisiones arquitectónicas clave

| Decisión | Documento | Resumen |
|---|---|---|
| Protocolo como enum de Rust con serde | `docs/adr/001-protocolo-eventos-rust-enum.md` | Una variante por mensaje, payload tipado, discriminante JSON automático |
| Bevy headless para Raspberry | `docs/prd-rpi-controller-bevy-headless.md` | `MinimalPlugins` + `ScheduleRunnerPlugin`, sin renderer, sin Tokio |
| Histéresis de 5°C | `docs/adr/002-hysteresis-control-termico.md` | Banda muerta para proteger relay, PID diferido |
| PC como read model | `openspec/changes/pc-app/proposal.md` | PC no crea hornos ni decide; recibe y muestra |
| Tokio como adaptador de transporte | `docs/adr/003-tokio-como-adaptador-transporte-bevy.md` | Bevy mantiene el dominio; Tokio mueve TCP sin bloquear schedules |
| Transporte TCP localhost | `openspec/changes/pc-rpi-transport-link/verify-report.md` | Dos procesos locales conectados, canales acotados de 64, framing JSON line, logs `[TX]`/`[RX]` |

## Documentación existente

| Archivo | Contenido | Vigente |
|---|---|---|
| `docs/prd-sistema-control-hornos.md` | PRD original del sistema | ✅ Sí |
| `docs/prd-rpi-controller-bevy-headless.md` | PRD del controller Bevy headless | ✅ Sí |
| `docs/panorama-app-ecs.md` | Visión general ECS + estado real | ✅ Actualizado |
| `docs/events.md` | Contrato completo de eventos | ✅ Sí |
| `docs/diagrama-flujo-protocolo.md` | Flujo de mensajes PC ↔ RPi | ✅ Actualizado |
| `docs/diagrama-clases.md` | Diagrama de componentes reales | ✅ Actualizado |
| `docs/flujo-datos-transporte.md` | Flujo completo de datos del transporte TCP | ✅ Creado |
| `docs/arquitectura-explicada.md` | Explicación docente de arquitectura PC/RPi/Bevy/Tokio | ✅ Creado |
| `docs/bevy-en-pc-y-raspberry.md` | Explicación específica de cómo se aplica Bevy ECS en ambos lados | ✅ Creado |
| `docs/adr/001-protocolo-eventos-rust-enum.md` | ADR: enum + serde | ✅ Sí |
| `docs/adr/002-hysteresis-control-termico.md` | ADR: histéresis 5°C | ✅ Sí |
| `docs/adr/003-tokio-como-adaptador-transporte-bevy.md` | ADR: Tokio como adaptador TCP de Bevy | ✅ Sí |
| `docs/estado-actual.md` | Este documento — trazabilidad | ✅ Creado |

## Cómo probar el flujo completo

```bash
# Terminal 1: Raspberry simulada como servidor TCP
cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000

# Terminal 2: PC como cliente TCP con demo automática
cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo
```

Qué observar:

- El servidor RPi arranca escuchando en `127.0.0.1:7000`.
- El cliente PC se conecta y ejecuta la demo.
- Los mensajes salen en consola con prefijos `[TX]` y `[RX]`.
- El flujo esperado incluye `OvenDetected`, `SetTargetTemperature`, `CommandAccepted`, `RequestStatus` y `OvenStatusUpdated`.

## Estado de tests verificado

```bash
# Reportado por openspec/changes/pc-rpi-transport-link/verify-report.md
cargo test --package transport        # 14 tests
cargo test --package protocol         # 93 tests
cargo test --package pc-app           # 23 tests
cargo test --package rpi-controller   # 38 tests
cargo test --workspace                # 168 tests total
```

## Cómo probar cada lado sin transporte

```bash
# RPi controller con simulación local
cargo run --package rpi-controller -- --simulate 3

# PC app (headless, sin output visible)
cargo run --package pc-app
```

## Qué sigue pendiente

| Aspecto | Detalle |
|---|---|
| UI visual | La PC todavía es headless; no hay interfaz gráfica final. |
| GPIO real | La Raspberry sigue usando simulación; no controla hardware físico todavía. |
| Archive SDD | `pc-app` y `pc-rpi-transport-link` ya están archivados. |
| Robustez futura | No incluye autenticación, multi-cliente ni reconexión robusta. |

## Historial de cambios SDD

| Change | Estado | Artefacto de archive |
|---|---|---|
| `protocol` | ✅ Archivado | `openspec/changes/archive/2026-05-14-protocol/` |
| `rpi-controller-bevy-headless` | ✅ Archivado | `openspec/changes/archive/2026-05-21-rpi-controller-bevy-headless/` |
| `pc-app` | ✅ Archivado | `openspec/changes/archive/2026-05-28-pc-app/` |
| `pc-rpi-transport-link` | ✅ Archivado | `openspec/changes/archive/2026-05-28-pc-rpi-transport-link/` |
