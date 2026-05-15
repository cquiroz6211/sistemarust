# Proposal: Protocolo Compartido — Crate `protocol`

## Intent

Definir el idioma común entre PC (Bevy) y Raspberry Pi para el sistema de control de hornos. Sin un protocolo tipado y versionado, ambos lados acoplan estructuras implícitas, generando errores de serialización difíciles de detectar en runtime.

## Scope

### In Scope
- Tipos Rust compartidos: `Message` enum con payloads strongly-typed por variante.
- `EventEnvelope` con metadatos obligatorios: `event_id`, `source`, `target`, `timestamp`, `version`, `correlation_id`, `payload`.
- Serialización/deserialización con `serde` + `serde_json` usando `#[serde(tag = "type")]`.
- Versionado inicial del protocolo (`v1`).
- Tests unitarios TDD: round-trip de serialización para cada variante.
- Crate `protocol` compilable como dependencia de `pc-app` y `rpi-controller`.

### Out of Scope
- UI / renderizado en Bevy.
- Lógica de control físico, GPIO o simulación de sensores.
- Transporte serial (el crate `protocol` solo define tipos, no abre puertos).
- Heartbeat, alertas de negocio o historial.
- Autenticación o cifrado.

## Capabilities

### New Capabilities
- `protocol-message-serialization`: Definición y serialización del enum `Message` y `EventEnvelope`.
- `protocol-versioning`: Estrategia de versionado para evolucionar eventos sin romper compatibilidad.

### Modified Capabilities
- None.

## Approach

Implementar **Opción A** recomendada en la exploración: un único enum `Message` con `#[serde(tag = "type")]` envuelto en `EventEnvelope`. Esto garantiza que el campo `type` se derive automáticamente del nombre de variante, eliminando duplicación y desincronización. Los tests se escriben primero (TDD): serializar cada comando y evento, deserializar, y verificar igualdad estructural.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `protocol/src/lib.rs` | New | Entry point del crate. |
| `protocol/src/message.rs` | New | Enum `Message` con variantes y payloads. |
| `protocol/src/envelope.rs` | New | `EventEnvelope<T>` con metadatos comunes. |
| `protocol/src/version.rs` | New | Constante/versionado del protocolo. |
| `protocol/tests/` | New | Tests de round-trip serde. |
| `Cargo.toml` (workspace) | Modified | Agregar dependencias `serde`, `serde_json`, `uuid`, `chrono` al crate `protocol`. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Desincronización de `type` si se renombra una variante | Low | Usar `#[serde(rename = "...")]` explícito en cada variante. |
| Cambios de schema que rompan compatibilidad | Med | Versionado explícito en `EventEnvelope`; nunca modificar payloads de v1, solo agregar variantes nuevas. |
| Dependencias pesadas en `protocol` afectan tiempos de compilación | Low | Limitar deps a `serde`, `uuid`, `chrono`; sin `tokio` ni `bevy`. |

## Rollback Plan

Revierte el commit que introduce el crate `protocol`. Como es un crate nuevo sin dependientes productivos todavía, el rollback es trivial: eliminar el directorio `protocol/` y revertir `Cargo.toml`.

## Dependencies

- `serde` y `serde_json` ya están disponibles en el workspace.
- `uuid` y `chrono` disponibles para timestamps e IDs.

## Success Criteria

- [ ] `cargo test -p protocol` pasa con tests de round-trip para todos los comandos y eventos del protocolo inicial.
- [ ] `cargo build -p protocol` compila sin warnings.
- [ ] El enum `Message` cubre: `RegisterOven`, `SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`, `CommandAccepted`, `CommandRejected`, `OvenStatusUpdated`, `FaultRaised`.
- [ ] `EventEnvelope` incluye todos los campos de metadatos definidos en `docs/events.md`.
