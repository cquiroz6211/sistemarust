# PRD — Sistema de Control de Hornos con Rust, Bevy y Raspberry Pi

Este PRD define el alcance inicial del sistema de control de hornos: una aplicación en PC que opera como centro visual y una Raspberry Pi que actúa como controlador físico confiable. El sistema se comunicará mediante eventos, manteniendo una separación clara entre intención, control y telemetría.

## Problem Statement

Necesitamos controlar múltiples hornos desde una aplicación en PC, mostrando temperatura actual, temperatura deseada y estado operativo de cada horno. La Raspberry Pi debe encargarse del control físico mediante GPIO, sensores y lógica de seguridad, sin depender de que el PC permanezca conectado para mantener el control del sistema.

El problema principal no es solo “mostrar hornos” o “mandar JSON”. El problema real es diseñar un flujo de datos coherente donde cada parte del sistema tenga una responsabilidad clara y donde los eventos definan qué se envía, qué se recibe y qué debe hacer cada componente.

## Solution

Construiremos un sistema dividido en tres partes principales:

1. PC / aplicación Bevy: centro visual y operativo.
2. Protocolo de eventos: idioma común entre PC y Raspberry.
3. Raspberry Pi / controlador Rust: autoridad del mundo físico.

La aplicación del PC enviará intenciones, como registrar un horno o cambiar su temperatura deseada. La Raspberry validará esas intenciones, actualizará su estado interno, ejecutará el control físico y reportará estados o fallas al PC.

La arquitectura se basará en ECS, separando entidades, componentes y sistemas para mantener el flujo de datos explícito, ordenado y fácil de razonar.

## Product Goals

- Permitir registrar y controlar múltiples hornos.
- Mantener el control físico dentro de la Raspberry Pi.
- Comunicar PC y Raspberry mediante eventos claros y versionables.
- Evitar que la UI del PC controle directamente GPIO o decisiones críticas.
- Empezar con un protocolo mínimo, vital y fácil de extender.
- Preparar la base para pruebas sin depender inmediatamente del hardware real.

## Non-Goals Iniciales

- No diseñar una UI final completa.
- No implementar control GPIO real en la primera etapa.
- No definir alertas de negocio todavía.
- No agregar eventos especulativos sin significado claro.
- No usar un hilo por horno como modelo principal de dominio.
- No mezclar protocolo, UI y control físico en una sola capa.

## System Responsibilities

| Parte | Responsabilidad |
|---|---|
| PC / Bevy | Mostrar hornos, enviar comandos, recibir estado, visualizar operación. |
| Protocolo | Definir eventos compartidos, nombres, payloads conceptuales y reglas de comunicación. |
| Raspberry Pi | Validar comandos, mantener estado real, leer sensores, controlar GPIO, aplicar seguridad y reportar estado. |
| Hornos físicos | Recibir señales eléctricas y devolver mediciones mediante sensores. |

## Event Model

El sistema distinguirá entre comandos y eventos.

| Tipo | Dirección | Significado |
|---|---|---|
| Comando | PC hacia Raspberry | Solicitud o intención de hacer algo. |
| Evento | Raspberry hacia PC | Hecho ocurrido, estado reportado o falla detectada. |

Regla central:

El PC declara intención. La Raspberry valida, ejecuta y reporta hechos.

## Minimal Event Protocol

### PC hacia Raspberry

| Evento | Propósito |
|---|---|
| SetTargetTemperature | Cambiar la temperatura deseada de un horno. |
| SetOvenEnabled | Habilitar o deshabilitar lógicamente un horno. |
| RequestStatus | Pedir el estado actual de uno o varios hornos. |
| EmergencyStop | Detener inmediatamente la operación por seguridad. |

### Raspberry hacia PC

| Evento | Propósito |
|---|---|
| OvenDetected | Informar que se detectó un horno nuevo en el hardware. |
| CommandAccepted | Confirmar que un comando fue recibido, validado y aceptado. |
| CommandRejected | Informar que un comando fue rechazado y por qué. |
| OvenStatusUpdated | Reportar el estado operativo de un horno. |
| FaultRaised | Informar una falla técnica o de seguridad que requiere atención. |

## Event Envelope Requirements

Cada mensaje del protocolo debe incluir metadatos comunes para que el sistema pueda auditar, correlacionar y evolucionar los eventos.

| Campo | Propósito |
|---|---|
| event_id | Identificar de forma única el mensaje. |
| type | Indicar el tipo de comando o evento. |
| source | Identificar quién envía el mensaje. |
| target | Identificar el destino esperado. |
| timestamp | Registrar cuándo se generó el mensaje. |
| correlation_id | Relacionar una respuesta con el comando que la originó. |
| version | Permitir evolucionar el protocolo sin romper compatibilidad. |
| payload | Contener los datos específicos del evento. |

## User Stories

1. Como operador, quiero registrar un horno desde el PC, para poder empezar a monitorearlo y controlarlo.
2. Como operador, quiero configurar la temperatura deseada de un horno, para controlar su proceso térmico.
3. Como operador, quiero habilitar o deshabilitar un horno, para decidir si puede operar o debe quedar inactivo.
4. Como operador, quiero ver la temperatura actual de cada horno, para saber su estado real.
5. Como operador, quiero ver la temperatura deseada de cada horno, para confirmar el objetivo configurado.
6. Como operador, quiero ver si un horno está calentando o no, para entender qué está haciendo el sistema.
7. Como operador, quiero pedir el estado actual de los hornos, para sincronizar la pantalla con la Raspberry.
8. Como operador, quiero ejecutar una parada de emergencia, para detener el sistema ante una situación riesgosa.
9. Como Raspberry, quiero validar cada comando recibido, para evitar configuraciones inválidas o peligrosas.
10. Como Raspberry, quiero rechazar comandos inválidos con una razón clara, para que el PC pueda informar correctamente al operador.
11. Como Raspberry, quiero reportar cambios de estado, para que el PC no tenga que adivinar qué ocurrió.
12. Como Raspberry, quiero mantener el control aunque el PC se desconecte, para que el sistema físico siga siendo seguro.
13. Como desarrollador, quiero que PC y Raspberry compartan el mismo protocolo, para evitar inconsistencias de comunicación.
14. Como desarrollador, quiero separar eventos de control y eventos de estado, para que el flujo sea fácil de razonar.
15. Como desarrollador, quiero evitar eventos prematuros, para no construir una arquitectura inflada antes de entender el dominio.

## Implementation Decisions

- El protocolo de eventos será la primera pieza a definir.
- El PC no controlará directamente GPIO ni tomará decisiones físicas críticas.
- La Raspberry será la autoridad del estado físico y de seguridad.
- El sistema usará ECS para modelar hornos como entidades con componentes de estado.
- Los sistemas procesarán lectura, control, estado y comunicación.
- El protocolo inicial será pequeño y vital.
- El evento genérico de alerta queda fuera del inicio porque todavía no tiene una definición de dominio clara.
- Las fallas técnicas se representarán como FaultRaised, no como alertas genéricas.
- OvenStatusUpdated cubrirá inicialmente temperatura, estado de habilitación, objetivo y estado de calentamiento.
- Heartbeat queda fuera del inicio salvo que la salud de conexión pase a ser requisito del primer hito.
- La simulación será útil antes del GPIO real, pero no bloquea la definición del protocolo.

## Testing Decisions

- Las primeras pruebas deben validar comportamiento externo del protocolo, no detalles internos.
- El protocolo debe poder verificar que un mensaje válido se interpreta correctamente.
- El controlador de Raspberry debe poder probarse inicialmente sin hardware real.
- Las pruebas deben confirmar que comandos inválidos son rechazados con razones claras.
- Las pruebas deben cubrir el flujo mínimo: registrar horno, cambiar temperatura, actualizar estado y reportarlo al PC.
- El comportamiento de parada de emergencia debe tener pruebas prioritarias.
- La UI del PC debe probarse después de estabilizar el contrato de eventos.

## Out of Scope

- Alertas de negocio no definidas.
- UI visual final.
- Persistencia histórica avanzada.
- Dashboard avanzado de métricas.
- Control remoto desde internet.
- Autenticación y autorización.
- Configuración dinámica compleja de sensores.
- Tuning avanzado de PID.
- Integración con GPIO real en la primera etapa documental.
- Protocolos alternativos múltiples al mismo tiempo.

## Acceptance Criteria

- Existe una definición clara de qué hace el PC y qué hace la Raspberry.
- Existe una lista mínima de eventos iniciales.
- Cada evento tiene una responsabilidad clara.
- El protocolo diferencia comandos de eventos.
- El sistema evita alertas genéricas prematuras.
- La Raspberry queda definida como autoridad de control físico.
- El PC queda definido como centro visual y emisor de intención.
- El siguiente paso técnico puede comenzar por el módulo de protocolo compartido.

## Further Notes

Este PRD prioriza comprensión y dirección arquitectónica antes que implementación. La decisión importante es no empezar por pantallas ni GPIO. Primero se define el idioma del sistema. Después se conectan las partes.

La primera victoria del proyecto será demostrar el flujo completo de intención y estado: el PC solicita un cambio, la Raspberry lo valida, actualiza su estado interno y reporta el nuevo estado al PC.
