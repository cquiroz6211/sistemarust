# pc-app-ui Specification

## Purpose

Proveer una interfaz grafica en tiempo real para visualizar el estado de los hornos, enviar comandos de control, registrar eventos y mostrar el estado de conexion con la Raspberry Pi, utilizando `bevy_egui` integrado al ECS existente.

## Requirements

### Requirement: Visualizacion del estado de hornos

El sistema MUST mostrar una tabla con todas las entidades de horno detectadas. Cada fila MUST incluir: `OvenId`, `CurrentTemperature` (°C, 1 decimal), `TargetTemperature` (°C, 1 decimal), `Enabled` (Si/No), `OvenStatus` (etiqueta legible), `Heating` (indicador visual) y `FaultState` (codigo y severidad si existe). La tabla MUST actualizarse cada frame leyendo los componentes ECS via `Query`.

#### Scenario: Tabla muestra hornos detectados

- GIVEN existen 2 entidades con componentes `OvenId`, `CurrentTemperature`, `TargetTemperature`, `Enabled`, `OvenStatus`
- WHEN la UI se renderiza
- THEN la tabla muestra 2 filas con los valores actuales de cada componente

#### Scenario: Tabla vacia cuando no hay hornos

- GIVEN no existen entidades de horno
- WHEN la UI se renderiza
- THEN la tabla muestra un mensaje "Sin hornos detectados"

#### Scenario: Temperatura formateada a un decimal

- GIVEN un horno con `CurrentTemperature = 185.567`
- WHEN la UI renderiza la fila
- THEN la temperatura se muestra como "185.6 °C"

### Requirement: Controles de habilitacion por horno

El sistema MUST proveer un boton por horno que alterne entre "Encender" y "Apagar". Al activarse, MUST encolar un `SetOvenEnabled` en `OutboundProtocolQueue` usando `author_set_oven_enabled_command`. El boton MUST reflejar el estado actual de `Enabled`.

#### Scenario: Encender horno

- GIVEN un horno con `Enabled = false`
- WHEN el operador presiona "Encender"
- THEN un `EventEnvelope` con `Message::SetOvenEnabled { enabled: true }` se encola en `OutboundProtocolQueue`

#### Scenario: Apagar horno

- GIVEN un horno con `Enabled = true`
- WHEN el operador presiona "Apagar"
- THEN un `EventEnvelope` con `Message::SetOvenEnabled { enabled: false }` se encola

### Requirement: Control de temperatura objetivo

El sistema MUST proveer un slider numerico por horno para ajustar `TargetTemperature`. El rango valido MUST ser `[0.0, MaxTemperature]`. Al confirmar el valor, el sistema MUST encolar un `SetTargetTemperature` via `author_set_target_temperature_command`.

#### Scenario: Temperatura dentro del rango valido

- GIVEN un horno con `MaxTemperature = 300.0`
- WHEN el operador ajusta el slider a `250.0`
- THEN se encola `SetTargetTemperature { target_celsius: 250.0 }`

#### Scenario: Temperatura excede limite rechazada localmente

- GIVEN un horno con `MaxTemperature = 300.0`
- WHEN el operador ingresa `350.0`
- THEN el sistema MUST rechazar la entrada sin enviar comando
- AND la UI muestra "Temperatura excede limite (300.0 °C)"

#### Scenario: Temperatura negativa rechazada

- GIVEN un horno con `MaxTemperature = 300.0`
- WHEN el operador ingresa `-10.0`
- THEN el sistema rechaza la entrada sin enviar comando

### Requirement: Solicitud de estado

El sistema MUST proveer un boton "Solicitar Estado" por horno y un boton global "Actualizar Todos". Al activarse, MUST encolar `RequestStatus` via `author_request_status_command`.

#### Scenario: Solicitud individual

- GIVEN un horno con `OvenId = "oven1"`
- WHEN el operador presiona "Solicitar Estado"
- THEN se encola `RequestStatus { oven_id: Some("oven1"), scope: Single }`

#### Scenario: Solicitud global

- WHEN el operador presiona "Actualizar Todos"
- THEN se encola `RequestStatus { oven_id: None, scope: All }`

### Requirement: Emergency stop global

El sistema MUST proveer un boton "Emergency Stop" siempre visible. Al activarse, MUST solicitar confirmacion y, al confirmar, encolar `EmergencyStop` via `author_emergency_stop_command`.

#### Scenario: Emergency stop con confirmacion

- WHEN el operador presiona "Emergency Stop"
- THEN la UI muestra dialogo de confirmacion
- AND al confirmar, se encola `EmergencyStop { reason: "Activado desde UI" }`

#### Scenario: Emergency stop cancelado

- WHEN el operador presiona "Emergency Stop"
- AND el operador cancela el dialogo
- THEN no se envia ningun comando

### Requirement: Log de eventos

El sistema MUST mantener un registro scrollable de eventos inbound y outbound. Cada entrada MUST incluir timestamp, direccion (IN/OUT), tipo de mensaje y resumen. El log MUST retener las ultimas 200 entradas, descartando las mas antiguas (FIFO).

#### Scenario: Evento inbound registrado

- GIVEN `InboundProtocolQueue` contiene un `OvenStatusUpdated` para "oven1"
- WHEN el sistema de ingestion procesa el evento
- THEN el log agrega "[HH:MM:SS] IN OvenStatusUpdated oven1: 185.0 °C"

#### Scenario: Comando outbound registrado

- WHEN el operador envia `SetTargetTemperature` para "oven1"
- THEN el log agrega "[HH:MM:SS] OUT SetTargetTemperature oven1: 250.0 °C"

#### Scenario: Limite de entradas

- GIVEN el log contiene 200 entradas
- WHEN una nueva entrada se agrega
- THEN la entrada mas antigua se descarta
- AND el log mantiene exactamente 200 entradas

### Requirement: Indicador de conexion

El sistema MUST mostrar un indicador visual del estado de conexion con la Raspberry Pi: "Conectado" (verde) o "Desconectado" (rojo). El estado MUST reflejar la disponibilidad del transporte TCP.

#### Scenario: Conexion activa

- GIVEN el transporte TCP esta conectado al RPi
- THEN el indicador muestra "Conectado" en verde

#### Scenario: Conexion perdida

- GIVEN el transporte TCP se desconecta
- THEN el indicador muestra "Desconectado" en rojo

### Requirement: No bloqueo del ECS

La UI MUST completar su renderizado dentro del frame ECS (Update schedule). El sistema MUST NOT introducir latencia medible en el game loop. Las operaciones de UI (render, input) MUST ejecutarse sincronicamente en el hilo principal.

#### Scenario: Frame rate mantenido

- GIVEN el sistema ejecuta 60 FPS con 10 hornos
- WHEN la UI renderiza la tabla completa y el log
- THEN el frame rate no baja de 55 FPS

### Requirement: Integracion no invasiva

La UI MUST consumir recursos existentes (`InboundProtocolQueue`, `OutboundProtocolQueue`, componentes ECS) sin modificar su comportamiento. Los sistemas de ingestion, read model y command authoring MUST permanecer inalterados.

#### Scenario: Sistemas existentes sin cambio

- GIVEN la UI esta activa
- WHEN se procesa un `OvenStatusUpdated`
- THEN el read model se actualiza identico a como lo hace sin UI
- AND los comandos se encolan usando las mismas funciones de authoring
