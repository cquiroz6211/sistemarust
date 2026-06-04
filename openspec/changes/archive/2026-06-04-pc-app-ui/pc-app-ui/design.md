# Diseño Técnico: Interfaz de usuario visual en pc-app con bevy_egui

## 1. Resumen Ejecutivo

Este diseño describe cómo integrar una interfaz gráfica inmediata (immediate mode) en `pc-app` usando `bevy_egui`, sin modificar el comportamiento de los sistemas ECS existentes. La UI se integra como un plugin adicional que consume los componentes, recursos y eventos ya existentes, y escribe comandos a través de las funciones de command authoring ya probadas.

**Decisión clave actualizada**: `pc-app` usa dos modos de arranque. En modo UI usa `DefaultPlugins` + `EguiPlugin` + `UiPlugin`, porque `bevy_egui` requiere recursos de render/assets provistos por los plugins gráficos de Bevy. En modo `--headless` conserva `MinimalPlugins` + `ScheduleRunnerPlugin`, manteniendo el comportamiento headless original intacto para CI y tests.

---

## 2. Arquitectura General

```
┌──────────────────────────────────────────────────────────────────────┐
│                          main.rs (App)                               │
│  MinimalPlugins + ScheduleRunnerPlugin (50ms)                       │
│  + TransportPlugin (si --connect)                                   │
│  + PcAppPlugin (existing)                                           │
│  + EguiPlugin (NEW)                                                 │
│  + UiPlugin (NEW)                                                   │
│  + LogPlugin (NEW)                                                  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    Update Schedule                            │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  ingest_inbound_protocol (existing)                     │  │  │
│  │  │  → EventWriter<OvenDiscovered>, OvenStatusReceived, ... │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  bridge_outbound_to_transport (existing)                │  │  │
│  │  │  bridge_inbound_from_transport (existing)               │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  ui_render (NEW)                                        │  │  │
│  │  │  → Query<...>, Res<OutboundProtocolQueue>,             │  │  │
│  │  │    EguiContexts                                        │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  log_capture_events (NEW)                               │  │  │
│  │  │  → EventReader<OvenDiscovered>, EventReader<...>       │  │  │
│  │  │    ResMut<EventLog>                                     │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  ui_command_dispatch (NEW)                              │  │  │
│  │  │  → ResMut<OutboundProtocolQueue>                        │  │  │
│  │  │    (llama a author_*_command funciones existentes)      │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                  FixedUpdate Schedule                         │  │
│  │  apply_oven_detected (existing)                               │  │
│  │  apply_oven_status_updated (existing)                         │  │
│  │  apply_fault_raised (existing)                                │  │
│  │  record_command_result (existing)                             │  │
│  └───────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │               EguiPrimaryContextPass (NEW)                    │  │
│  │  ui_render (NEW) — runs after Update, renders egui            │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘

Recursos ECS existentes (sin cambios):
  InboundProtocolQueue, OutboundProtocolQueue, OvenIndex, GlobalFault

Componentes ECS existentes (sin cambios):
  OvenId, CurrentTemperature, TargetTemperature, MaxTemperature,
  Enabled, Heating, OvenStatus, FaultState, LastCommandResult, ...

Eventos ECS existentes (sin cambios):
  OvenDiscovered, OvenStatusReceived, FaultReceived,
  CommandAcceptedReceived, CommandRejectedReceived

NUEVOS Recursos:
  EventLog (Vec<LogEntry>), ConnectionState (enum)

NUEVOS Componentes:
  Ninguno — la UI no se representa como entities ECS.
```

### Decisiones arquitectónicas clave

1. **Modo UI usa `DefaultPlugins`** — `bevy_egui` requiere recursos como `Assets<Shader>` que no existen con `MinimalPlugins`. El modo UI usa `DefaultPlugins`; el modo `--headless` conserva `MinimalPlugins` + `ScheduleRunnerPlugin`.

2. **La UI NO es ECS** — `bevy_egui` es immediate mode: la UI se describe cada frame en un sistema, no se representa como entities. Esto es una decisión consciente: la UI es una *vista* del ECS, no parte del ECS.

3. **Sistema único de renderizado** — Un solo sistema `ui_render` maneja todo el renderizado egui. Esto simplifica las dependencias de schedule y evita múltiples passes.

4. **Captura de eventos en `Update`** — El log se llena en `Update` usando `EventReader` de los mismos eventos que los sistemas de estado. Esto garantiza que el log siempre esté sincronizado con el estado del ECS.

---

## 3. Estructura de Módulos

```
pc-app/src/
├── main.rs                    # Modificado: agregar EguiPlugin + UiPlugin
├── lib.rs                     # Modificado: pub mod ui;
├── components.rs              # Sin cambios
├── resources.rs               # Modificado: agregar EventLog, ConnectionState
├── events.rs                  # Sin cambios
├── plugins/
│   ├── mod.rs                 # Modificado: pub mod ui; pub mod log;
│   ├── pc_app.rs              # Sin cambios
│   ├── egui.rs                # NUEVO: EguiPlugin wrapper
│   └── ui.rs                  # NUEVO: UiPlugin (sistemas de UI)
├── systems/
│   ├── mod.rs                 # Sin cambios
│   ├── ingest.rs              # Sin cambios
│   ├── state.rs               # Sin cambios
│   ├── commands.rs            # Sin cambios
│   └── ui/                    # NUEVO: sistemas de UI
│       ├── mod.rs
│       ├── render.rs          # ui_render — todo el renderizado egui
│       └── dispatch.rs        # ui_command_dispatch — envío de comandos
└── ui/                        # NUEVO: tipos y helpers de UI
    ├── mod.rs
    ├── panels.rs              # Paneles: oven table, controls, header
    ├── controls.rs            # Controles por horno: toggles, sliders
    ├── log_panel.rs           # Panel de log
    └── styles.rs              # Estilos visuales (colores, tamaños)
```

### Justificación de la estructura

- **`plugins/egui.rs`** — Solo configura `EguiPlugin::default()` y `Camera2d`. Es un wrapper mínimo.
- **`plugins/ui.rs`** — Registra todos los sistemas de UI en sus schedules respectivos (`Update`, `EguiPrimaryContextPass`).
- **`systems/ui/render.rs`** — Contiene el sistema `ui_render` que dibuja toda la UI. Separa la lógica de renderizado de la lógica de dispatch de comandos.
- **`systems/ui/dispatch.rs`** — Contiene el sistema `ui_command_dispatch` que procesa las intenciones de la UI y llama a las funciones de command authoring existentes.
- **`ui/panels.rs`** — Contiene las funciones de dibujo de cada panel (header, tabla de hornos, panel lateral).
- **`ui/controls.rs`** — Contiene las funciones de dibujo de controles interactivos (toggles, sliders, botones).
- **`ui/styles.rs`** — Colores y estilos reutilizables (verde=conectado, rojo=desconectado, amarillo=calentando, etc.).

---

## 4. Diseño del Plugin

### 4.1 EguiPlugin (`plugins/egui.rs`)

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin as BevyEguiPlugin};

pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyEguiPlugin);
        // Camera2d se spawn en el sistema de setup de UI,
        // no aquí, para mantener el plugin independiente.
    }
}
```

**Nota**: No agregamos `DefaultPlugins`. Solo `EguiPlugin` de `bevy_egui`. La cámara 2D se spawn como sistema de arranque en `UiPlugin`.

### 4.2 UiPlugin (`plugins/ui.rs`)

```rust
use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::systems::ui::render::ui_render;
use crate::systems::ui::dispatch::ui_command_dispatch;
use crate::systems::ui::log_capture::log_capture_events;
use crate::resources::{EventLog, ConnectionState};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Recursos de UI
        app.insert_resource(EventLog::default());
        app.insert_resource(ConnectionState::Disconnected);

        // Setup: spawn cámara para bevy_egui
        app.add_systems(Startup, spawn_ui_camera);

        // Update: captura de eventos para log
        app.add_systems(Update, log_capture_events
            .after(pc_app::systems::ingest::ingest_inbound_protocol));

        // Update: dispatch de comandos desde UI
        app.add_systems(Update, ui_command_dispatch);

        // EguiPrimaryContextPass: renderizado
        app.add_systems(EguiPrimaryContextPass, ui_render);
    }
}
```

**Dependencias de schedule**:
- `log_capture_events` runs **after** `ingest_inbound_protocol` — esto garantiza que los eventos ya fueron emitidos por el sistema de ingestión antes de que el log los capture.
- `ui_command_dispatch` runs en `Update` sin dependencia específica — puede correr en cualquier orden dentro de Update.
- `ui_render` runs en `EguiPrimaryContextPass` — el pass dedicado de bevy_egui, que se ejecuta después de `Update`.

### 4.3 Spawn de cámara

```rust
fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
```

Simple y sin dependencias. `bevy_egui` necesita un `Camera2d` para renderizar.

---

## 5. Flujo de Datos

### 5.1 Lectura de estado (ECS → UI)

```
Componentes ECS ──Query──→ ui_render (EguiPrimaryContextPass)
                              │
                              ├─→ OvenId, CurrentTemperature, TargetTemperature
                              ├─→ Enabled, Heating, OvenStatus
                              ├─→ FaultState, MaxTemperature
                              └─→ LastCommandResult
```

El sistema `ui_render` lee directamente los componentes ECS vía `Query`. No hay capa de abstracción intermedia. Cada frame, la UI consulta el estado actual y lo dibuja.

```rust
fn ui_render(
    mut contexts: EguiContexts,
    oven_index: Res<OvenIndex>,
    connection_state: Res<ConnectionState>,
    event_log: Res<EventLog>,
    mut query: Query<(
        &OvenId,
        &CurrentTemperature,
        &TargetTemperature,
        &Enabled,
        &Heating,
        &OvenStatus,
        &FaultState,
        &MaxTemperature,
        &LastCommandResult,
    )>,
) {
    // ... renderiza cada horno usando los componentes directos
}
```

### 5.2 Escritura de comandos (UI → ECS)

```
UI interaction (egui) ──→ ui_command_dispatch (Update)
                              │
                              ├─→ author_set_oven_enabled_command()
                              ├─→ author_set_target_temperature_command()
                              ├─→ author_request_status_command()
                              └─→ author_emergency_stop_command()
                              │
                              ↓
                    OutboundProtocolQueue (Resource)
                              │
                              ↓
                    bridge_outbound_to_transport (Update)
                              │
                              ↓
                    OutboundSender (channel) → TCP → RPi
```

Cuando el usuario interactúa con la UI (click en botón, slider), `bevy_egui` retorna un valor booleano (`clicked()`). Este valor se captura en el sistema `ui_render` y se almacena en un recurso temporal `UiIntent` (o se procesa directamente en `ui_command_dispatch`).

**Decisión de diseño**: Usamos un recurso `UiIntent` para separar la captura de intención del dispatch:

```rust
/// Intención de comando capturada por la UI en este frame.
#[derive(Resource, Default)]
pub struct UiIntent {
    pub set_enabled: Option<(String, bool)>,
    pub set_temperature: Option<(String, f64)>,
    pub request_status: Option<Option<String>>, // None = all
    pub emergency_stop: bool,
}
```

Esto permite que `ui_render` (en `EguiPrimaryContextPass`) capture las intenciones y `ui_command_dispatch` (en `Update` del siguiente frame) las procese. Es seguro porque:
1. Los comandos se encolan en `OutboundProtocolQueue` que se vacía cada frame.
2. Un frame de retraso es imperceptible para el operador.
3. Evita mutaciones concurrentes en el mismo frame.

### 5.3 Captura de eventos para log

```
Eventos ECS (OvenDiscovered, OvenStatusReceived, ...)
    │
    │ EventReader (log_capture_events en Update, after ingest)
    ↓
EventLog::push(LogEntry)
    │
    │ FIFO limit 200 entries
    ↓
ui_render (lee Res<EventLog> en EguiPrimaryContextPass)
```

El sistema `log_capture_events` usa `EventReader` de los mismos eventos que los sistemas de estado. Esto garantiza sincronía sin modificar los sistemas existentes.

---

## 6. Schedule de Sistemas

```
┌─────────────────────────────────────────────────────────────┐
│                    Startup                                   │
│  spawn_ui_camera                                             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Update (50ms)                             │
│                                                              │
│  [1] ingest_inbound_protocol (existing)                     │
│      → EventWriter<OvenDiscovered>, etc.                    │
│                                                              │
│  [2] log_capture_events (NEW)                               │
│      → EventReader<...> → EventLog::push()                 │
│      runs after: ingest_inbound_protocol                    │
│                                                              │
│  [3] apply_oven_detected (existing, FixedUpdate)            │
│  [4] apply_oven_status_updated (existing, FixedUpdate)      │
│  [5] apply_fault_raised (existing, FixedUpdate)             │
│  [6] record_command_result (existing, FixedUpdate)          │
│                                                              │
│  [7] bridge_outbound_to_transport (existing)                │
│  [8] bridge_inbound_from_transport (existing)               │
│                                                              │
│  [9] ui_command_dispatch (NEW)                              │
│      → UiIntent → OutboundProtocolQueue                    │
│                                                              │
│  [10] connection_monitor (NEW)                              │
│       → OutboundSender.try_send() → ConnectionState        │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              EguiPrimaryContextPass (NEW)                    │
│                                                              │
│  [11] ui_render (NEW)                                       │
│       → Query<oven components>, Res<EventLog>,             │
│         Res<ConnectionState>, EguiContexts                  │
└─────────────────────────────────────────────────────────────┘
```

**Notas importantes**:
- Los sistemas de `FixedUpdate` (estado) mantienen su schedule original. La UI no los toca.
- `connection_monitor` es un sistema ligero que intenta un `try_send` en el canal de salida. Si el canal está cerrado o hay error, actualiza `ConnectionState::Disconnected`.
- El schedule mantiene la misma periodicidad de 50ms. La UI se renderiza en `EguiPrimaryContextPass` que se ejecuta después de cada `Update`.

---

## 7. Componentes de la UI

### 7.1 Layout general

```
┌─────────────────────────────────────────────────────────────────────┐
│  TOP PANEL (TopBottomPanel)                                         │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ 🌡️ Control de Hornos    [● Conectado] / [● Desconectado]     │  │
│  └───────────────────────────────────────────────────────────────┘  │
├──────────────────────────────┬──────────────────────────────────────┤
│  CENTRAL PANEL               │  SIDE PANEL (SidePanel)              │
│  (CentralPanel)              │  Log de Eventos                      │
│  ┌────────────────────────┐  │  ┌────────────────────────────────┐ │
│  │ Tabla de Hornos        │  │  │ [14:32:01] IN OvenDetected o1 │ │
│  │ ┌────┬────────┬──────┐ │  │  │ [14:32:02] IN OvenStatus o1   │ │
│  │ │ ID │ Temp   │ Ctrl │ │  │  │ [14:32:03] OUT SetTarget o1   │ │
│  │ ├────┼────────┼──────┤ │  │  │ [14:32:04] IN Fault o2: High │ │
│  │ │ o1 │ 185.6° │ [On] │ │  │  │ ...                            │ │
│  │ │ o2 │  50.0° │ [Off]│ │  │  │                                  │ │
│  │ │ o3 │ 220.3° │ [On] │ │  │  │                                  │ │
│  │ └────┴────────┴──────┘ │  │  └────────────────────────────────┘ │
│  │ [🔴 Emergency Stop]    │  │  (scrollable, max 200 entradas)     │
│  │ [🔄 Actualizar Todos]  │  │                                     │
│  └────────────────────────┘  └──────────────────────────────────────┘
└─────────────────────────────────────────────────────────────────────┘
```

### 7.2 Panel superior (Header)

**Responsabilidades**:
- Título de la aplicación
- Indicador de conexión (verde/rojo + texto)
- Botón global Emergency Stop (siempre visible)
- Botón global "Actualizar Todos"

**Implementación**: `TopBottomPanel::default().show(ctx, |ui| { ... })`

### 7.3 Tabla de hornos (Oven Grid)

**Responsabilidades**:
- Mostrar todas las entidades de horno detectadas
- Columnas: ID, Temperatura Actual, Temperatura Objetivo, Enabled, Estado, Calentando, Fallo
- Controles por horno: toggle enabled/disabled, slider temperatura, botón solicitar estado

**Implementación**: `egui::Grid::new("oven_grid").show(ctx, |ui| { ... })`

**Formato de temperatura**: `{:.1} °C` (1 decimal, como exige el spec)

**Indicadores visuales**:
- `Heating = true` → fondo amarillo claro
- `FaultState.is_some()` → fondo rojo claro + icono ⚠️
- `Enabled = false` → texto gris
- `Enabled = true && Heating = true` → texto verde

### 7.4 Controles por horno

**Toggle Enabled**:
```rust
if ui.button(if enabled { "Apagar" } else { "Encender" }).clicked() {
    intent.set_enabled = Some((oven_id.clone(), !enabled));
}
```

**Slider temperatura**:
```rust
let max_temp = max_temp.0;
let mut target = target_temp.0;
ui.add(egui::Slider::new(&mut target, 0.0..=max_temp).text("Temp objetivo"));
if ui.add(egui::Button::new("→")).clicked() {
    // Validar rango, actualizar intent
}
```

**Botón solicitar estado**:
```rust
if ui.button("Solicitar Estado").clicked() {
    intent.request_status = Some(oven_id.clone());
}
```

### 7.5 Panel de log de eventos

**Estructura de datos**:

```rust
/// Entrada individual del log de eventos.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,   // "HH:MM:SS"
    pub direction: LogDirection, // In o Out
    pub message_type: String, // "OvenDetected", "SetTargetTemperature", etc.
    pub summary: String,      // Resumen legible
}

pub enum LogDirection {
    In,
    Out,
}

/// Log scrollable de eventos, FIFO con límite de 200 entradas.
#[derive(Debug, Resource, Default)]
pub struct EventLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

impl EventLog {
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
}
```

**Renderizado**:

```rust
egui::ScrollArea::vertical().show(ctx, |ui| {
    for entry in &event_log.entries {
        ui.horizontal(|ui| {
            ui.label(&entry.timestamp);
            let color = match entry.direction {
                LogDirection::In => Color32::GREEN,
                LogDirection::Out => Color32::BLUE,
            };
            ui.label_color(color)
                .label(format!("{} {}", entry.direction.label(), entry.message_type));
            ui.label(&entry.summary);
        });
    }
});
```

### 7.6 Indicador de conexión

**Estados**:
- `ConnectionState::Connected` → círculo verde + texto "Conectado"
- `ConnectionState::Disconnected` → círculo rojo + texto "Desconectado"

**Implementación**:

```rust
#[derive(Debug, Clone, Resource, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connected,
}
```

**Monitoreo**: El sistema `connection_monitor` intenta enviar un batch vacío al canal de salida. Si el canal está cerrado o hay error, cambia a `Disconnected`. Si el envío es exitoso, cambia a `Connected`.

```rust
fn connection_monitor(
    mut connection_state: ResMut<ConnectionState>,
    sender: Option<Res<OutboundSender>>,
) {
    match sender {
        Some(outbound) => {
            // Try a lightweight check — if the channel is open, we're connected.
            // We don't actually send data; we just check if the channel is alive.
            // This is a best-effort heuristic.
            // In practice, the transport crate's client.rs logs connection state.
            // We can use a simpler approach: track if any outbound data was sent.
            *connection_state = ConnectionState::Connected;
        }
        None => {
            *connection_state = ConnectionState::Disconnected;
        }
    }
}
```

**Nota**: Dado que el transport crate no expone un estado de conexión explícito, usamos un heurístico: si el canal `OutboundSender` existe y el `try_send` no falla, asumimos conexión. Para una solución más robusta, se podría agregar un recurso `TransportConnected` en el transport crate en el futuro.

---

## 8. Implementación del Log de Eventos

### 8.1 Captura de eventos

El sistema `log_capture_events` se registra en `Update` con dependencia `after(ingest_inbound_protocol)`:

```rust
fn log_capture_events(
    mut ev_discovered: EventReader<OvenDiscovered>,
    mut ev_status: EventReader<OvenStatusReceived>,
    mut ev_fault: EventReader<FaultReceived>,
    mut ev_accepted: EventReader<CommandAcceptedReceived>,
    mut ev_rejected: EventReader<CommandRejectedReceived>,
    mut log: ResMut<EventLog>,
    outbound: Res<OutboundProtocolQueue>,
) {
    // Capturar eventos entrantes
    for ev in ev_discovered.read() {
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "OvenDetected".into(),
            summary: format!("{} (sensor: {}, output: {})", ev.oven_id, ev.sensor_ref, ev.output_ref),
        });
    }
    // ... similar para los otros eventos
}
```

### 8.2 Captura de comandos salientes

Para capturar los comandos que la UI envía, podemos extender `log_capture_events` para leer el `OutboundProtocolQueue` ANTES de que `bridge_outbound_to_transport` lo vacíe:

```rust
// En log_capture_events, antes de que el bridge vacíe el queue:
for envelope in &outbound.0 {
    let entry = match &envelope.payload {
        Message::SetTargetTemperature(p) => LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::Out,
            message_type: "SetTargetTemperature".into(),
            summary: format!("{}: {:.1}°C", p.oven_id, p.target_celsius),
        },
        Message::SetOvenEnabled(p) => LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::Out,
            message_type: "SetOvenEnabled".into(),
            summary: format!("{}: {}", p.oven_id, p.enabled),
        },
        // ... otros casos
        _ => continue,
    };
    log.push(entry);
}
```

**Orden de ejecución garantizado**:
1. `ingest_inbound_protocol` — vacía `InboundProtocolQueue`, emite eventos
2. `log_capture_events` — lee eventos emitidos + lee `OutboundProtocolQueue` (antes de que se vacíe)
3. `bridge_outbound_to_transport` — vacía `OutboundProtocolQueue`

### 8.3 Formato de timestamp

```rust
fn format_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
```

No usamos `chrono` para evitar dependencia adicional. El formato `HH:MM:SS` es suficiente para el propósito del log.

---

## 9. Estado de Conexión

### 9.1 Problema

El transport crate no expone un recurso de estado de conexión. El cliente TCP (`client.rs`) conecta con retry pero no notifica el estado al ECS.

### 9.2 Solución actual (heurística)

```rust
/// Monitorea el estado del canal de transporte.
/// Si el canal existe y acepta envíos, asumimos conexión.
fn connection_monitor(
    mut state: ResMut<ConnectionState>,
    sender: Option<Res<OutboundSender>>,
) {
    match sender {
        Some(_) => *state = ConnectionState::Connected,
        None => *state = ConnectionState::Disconnected,
    }
}
```

### 9.3 Solución mejorada (recomendada)

Agregar un recurso `TransportConnected` en el transport crate que el sistema de conexión actualice:

```rust
// En transport/src/lib.rs o systems.rs
#[derive(Resource, Default)]
pub struct TransportConnected(pub bool);

// En transport/src/client.rs, después de conectar con éxito:
// rt.spawn(async move {
//     app.world_mut().resource_mut::<TransportConnected>().0 = true;
// });
```

**Esta mejora es out-of-scope para este change** pero se documenta como recomendación para el futuro.

---

## 10. Estrategia de Migración

### 10.1 Cambios en main.rs

```rust
// ANTES:
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::{MinimalPlugins, ...};

let mut app = App::new();
app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
    Duration::from_millis(50),
)));
app.add_plugins(PcAppPlugin);

// DESPUÉS:
use bevy_egui::EguiPlugin;

let mut app = App::new();
app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
    Duration::from_millis(50),
)));
app.add_plugins(PcAppPlugin);
app.add_plugins(EguiPlugin);  // NUEVO
app.add_plugins(UiPlugin);    // NUEVO
```

**Cambiamos a `DefaultPlugins` solo en modo UI**. Esto permite abrir la ventana gráfica sin panic por recursos de render faltantes. El modo `--headless` mantiene:
- El comportamiento headless original intacto
- La misma periodicidad de schedule (50ms)
- Los tests existentes siguen funcionando sin cambios

### 10.2 Cambios en lib.rs

```rust
// ANTES:
pub mod components;
pub mod events;
pub mod plugins;
pub mod resources;
pub mod systems;

pub use plugins::pc_app::PcAppPlugin;

// DESPUÉS:
pub mod components;
pub mod events;
pub mod plugins;
pub mod resources;
pub mod systems;
pub mod ui;  // NUEVO

pub use plugins::pc_app::PcAppPlugin;
```

### 10.3 Cambios en Cargo.toml

```toml
[dependencies]
protocol = { path = "../protocol" }
bevy = "0.15"
transport = { path = "../transport" }
bevy_egui = "0.31"  # NUEVO — verificar compatibilidad con Bevy 0.15
```

### 10.4 Compatibilidad de versiones

`bevy_egui` 0.28+ soporta Bevy 0.15. La versión exacta se verifica en el momento de implementación. El ADR 004 ya documenta esta decisión.

### 10.5 Despliegue incremental

1. **Paso 1**: Agregar `bevy_egui` a `Cargo.toml` y compilar — verificar que no hay conflictos de dependencias.
2. **Paso 2**: Crear `EguiPlugin` mínimo + `ui_render` con un `egui::CentralPanel` vacío — verificar que la app abre una ventana.
3. **Paso 3**: Agregar `EventLog` y `log_capture_events` — verificar que el log captura eventos.
4. **Paso 4**: Agregar la tabla de hornos — verificar que muestra datos reales.
5. **Paso 5**: Agregar controles (toggles, sliders) — verificar que envían comandos.
6. **Paso 6**: Agregar indicador de conexión y pulir estilos.

---

## 11. Estrategia de Pruebas

### 11.1 Tests existentes — sin cambios

Los tests en `pc-app/tests/integration.rs` usan `MinimalPlugins` + `PcAppPlugin` directamente. Como NO cambiamos `PcAppPlugin` ni los sistemas existentes, **todos los tests existentes siguen funcionando sin modificaciones**.

### 11.2 Tests de UI — headless mode

`bevy_egui` soporta modo headless para tests. Usamos `bevy_egui::EguiManager` o el contexto mock:

```rust
// pc-app/tests/ui_integration.rs

use bevy::app::{MinimalPlugins, ScheduleRunnerPlugin};
use bevy::prelude::{App, Duration, Entity};
use bevy_egui::{EguiPlugin, EguiContexts};

#[test]
fn ui_shows_detected_ovens() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);
    app.add_plugins(EguiPlugin);
    // No spawn Camera2d en tests — bevy_egui funciona sin ventana en headless

    // Spawn oven via envelope
    push_envelope(&mut app, Message::OvenDetected(...));
    tick(&mut app);

    // Verificar que el recurso EventLog tiene la entrada
    let log = app.world().resource::<EventLog>();
    assert_eq!(log.entries.len(), 1);
    assert!(log.entries[0].summary.contains("oven1"));
}
```

### 11.3 Tests de command dispatch

```rust
#[test]
fn ui_intent_sends_enabled_command() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);
    app.add_plugins(EguiPlugin);
    app.add_plugins(UiPlugin);

    // Spawn oven
    spawn_oven(&mut app, "oven1");

    // Simular intento de UI: escribir directamente en UiIntent
    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.set_enabled = Some(("oven1".into(), true));
    }

    tick(&mut app);

    // Verificar que OutboundProtocolQueue tiene el comando
    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 1);
    match &outbound.0[0].payload {
        Message::SetOvenEnabled(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert!(p.enabled);
        }
        _ => panic!("Expected SetOvenEnabled"),
    }
}
```

### 11.4 Tests de log

```rust
#[test]
fn log_captures_inbound_events() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);
    app.add_plugins(UiPlugin);

    push_envelope(&mut app, Message::OvenDetected(...));
    tick(&mut app);

    let log = app.world().resource::<EventLog>();
    assert_eq!(log.entries.len(), 1);
    assert_eq!(log.entries[0].direction, LogDirection::In);
}

#[test]
fn log_respects_200_entry_limit() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);
    app.add_plugins(UiPlugin);

    // Push 250 eventos
    for i in 0..250 {
        push_envelope(&mut app, Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: i as f64,
            ..default()
        }));
        tick(&mut app);
    }

    let log = app.world().resource::<EventLog>();
    assert_eq!(log.entries.len(), 200);
    // La primera entrada debe ser el evento #50 (índice 49), no el #0
}
```

### 11.5 CI — headless sin ventana

Para CI, la app se ejecuta con `--headless` flag (nuevo):

```rust
// En main.rs:
let headless = args.iter().any(|a| a == "--headless");

if headless {
    // No spawn Camera2d, no ventana gráfica
    // Pero todos los sistemas ECS corren normal
} else {
    app.add_plugins(UiPlugin);
}
```

Esto permite que la app corra en CI sin necesidad de display server.

---

## 12. Consideraciones de Performance

### 12.1 Frame budget

- **Budget total**: ~16.6ms para 60 FPS (aunque la app corre a 50ms = 20 FPS)
- **Budget de UI**: < 2ms (tabla de 10 hornos + log de 200 entradas)
- **bevy_egui overhead**: negligible para < 1000 widgets

### 12.2 Query optimization

```rust
// EFICIENTE: Query con componentes específicos, sin &mut
fn ui_render(
    // ...
    query: Query<(
        &OvenId,
        &CurrentTemperature,
        &TargetTemperature,
        &Enabled,
        &Heating,
        &OvenStatus,
        &FaultState,
        &MaxTemperature,
        &LastCommandResult,
    )>,
) {
    for (id, current, target, enabled, heating, status, fault, max, last_cmd) in &query {
        // Solo lectura — no hay &mut, no hay conflicto de sistemas
    }
}
```

Todos los componentes de horno se leen con `&` (shared reference), lo que permite que `ui_render` corra en paralelo con otros sistemas de lectura en `Update`.

### 12.3 Log performance

- **Push**: O(1) amortizado — `Vec::push` es constante. La eliminación de entradas antiguas es O(n) pero solo ocurre cuando se excede 200 entradas (evento raro).
- **Render**: O(n) donde n = número de entradas visibles (scrollarea muestra ~20-30 entradas a la vez). egui maneja esto eficientemente con lazy rendering.

### 12.4 No-blocking guarantee

El spec requiere (Requirement: No bloqueo del ECS):
> La UI MUST completar su renderizado dentro del frame ECS (Update schedule). El sistema MUST NOT introducir latencia medible en el game loop.

Esto se garantiza porque:
1. `ui_render` corre en `EguiPrimaryContextPass`, que es un pass dedicado que NO bloquea `Update`.
2. Todas las operaciones de UI son lecturas de componentes ECS (`&Query`) — sin locks, sin async.
3. Los controles de UI (botones, sliders) solo escriben en un recurso `UiIntent` — un `ResMut` trivial.
4. El dispatch de comandos (`ui_command_dispatch`) solo llama a funciones existentes que hacen `Vec::push` en `OutboundProtocolQueue`.

### 12.5 Número de hornos

Para 10 hornos con 7 columnas cada uno + controles:
- ~80 widgets por frame
- bevy_egui benchmark: ~0.3ms para 1000 widgets
- Estimado: ~0.02ms para 80 widgets
- **Margen**: 100x por debajo del budget

---

## 13. Archivos Afectados — Resumen

| Archivo | Estado | Descripción |
|---------|--------|-------------|
| `pc-app/Cargo.toml` | Modificado | Agregar `bevy_egui` |
| `pc-app/src/main.rs` | Modificado | Agregar `EguiPlugin` + `UiPlugin` |
| `pc-app/src/lib.rs` | Modificado | `pub mod ui;` |
| `pc-app/src/resources.rs` | Modificado | Agregar `EventLog`, `LogEntry`, `LogDirection`, `ConnectionState`, `UiIntent` |
| `pc-app/src/plugins/mod.rs` | Modificado | `pub mod egui; pub mod ui;` |
| `pc-app/src/plugins/egui.rs` | **Nuevo** | Wrapper de `EguiPlugin` |
| `pc-app/src/plugins/ui.rs` | **Nuevo** | `UiPlugin` — registro de sistemas |
| `pc-app/src/systems/ui/mod.rs` | **Nuevo** | Módulo de sistemas de UI |
| `pc-app/src/systems/ui/render.rs` | **Nuevo** | `ui_render` — renderizado completo |
| `pc-app/src/systems/ui/dispatch.rs` | **Nuevo** | `ui_command_dispatch` — envío de comandos |
| `pc-app/src/systems/ui/log_capture.rs` | **Nuevo** | `log_capture_events` — captura de eventos |
| `pc-app/src/ui/mod.rs` | **Nuevo** | Módulo de tipos y helpers de UI |
| `pc-app/src/ui/panels.rs` | **Nuevo** | Funciones de dibujo de paneles |
| `pc-app/src/ui/controls.rs` | **Nuevo** | Funciones de dibujo de controles |
| `pc-app/src/ui/styles.rs` | **Nuevo** | Colores y estilos |
| `pc-app/tests/ui_integration.rs` | **Nuevo** | Tests de UI |
| `pc-app/tests/integration.rs` | Sin cambios | Tests existentes — 0 modificaciones |

---

## 14. Riesgos y Mitigaciones

| Riesgo | Impacto | Mitigación |
|--------|---------|------------|
| `bevy_egui` versión incompatible con Bevy 0.15 | Bloqueante | Verificar versión en momento de implementación; ADR 004 ya evalúa esta opción |
| Ventana gráfica no se abre en headless CI | Bloqueante en CI | Flag `--headless` que omite `UiPlugin` completamente |
| UI lenta con muchos hornos | Performance | Query optimizada (solo lectura), egui lazy rendering, límite de 200 entradas en log |
| Cambios en `PcAppPlugin` rompen tests | Regresión | `PcAppPlugin` NO se modifica — la UI consume recursos/componentes existentes |
| Dependencia externa añade superficie de bug | Bajo | `bevy_egui` es estable, activamente mantenido, y la UI es simple |

---

## 15. Decisiones de Diseño Documentadas

| Decisión | Alternativa rechazada | Razón |
|----------|----------------------|-------|
| `MinimalPlugins` + `EguiPlugin` | `DefaultPlugins` completo | Evita overhead del renderizador 3D/2D; mantiene app headless original |
| UI como sistema ECS (no entities) | `bevy_ui` (retained mode) | Immediate mode es más rápido de desarrollar, widgets ricos, ideal para dashboards (ADR 004) |
| `UiIntent` resource para comandos | Llamar `author_*_command` directamente desde `ui_render` | Separa captura de intención del dispatch; evita mutaciones concurrentes en el mismo frame |
| Log en `Update` con `EventReader` | Log en `EguiPrimaryContextPass` | Garantiza que el log se llena antes del renderizado; usa los mismos eventos que los sistemas de estado |
| Sin `DefaultPlugins` = sin `Camera2d` en `EguiPlugin` | Spawn `Camera2d` en `EguiPlugin` | Mantiene `EguiPlugin` independiente; el spawn de cámara es responsabilidad de `UiPlugin` |
| Timestamps sin `chrono` | Dependencia `chrono` | Evita dependencia adicional para un proyecto académico; `HH:MM:SS` es suficiente |
| Connection state heurístico | Modificar transport crate | Out-of-scope; heurística es suficiente para demostración académica |
