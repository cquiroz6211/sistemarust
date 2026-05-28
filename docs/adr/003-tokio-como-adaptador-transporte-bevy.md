# ADR 003 — Tokio como adaptador de transporte para Bevy ECS

## Estado

Aceptado

## Decisión

El sistema usará **Bevy ECS para el dominio** y **Tokio solo como adaptador de transporte TCP async**.

Tokio no reemplaza a Bevy. Tokio queda en el borde del sistema para leer/escribir sockets sin bloquear. Bevy sigue siendo el modelo principal para hornos, estado, comandos, eventos internos y schedules.

```text
Bevy ECS domain
    ↓ systems no bloqueantes
Bounded channels
    ↓
Tokio TCP transport task
    ↓
TCP localhost
```

## Contexto

El próximo cambio (`pc-rpi-transport-link`) necesita conectar dos procesos locales:

```bash
cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000
cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo
```

Ambos crates ya usan Bevy ECS headless y colas internas:

- `InboundProtocolQueue`
- `OutboundProtocolQueue`

La pregunta arquitectónica es cómo conectar esas colas por TCP sin sacrificar el rendimiento ni la predictibilidad del scheduler de Bevy.

## Problema

Un system de Bevy debe ser rápido y no bloqueante. Si un system hace I/O bloqueante:

```rust
stream.read(...)
```

y no hay datos disponibles, el tick de Bevy puede quedar congelado. Eso rompe el modelo ECS: los schedules dejan de avanzar, la simulación se detiene y los systems dejan de ser predecibles.

## Alternativas consideradas

| Opción | Resultado | Motivo |
|---|---|---|
| TCP bloqueante dentro de systems Bevy | Rechazada | Puede congelar `Update` / `FixedUpdate`. |
| `std::net` non-blocking manual dentro de systems | Rechazada por ahora | Menos dependencias, pero más código manual y más riesgo de errores sutiles. |
| `IoTaskPool` de Bevy | Considerada | Más Bevy-native, útil para tareas I/O breves; menos ergonómica para TCP bidireccional persistente. |
| Tokio como task async + canales acotados | Aceptada | Resuelve TCP persistente sin bloquear Bevy y mantiene el dominio limpio. |

## Diseño aceptado

El transporte se integrará como un plugin/adaptador:

```text
PcAppPlugin / OvenControllerPlugin
        +
TransportPlugin
```

El plugin registrará systems Bevy livianos:

- `flush_outbound`: mueve mensajes de `OutboundProtocolQueue` hacia un canal async.
- `flush_inbound`: mueve mensajes recibidos desde un canal async hacia `InboundProtocolQueue`.

El task Tokio hará el trabajo de red:

- aceptar/conectar TCP
- serializar `EventEnvelope` como JSON line
- escribir al socket
- leer JSON line del socket
- deserializar `EventEnvelope`
- publicar hacia el canal inbound

Regla importante:

> El task async **no muta el `World` de Bevy**. Solo se comunica por canales.

## Consecuencias positivas

- Bevy mantiene systems rápidos y no bloqueantes.
- El dominio no conoce TCP, sockets ni Tokio.
- El transporte se puede testear como adaptador separado.
- TCP localhost permite demo real en dos terminales sin hardware.
- JSON line permite observar mensajes con logs o herramientas simples.

## Consecuencias negativas

- Agrega Tokio como dependencia de transporte.
- Hay que manejar runtime async en los binarios.
- Aparece una frontera adicional: Bevy queues ↔ channels ↔ Tokio task.
- Hay que definir comportamiento ante canales llenos o desconexión.

## Reglas para implementación

- Los systems de Bevy **MUST NOT** bloquear esperando red.
- El transporte **MUST NOT** mutar el `World` desde tareas async.
- La comunicación Bevy ↔ Tokio **MUST** pasar por canales acotados.
- La v1 **MUST** soportar una conexión local simple PC ↔ Raspberry.
- La v1 **MAY** cerrar o reportar desconexión sin reconexión robusta.
- La v1 **MUST** mostrar logs `[TX]` / `[RX]` para explicar el flujo a estudiantes.

## Explicación docente

Bevy es el modelo del sistema: organiza estado y comportamiento.

Tokio es el adaptador de red: mueve bytes por TCP sin frenar el modelo.

La idea no es mezclar dos arquitecturas, sino separar responsabilidades:

| Responsabilidad | Herramienta |
|---|---|
| Hornos, componentes, estado, comandos y reglas | Bevy ECS |
| Sockets TCP, lectura/escritura async, conexión local | Tokio |
| Contrato de mensajes | `protocol` |

## Referencias

- `docs/estado-actual.md`
- `docs/diagrama-flujo-protocolo.md`
- `openspec/changes/pc-rpi-transport-link/exploration.md`
- `openspec/changes/pc-rpi-transport-link/proposal.md`
- `openspec/changes/pc-rpi-transport-link/design.md`
