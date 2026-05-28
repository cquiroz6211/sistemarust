# Flujo de Protocolo — PC ↔ Raspberry Pi

Este documento muestra cómo se ve el protocolo en acción. PC y Raspberry Pi se tratan como **cajas negras**: no importa qué hay adentro, solo qué mensajes se envían y qué se espera recibir.

> **Regla base**: El PC declara intención. La Raspberry valida, ejecuta y reporta hechos.

---

## Diagrama General de Secuencia

```mermaid
sequenceDiagram
    autonumber
    participant PC as PC / Bevy
    participant RPi as Raspberry Pi

    Note over PC,RPi: 1. La Raspberry detecta un horno nuevo
    RPi-->>PC: OvenDetected { oven_id, sensor_ref, output_ref, max_celsius }
    PC->>RPi: SetTargetTemperature { oven_id, target_celsius: 150.0 }
    RPi-->>PC: CommandAccepted { accepted_type: SetTargetTemperature, oven_id, message }
    PC->>RPi: SetOvenEnabled { oven_id, enabled: true }
    RPi-->>PC: CommandAccepted { accepted_type: SetOvenEnabled, oven_id, message }
    RPi-->>PC: OvenStatusUpdated { oven_id, current_celsius, target_celsius, enabled, heating, state: "idle" }

    Note over PC,RPi: 2. Cambiar temperatura deseada
    PC->>RPi: SetTargetTemperature { oven_id, target_celsius: 180.0 }
    RPi-->>PC: CommandAccepted { accepted_type: SetTargetTemperature, oven_id, message }
    RPi-->>PC: OvenStatusUpdated { oven_id, target_celsius: 180.0, state: "heating" }

    Note over PC,RPi: 3. Solicitar estado (sincronización)
    PC->>RPi: RequestStatus { oven_id, scope: "all" }
    RPi-->>PC: OvenStatusUpdated { ... }

    Note over PC,RPi: 4. Comando inválido (rechazo)
    PC->>RPi: SetTargetTemperature { oven_id: "Horno-99", target_celsius: 500.0 }
    RPi-->>PC: CommandRejected { rejected_type: SetTargetTemperature, oven_id: "Horno-99", reason: "oven_not_found", message }

    Note over PC,RPi: 5. Parada de emergencia
    PC->>RPi: EmergencyStop { reason: "Sobrecalentamiento detectado por operador" }
    RPi-->>PC: CommandAccepted { accepted_type: EmergencyStop, message }
    RPi-->>PC: OvenStatusUpdated { oven_id: "Horno-01", state: "emergency_stopped", enabled: false, heating: false }

    Note over PC,RPi: 6. Falla técnica detectada por Raspberry
    RPi-->>PC: FaultRaised { oven_id: "Horno-01", fault_code: "sensor_unavailable", severity: "high", message: "Sensor TEMP_A1 no responde" }
    RPi-->>PC: OvenStatusUpdated { oven_id: "Horno-01", state: "faulted" }
```

---

## Flujo 1: La Raspberry detecta un horno nuevo

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | Raspberry | `OvenDetected` | "Detecté un sensor y un actuador nuevos: esto es un horno." |
| 2 | PC | `SetTargetTemperature` | "Configurá la temperatura deseada a 150°C." |
| 3 | Raspberry | `CommandAccepted` | "Temperatura configurada." |
| 4 | PC | `SetOvenEnabled` | "Habilitá el horno para que opere." |
| 5 | Raspberry | `CommandAccepted` | "Horno habilitado." |
| 6 | Raspberry | `OvenStatusUpdated` | "Este es el estado actual del horno." |

**Notas**:
- La Raspberry es la autoridad física: detecta hardware nuevo y avisa a la PC.
- La PC solo muestra lo que existe. Nunca crea hornos.
- La configuración operativa (temperatura, habilitación) se hace con comandos normales después del descubrimiento.

---

## Flujo 2: Cambiar temperatura deseada

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | PC | `SetTargetTemperature` | "Quiero que este horno vaya a 180°C." |
| 2 | Raspberry | `CommandAccepted` | "La temperatura es válida y el horno existe." |
| 3 | Raspberry | `OvenStatusUpdated` | "El horno ahora está calentando hacia 180°C." |

**Notas**:
- La Raspberry valida que el horno exista y que la temperatura no exceda `max_celsius`.
- La Raspberry decide cuándo cambiar el estado a `heating` según su lógica de control.
- El PC no sabe si el horno está calentando hasta que recibe `OvenStatusUpdated`.

---

## Flujo 3: Solicitar estado (sincronización)

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | PC | `RequestStatus` | "Dame el estado actual de todos los hornos." |
| 2 | Raspberry | `OvenStatusUpdated` (xN) | "Estado del horno 1... del horno 2... etc." |

**Notas**:
-Útil cuando la PC se reconecta o arranca y necesita sincronizarse.
- La Raspberry responde con un `OvenStatusUpdated` por cada horno registrado.
- No hay `CommandAccepted` para `RequestStatus`: la respuesta directa es el estado.

---

## Flujo 4: Comando inválido (rechazo)

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | PC | `SetTargetTemperature` (a horno inexistente) | "Quiero cambiar temperatura de Horno-99." |
| 2 | Raspberry | `CommandRejected` | "Ese horno no existe. Comando rechazado." |

**Notas**:
- El PC debe usar el `correlation_id` del mensaje original para saber qué comando fue rechazado.
- La razón del rechazo usa un `FaultCode` estándar (`oven_not_found`, `invalid_temperature`, etc.).
- El PC nunca debe asumir que un comando fue aplicado hasta recibir `CommandAccepted`.

---

## Flujo 5: Parada de emergencia

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | PC | `EmergencyStop` | "Detené todo AHORA." |
| 2 | Raspberry | `CommandAccepted` | "Parada de emergencia activada." |
| 3 | Raspberry | `OvenStatusUpdated` (por cada horno) | "Todos los hornos están en estado seguro." |

**Notas**:
- La Raspberry desactiva salidas físicas inmediatamente, independientemente de la conexión con la PC.
- El sistema queda en estado `emergency_stopped` hasta que se resetee manualmente (futuro).
- Es el único comando que la Raspberry ejecuta incluso si hay fallas activas.

---

## Flujo 6: Falla técnica detectada por Raspberry

| Paso | Quién envía | Mensaje | Qué significa |
|---|---|---|---|
| 1 | Raspberry | `FaultRaised` | "Detecté una falla: el sensor no responde." |
| 2 | Raspberry | `OvenStatusUpdated` | "El horno afectado está en estado faulted." |

**Notas**:
- La falla puede o no estar asociada a un horno específico (`oven_id` puede ser `None` si es falla del sistema).
- El PC no pidió nada. La falla es un evento asíncrono generado por la Raspberry.
- Después de `FaultRaised`, la Raspberry puede seguir operando otros hornos o entrar en estado seguro según la severidad.

---

## Reglas del protocolo en resumen

| Regla | Explicación |
|---|---|
| **El PC solo pide** | Nunca actualiza estado directamente. Espera confirmación. |
| **La Raspberry decide** | Valida, ejecuta y reporta. Es la autoridad del estado físico. |
| **Cada comando tiene respuesta** | `CommandAccepted` o `CommandRejected`. Nunca silencio. |
| **El estado cambia, se reporta** | Cada cambio relevante genera `OvenStatusUpdated`. |
| **Las fallas son eventos** | `FaultRaised` informa condiciones anormales sin depender de comandos previos. |
| **Envelope obligatorio** | Todo mensaje lleva `event_id`, `timestamp`, `version`, `source`, `target`. |
| **Correlación** | Respuestas usan `correlation_id` del comando que las originó. |

---

## Estado actual: arquitectura implementada (Mayo 2026)

Este documento describe el protocolo entre PC y Raspberry. El flujo principal ya está implementado y verificado en dos procesos locales conectados por TCP localhost.

| Capa | Estado | Detalle |
|---|---|---|
| Protocolo (`Message` enum + `EventEnvelope`) | ✅ Verificado | Tipado, serde, 93 tests |
| `transport` | ✅ Verificado | TCP localhost, JSON line framing, canales acotados, logs `[TX]`/`[RX]`, 14 tests |
| `rpi-controller` (Bevy ECS headless) | ✅ Verificado | Procesa comandos, simula temperatura, emite eventos, 38 tests |
| `pc-app` (Bevy ECS headless) | ✅ Verificado | Read model, authoring de comandos, 23 tests |
| **Transporte PC ↔ Raspberry** | ✅ Verificado | RPi escucha, PC conecta, ambos intercambian `EventEnvelope` por TCP |

### Demo verificada

```bash
# Terminal 1: Raspberry simulada
cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000

# Terminal 2: PC con demo automática
cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo
```

### Flujo real de la demo

```mermaid
sequenceDiagram
    autonumber
    participant RPi as rpi-controller<br/>Servidor TCP
    participant TCP as TCP localhost<br/>127.0.0.1:7000
    participant PC as pc-app<br/>Cliente TCP demo

    RPi->>RPi: Arranca con --simulate 2 --listen
    RPi->>TCP: Escucha conexión TCP
    PC->>TCP: Conecta con --connect + --demo
    RPi-->>PC: OvenDetected { oven_id, sensor_ref, output_ref, max_celsius }
    PC->>RPi: SetTargetTemperature { oven_id, target_celsius }
    RPi-->>PC: CommandAccepted { accepted_type: SetTargetTemperature, oven_id }
    PC->>RPi: RequestStatus { scope }
    RPi-->>PC: OvenStatusUpdated { oven_id, current_celsius, target_celsius, enabled, heating, state }
```

### Consecuencia

La arquitectura distribuida ya está viva en local. Lo que todavía no existe es la UI visual ni el GPIO real: la demo usa PC headless y hornos simulados.

### Cómo probar cada lado sin transporte

**rpi-controller** (simula hornos y responde comandos):
```bash
cargo run --package rpi-controller -- --simulate 3
```

**pc-app** (procesa eventos y autoriza comandos en vacío):
```bash
cargo run --package pc-app
```

Para revisar toda la base verificada:
```bash
cargo test --workspace   # 168 tests total
```

## Referencias

- `docs/prd-sistema-control-hornos.md`
- `docs/events.md`
- `docs/adr/001-protocolo-eventos-rust-enum.md`
- `docs/adr/002-hysteresis-control-termico.md`
- `docs/estado-actual.md`
- `docs/flujo-datos-transporte.md`
- `openspec/changes/protocol/specs/protocol-message-serialization/spec.md`
- `openspec/changes/pc-app/verify-report.md`
- `openspec/changes/pc-rpi-transport-link/verify-report.md`
- `openspec/changes/archive/2026-05-21-rpi-controller-bevy-headless/archive-report.md`
