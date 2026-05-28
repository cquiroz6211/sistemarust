# PRD — rpi-controller Headless con Bevy ECS

Este PRD redefine el enfoque del `rpi-controller` para usar **Bevy como runtime ECS headless** en lugar de un loop principal basado en Tokio. La meta es unificar el modelo mental del sistema: la PC usa Bevy con UI, y la Raspberry usa Bevy sin UI, aprovechando entidades, componentes, recursos, eventos y schedules para procesar el dominio de hornos.

## Decisión principal

El `rpi-controller` será una aplicación **headless** construida con:

- `App::new()`
- `MinimalPlugins`
- `ScheduleRunnerPlugin`
- `FixedUpdate`
- `Events`
- `Resources`

Regla base:

> La Raspberry sigue siendo la autoridad física. Bevy no cambia la responsabilidad del sistema; cambia cómo organizamos el procesamiento interno.

## Por qué cambiamos el enfoque

El diseño previo con `Tokio + HashMap + handlers` era viable, pero no aprovechaba que el problema ya es naturalmente ECS:

- múltiples hornos independientes
- estado por horno
- procesamiento periódico
- cambios de estado
- eventos entre sistemas
- lógica desacoplable por responsabilidad

Además, la documentación de Bevy confirma que puede correr **sin renderer y sin ventana** usando `MinimalPlugins` y `ScheduleRunnerPlugin`, por lo que también sirve para procesos headless.

## Base documental verificada

Se validó en documentación de Bevy:

| Capacidad | Evidencia documental | Por qué importa |
|---|---|---|
| App headless | `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(...)` | Permite correr el controller sin UI. |
| Ticks fijos | `Time::<Fixed>::from_seconds(...)` y `FixedUpdate` | Ideal para simulación térmica y control periódico. |
| Eventos ECS | `add_event::<T>()` | Permite desacoplar ingestión, validación y emisión. |
| Testabilidad | `App::update()` y stepping manual | Permite tests deterministas sin hardware real. |

## Problema que resuelve este cambio

Necesitamos un `rpi-controller` que:

- pueda detectar hornos simulados
- mantenga estado interno confiable
- procese comandos del PC
- simule temperatura
- aplique control térmico con histéresis
- emita eventos del protocolo compartido
- sea fácilmente testeable sin GPIO real

Con ECS headless, cada responsabilidad se vuelve un sistema explícito en lugar de lógica mezclada en un loop manual.

## Objetivo del producto

Construir un `rpi-controller` que funcione como **motor de dominio de hornos** sobre Bevy ECS, sin UI, con simulación inicial y listo para recibir hardware real más adelante.

## Qué queremos lograr en v1

- Crear hornos simulados manualmente al inicio.
- Emitir `OvenDetected` por cada horno creado.
- Recibir comandos del protocolo compartido.
- Validar comandos y responder con `CommandAccepted` o `CommandRejected`.
- Simular temperatura con ticks fijos.
- Aplicar control térmico con histéresis de 5 °C.
- Emitir `OvenStatusUpdated` periódicamente.
- Emitir `FaultRaised` cuando se supera un límite de seguridad.
- Ejecutar todo esto sin UI y sin hardware real.

## No-goals de v1

- No usar GPIO real.
- No usar `serialport` real.
- No usar Tokio como runtime principal del dominio.
- No implementar PID.
- No persistir estado a disco.
- No permitir múltiples clientes PC simultáneos.
- No agregar UI local en Raspberry.

## Modelo conceptual con Bevy ECS

### Entidades

- `Oven`

### Componentes sugeridos

| Componente | Qué representa |
|---|---|
| `OvenId` | Identidad estable del horno. |
| `SensorRef` | Sensor asociado. |
| `OutputRef` | Salida asociada. |
| `CurrentTemperature` | Temperatura actual simulada. |
| `TargetTemperature` | Temperatura objetivo. |
| `MaxTemperature` | Límite de seguridad. |
| `Enabled` | Si el horno puede operar. |
| `Heating` | Si actualmente está aplicando calor. |
| `OvenStatus` | Estado resumido (`Disabled`, `Idle`, `Heating`, `Faulted`, `EmergencyStopped`). |

### Resources sugeridos

| Resource | Qué representa |
|---|---|
| `EmergencyStopActive` | Si el sistema está en parada de emergencia. |
| `SimulationConfig` | Drift, ruido, temperatura ambiente, histéresis. |
| `InboundProtocolQueue` | Mensajes entrantes a procesar. |
| `OutboundProtocolQueue` | Mensajes salientes a publicar. |
| `ControllerConfig` | Configuración general del proceso. |

### Eventos internos sugeridos

| Evento interno | Propósito |
|---|---|
| `CommandReceivedEvent` | Desacoplar ingreso de comandos del procesamiento. |
| `OvenDetectedEvent` | Representar detección simulada de horno. |
| `FaultDetectedEvent` | Desacoplar detección de fallas de la publicación. |
| `StatusPublishEvent` | Pedir publicación de estado. |

## Schedules propuestos

### `Startup`

- Crear N hornos simulados.
- Emitir `OvenDetected` inicial.

### `Update`

- Ingerir comandos mock.
- Validar comandos.
- Aplicar cambios de estado.
- Encolar mensajes salientes del protocolo.

### `FixedUpdate`

- Simular temperatura.
- Aplicar histéresis.
- Detectar fallas.
- Publicar estado periódico.

## Flujo de alto nivel

### Detección inicial

1. El proceso arranca con `--simulate N`.
2. El sistema `spawn_simulated_ovens_system` crea N entidades `Oven`.
3. Se emite `OvenDetected` por cada horno.
4. Los eventos salen por la cola de protocolo.

### Cambio de temperatura

1. Entra `SetTargetTemperature` en la cola entrante.
2. Un sistema lo valida.
3. Si es válido, actualiza `TargetTemperature`.
4. Se emite `CommandAccepted`.
5. En el próximo `FixedUpdate`, el sistema de control decide `Heating`.
6. Se publica `OvenStatusUpdated`.

### Parada de emergencia

1. Entra `EmergencyStop`.
2. Se activa `EmergencyStopActive`.
3. Todos los hornos pasan a `EmergencyStopped` y `Heating = false`.
4. Se emite `CommandAccepted`.
5. Se publica `OvenStatusUpdated` por cada horno afectado.

## Control térmico

El sistema usará la decisión ya aprobada en ADR 002:

- si `current < target - 5` → `Heating = true`
- si `current >= target` → `Heating = false`
- entre ambos valores → mantener estado anterior

Esto protege la salida de control de oscilaciones rápidas y mantiene el modelo cerca del comportamiento industrial real.

## Testing strategy

La documentación de Bevy permite usar `App::update()` en tests, así que la estrategia de v1 será:

- crear `App` headless
- insertar resources
- spawnear entidades simuladas
- enviar eventos o poblar colas entrantes
- correr `app.update()` / `FixedUpdate`
- verificar componentes y mensajes salientes

## Criterios de aceptación

- Existe un `rpi-controller` headless basado en Bevy ECS.
- El proceso puede crear hornos simulados al inicio.
- El proceso emite `OvenDetected` usando el crate `protocol`.
- Los comandos válidos actualizan el estado ECS.
- Los comandos inválidos producen `CommandRejected` con `FaultCode` correcto.
- La simulación térmica corre en ticks fijos.
- La histéresis de 5 °C está implementada y testeada.
- `EmergencyStop` detiene todos los hornos.
- El sistema se puede testear sin hardware real.

## Fuera de alcance inmediato pero compatibles con este diseño

- `serialport` real como plugin/adaptador posterior
- GPIO real con `rppal`
- sensores reales
- PID como sistema adicional
- conexión con `pc-app`

## Próximo paso recomendado

Reescribir el diseño y las tasks de `rpi-controller` alrededor de Bevy ECS headless, descartando definitivamente el enfoque Tokio-first para el dominio.
