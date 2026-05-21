# Exploration: rpi-controller Architecture

## Topic
`rpi-controller` — Controlador físico Rust para Raspberry Pi (headless, sin UI)

## Recommendation
**Opción B — Async Tokio + HashMap + Channels**. Runtime async con tokio, estado en structs plano dentro de un HashMap, comunicación serie via channels asincrónicos. Simple, testable, idiomático para Rust.

---

## Opción A: Async Tokio + State Machine por Horno + Channels

**Descripción**: Cada horno se modela como una future/StateMachine que avanza con eventos. El loop principal usa `tokio::select!` para multiplexar comandos entrantes y timers de sensado. Un `HashMap<OvenId, OvenHandle>` gestiona los hornos activos.

**Pros**:
- No requiere dependencias externas (más allá de tokio y serialport)
- Cada horno es una unidad de procesamiento independiente
- Fácil de razonar: un future por horno avanza con eventos
- Timer-driven temperature simulation es natural en async
- Serial RX/TX corre en tasks separadas comunicándose por channels

**Cons**:
- State machine por horno puede crecer en complejidad si hay muchos estados
- Requiere cuidar el borrowing en el HashMap con Mutex/RwLock si se accede desde múltiples tasks
- Para tests de integración se necesita mock del serial port

**Complejidad**: Media

---

## Opción B: Async Tokio + HashMap<OvenId, OvenState> + Channels (RECOMENDADA)

**Descripción**: Estado global de todos los hornos en un `HashMap<String, OvenState>`. Un solo loop async principal con `tokio::select!` recibe comandos del serial, procesa validaciones sincronamente, actualiza el HashMap, y envía eventos. Los timers de simulación son `tokio::spawn` intervals. Sin state machine por entidad.

**Pros**:
- Simple de entender y mantener — el estado es un HashMap, no estado distribuido
- Command validation es una función pura que toma OvenState y devuelve Result/Rejection
- Fácil de testear: se prueba la función de validación con structs de test
- Serial comunicación en task dedicada, resultados via mpsc::channel al loop principal
- Heating logic: simple threshold check (current < target - hysteresis)
- Todavía permite paralelismo si cada sensor read es un task separado
- Menor superficie de código que A

**Cons**:
- No es ECS puro — el HashMap no es un ECS framework
- Si se necesita ECS real, habría que usar `hecs` (otro crate), pero no lo amerita para esta escala
- Mutex en el HashMap si se accede desde múltiples tasks async (mitigable con ownership)

**Complejidad**: Baja

---

## Opción C: hecs (Light ECS) + tokio

**Descripción**: Usa el crate `hecs` para tener entidades, componentes y sistemas reales sin Bevy. Un `World` administra las entidades de horno. Los sistemas son funciones que operan sobre Queries del World.

**Pros**:
- ECS "real" sin Bevy — mismo paradigma que la PC app (si la PC usa Bevy)
- Componentes separados: OvenId, Temperature, Target, Enabled, Heating, Fault
- Sistemas independientes: CommandValidationSystem, TemperatureSensingSystem, HeatingControlSystem, SerialPublishSystem
- Extensible: agregar logging, métricas, redundancia es agregar sistemas, no tocar lógica existente

**Cons**:
- `hecs` es otro crate con su propia API — curva de aprendizaje para el equipo
- Overhead de ECS puede ser innecesario para 1-4 hornos (caso de uso típico)
- Un solo HashMap es suficiente y más simple para esta escala
- Más código boilerplate que B

**Complejidad**: Media-Alta

---

## Simulación de Hardware

**Detección de hornos** (sin GPIO real):
- CLI flag `--simulate` o `-n N` para generar N hornos simulados
- Cada horno simulado tiene: `oven_id`, `sensor_ref`, `output_ref`, `max_celsius`
- Emite `OvenDetected` por cada horno al iniciar
- Alternativa: comando REPL o HTTP API interna para "insertar" hornos en runtime

**Lectura de temperatura** (simulada):
- Cada 2 segundos, un timer actualiza `current_celsius`
- Lógica: si `heating == true`, increment +random noise hasta `target_celsius`; luego flat; si `enabled == false`, decrement gradual
- Para v1: ruido simple (rand -5..+5°C) para hacer la simulación creíble

**Control de salida**:
- `heating` se activa cuando `enabled == true && current_celsius < target_celsius - hysteresis`
- `heating` se desactiva cuando `current_celsius >= target_celsius`
- `hysteresis` = 5°C (simple threshold, no PID en v1)
- `output_level` = 100% cuando heating, 0% cuando no

---

## Runtime Principal

```
tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(async {
        // 1. Spawn serial read task → mpsc::channel → comando_events
        // 2. Spawn timer task (cada 2s) → temperature_events
        // 3. Loop principal:
        //    tokio::select! {
        //        cmd = comando_events.recv() => handle_command(cmd),
        //        tick = temperature_ticker.recv() => update_temperatures(),
        //    }
        // 4. En cada handle: validar → mutar HashMap → serial_send response/event
    })
```

---

## Validación de Comandos

Cada comando recibido via `EventEnvelope` pasa por:

```rust
fn validate_command(cmd: &SetTargetTemperature, ovens: &HashMap<String, OvenState>) -> Result<(), CommandRejectedPayload>
```

Reglas:
- `oven_id` debe existir en HashMap
- `target_celsius` debe estar entre 0 y `max_celsius` del horno
- Si `emergency_stop_active`, todos los comandos menos `EmergencyStop` se rechazan

Validación es función pura → trivial de testear con unit tests.

---

## Estructura de Módulos Sugerida

```
rpi-controller/src/
├── main.rs              # Entry point, tokio runtime setup
├── config.rs            # CLI args, simulation mode
├── state.rs             # OvenState struct, OvenStore (HashMap wrapper)
├── simulation.rs        # Mock temperature generator, mock detection
├── validation.rs        # Command validation functions
├── control.rs           # Heating logic (threshold-based)
├── serial.rs            # Serial port read/write (mockable for tests)
├── handlers.rs          # Command handlers (map Message → Response)
└── emit.rs              # Helper para crear EventEnvelope reply
```

---

## Comparación Final

| Criterio | A (StateMachine) | B (HashMap+Channels) | C (hecs) |
|----------|------------------|----------------------|----------|
| Simplicidad | Media | **Alta** | Alta |
| Testabilidad | Media | **Alta** | Alta |
| Familiaridad ECS | Nula | Nula | Media |
| Curva aprendizaje | Baja | **Baja** | Media |
| Overhead runtime | Bajo | **Bajo** | Medio |
| Escala (N hornos) | Cualquiera | **1-10** | Cualquiera |
| Dependencias extra | Ninguna | **Ninguna** | hecs crate |
| Adequado para v1? | Sí | **Sí** | Sí (overkill) |

---

## Recomendación

**Opción B** para v1. Permite iterar rápido, tests triviales, y si en el futuro el dominio crece a muchos hornos o lógica compleja, se puede migrar a C (hecs) sin mayor dolor.

El PID real queda fuera de scope initial — threshold con hysteresis es suficiente para demostrar el sistema funcionando.

---

## Risks

1. **Serial port mocking es necesario para tests** — hay que abstraer `serialport` detrás de un trait para poder inyectar un mock en tests unitarios.
2. **Simulación creíble** — la simulación de temperatura sin ruido parece artificial. Agregar jitter/simple noise.
3. **CLI flag para modo simulado** — si se compila en RPi real, debe poder deshabilitar la simulación y usar `rppal` directo.