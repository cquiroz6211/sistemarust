# Tareas: Interfaz de usuario visual en pc-app con bevy_egui

## Resumen

Este change agrega una interfaz gráfica al `pc-app` usando `bevy_egui` para visualizar el estado de los hornos en tiempo real y enviar comandos de control. La UI se integra como un plugin adicional sobre `MinimalPlugins` sin modificar los sistemas ECS existentes.

**Spec**: `specs/pc-app-ui/spec.md` (8 requisitos, 20 escenarios)
**Design**: `design.md`
**Fase**: Tasks — desglose de implementación

---

## Pronóstico de Carga de Trabajo

| Métrica | Valor |
|---------|-------|
| Archivos nuevos | 10 |
| Archivos modificados | 4 |
| Líneas estimadas totales | ~1,200–1,500 |
| Riesgo de presupuesto 400 líneas | **Sí — se requieren chained PRs** |
| PRs recomendados | 4–5 chained PRs (uno por fase) |

### Recomendación de PRs encadenados

| PR | Fase | Líneas estimadas |
|----|------|-----------------|
| 1/5 | Fase 1 — Setup & Dependencias | ~15 |
| 2/5 | Fase 2 — Infraestructura Core de UI | ~80 |
| 3/5 | Fase 3 — Flujo de Datos (UiIntent + dispatch) | ~120 |
| 4/5 | Fase 4 — Tabla de Hornos y Controles | ~450 |
| 5/5 | Fase 5–7 — Log, Conexión, Tests | ~600 |

Las Fases 4 y 5 exceden el presupuesto de 400 líneas por PR individual, por lo que se recomienda usar el PR 4/5 para la tabla/controles y el PR 5/5 para log + conexión + tests.

---

## Orden de Implementación

Las fases son **secuenciales** (cada una depende de la anterior):

```
Fase 1 → Fase 2 → Fase 3 → Fase 4 → Fase 5 → Fase 6 → Fase 7
```

No se puede saltar fases porque:
- La Fase 3 necesita los sistemas de la Fase 2
- La Fase 4 necesita el UiIntent de la Fase 3
- La Fase 5 necesita el sistema de renderizado de la Fase 4
- La Fase 6 necesita el log de la Fase 5
- La Fase 7 necesita todo lo anterior funcionando

---

## Módulo Summary

```
pc-app/src/
├── main.rs                    # Modificado: agregar EguiPlugin + UiPlugin + --headless
├── lib.rs                     # Modificado: pub mod ui;
├── resources.rs               # Modificado: EventLog, LogEntry, LogDirection, ConnectionState, UiIntent
├── plugins/
│   ├── mod.rs                 # Modificado: pub mod egui; pub mod ui;
│   ├── egui.rs                # NUEVO: wrapper de EguiPlugin
│   └── ui.rs                  # NUEVO: UiPlugin con registro de sistemas
├── systems/
│   └── ui/
│       ├── mod.rs             # NUEVO: re-export systems
│       ├── render.rs          # NUEVO: ui_render — todo el renderizado egui
│       ├── dispatch.rs        # NUEVO: ui_command_dispatch — envío de comandos
│       └── log_capture.rs     # NUEVO: log_capture_events — captura de eventos
└── ui/
    ├── mod.rs                 # NUEVO: re-export types/helpers
    ├── panels.rs              # NUEVO: paneles (header, oven table, side panel)
    ├── controls.rs            # NUEVO: controles por horno (toggles, sliders)
    ├── log_panel.rs           # NUEVO: renderizado del log
    └── styles.rs              # NUEVO: colores y estilos reutilizables
```

---

## Fase 1 — Setup & Dependencias

- [x] 1.1 Agregar `bevy_egui` a `Cargo.toml`
- [x] 1.2 Verificar compilación limpia

---

## Fase 2 — Infraestructura Core de UI

- [x] 2.1 Crear `plugins/egui.rs` — Wrapper de EguiPlugin
- [x] 2.2 Crear `plugins/ui.rs` — UiPlugin con registro de sistemas
- [x] 2.3 Spawn de cámara 2D
- [x] 2.4 Panel Central vacío
- [x] 2.5 Integrar plugins en `main.rs` y `lib.rs`

---

## Fase 3 — Flujo de Datos (UiIntent + Dispatch)

- [x] 3.1 Agregar recursos de UI a `resources.rs`
- [x] 3.2 Sistema `ui_command_dispatch` — Envío de comandos
- [x] 3.3 Verificar flujo de datos

---

## Fase 4 — Tabla de Hornos y Controles

- [x] 4.1 Crear `ui/styles.rs` — Estilos visuales
- [x] 4.2 Crear `ui/controls.rs` — Controles por horno
- [x] 4.3 Crear `ui/panels.rs` — Paneles principales
- [x] 4.4 Actualizar `ui_render` con contenido real
- [x] 4.5 Verificar controles envían comandos

---

## Fase 5 — Log de Eventos

- [x] 5.1 Sistema `log_capture_events` — Captura de eventos
- [x] 5.2 Panel de log — `ui/log_panel.rs`
- [x] 5.3 Integrar panel de log en `ui_render`
  1. `LogEntry` — estructura con `timestamp: String`, `direction: LogDirection`, `message_type: String`, `summary: String`
  2. `LogDirection` — enum `In` / `Out` con método `label() -> &'static str`
  3. `EventLog` — recurso con `entries: Vec<LogEntry>`, `max_entries: usize` (default 200), método `push()` con FIFO
  4. `ConnectionState` — enum `Connected` / `Disconnected`
  5. `UiIntent` — recurso con `set_enabled: Option<(String, bool)>`, `set_temperature: Option<(String, f64)>`, `request_status: Option<Option<String>>`, `emergency_stop: bool`
- **Por qué**: Son los recursos que la UI consume y produce. `UiIntent` separa captura de intención del dispatch (design §5.2). `EventLog` es el storage del panel de eventos.
- **Archivos**: `pc-app/src/resources.rs`
- **Líneas**: ~60
- **Criterios de aceptación**:
  - `EventLog::push()` agrega entrada y descarta las más antiguas si supera `max_entries`
  - `LogDirection::label()` retorna `"IN"` o `"OUT"`
  - Todos los tipos implementan `Debug`, `Clone`, `Default` (o `Resource` donde corresponda)
  - Compila sin warnings

### Tarea 3.2: Sistema `ui_command_dispatch` — Envío de comandos

- **Qué**: Crear `pc-app/src/systems/ui/dispatch.rs` con `ui_command_dispatch` que:
  1. Lea `ResMut<UiIntent>` y `ResMut<OutboundProtocolQueue>`
  2. Si `set_enabled` tiene valor → llamar `author_set_oven_enabled_command(oven_id, enabled, &mut outbound)`
  3. Si `set_temperature` tiene valor → llamar `author_set_target_temperature_command(oven_id, temp, &mut outbound)`
  4. Si `request_status` tiene valor → llamar `author_request_status_command(oven_id, scope, &mut outbound)`
  5. Si `emergency_stop` es true → llamar `author_emergency_stop_command("Activado desde UI", &mut outbound)`
  6. Resetear `UiIntent` a `Default` al final
- **Por qué**: Traduce las intenciones capturadas por la UI en comandos reales en `OutboundProtocolQueue`. Usa las funciones de command authoring existentes (design §5.2).
- **Archivos**: `pc-app/src/systems/ui/dispatch.rs` (nuevo)
- **Líneas**: ~40
- **Criterios de aceptación**:
  - Cada campo de `UiIntent` se procesa con un `if let Some(...)`
  - Se llama a la función correcta de `author_*_command` con los parámetros correctos
  - `UiIntent` se resetea a `Default` al final del sistema
  - `request_status: Some(None)` → `scope: RequestScope::All`
  - `request_status: Some(id)` → `scope: RequestScope::Single`
  - Compila y no introduce warnings

### Tarea 3.3: Verificar flujo de datos

- **Qué**: Confirmar que `ui_command_dispatch` está registrado en `Update` dentro de `UiPlugin` (Tarea 2.2 ya lo hace). Ejecutar `cargo check` para verificar.
- **Por qué**: Asegurar que el sistema se ejecuta en el schedule correcto.
- **Archivos**: `pc-app/src/plugins/ui.rs` (verificación)
- **Líneas**: 0
- **Criterios de aceptación**:
  - `ui_command_dispatch` está registrado en `Update`
  - `cargo check --package pc-app` compila

---

## Fase 4 — Tabla de Hornos y Controles

### Tarea 4.1: Crear `ui/styles.rs` — Estilos visuales

- **Qué**: Crear `pc-app/src/ui/styles.rs` con constantes y funciones helper:
  - Colores: `connected_color()`, `disconnected_color()`, `heating_color()`, `fault_color()`, `disabled_color()`
  - Formatos: `format_temp(f64) -> String` (1 decimal + "°C")
  - Estados: `format_oven_status(OvenState) -> &'static str`
  - Colores de estado: `oven_row_color(enabled: bool, heating: bool, fault: bool) -> Color32`
- **Por qué**: Colores y formatos reutilizables en todos los paneles. Evita duplicación y facilita el mantenimiento visual.
- **Archivos**: `pc-app/src/ui/styles.rs` (nuevo)
- **Líneas**: ~35
- **Criterios de aceptación**:
  - `format_temp(185.567)` retorna `"185.6 °C"` (cumple spec §Requirement: Visualización del estado de hornos)
  - `oven_row_color` retorna amarillo si `heating=true`, rojo si `fault=true`, gris si `!enabled`
  - `format_oven_status` mapea `OvenState` a etiquetas legibles ("Heating", "Idle", "Cooling", "Fault")
  - Compila sin warnings

### Tarea 4.2: Crear `ui/controls.rs` — Controles por horno

- **Qué**: Crear `pc-app/src/ui/controls.rs` con funciones de dibujo de controles interactivos:
  1. `render_enabled_toggle(ui, oven_id, enabled, mut intent) -> bool` — botón "Encender"/"Apagar"
  2. `render_temperature_slider(ui, oven_id, target, max_temp, mut intent) -> bool` — slider + botón confirmar
  3. `render_status_request(ui, oven_id, mut intent) -> bool` — botón "Solicitar Estado"
- **Por qué**: Controles interactivos que capturan la intención del operador y la escriben en `UiIntent`. Cada función retorna `true` si se capturó una intención este frame.
- **Archivos**: `pc-app/src/ui/controls.rs` (nuevo)
- **Líneas**: ~80
- **Criterios de aceptación**:
  - Toggle: botón muestra "Encender" si `!enabled`, "Apagar" si `enabled`. Al hacer clic, escribe en `intent.set_enabled`
  - Slider: rango `[0.0, max_temp]`. Si el usuario ingresa un valor fuera de rango, muestra mensaje de error y **no** escribe en intent (cumple spec §Scenario: Temperatura excede limite)
  - Slider: si valor negativo → rechazado localmente sin enviar comando (cumple spec §Scenario: Temperatura negativa)
  - Status request: al hacer clic, escribe en `intent.request_status = Some(oven_id.clone())`
  - Compila sin warnings

### Tarea 4.3: Crear `ui/panels.rs` — Paneles principales

- **Qué**: Crear `pc-app/src/ui/panels.rs` con funciones de dibujo de paneles:
  1. `render_oven_grid(ui, oven_index, query, mut intent)` — grilla egui con columnas: ID, Temp Actual, Temp Objetivo, Enabled, Estado, Calentando, Fallo
  2. Manejo de caso vacío: "Sin hornos detectados"
  3. Cada fila incluye los controles de la Tarea 4.2
  4. Botones globales: "🔴 Emergency Stop" y "🔄 Actualizar Todos"
- **Por qué**: El panel central es el núcleo de la UI. Muestra toda la información de hornos y los controles para interactuar con ellos.
- **Archivos**: `pc-app/src/ui/panels.rs` (nuevo)
- **Líneas**: ~120
- **Criterios de aceptación**:
  - Tabla muestra todas las entidades de horno (cumple spec §Scenario: Tabla muestra hornos detectados)
  - Muestra "Sin hornos detectados" cuando no hay entidades (cumple spec §Scenario: Tabla vacía)
  - Temperatura formateada a 1 decimal (cumple spec §Scenario: Temperatura formateada)
  - Cada horno tiene: toggle enabled, slider temp, botón status request
  - Botón global Emergency Stop visible siempre
  - Botón global "Actualizar Todos" visible siempre
  - Indicadores visuales: fondo amarillo si heating, rojo si fault, texto gris si disabled
  - Compila sin warnings

### Tarea 4.4: Actualizar `ui_render` con contenido real

- **Qué**: Reemplazar el `CentralPanel` vacío (Tarea 2.4) con la UI real:
  1. Header panel con título "Control de Hornos" + indicador de conexión
  2. Grid de hornos con `egui::Grid::new("oven_grid")`
  3. Botones globales debajo de la tabla
- **Por qué**: La UI pasa de "ventana vacía" a "interfaz funcional con datos reales".
- **Archivos**: `pc-app/src/systems/ui/render.rs`
- **Líneas**: ~80
- **Criterios de acceptance**:
  - `ui_render` recibe: `EguiContexts`, `Res<OvenIndex>`, `Res<ConnectionState>`, `Res<EventLog>`, `Query<(...)>`
  - El header muestra título + indicador de conexión (verde/rojo)
  - La grilla muestra los datos reales de los hornos
  - Los controles escriben en `ResMut<UiIntent>`
  - Compila sin warnings

### Tarea 4.5: Verificar controles envían comandos

- **Qué**: Ejecutar `cargo test --package pc-app` y verificar que los tests existentes siguen pasando. Manualmente probar que los botones de UI encolan comandos.
- **Por qué**: Confirmar que la integración con el sistema de comandos existente funciona correctamente.
- **Archivos**: Ninguno (verificación)
- **Líneas**: 0
- **Criterios de aceptación**:
  - Todos los tests existentes pasan (`cargo test --package pc-app`)
  - Manual: hacer clic en "Encender" encola `SetOvenEnabled` en `OutboundProtocolQueue`
  - Manual: ajustar slider encola `SetTargetTemperature` en `OutboundProtocolQueue`
  - Manual: clic en "Emergency Stop" con confirmación encola `EmergencyStop`
  - Manual: clic en "Actualizar Todos" encola `RequestStatus(All)`

---

## Fase 5 — Log de Eventos

### Tarea 5.1: Sistema `log_capture_events` — Captura de eventos

- **Qué**: Crear `pc-app/src/systems/ui/log_capture.rs` con `log_capture_events` que:
  1. Use `EventReader` de: `OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, `CommandRejectedReceived`
  2. Para cada evento entrante → `EventLog::push(LogEntry { direction: In, ... })`
  3. Lea `Res<OutboundProtocolQueue>` y capture comandos pendientes → `EventLog::push(LogEntry { direction: Out, ... })`
  4. Formatee timestamps como `"HH:MM:SS"` sin dependencia de `chrono`
- **Por qué**: El log permite al operador ver el historial de eventos. Se llena en `Update` con `EventReader` de los mismos eventos que los sistemas de estado (design §8.1).
- **Archivos**: `pc-app/src/systems/ui/log_capture.rs` (nuevo)
- **Líneas**: ~70
- **Criterios de aceptación**:
  - Cada evento entrante genera una entrada de log con formato `"[HH:MM:SS] IN TypeName resumen"` (cumple spec §Scenario: Evento inbound registrado)
  - Cada comando saliente genera una entrada con `"[HH:MM:SS] OUT TypeName resumen"` (cumple spec §Scenario: Comando outbound registrado)
  - El log mantiene máximo 200 entradas (cumple spec §Scenario: Limite de entradas)
  - `log_capture_events` se ejecuta **after** `ingest_inbound_protocol` (garantiza eventos ya emitidos)
  - Compila sin warnings

### Tarea 5.2: Panel de log — `ui/log_panel.rs`

- **Qué**: Crear `pc-app/src/ui/log_panel.rs` con `render_event_log(ui, event_log)` que:
  1. Use `egui::ScrollArea::vertical()` para scroll
  2. Cada entrada muestra: timestamp + dirección (color verde/azul) + tipo + resumen
  3. Auto-scroll al final cuando hay nuevas entradas
- **Por qué**: Renderiza el contenido de `EventLog` en un panel lateral scrollable.
- **Archivos**: `pc-app/src/ui/log_panel.rs` (nuevo)
- **Líneas**: ~25
- **Criterios de aceptación**:
  - Panel scrollable con `egui::ScrollArea::vertical()`
  - Entradas IN en verde, OUT en azul
  - Formato: `[HH:MM:SS] IN/OUT TypeName resumen`
  - Compila sin warnings

### Tarea 5.3: Integrar panel de log en `ui_render`

- **Qué**: Agregar un `SidePanel` derecho con el log de eventos al sistema `ui_render`. Layout:
  ```
  ┌─────────────────────────────────────────────────┐
  │ Header (título + conexión)                       │
  ├──────────────────────┬───────────────────────────┤
  │ Tabla de hornos      │ Panel de log de eventos   │
  │ (CentralPanel)       │ (SidePanel derecho)       │
  └──────────────────────┴───────────────────────────┘
  ```
- **Por qué**: El log es un panel lateral, no parte de la tabla central. El layout sigue el diseño del design.md §7.1.
- **Archivos**: `pc-app/src/systems/ui/render.rs`
- **Líneas**: ~30
- **Criterios de aceptación**:
  - Panel lateral derecho con `egui::SidePanel::right("event_log_panel")`
  - Muestra las entradas del `EventLog`
  - Layout cumple con el diseño del design.md §7.1
  - Compila sin warnings

---

## Fase 6 — Estado de Conexión y Pulido

- [x] 6.1 Sistema `connection_monitor` — Estado de conexión
- [x] 6.2 Header panel con indicador de conexión
- [x] 6.3 Pulido visual

---

## Fase 7 — Testing

- [x] 7.1 Flag `--headless` para CI
- [x] 7.2 Tests de UI — `tests/ui_integration.rs`
- [x] 7.3 Verificar tests existentes

---

## Fase 8 — UX Refinement (Operator-Friendly Cards)

The initial UI compiled and passed tests but was a technical debug table.
User testing revealed it lacked operator value. This phase refines the UI.

- [x] 8.1 Add `OvenEditStates` resource — per-oven local temperature editing state
- [x] 8.2 Rewrite `ui/styles.rs` — add oven state labels (Spanish), card colors, emergency color
- [x] 8.3 Rewrite `ui/controls.rs` — temperature editor with quick buttons (-10/-1/+1/+10), explicit "Aplicar temperatura" apply button, Spanish labels ("Encender horno", "Apagar horno", "Solicitar estado")
- [x] 8.4 Rewrite `ui/panels.rs` — oven card layout with prominent temperatures, state badges, emergency warning, fault details, empty state instructions, disconnected warning
- [x] 8.5 Rewrite `systems/ui/render.rs` — replace grid with oven cards, add `OvenEditStates` system param
- [x] 8.6 Update `plugins/ui.rs` — insert `OvenEditStates` resource
- [x] 8.7 Update `tests/ui_integration.rs` — insert `OvenEditStates` in test app

---

## Resumen del Desglose

| Fase | Tareas | Líneas estimadas | Prerrequisito |
|------|--------|-----------------|---------------|
| 1. Setup & Dependencias | 2 | ~15 | — |
| 2. Infraestructura Core | 5 | ~80 | Fase 1 |
| 3. Flujo de Datos | 3 | ~120 | Fase 2 |
| 4. Tabla & Controles | 5 | ~450 | Fase 3 |
| 5. Log de Eventos | 3 | ~125 | Fase 4 |
| 6. Conexión & Pulido | 3 | ~95 | Fase 5 |
| 7. Testing | 3 | ~110 | Fase 6 |
| **Total** | **24** | **~1,000–1,200** | — |

### Archivos totales

| Estado | Cantidad |
|--------|----------|
| Nuevos | 10 |
| Modificados | 4 |
| **Total** | **14** |
