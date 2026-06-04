# pc-app Specification

## Purpose

[To be defined by the UI implementation/requirements]

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

### Requirement: Selección por rango de índices

El sistema MUST permitir al operador especificar un rango de índices numéricos (desde-hasta) para seleccionar hornos. Los hornos se identifican como `oven-N` donde N es un entero; el sistema parsea el sufijo numérico y selecciona los hornos cuyos índices caen dentro del rango especificado.

#### Scenario: Selección de rango válido

- GIVEN existen 100 hornos detectados (`oven-0` a `oven-99`)
- WHEN el operador ingresa "From: 10" y "To: 29"
- THEN los hornos `oven-10` a `oven-29` quedan seleccionados (20 hornos)

#### Scenario: Rango con índices no existentes

- GIVEN existen 10 hornos detectados (`oven-0` a `oven-9`)
- WHEN el operador ingresa "From: 5" y "To: 15"
- THEN solo los hornos existentes en el rango se seleccionan: `oven-5` a `oven-9` (5 hornos)
- AND la UI muestra un aviso "Índice 10-15 no existen"

#### Scenario: Rango invertido

- GIVEN existen 50 hornos
- WHEN el operador ingresa "From: 30" y "To: 10"
- THEN la UI muestra un error "Rango inválido: from debe ser <= to"
- AND ningún horno queda seleccionado

#### Scenario: Rangos negativos

- WHEN el operador ingresa "From: -1" y "To: 10"
- THEN la UI muestra un error "Índices deben ser >= 0"
- AND ningún horno queda seleccionado



### Requirement: Seleccionar todos

El sistema MUST proveer un botón "Seleccionar Todos" que seleccione todos los hornos detectados actualmente.

#### Scenario: Select All con hornos detectados

- GIVEN existen 50 hornos detectados
- WHEN el operador presiona "Seleccionar Todos"
- THEN los 50 hornos quedan seleccionados

#### Scenario: Select All sin hornos

- GIVEN no existen hornos detectados
- WHEN el operador presiona "Seleccionar Todos"
- THEN la UI muestra "No hay hornos para seleccionar"



### Requirement: Encender seleccionados

El sistema MUST encolar un comando `SetOvenEnabled { enabled: true }` para cada horno seleccionado.

#### Scenario: Encender rango

- GIVEN los hornos `oven-0` a `oven-9` están seleccionados
- WHEN el operador presiona "Encender seleccionados"
- THEN se encolan 10 comandos `SetOvenEnabled { enabled: true }` en `OutboundProtocolQueue`
- AND cada comando contiene el `oven_id` correcto

#### Scenario: Encender sin selección

- GIVEN ningún horno está seleccionado
- WHEN el operador presiona "Encender seleccionados"
- THEN ningún comando se encola
- AND la UI muestra "No hay hornos seleccionados"



### Requirement: Apagar seleccionados

El sistema MUST encolar un comando `SetOvenEnabled { enabled: false }` para cada horno seleccionado.

#### Scenario: Apagar rango

- GIVEN los hornos `oven-0` a `oven-9` están seleccionados
- WHEN el operador presiona "Apagar seleccionados"
- THEN se encolan 10 comandos `SetOvenEnabled { enabled: false }` en `OutboundProtocolQueue`



### Requirement: Aplicar temperatura

El sistema MUST encolar un comando `SetTargetTemperature` para cada horno seleccionado con la temperatura especificada.

#### Scenario: Aplicar temperatura a rango

- GIVEN los hornos `oven-0` a `oven-29` están seleccionados
- WHEN el operador ingresa temperatura "50" y presiona "Aplicar temperatura"
- THEN se encolan 30 comandos `SetTargetTemperature { target_celsius: 50.0 }` en `OutboundProtocolQueue`

#### Scenario: Temperatura fuera de rango

- GIVEN los hornos `oven-0` a `oven-9` están seleccionados, cada uno con `MaxTemperature = 300.0`
- WHEN el operador ingresa "350" y presiona "Aplicar temperatura"
- THEN ningún comando se encola
- AND la UI muestra "Temperatura excede límite (300.0 °C)"

#### Scenario: Temperatura negativa

- WHEN el operador ingresa "-10" y presiona "Aplicar temperatura"
- THEN ningún comando se encola
- AND la UI muestra "Temperatura debe ser >= 0"



### Requirement: Encender + aplicar temperatura

El sistema MUST encolar primero los comandos `SetOvenEnabled { enabled: true }` y luego `SetTargetTemperature` para cada horno seleccionado.

#### Scenario: Encender + temperatura

- GIVEN los hornos `oven-0` a `oven-29` están seleccionados
- WHEN el operador ingresa temperatura "40" y presiona "Encender + aplicar temperatura"
- THEN se encolan 30 comandos `SetOvenEnabled { enabled: true }` seguidos de 30 comandos `SetTargetTemperature { target_celsius: 40.0 }`
- AND el total de comandos encolados es 60



### Requirement: Validación de rangos y temperatura

El sistema MUST validar antes de encolar cualquier comando:
- Índices deben ser >= 0
- `from` debe ser <= `to`
- Temperatura debe estar en rango `[0.0, min_max_temp]` de los hornos seleccionados
- Debe haber al menos un horno seleccionado

#### Scenario: Validación completa previene envío

- GIVEN rangos inválidos o temperatura fuera de rango
- WHEN el operador presiona cualquier acción bulk
- THEN ningún comando se encola
- AND la UI muestra mensajes de error descriptivos



### Requirement: Comandos encolados correctos

Cada comando encolado por bulk operations MUST ser idéntico al que se encolaría con controles individuales.

#### Scenario: Comandos bulk idénticos a individuales

- GIVEN un horno `oven-5` seleccionado
- WHEN el operador presiona "Encender seleccionados"
- THEN el comando encolado es `SetOvenEnabled { oven_id: "oven-5", enabled: true }`
- AND es idéntico al que se encolaría presionando el botón individual en la card del horno



### Requirement: Orden determinístico

Los comandos encolados por bulk operations DEBEN seguir orden determinístico ascendente por índice numérico.

#### Scenario: Comandos en orden ascendente

- GIVEN hornos `oven-5`, `oven-2`, `oven-10` seleccionados (por select all con 3 hornos)
- WHEN se encienden
- THEN los comandos se encolan en orden: `oven-2`, `oven-5`, `oven-10`



### Requirement: Preservar controles individuales

Los controles individuales por horno existentes NO deben modificarse en su comportamiento.

#### Scenario: Controles individuales intactos

- GIVEN el panel bulk está presente
- WHEN el operador usa controles individuales de un horno
- THEN el comportamiento es idéntico al anterior a este change



### Requirement: ECS Demo Monitor panel
        
The system MUST render a pedagogical panel titled "ECS Demo Monitor" inside the left sidebar below bulk operations. The panel MUST display a 2-column egui grid with all `EcsDemoMetrics` fields as label-value rows (Total ovens, Enabled, Heating, Faulted, Last bulk op, Commands, Dispatch (us), FPS, Frame time (ms), Logged events/sec). The panel MUST include a pedagogical disclaimer stating these are app-level metrics, NOT Bevy scheduler internals.

#### Scenario: Panel shows all metrics

- GIVEN `EcsDemoMetrics` with non-zero values
- WHEN the UI renders the left sidebar
- THEN the grid shows rows for: Total ovens, Enabled, Heating, Faulted, Last bulk op, Commands, Dispatch (us), FPS, Frame time (ms), Logged events/sec

Cada comando encolado por bulk operations se registra en el event log con prefijo "[BULK]" para distinguirlo de comandos individuales.

#### Scenario: Log registra comandos en bloque

- GIVEN 10 hornos seleccionados y se presiona "Encender seleccionados"
- WHEN los comandos se encolan
- THEN el log registra 10 entradas: "[BULK] OUT SetOvenEnabled oven-3: true", etc.

