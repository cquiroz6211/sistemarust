# pc-app-bulk-oven-controls Specification

## Purpose

Agregar controles de operaciones en bloque al `pc-app` para seleccionar rangos de hornos y enviar comandos `SetOvenEnabled` y `SetTargetTemperature` a grupos enteros, preservando los controles individuales existentes.

## Requirements

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

### Requirement: Registro en event log

Cada comando encolado por bulk operations se registra en el event log con prefijo "[BULK]" para distinguirlo de comandos individuales.

#### Scenario: Log registra comandos en bloque

- GIVEN 10 hornos seleccionados y se presiona "Encender seleccionados"
- WHEN los comandos se encolan
- THEN el log registra 10 entradas: "[BULK] OUT SetOvenEnabled oven-3: true", etc.
