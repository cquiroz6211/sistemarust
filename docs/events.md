# Protocolo de Eventos — Sistema de Control de Hornos

Este documento define el contrato inicial de eventos entre la aplicación de PC y la Raspberry Pi. La meta es empezar con pocos eventos, vitales y bien nombrados, para que el sistema sea fácil de entender antes de escribir implementación.

## Decisión principal

El sistema se comunica por eventos, pero no todos los mensajes significan lo mismo.

| Tipo | Dirección | Qué significa |
|---|---|---|
| Comando | PC → Raspberry | Una intención o solicitud de acción. |
| Evento | Raspberry → PC | Un hecho confirmado, estado reportado o falla detectada. |

Regla base:

> El PC declara intención. La Raspberry valida, ejecuta y reporta hechos.

## Responsabilidades por lado

| Lado | Responsabilidad |
|---|---|
| PC | Mostrar estado, pedir cambios, enviar comandos, recibir reportes. |
| Raspberry | Validar comandos, mantener estado real, controlar el mundo físico, reportar estado. |
| Protocolo | Definir el idioma común entre ambos. |

## Forma general de cada mensaje

Cada mensaje debe tener dos partes conceptuales:

| Parte | Propósito |
|---|---|
| Envelope | Datos comunes del mensaje: identificación, origen, destino, fecha, versión y correlación. |
| Payload | Datos propios del evento o comando. |

## Envelope obligatorio

| Campo | Obligatorio | Propósito |
|---|---:|---|
| event_id | Sí | Identificar un mensaje único. |
| type | Sí | Indicar qué comando o evento es. |
| source | Sí | Saber quién lo envió. |
| target | Sí | Saber quién debe recibirlo. |
| timestamp | Sí | Saber cuándo se generó. |
| correlation_id | Según caso | Relacionar una respuesta con el comando original. |
| version | Sí | Permitir evolución del protocolo. |
| payload | Sí | Contener los datos específicos del mensaje. |

## Eventos mínimos iniciales

### PC hacia Raspberry

Estos mensajes son comandos. El PC pide algo; la Raspberry decide si lo acepta.

| Comando | Propósito | Respuesta esperada |
|---|---|---|
| RegisterOven | Registrar un horno con su configuración inicial. | CommandAccepted o CommandRejected |
| SetTargetTemperature | Cambiar la temperatura deseada de un horno. | CommandAccepted o CommandRejected |
| SetOvenEnabled | Habilitar o deshabilitar lógicamente un horno. | CommandAccepted o CommandRejected |
| RequestStatus | Pedir estado actual de uno o varios hornos. | OvenStatusUpdated |
| EmergencyStop | Detener inmediatamente la operación por seguridad. | CommandAccepted y OvenStatusUpdated |

### Raspberry hacia PC

Estos mensajes son eventos. La Raspberry informa algo que ya validó, ocurrió o detectó.

| Evento | Propósito | Cuándo ocurre |
|---|---|---|
| CommandAccepted | Confirmar que un comando fue aceptado. | Después de validar correctamente un comando. |
| CommandRejected | Informar que un comando fue rechazado. | Cuando el comando no puede aplicarse. |
| OvenStatusUpdated | Reportar estado actual de un horno. | Luego de cambios relevantes o ante RequestStatus. |
| FaultRaised | Informar una falla técnica o de seguridad. | Cuando la Raspberry detecta una condición anormal. |

## Definición conceptual de comandos

### RegisterOven

Registra un horno en la Raspberry.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Identificador del horno. |
| sensor_ref | Referencia al sensor de temperatura. |
| output_ref | Referencia al actuador, relay o salida de control. |
| target_celsius | Temperatura objetivo inicial, si aplica. |
| max_celsius | Límite técnico máximo permitido. |
| enabled | Si el horno arranca habilitado o no. |

Debe rechazarse si:

- El horno ya existe.
- El sensor ya está asignado.
- La salida ya está asignada.
- La temperatura objetivo excede límites permitidos.
- Falta información mínima para operar.

### SetTargetTemperature

Cambia la temperatura deseada de un horno existente.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Horno a modificar. |
| target_celsius | Nueva temperatura deseada. |

Debe rechazarse si:

- El horno no existe.
- La temperatura está fuera del rango permitido.
- La Raspberry está en parada de emergencia.

### SetOvenEnabled

Habilita o deshabilita lógicamente un horno.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Horno a modificar. |
| enabled | Estado lógico deseado. |

Debe rechazarse si:

- El horno no existe.
- Hay una falla activa que impide habilitarlo.
- La Raspberry está en parada de emergencia.

### RequestStatus

Solicita el estado actual.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Horno específico, si se pide uno solo. |
| scope | Alcance de la consulta: un horno o todos. |

Debe responder con:

- Uno o varios OvenStatusUpdated.

### EmergencyStop

Solicita detener la operación inmediatamente.

| Dato conceptual | Descripción |
|---|---|
| reason | Motivo informado por el PC u operador. |

Debe producir:

- Deshabilitar salidas de control.
- Marcar el sistema en estado seguro.
- Reportar estado actualizado.

## Definición conceptual de eventos

### CommandAccepted

Confirma que un comando fue recibido, validado y aceptado.

| Dato conceptual | Descripción |
|---|---|
| accepted_type | Tipo de comando aceptado. |
| oven_id | Horno relacionado, si aplica. |
| message | Texto corto para diagnóstico humano. |

Debe usar correlation_id para apuntar al comando original.

### CommandRejected

Informa que un comando fue recibido, pero no se aplicó.

| Dato conceptual | Descripción |
|---|---|
| rejected_type | Tipo de comando rechazado. |
| oven_id | Horno relacionado, si aplica. |
| reason | Motivo técnico del rechazo. |
| message | Texto corto para diagnóstico humano. |

Debe usar correlation_id para apuntar al comando original.

### OvenStatusUpdated

Reporta el estado operativo conocido de un horno.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Horno reportado. |
| current_celsius | Temperatura actual medida. |
| target_celsius | Temperatura deseada actual. |
| enabled | Si el horno está habilitado lógicamente. |
| heating | Si el sistema está aplicando calor. |
| output_level | Nivel actual de salida, si aplica. |
| state | Estado operativo resumido. |

Estados iniciales sugeridos:

| Estado | Significado |
|---|---|
| disabled | El horno está registrado pero deshabilitado. |
| idle | El horno está habilitado pero no está calentando. |
| heating | El horno está aplicando calor. |
| faulted | El horno tiene una falla activa. |
| emergency_stopped | El sistema está detenido por emergencia. |

### FaultRaised

Reporta una falla técnica o de seguridad. No es una alerta genérica de UI.

| Dato conceptual | Descripción |
|---|---|
| oven_id | Horno afectado, si aplica. |
| fault_code | Código estable de la falla. |
| severity | Severidad técnica. |
| message | Descripción humana corta. |

Fallas iniciales posibles:

| Falla | Significado |
|---|---|
| oven_not_found | Se intentó operar sobre un horno inexistente. |
| sensor_unavailable | El sensor no está disponible o no responde. |
| invalid_temperature | La lectura o temperatura objetivo no es válida. |
| output_unavailable | La salida de control no está disponible. |
| safety_limit_exceeded | Se superó un límite técnico de seguridad. |
| emergency_stop_active | El sistema está en parada de emergencia. |

## Flujo mínimo esperado

### Registrar horno

1. El PC envía RegisterOven.
2. La Raspberry valida configuración.
3. La Raspberry responde CommandAccepted o CommandRejected.
4. Si fue aceptado, la Raspberry puede emitir OvenStatusUpdated.

### Cambiar temperatura

1. El PC envía SetTargetTemperature.
2. La Raspberry valida que el horno exista y que la temperatura sea segura.
3. La Raspberry responde CommandAccepted o CommandRejected.
4. Si fue aceptado, la Raspberry actualiza el estado del horno.
5. La Raspberry reporta OvenStatusUpdated.

### Parada de emergencia

1. El PC envía EmergencyStop.
2. La Raspberry desactiva salidas de control.
3. La Raspberry responde CommandAccepted.
4. La Raspberry reporta OvenStatusUpdated para reflejar el estado seguro.

## Fuera del protocolo inicial

| Elemento | Motivo |
|---|---|
| OvenAlarm | Todavía no sabemos qué significa una alerta de negocio. |
| HeatingStarted | OvenStatusUpdated ya cubre el estado de calentamiento inicial. |
| HeatingStopped | OvenStatusUpdated ya cubre el estado de calentamiento inicial. |
| TemperatureUpdated | OvenStatusUpdated ya cubre temperatura y estado. |
| Heartbeat | Puede agregarse luego si la salud de conexión entra en el primer hito. |
| RemoveOven | No es vital para el primer flujo de control. |
| Historial avanzado | Pertenece a una etapa posterior. |

## Criterios de aceptación del protocolo inicial

- Cada mensaje tiene una dirección clara.
- Cada comando tiene una respuesta esperada.
- Cada rechazo explica por qué no se aplicó el comando.
- El PC no asume que un comando fue aplicado hasta recibir confirmación.
- La Raspberry es la autoridad sobre el estado físico.
- Las fallas técnicas no se mezclan con alertas visuales genéricas.
- El protocolo puede crecer sin romper el contrato inicial.
