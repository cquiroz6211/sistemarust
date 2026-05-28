# Estado Actual del Sistema — Trazabilidad Consolidada

> **Actualizado**: 2026-05-28
> Proyecto: Sistema de Control de Hornos (Rust + Bevy ECS)
> Cratas: `protocol`, `rpi-controller`, `pc-app`

## Resumen ejecutivo

El sistema está diseñado como una aplicación distribuida con tres módulos independientes. El protocolo y ambos controladores (Raspberry y PC) existen y están verificados. **El transporte que los conecta sigue pendiente.**

## ¿Qué existe hoy?

| Crate | Rol | Estado | Tests | Change SDD |
|---|---|---|---|---|
| `protocol` | Mensajes compartidos (enum + serde) | ✅ Archivado | 85 tests | `protocol` |
| `rpi-controller` | Bevy headless: comandos, simulación, histéresis | ✅ Archivado | 59 tests | `rpi-controller-bevy-headless` |
| `pc-app` | Bevy headless: read model, authoring de comandos | ✅ Verificado | 23 tests | `pc-app` |
| **Transporte** | Conectar PC ↔ Raspberry | ❌ No existe | — | `pc-rpi-transport-link` (próximo) |
| UI | Interfaz visual en PC | ❌ No existe | — | `pc-app-ui` (futuro) |
| GPIO | Control físico real en Raspberry | ❌ No existe | — | Futuro |

## Arquitectura actual

### Comunicación

Cada crate tiene sus colas internas. No hay conexión entre procesos:

```
┌─────────────────────────────────────────────────────────┐
│                        PC (pc-app)                       │
│                                                          │
│  InboundProtocolQueue  ←  (vacío, espera transporte)     │
│  OutboundProtocolQueue →  (lleno, espera transporte)     │
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
                              ┃  ❌ TRANSPORTE FALTA
                              ┃
┌─────────────────────────────────────────────────────────┐
│                   Raspberry Pi (rpi-controller)           │
│                                                          │
│  InboundProtocolQueue  ←  (vacío, espera transporte)     │
│  OutboundProtocolQueue →  (lleno, espera transporte)     │
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
| Transporte diferido | `docs/estado-actual.md` | Queues internas listas, conexión entre procesos pendiente |

## Documentación existente

| Archivo | Contenido | Vigente |
|---|---|---|
| `docs/prd-sistema-control-hornos.md` | PRD original del sistema | ✅ Sí |
| `docs/prd-rpi-controller-bevy-headless.md` | PRD del controller Bevy headless | ✅ Sí |
| `docs/panorama-app-ecs.md` | Visión general ECS + estado real | ✅ Actualizado |
| `docs/events.md` | Contrato completo de eventos | ✅ Sí |
| `docs/diagrama-flujo-protocolo.md` | Flujo de mensajes PC ↔ RPi | ✅ Actualizado |
| `docs/diagrama-clases.md` | Diagrama de componentes reales | ✅ Actualizado |
| `docs/adr/001-protocolo-eventos-rust-enum.md` | ADR: enum + serde | ✅ Sí |
| `docs/adr/002-hysteresis-control-termico.md` | ADR: histéresis 5°C | ✅ Sí |
| `docs/adr/003-tokio-como-adaptador-transporte-bevy.md` | ADR: Tokio como adaptador TCP de Bevy | ✅ Sí |
| `docs/estado-actual.md` | Este documento — trazabilidad | ✅ Creado |

## Cómo probar cada lado

```bash
# Todo
cargo test --workspace           # 154 tests total

# RPi controller con simulación de 3 hornos
cargo run --package rpi-controller -- --simulate 3

# PC app (headless, sin output visible)
cargo run --package pc-app

# Tests individuales
cargo test -p protocol           # 85 tests
cargo test -p rpi-controller     # 59 tests
cargo test -p pc-app             # 23 tests
```

## Próximo cambio: Transporte PC ↔ Raspberry

| Aspecto | Detalle |
|---|---|
| Nombre sugerido | `pc-rpi-transport-link` |
| Objetivo | Conectar `OutboundProtocolQueue` de un lado con `InboundProtocolQueue` del otro |
| Medio candidato | TCP local (localhost), stdin/stdout, o serial loopback |
| Demo | Permitir `cargo run --package demo` que muestre el flujo completo en consola |
| No incluye | UI visual, GPIO real, autenticación, multi-cliente |

## Historial de cambios SDD

| Change | Estado | Artefacto de archive |
|---|---|---|
| `protocol` | ✅ Archivado | `openspec/changes/archive/2026-05-14-protocol/` |
| `rpi-controller-bevy-headless` | ✅ Archivado | `openspec/changes/archive/2026-05-21-rpi-controller-bevy-headless/` |
| `pc-app` | ✅ Verificado (PASS) | `openspec/changes/pc-app/` (pendiente de archive) |
