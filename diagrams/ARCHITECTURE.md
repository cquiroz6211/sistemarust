# rpi-controller — Arquitectura

Controlador de hornos industriales simulado que corre en Raspberry Pi. Se comunica con una aplicación PC mediante un protocolo de eventos basado en JSON.

## Visión general

```mermaid
graph TB
    subgraph PC["PC Application"]
        PCAPP["pc-app"]
    end

    subgraph RPI["rpi-controller (Raspberry Pi)"]
        MAIN["main.rs<br/>Entry point"]
        APP["app.rs<br/>Event loop"]
        HANDLERS["handlers.rs<br/>Command dispatch"]
        VALIDATION["validation.rs<br/>Input validation"]
        CONTROL["control.rs<br/>Hysteresis logic"]
        SIM["simulation.rs<br/>Temp simulation"]
        STATE["state.rs<br/>Oven model + store"]
        MOCK["mock_transport.rs<br/>Transport abstraction"]
    end

    PROTO["protocol crate<br/>Shared types"]

    PCAPP <-->|"EventEnvelope<br/>(JSON over transport)"| MOCK
    MAIN --> APP
    APP --> HANDLERS
    APP --> SIM
    HANDLERS --> VALIDATION
    HANDLERS --> STATE
    SIM --> CONTROL
    SIM --> STATE
    HANDLERS --> PROTO
    SIM --> PROTO
    STATE --> PROTO
```

## Flujo de datos principal

```mermaid
sequenceDiagram
    participant PC as pc-app
    participant T as Transport
    participant App as App (event loop)
    participant H as Handlers
    participant S as OvenStore

    PC->>T: EventEnvelope (command)
    App->>T: transport.read()
    T-->>App: EventEnvelope
    App->>H: handle(envelope, store, &mut emergency)
    H->>S: read/validate/mutate
    H-->>App: Vec<EventEnvelope> (replies)
    App->>T: transport.write(reply)

    Note over App: Every 2 seconds (tick)
    App->>App: update_temperature() for each oven
    App->>T: OvenStatusUpdated + FaultRaised (if any)
```

## Protocolo de comunicación

El crate `protocol` define tipos compartidos serializados como JSON con tagging externo: `{"type": "VariantName", "payload": {...}}`.

```mermaid
graph LR
    subgraph Commands["Comandos (PC → RPi)"]
        ST["SetTargetTemperature"]
        SE["SetOvenEnabled"]
        RS["RequestStatus"]
        ES["EmergencyStop"]
    end

    subgraph Events["Eventos (RPi → PC)"]
        OD["OvenDetected"]
        CA["CommandAccepted"]
        CR["CommandRejected"]
        OS["OvenStatusUpdated"]
        FR["FaultRaised"]
    end
```

Cada mensaje se envuelve en un `EventEnvelope` con metadatos:

```mermaid
classDiagram
    class EventEnvelope {
        +UUID event_id
        +String source
        +String target
        +DateTime timestamp
        +Option~UUID~ correlation_id
        +String version
        +Message payload
        +new(source, target, payload) EventEnvelope
        +reply_to(payload) EventEnvelope
    }
```

`reply_to()` invierte source/target y guarda el `event_id` original como `correlation_id`.

## Máquina de estados del horno

```mermaid
stateDiagram-v2
    [*] --> Disabled : Oven creada

    Disabled --> Idle : SetOvenEnabled(true)
    Idle --> Heating : temp < target - 5°C
    Heating --> Idle : temp >= target
    Idle --> Disabled : SetOvenEnabled(false)
    Heating --> Disabled : SetOvenEnabled(false)

    Disabled --> Faulted : temp > max_celsius
    Idle --> Faulted : temp > max_celsius
    Heating --> Faulted : temp > max_celsius

    Disabled --> EmergencyStopped : EmergencyStop
    Idle --> EmergencyStopped : EmergencyStop
    Heating --> EmergencyStopped : EmergencyStopped
    Faulted --> EmergencyStopped : EmergencyStop

    note right of Faulted : Requiere reset manual
    note right of EmergencyStopped : Requiere reset manual
```

## Control de histéresis

El calentador usa una banda de histéresis de **5°C** (ADR 002) para evitar ciclos rápidos on/off:

```mermaid
graph LR
    subgraph "Reglas de decisión"
        A["current < target - 5°C<br/>→ heating = ON"]
        B["current ≥ target<br/>→ heating = OFF"]
        C["target - 5°C ≤ current < target<br/>→ mantener estado anterior"]
    end

    A --> D["Zona de encendido"]
    B --> E["Zona de apagado"]
    C --> F["Banda muerta (dead band)"]
```

Ejemplo con target = 150°C:

| Temperatura actual | Estado heating |
|---|---|
| < 145°C | ON |
| 145°C – 149.9°C | Mantiene el anterior |
| ≥ 150°C | OFF |

## Simulación de temperatura

Cada tick (2 segundos) actualiza la temperatura de todos los hornos:

```mermaid
flowchart TD
    START[Tick cada 2s] --> CHECK{current > max?}
    CHECK -->|Sí| FAULT[State = Faulted<br/>enabled = false<br/>heating = false<br/>Emit FaultRaised]
    CHECK -->|No| DECIDE[heating_decision<br/>hysteresis]
    DECIDE --> HEAT{heating?}
    HEAT -->|Sí| UP[current += 4°C + noise ±2°C]
    HEAT -->|No| DOWN[current -= 1°C + noise ±2°C<br/>min = 20°C room temp]
    UP --> CHECK2{current > max?}
    DOWN --> CHECK2
    CHECK2 -->|Sí| FAULT
    CHECK2 -->|No| STATE2[Actualizar OvenState]
    STATE2 --> EMIT[Emit OvenStatusUpdated]
    FAULT --> EMIT
```

Constantes de simulación:

| Constante | Valor | Unidad |
|---|---|---|
| HEATING_DRIFT | 4.0 | °C/tick al calentar |
| COOLING_DRIFT | 1.0 | °C/tick al enfriar |
| ROOM_TEMP | 20.0 | °C mínimo absoluto |
| NOISE_AMPLITUDE | ±2.0 | °C ruido aleatorio |
| HYSTERESIS | 5.0 | °C banda muerta |
| TICK_INTERVAL | 2 | segundos entre ticks |

## Transporte

```mermaid
graph LR
    subgraph "Trait Transport"
        R["async read() → Option~EventEnvelope~"]
        W["async write(&EventEnvelope)"]
    end

    subgraph "Implementaciones"
        MT["MockTransport<br/>tokio::mpsc channels<br/>in-memory, para testing"]
        FUTURE["RealTransport<br/>(futuro: serial port)"]
    end

    R -.-> MT
    W -.-> MT
    R -.-> FUTURE
    W -.-> FUTURE
```

`MockTransport::channel()` crea un par conectado A↔B. Lo que A escribe, B lo lee y viceversa. Esto permite tests end-to-end sin hardware.

## Estructura de módulos

| Módulo | Responsabilidad |
|---|---|
| `main.rs` | Parseo de `--simulate N`, creación de App, event loop |
| `app.rs` | Orquestador: `tokio::select!` multiplexa comandos entrantes + ticks de simulación |
| `handlers.rs` | Dispatch de comandos → validación → mutación de estado → generación de eventos respuesta |
| `validation.rs` | Validación pura de comandos (rango de temperatura, estado del horno, emergency stop) |
| `control.rs` | Lógica de histéresis para decidir si el calentador está ON u OFF |
| `simulation.rs` | Simulación de temperatura: drift, ruido, límites de seguridad |
| `state.rs` | Modelo de dominio (`Oven`) y store thread-safe (`OvenStore = Arc<RwLock<HashMap>>`) |
| `mock_transport.rs` | Abstracción `Transport` + implementación mock con canales |

## Manejo de EmergencyStop

```mermaid
sequenceDiagram
    participant PC as pc-app
    participant App as App
    participant H as Handlers
    participant S as OvenStore

    PC->>App: EmergencyStop
    App->>H: handle()
    H->>H: emergency_active = true
    H->>S: ALL ovens: enabled=false, heating=false, state=EmergencyStopped
    H-->>App: CommandAccepted + FaultRaised(High)
    App->>PC: CommandAccepted + FaultRaised

    Note over App,H: Cualquier comando subsiguiente es rechazado con FaultCode::EmergencyStopActive
```

## Ejecución

```bash
# Sin hornos simulados
cargo run

# Con 3 hornos simulados
cargo run -- --simulate 3
```
