# Panorama General — Aplicación de Control de Hornos con ECS

Este documento explica qué vamos a construir, por qué lo vamos a dividir en partes y cuál es el objetivo real de usar ECS en el sistema. La meta es tener una visión completa antes de avanzar con implementación.

## Decisión central

Vamos a construir una aplicación distribuida para controlar múltiples hornos mediante una PC y una Raspberry Pi.

| Parte | Rol principal |
|---|---|
| PC / Bevy | Centro visual de operación. |
| Protocolo | Idioma común entre PC y Raspberry. |
| Raspberry Pi | Controlador físico confiable. |
| ECS | Modelo para organizar estado y comportamiento. |

Regla base:

> El PC pide. La Raspberry decide y ejecuta. ECS organiza el estado para que el sistema no se vuelva una bola de cables.

## Qué queremos lograr

Queremos que un operador pueda usar una aplicación en PC para:

- Registrar hornos.
- Ver el estado de cada horno.
- Cambiar la temperatura deseada.
- Habilitar o deshabilitar hornos.
- Pedir el estado actual del sistema.
- Ejecutar una parada de emergencia.

Pero el control físico real debe vivir en la Raspberry Pi.

Eso significa que la Raspberry debe poder:

- Recibir comandos.
- Validarlos.
- Mantener el estado real de los hornos.
- Leer sensores.
- Controlar salidas GPIO.
- Reportar estado al PC.
- Entrar en estado seguro si algo falla.

## Por qué no empezamos por la UI ni por GPIO

Empezar por pantallas o por GPIO sería correr antes de caminar.

Primero necesitamos definir el idioma del sistema:

1. Qué puede pedir el PC.
2. Qué puede responder la Raspberry.
3. Qué significa cada evento.
4. Qué estado debe mantenerse.
5. Qué parte es responsable de cada decisión.

Sin eso, la UI queda linda pero sin fundamento, y el GPIO queda conectado a una lógica improvisada.

## División de la aplicación

### PC / Bevy

La aplicación de PC es el tablero de control.

Responsabilidades:

- Mostrar hornos registrados.
- Mostrar temperatura actual y deseada.
- Mostrar estado operativo.
- Enviar comandos a la Raspberry.
- Recibir eventos desde la Raspberry.
- Actualizar la pantalla según el estado recibido.

La PC no debe controlar directamente el hardware.

### Protocolo

El protocolo es el contrato compartido.

Responsabilidades:

- Definir los comandos disponibles.
- Definir los eventos de respuesta.
- Definir la forma común de los mensajes.
- Permitir que PC y Raspberry hablen el mismo idioma.

Este módulo debe ser compartido por ambos lados.

### Raspberry Pi

La Raspberry es la autoridad del mundo físico.

Responsabilidades:

- Recibir comandos del PC.
- Validar si un comando puede aplicarse.
- Mantener el estado real de los hornos.
- Leer sensores de temperatura.
- Aplicar lógica de control.
- Controlar GPIO, relays o salidas PWM.
- Reportar estado y fallas técnicas.

Si el PC se desconecta, la Raspberry no debe quedar inútil. Tiene que conservar el control seguro del sistema.

## Objetivo de usar ECS

ECS significa Entity Component System.

No lo usamos porque suena moderno. Lo usamos porque este problema encaja naturalmente con ese modelo.

### Entity

Una entidad representa algo identificable dentro del sistema.

En nuestro caso, principalmente:

- Un horno.

La entidad no contiene lógica. Es solo identidad.

### Component

Un componente representa un dato asociado a una entidad.

Para un horno, algunos componentes conceptuales pueden ser:

| Componente | Qué representa |
|---|---|
| OvenId | Identidad estable del horno. |
| CurrentTemperature | Temperatura medida actual. |
| TargetTemperature | Temperatura deseada. |
| EnabledState | Si el horno está habilitado. |
| HeatingState | Si se está aplicando calor. |
| OutputLevel | Nivel de salida actual. |
| SensorBinding | Sensor asignado. |
| OutputBinding | Salida física asignada. |
| SafetyLimits | Límites técnicos permitidos. |
| FaultState | Fallas activas. |

La idea importante es esta:

> Los datos viven en componentes pequeños, no en una clase gigante llamada Horno.

### System

Un sistema es una función o proceso que opera sobre componentes.

Ejemplos conceptuales:

| Sistema | Responsabilidad |
|---|---|
| CommandReceiverSystem | Recibir comandos externos. |
| CommandValidationSystem | Validar comandos antes de aplicarlos. |
| OvenRegistrationSystem | Registrar hornos nuevos. |
| TargetTemperatureSystem | Actualizar temperatura deseada. |
| SensorReadSystem | Leer temperatura desde sensores. |
| ControlSystem | Decidir si aplicar calor o no. |
| GpioOutputSystem | Escribir salidas físicas. |
| StatusPublisherSystem | Publicar estado hacia el PC. |
| FaultDetectionSystem | Detectar condiciones anormales. |
| EmergencyStopSystem | Llevar el sistema a estado seguro. |

Cada sistema debe tener una responsabilidad clara.

## Flujo de datos de alto nivel

### Cambiar temperatura deseada

1. El operador cambia la temperatura en el PC.
2. El PC envía un comando al protocolo.
3. La Raspberry recibe el comando.
4. La Raspberry valida el comando.
5. Si es válido, actualiza el estado ECS.
6. Los sistemas de control actúan sobre ese estado.
7. La Raspberry reporta el nuevo estado al PC.
8. El PC actualiza la pantalla.

### Registrar un horno

1. El operador registra un horno en el PC.
2. El PC envía la intención a la Raspberry.
3. La Raspberry valida identificador, sensor, salida y límites.
4. Si todo es correcto, crea la entidad del horno.
5. La Raspberry reporta el estado inicial.
6. El PC muestra el nuevo horno.

### Parada de emergencia

1. El operador activa parada de emergencia.
2. El PC envía el comando.
3. La Raspberry desactiva salidas físicas.
4. La Raspberry marca el sistema en estado seguro.
5. La Raspberry reporta el nuevo estado al PC.

## Eventos mínimos acordados

### PC hacia Raspberry

| Comando | Propósito |
|---|---|
| RegisterOven | Registrar un horno. |
| SetTargetTemperature | Cambiar temperatura deseada. |
| SetOvenEnabled | Habilitar o deshabilitar un horno. |
| RequestStatus | Solicitar estado actual. |
| EmergencyStop | Detener operación por seguridad. |

### Raspberry hacia PC

| Evento | Propósito |
|---|---|
| CommandAccepted | Confirmar comando aceptado. |
| CommandRejected | Informar comando rechazado. |
| OvenStatusUpdated | Reportar estado de horno. |
| FaultRaised | Reportar falla técnica o de seguridad. |

## Qué NO vamos a construir todavía

| Elemento | Motivo |
|---|---|
| UI final completa | Antes necesitamos protocolo y flujo confiable. |
| GPIO real completo | Primero validamos la arquitectura. |
| Alertas genéricas | Todavía no sabemos qué significa una alerta de negocio. |
| Historial avanzado | No es necesario para el primer flujo funcional. |
| Control remoto desde internet | Agrega seguridad y complejidad fuera del alcance inicial. |
| Hilo por horno como arquitectura central | ECS debe modelar hornos como entidades, no como threads. |

## Camino de desarrollo — estado real (Mayo 2026)

### Lo completado

| Paso | Estado | Change SDD |
|---|---|---|
| 1. Documentación de arquitectura y eventos | ✅ Completado | `docs/events.md`, ADRs, panorama |
| 2. Workspace Rust mínimo | ✅ Completado | Skeleton inicial |
| 3. Módulo compartido de protocolo | ✅ Completado | `protocol` — 85 tests |
| 4. Serialización y lectura de eventos | ✅ Completado | Serde roundtrip, discriminante JSON |
| 5. Raspberry headless con Bevy ECS + simulación | ✅ Completado | `rpi-controller-bevy-headless` — 59 tests |
| 6. PC-app como read model + command author | ✅ Completado | `pc-app` — 23 tests |
| 7. ECS en el controlador | ✅ Completado | Replace Tokio-first por Bevy ECS |
| 8. Simulación de sensores y salidas | ✅ Completado | Drift térmico, histéresis 5°C, noise |

### Lo pendiente

| Paso | Prioridad | Próximo cambio |
|---|---|---|
| 9. Transporte PC ↔ Raspberry | 🔜 Próximo | `pc-rpi-transport-link` |
| 10. UI Bevy en PC | Siguiente | `pc-app-ui` |
| 11. Integración GPIO real | Futuro | Necesita transporte primero |
| 12. Demo observable para estudiantes | En paralelo | Demo runner o logging mode |

## Primera victoria técnica

La primera victoria no es una pantalla linda ni un relay prendiendo.

La primera victoria es demostrar este flujo completo:

1. PC pide cambiar temperatura.
2. Raspberry acepta el comando.
3. Raspberry actualiza el estado interno.
4. Raspberry reporta el estado actualizado.
5. PC muestra el cambio.

Si eso funciona, la arquitectura está viva.

## Nota sobre la primera victoria técnica

La primera victoria planteada originalmente era:

> 1. PC pide cambiar temperatura.
> 2. Raspberry acepta el comando.
> 3. Raspberry actualiza el estado interno.
> 4. Raspberry reporta el estado actualizado.
> 5. PC muestra el cambio.

**Esto todavía no es posible.** Cada lado funciona y se testea por separado, pero **falta el transporte** que conecte PC y Raspberry. Ese es el próximo cambio planeado (`pc-rpi-transport-link`).

## Criterios de aceptación de este panorama

- Se entiende qué hace el PC.
- Se entiende qué hace la Raspberry.
- Se entiende para qué existe el protocolo.
- Se entiende por qué usamos ECS.
- Se entiende por qué no arrancamos por UI o GPIO.
- Se entiende cuál es el primer objetivo técnico.
- Se entiende QUÉ está implementado y QUÉ falta.

## Documentos relacionados

- `docs/prd-sistema-control-hornos.md`: PRD del sistema.
- `docs/prd-rpi-controller-bevy-headless.md`: PRD del controller Bevy headless.
- `docs/events.md`: contrato completo de eventos.
- `docs/diagrama-clases.md`: diagrama ECS actualizado con componentes reales.
- `docs/diagrama-flujo-protocolo.md`: flujo del protocolo con estado de implementación.
- `docs/adr/001-protocolo-eventos-rust-enum.md`: decisión de usar Enum con serde.
- `docs/adr/002-hysteresis-control-termico.md`: decisión de histéresis 5°C.
- `docs/estado-actual.md`: trazabilidad consolidada del proyecto.
