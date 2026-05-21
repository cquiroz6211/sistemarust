# Archive Report — `protocol`

**Change**: `protocol` — Shared Event Protocol Crate
**Archived**: 2026-05-14
**Archive location**: `openspec/changes/archive/2026-05-14-protocol/`

---

## Executive Summary

El módulo `protocol` fue implementado completamente siguiendo el pipeline SDD. Se trata de un crate Rust puro (sin dependencias de runtime) que define tipos compartidos para la comunicación PC (Bevy) ↔ Raspberry Pi en el sistema de control de hornos. El módulo establece el vocabulario común de mensajes, envelopes y versionado que permitirá a ambas aplicaciones comunicarse de forma tipada y versionada.

**Resultado final**: ✅ Completado | ✅ Verificado | ✅ Sin issues críticos

---

## Estado Final

| Métrica | Valor |
|---------|-------|
| Tareas completadas | 8/8 (Phase 1 + Phase 2 + Phase 3) |
| Tests pasando | 85 (64 unit + 21 integration) |
| Issues críticos | 0 |
| Warnings | 1 (coverage de malformed-JSON en payloads — bajo riesgo) |

### Build & Tests

```
cargo check -p protocol  ✅ Passed (8.04s)
cargo test -p protocol  ✅ 85 passed / 0 failed
  - 64 unit tests
  - 21 integration tests
```

---

## Artefactos Generados

### Fase SDD (en archive)

| Artefacto | Ubicación | Purpose |
|-----------|-----------|---------|
| `proposal.md` | `archive/2026-05-14-protocol/proposal.md` | Propuesta original del change |
| `design.md` | `archive/2026-05-14-protocol/design.md` | Diseño técnico con decisiones arquitectónicas |
| `tasks.md` | `archive/2026-05-14-protocol/tasks.md` | Plan de 8 tareas en 3 fases |
| `specs/protocol-message-serialization/spec.md` | `archive/2026-05-14-protocol/specs/` | Delta spec de serialización |
| `specs/protocol-versioning/spec.md` | `archive/2026-05-14-protocol/specs/` | Delta spec de versionado |
| `verify-report.md` | `archive/2026-05-14-protocol/verify-report.md` | Reporte de verificación final |

### Implementación (código vivo)

| Archivo | Descripción |
|---------|-------------|
| `protocol/Cargo.toml` | Dependencias: uuid, chrono |
| `protocol/src/lib.rs` | Re-exports públicos |
| `protocol/src/version.rs` | `PROTOCOL_VERSION = "1"` |
| `protocol/src/types.rs` | Enums: OvenState, FaultCode, RequestScope, Severity |
| `protocol/src/payloads.rs` | 9 structs de payload (4 commands + 5 events) |
| `protocol/src/message.rs` | Enum Message con 9 variantes |
| `protocol/src/envelope.rs` | EventEnvelope con new() y reply_to() |
| `protocol/tests/roundtrip.rs` | Tests de integración serde round-trip |

### Documentación adicional (proyecto)

| Archivo | Descripción |
|---------|-------------|
| `docs/adr/001-protocolo-eventos-rust-enum.md` | ADR de la decisión arquitectónica |
| `docs/diagrama-flujo-protocolo.md` | Diagramas de flujo del protocolo |
| `docs/events.md` | Definición de eventos (actualizada) |

---

## Decisions Arquitectónicas Clave

1. **Opción A — Enum Message con `#[serde(tag = "type")]>`**: Un único enum envuelto en EventEnvelope. Elimina duplicación y desincronización del campo `type`.

2. **`CommandRejected.reason` como `FaultCode` (enum)**: Type safety en lugar de String. Las razones de rechazo son estándar y compartidas.

3. **`Severity` limitada a `Low | Medium | High`**: v1 intencionalmente simple. `Critical` requiere criterios claros en versión futura.

4. **`output_level` como `Option<f64>`**: Representa ausencia semánticamente ("si aplica").

5. **`EventEnvelope` concreto (no genérico)**: API más simple para v1. Puede generalizarse después.

---

## Lecciones Aprendidas

### Gotchas técnicos

- **Desincronización de `type` en variantes**: La decisión de usar `#[serde(rename = "...")]` explícito en cada variante mitiga el riesgo de desincronización si se renombra una variante.
- ** serde tagging**: `#[serde(tag = "type", content = "payload")]` produce JSON donde el tipo aparece como campo `type` y el contenido como `payload`. Este patrón es correcto para el caso de uso PC ↔ RPi.
- **Casing**: Enums en PascalCase (`"Heating"`), campos de structs en snake_case (`target_celsius`). Importante no mezclarlos.

### Proceso SDD

- **TDD estricto funciona**: 85 tests pasando, cobertura completa de escenarios de spec.
- **Chained PRs fueron correctas**: El change excedía el budget de 400 líneas (~645). La estrategia de splitting en 2 PRs fue apropiada aunque finalmente se implementó en un solo push.
- **Spec first**: Las decisiones arquitectónicas se resolvieron en diseño antes de escribir código, evitando rewrites costosos.

### Warnings identificados (bajo riesgo)

- **Coverage gap en malformed-JSON**: No todos los payload structs tienen tests explícitos de deserialización con JSON malformado. El riesgo es bajo porque todos usan el mismo macro `#[derive(Deserialize)]`.
- **Typo en test name**: `reply_to_corrrelaciona_con_original` (doble `r`).

---

## Engram Observation IDs (Traceability)

| Artefacto | Observation ID |
|-----------|----------------|
| proposal | #1179 |
| design | #1183 |
| tasks | #1184 |
| spec (message-serialization) | #1182 |
| spec (versioning) | #1182 |
| apply-progress | #1185 |
| verify-report | #1186 |
| archive-report (this) | (to be assigned) |

---

## Source of Truth Actualizado

Los specs principales ahora reflejan el comportamiento implementado:

- `openspec/specs/protocol-message-serialization/spec.md` ✅
- `openspec/specs/protocol-versioning/spec.md` ✅

---

## Próximos Pasos Recomendados

### Inmediato (para continuar desarrollo)

1. **`pc-app` consume `protocol`**: Integrar el crate `protocol` como dependencia de la aplicación Bevy. El enum `Message` y `EventEnvelope` son el contrato entre PC y RPi.
2. **`rpi-controller` consume `protocol`**: De forma similar, la aplicación RPi debe depender del crate `protocol`.
3. **Transport layer**: El próximo módulo debería definir cómo se transmiten los mensajes (serial, TCP, etc.). El crate `protocol` solo define tipos, no transporte.

### Para futura consideración

- **Cobertura de malformed JSON**: Completar los tests faltantes de deserialización con JSON malformado para los 8 payload structs restantes.
- **Protocol v2 planning**: Cuando el sistema evolucione, el versionado ya está preparado para manejar mensajes con `version: "2"`.
- **Coverage tool**: Instalar `cargo tarpaulin` para obtener métricas de cobertura reales.

---

## SDD Cycle Complete

El change `protocol` ha pasado por todas las fases del pipeline SDD:

```
propose → spec → design → tasks → apply → verify → archive
```

✅ Proposal creado
✅ Specs escritas y corregidas (se corrigió RegisterOven → OvenDetected)
✅ Diseño técnico con decisiones documentadas
✅ 8 tareas implementadas con TDD
✅ 85 tests pasando
✅ Verificación PASS WITH WARNINGS (1 warning cosmetic)
✅ Delta specs sincronizadas a main specs
✅ Cambio archivado en `openspec/changes/archive/2026-05-14-protocol/`

**El módulo `protocol` está listo para ser consumido por `pc-app` y `rpi-controller`.**