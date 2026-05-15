# ADR 001 — Uso de Enum de Rust como Protocolo de Eventos

## Estado

Aceptado

## Contexto

El sistema `sistemarust` necesita un protocolo de comunicación entre la aplicación de PC (Bevy) y la Raspberry Pi (controlador físico). El protocolo debe:

- Ser tipado y seguro en tiempo de compilación.
- Permitir serialización/deserialización automática vía serde.
- Evitar que estados ilegales o mensajes mal formados existan en el tipo.
- Ser fácil de extender sin romper compatibilidad.

Se evaluaron tres aproximaciones para modelar los mensajes del protocolo:

1. **Structs separados + campo `type` como string** (`Stringly typed`).
2. **Traits con `Box<dyn Trait>`** (polimorfismo dinámico).
3. **Enum de Rust con payloads por variante** (Algebraic Data Type).

## Decisión

Se adopta la **Opción 3: un único enum `Message` de Rust con payloads strongly-typed por variante**, envuelto en un `EventEnvelope`.

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    // Comandos (PC → Raspberry)
    SetTargetTemperature(SetTargetTemperaturePayload),
    SetOvenEnabled(SetOvenEnabledPayload),
    RequestStatus(RequestStatusPayload),
    EmergencyStop(EmergencyStopPayload),

    // Eventos (Raspberry → PC)
    OvenDetected(OvenDetectedPayload),
    CommandAccepted(CommandAcceptedPayload),
    CommandRejected(CommandRejectedPayload),
    OvenStatusUpdated(OvenStatusUpdatedPayload),
    FaultRaised(FaultRaisedPayload),
}
```

El envelope contiene metadatos comunes:

```rust
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub source: String,
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub version: String,
    pub payload: Message,
}
```

## Consecuencias

### Positivas

- **Type safety exhaustiva**: el compilador obliga a manejar todas las variantes en un `match`. No hay mensajes "sorpresa" en runtime.
- **Payloads inconfundibles**: cada variante lleva su propio struct. Es imposible confundir un `RegisterOvenPayload` con un `SetTargetTemperaturePayload`.
- **Serde integrado**: `#[serde(tag = "type", content = "payload")]` sincroniza automáticamente el discriminante JSON con la variante del enum. No puede haber desincronización entre el campo `type` y los datos.
- **Estados ilegales inrepresentables**: un JSON como `{"type": "RegisterOven", "fault_code": "..."}` simplemente no se puede deserializar al tipo correcto.
- **DRY**: un solo enum para todo el protocolo. No se duplican structs de envelope ni lógica de serialización.
- **Ergonomía de TDD**: tipos puros, sin hardware, tests de serde roundtrip triviales.

### Negativas

- **El enum crece con el protocolo**: a medida que se agregan mensajes, el enum se vuelve más grande. Esto es manejable con buena organización y el protocolo se mantiene intencionalmente mínimo.
- **Acoplamiento léxico**: todos los mensivos viven en el mismo enum. Si se quiere separar estrictamente comandos de eventos en tipos distintos, esta aproximación no lo hace nativamente (aunque se puede mitigar con módulos y convenciones de nombres).
- **Versión única**: el enum `Message` tiene una única versión. Si se necesita soporte de múltiples versiones del protocolo simultáneamente, se requiere una capa adicional de traducción.

## Alternativas consideradas

### Opción A: Structs separados + campo `type` como `String`

```rust
struct Message {
    type: String,
    payload: serde_json::Value,
}
```

**Rechazada porque**: pierde todo el type checking del compilador. El campo `type` puede tener typos, valores inválidos, o payloads mal formados. Los errores se detectan en runtime, no en compilación.

### Opción B: Traits con `Box<dyn Message>`

```rust
trait Message { fn serialize(&self) -> String; }
```

**Rechazada porque**: complica enormemente la serialización con serde (requiere tagging manual o enums de todos modos). Además, no permite `match` exhaustivo en tiempo de compilación, y introduce dynamic dispatch innecesario en un sistema de control industrial donde se prefiere estática predictibilidad.

### Opción C: Dos enums separados (`Command` y `Event`)

```rust
enum Command { ... }
enum Event { ... }
```

**Rechazada porque**: duplica la estructura del envelope (`CommandEnvelope` y `EventEnvelope`) sin agregar valor real. La dirección del mensaje es una propiedad del canal de comunicación, no del tipo en sí. Se prefiere un solo tipo con organización modular.

## Notas

- La separación entre comandos (PC → Raspberry) y eventos (Raspberry → PC) se mantiene **conceptual y documental**, no a nivel de tipo.
- El versionado del protocolo se maneja con el campo `version` del envelope y, en el futuro, posiblemente con `#[serde(rename)]` para evolucionar nombres de variantes.
- Se recomienda usar `#[serde(tag = "type", content = "payload")]` para mantener el JSON limpio y evitar que los campos del payload colisionen con los metadatos del envelope.

## Referencias

- `docs/prd-sistema-control-hornos.md`
- `docs/events.md`
- `docs/panorama-app-ecs.md`
- Exploración previa guardada en engram: `sdd/protocol-shared/explore`
- Propuesta guardada en engram: `sdd/protocol/proposal`
