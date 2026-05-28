# Flujo de Datos del Transporte PC ↔ Raspberry

El transporte ya conecta dos procesos locales: `rpi-controller` escucha como servidor TCP y `pc-app` se conecta como cliente. La comunicación usa `EventEnvelope` serializado como una línea JSON por mensaje.

La idea clave para estudiantes: **Bevy ECS sigue manejando el dominio; Tokio sólo mueve bytes por TCP**.

## Camino rápido

Abrí dos terminales.

```bash
# Terminal 1: Raspberry simulada como servidor
cargo run --package rpi-controller -- --simulate 2 --listen 127.0.0.1:7000

# Terminal 2: PC como cliente con demo automática
cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo
```

Qué deberías observar:

- La Raspberry escucha en `127.0.0.1:7000`.
- La PC se conecta a esa dirección.
- Aparecen logs con `[TX]` cuando un proceso envía un mensaje.
- Aparecen logs con `[RX]` cuando un proceso recibe un mensaje.
- La demo muestra descubrimiento de hornos, envío de comando y actualización de estado.

## Componentes

```mermaid
flowchart LR
    subgraph PC[pc-app - Cliente]
        PC_ECS[Bevy ECS\nRead model + comandos]
        PC_Q_OUT[OutboundProtocolQueue]
        PC_Q_IN[InboundProtocolQueue]
    end

    subgraph TPC[transport crate]
        PC_BRIDGE[Bridge ECS ↔ Tokio\ntry_send / try_recv]
        TCP[TCP localhost\n127.0.0.1:7000]
        RPI_BRIDGE[Bridge ECS ↔ Tokio\ntry_send / try_recv]
    end

    subgraph RPI[rpi-controller - Servidor]
        RPI_Q_IN[InboundProtocolQueue]
        RPI_Q_OUT[OutboundProtocolQueue]
        RPI_ECS[Bevy ECS\nSimulación + control]
    end

    PC_ECS --> PC_Q_OUT --> PC_BRIDGE --> TCP --> RPI_BRIDGE --> RPI_Q_IN --> RPI_ECS
    RPI_ECS --> RPI_Q_OUT --> RPI_BRIDGE --> TCP --> PC_BRIDGE --> PC_Q_IN --> PC_ECS
```

## Secuencia de la demo

```mermaid
sequenceDiagram
    autonumber
    participant RPi as rpi-controller<br/>Servidor TCP
    participant TCP as TCP localhost
    participant PC as pc-app<br/>Cliente demo

    RPi->>RPi: Arranca con --simulate 2
    RPi->>TCP: Escucha con --listen 127.0.0.1:7000
    PC->>TCP: Conecta con --connect 127.0.0.1:7000
    RPi-->>PC: OvenDetected
    PC->>RPi: SetTargetTemperature
    RPi-->>PC: CommandAccepted
    PC->>RPi: RequestStatus
    RPi-->>PC: OvenStatusUpdated
```

## Puente entre colas ECS y Tokio

```mermaid
flowchart TB
    subgraph Bevy[Bevy ECS - dominio]
        OUT[OutboundProtocolQueue]
        IN[InboundProtocolQueue]
        SYS_OUT[bridge_outbound_to_transport\nno bloquea]
        SYS_IN[bridge_inbound_from_transport\nno bloquea]
    end

    subgraph Channels[Canales acotados - capacidad 64]
        CH_OUT[OutboundSender]
        CH_IN[InboundReceiver]
    end

    subgraph Tokio[Tokio - adaptador de transporte]
        WRITER[writer task\nserialize JSON + write TCP]
        READER[reader task\nread TCP + parse JSON]
    end

    OUT --> SYS_OUT -->|try_send| CH_OUT --> WRITER --> TCP[(TCP socket)]
    TCP --> READER --> CH_IN -->|try_recv| SYS_IN --> IN
```

## Logs esperados

Los IDs cambian en cada ejecución, pero el patrón debería verse así:

```text
[TX] rpi-controller → pc-app | OvenDetected | id=...
[RX] rpi-controller → pc-app | OvenDetected | id=...
[TX] pc-app → rpi-controller | SetTargetTemperature | id=...
[RX] pc-app → rpi-controller | SetTargetTemperature | id=...
[TX] rpi-controller → pc-app | CommandAccepted | id=...
[RX] rpi-controller → pc-app | CommandAccepted | id=...
[TX] pc-app → rpi-controller | RequestStatus | id=...
[TX] rpi-controller → pc-app | OvenStatusUpdated | id=...
```

## Qué observar con cabeza de ingeniería

| Observación | Qué demuestra |
|---|---|
| `[TX]` en un proceso y `[RX]` en el otro | El mensaje cruzó el transporte TCP. |
| `OvenDetected` nace en RPi | La Raspberry es la autoridad sobre hornos existentes. |
| `SetTargetTemperature` nace en PC | El PC declara intención, no cambia estado físico directamente. |
| `CommandAccepted` vuelve desde RPi | La Raspberry valida antes de aceptar. |
| `OvenStatusUpdated` vuelve desde RPi | El estado real se reporta desde el controlador físico. |

## Por qué no es UI ni GPIO todavía

Este cambio prueba la comunicación distribuida, no el producto final.

| Parte | Estado actual | Qué falta |
|---|---|---|
| UI | PC headless con demo automática | Pantalla Bevy para operador. |
| GPIO | Hornos simulados con drift térmico | Lectura de sensores y control de salidas reales. |
| Transporte | TCP localhost verificado | Robustez futura: reconexión, seguridad, despliegue real. |

Esta separación es intencional. Si mezclamos UI, GPIO y transporte al mismo tiempo, después no sabemos qué falló. Primero se verifica el flujo de datos; después se agregan bordes reales.

## Evidencia de verificación

El cambio `pc-rpi-transport-link` quedó verificado PASS en `openspec/changes/pc-rpi-transport-link/verify-report.md`.

| Paquete | Tests verificados |
|---|---:|
| `transport` | 14 |
| `protocol` | 93 |
| `pc-app` | 23 |
| `rpi-controller` | 38 |
| Workspace completo | 168 |

## Referencias

- `docs/adr/003-tokio-como-adaptador-transporte-bevy.md`
- `docs/diagrama-flujo-protocolo.md`
- `docs/diagrama-clases.md`
- `openspec/changes/pc-rpi-transport-link/verify-report.md`
