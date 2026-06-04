# Diseño Técnico: Bulk Oven Controls

## 1. Resumen Ejecutivo

Agregar un panel de operaciones en bloque al `pc-app` que permita seleccionar rangos de hornos por índice numérico y enviar comandos `SetOvenEnabled` y `SetTargetTemperature` a grupos enteros. La implementación reutiliza las funciones de command authoring existentes, el recurso `OutboundProtocolQueue`, y el sistema `ui_command_dispatch` ya existente.

**Decisión clave**: El panel bulk se integra como un `SidePanel` izquierdo (al lado del event log que está a la derecha), manteniendo las oven cards en el `CentralPanel` intactas. Esto evita interferir con los controles individuales existentes.

## 2. Arquitectura General

```
┌──────────────────────────────────────────────────────────────────────┐
│                          main.rs (App)                               │
│  DefaultPlugins (modo UI)                                            │
│  + PcAppPlugin (existing)                                            │
│  + UiPlugin (existing)                                               │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    Update Schedule                            │  │
│  │  [1] ingest_inbound_protocol (existing)                       │  │
│  │  [2] log_capture_events (existing)                            │  │
│  │  [3] ui_command_dispatch (existing — MODIFIED: handles bulk)  │  │
│  │  [4] connection_monitor (existing)                            │  │
│  └───────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              EguiPrimaryContextPass                           │  │
│  │  [5] ui_render (existing — MODIFIED: renders bulk panel)      │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

**No se modifican**:
- `PcAppPlugin` (sistemas de ingestión y state mutation)
- Protocol types, payloads, message enum
- Transport layer
- RPi controller

**Se modifican**:
- `pc-app/src/resources.rs` — agregar `BulkSelection` resource
- `pc-app/src/ui/panels.rs` — agregar función `render_bulk_panel`
- `pc-app/src/systems/ui/render.rs` — integrar bulk panel en layout
- `pc-app/src/systems/ui/dispatch.rs` — procesar intentos de bulk commands
- `pc-app/src/ui/styles.rs` — estilos para panel bulk

## 3. Recursos Nuevos

### 3.1 `BulkSelection`

```rust
/// Selección de hornos para operaciones en bloque.
/// Procesado por `ui_command_dispatch` en el siguiente frame.
#[derive(Debug, Clone, Resource, Default)]
pub struct BulkSelection {
    /// Índice mínimo del rango (inclusive).
    pub from_index: u32,
    /// Índice máximo del rango (inclusive).
    pub to_index: u32,
    /// Si true, seleccionar todos los hornos detectados.
    pub select_all: bool,
    /// Temperatura objetivo para aplicar a los seleccionados.
    pub target_temp: f64,
    /// Acción a ejecutar sobre los seleccionados.
    pub action: BulkAction,
    /// IDs de hornos seleccionados (calculados al procesar).
    pub selected_oven_ids: Vec<String>,
}

/// Acción de bulk operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkAction {
    #[default]
    None,
    Enable,
    Disable,
    ApplyTemperature,
    EnableAndApplyTemperature,
    RequestStatus,
}
```

### 3.2 `BulkValidation`

```rust
/// Mensajes de validación para el panel bulk.
#[derive(Debug, Resource, Default)]
pub struct BulkValidation {
    pub errors: Vec<String>,
}
```

## 4. Parsing de Oven IDs

### 4.1 Función `parse_oven_index`

```rust
/// Extrae el sufijo numérico de un oven_id como "oven-42".
/// Retorna None si el formato no coincide.
pub fn parse_oven_index(oven_id: &str) -> Option<u32> {
    // "oven-42" → 42
    // "oven-0" → 0
    // "oven1" → None (formato antiguo sin guion)
    oven_id
        .strip_prefix("oven-")
        .and_then(|s| s.trim().parse::<u32>().ok())
}
```

### 4.2 Orden determinístico

Los hornos seleccionados se ordenan por su índice numérico ascendente antes de encolar comandos:

```rust
pub fn resolve_selected_ovens<'a>(
    oven_index: &'a OvenIndex,
    selection: &BulkSelection,
) -> Vec<(&'a String, u32)> {
    let mut candidates: Vec<_> = oven_index
        .0
        .keys()
        .filter_map(|id| parse_oven_index(id).map(|idx| (id, idx)))
        .collect();

    if selection.select_all {
        // Ordenar por índice numérico ascendente
        candidates.sort_by_key(|(_, idx)| *idx);
        candidates.into_iter().map(|(id, _)| (id, _)).collect()
    } else {
        // Filtrar por rango
        candidates
            .into_iter()
            .filter(|(_, idx)| *idx >= selection.from_index && *idx <= selection.to_index)
            .collect()
    }
}
```

## 5. Panel de UI

### 5.1 Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  TOP PANEL: Título + Estado + Emergency Stop + Refresh All         │
├──────────────┬──────────────────────────┬───────────────────────────┤
│ SIDE PANEL   │  CENTRAL PANEL           │  SIDE PANEL (existing)    │
│ (NEW: Bulk   │  Oven Cards              │  Event Log                │
│  Operations) │  ┌────────────────────┐  │  [14:32:01] IN ...        │
│              │  │ oven-0  185°C [On] │  │  [14:32:02] OUT ...       │
│ ┌──────────┐ │  │ oven-1  200°C [On] │  │  ...                      │
│ │ BULK     │ │  │ oven-2  220°C [On] │  │                           │
│ │ OPERATIONS│ │  │ ...              │  │                           │
│ │          │ │  └────────────────────┘  │                           │
│ │ From: [10]│ │                          │                           │
│ │ To:  [29] │ │  [Encender Todos]       │                           │
│ │ ☑ All    │ │  [Solicitar Todos]      │                           │
│ │ Temp: [50]│ │                          │                           │
│ │ [Encender│ │                          │                           │
│ │  [Apagar] │ │                          │                           │
│ │ [Aplicar  │ │                          │                           │
│ │ [Enc +    │ │                          │                           │
│ │ [Status]  │ │                          │                           │
│ │          │ │                          │                           │
│ │ [Error msg│ │                          │                           │
│ └──────────┘ │                          │                           │
└──────────────┴──────────────────────────┴───────────────────────────┘
```

El panel bulk se implementa como `egui::SidePanel::left("bulk_panel")` con width fija de 240px.

### 5.2 Función `render_bulk_panel`

```rust
pub fn render_bulk_panel(
    ui: &mut egui::Ui,
    oven_count: usize,
    selection: &mut BulkSelection,
    validation: &mut BulkValidation,
    intent: &mut UiIntent,
    connection_state: &ConnectionState,
) {
    ui.heading("Operaciones en Bloque");
    ui.separator();

    // Rango de índices
    ui.label("Índice desde:");
    ui.add(egui::DragValue::new(&selection.from_index).speed(1.0));

    ui.label("Índice hasta:");
    ui.add(egui::DragValue::new(&selection.to_index).speed(1.0));

    // Select All
    ui.checkbox(&mut selection.select_all, "Seleccionar todos");

    // Deshabilitar rangos cuando select_all está activo
    let range_disabled = selection.select_all;
    if range_disabled {
        ui.disable();
    }
    // ... render from/to inputs disabled
    if range_disabled {
        ui.restore();
    }

    // Temperatura
    ui.label("Temperatura objetivo (°C):");
    ui.add(egui::DragValue::new(&selection.target_temp).speed(1.0).range(0.0..=300.0));

    ui.separator();

    // Botones de acción
    let disabled = *connection_state == ConnectionState::Disconnected;

    if ui.small_button("Encender seleccionados").clicked() && !disabled {
        selection.action = BulkAction::Enable;
    }
    if ui.small_button("Apagar seleccionados").clicked() && !disabled {
        selection.action = BulkAction::Disable;
    }
    if ui.small_button("Aplicar temperatura").clicked() && !disabled {
        selection.action = BulkAction::ApplyTemperature;
    }
    if ui.small_button("Encender + aplicar temp.").clicked() && !disabled {
        selection.action = BulkAction::EnableAndApplyTemperature;
    }
    if ui.small_button("Solicitar estado").clicked() && !disabled {
        selection.action = BulkAction::RequestStatus;
    }

    // Mostrar errores de validación
    for error in &validation.errors {
        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), error);
    }
}
```

## 6. Bulk Dispatch

### 6.1 Integración en `ui_command_dispatch`

El sistema existente `ui_command_dispatch` se extiende para procesar `BulkSelection`:

```rust
pub fn ui_command_dispatch(
    mut intent: ResMut<UiIntent>,
    mut bulk_selection: ResMut<BulkSelection>,
    mut bulk_validation: ResMut<BulkValidation>,
    mut outbound: ResMut<OutboundProtocolQueue>,
    oven_index: Res<OvenIndex>,
) {
    // ... (existing code: intent processing) ...

    // ── Bulk command dispatch ──────────────────────────────────────────
    if bulk_selection.action != BulkAction::None {
        // 1. Validate
        let errors = validate_bulk_selection(&bulk_selection, &oven_index);
        if !errors.is_empty() {
            bulk_validation.errors = errors;
            bulk_selection.action = BulkAction::None;
            return;
        }
        bulk_validation.errors.clear();

        // 2. Resolve selected ovens
        let selected = resolve_selected_ovens(&oven_index, &bulk_selection);

        // 3. Dispatch commands per oven, sorted by index
        for (oven_id, _idx) in &selected {
            match bulk_selection.action {
                BulkAction::Enable => {
                    author_set_oven_enabled_command(oven_id.clone(), true, &mut outbound);
                }
                BulkAction::Disable => {
                    author_set_oven_enabled_command(oven_id.clone(), false, &mut outbound);
                }
                BulkAction::ApplyTemperature => {
                    author_set_target_temperature_command(oven_id.clone(), bulk_selection.target_temp, &mut outbound);
                }
                BulkAction::EnableAndApplyTemperature => {
                    author_set_oven_enabled_command(oven_id.clone(), true, &mut outbound);
                    author_set_target_temperature_command(oven_id.clone(), bulk_selection.target_temp, &mut outbound);
                }
                BulkAction::RequestStatus => {
                    let scope = if selected.len() == 1 {
                        RequestScope::Single
                    } else {
                        RequestScope::All
                    };
                    author_request_status_command(Some(oven_id.clone()), scope, &mut outbound);
                }
                BulkAction::None => {}
            }
        }

        // 4. Reset selection action
        bulk_selection.action = BulkAction::None;
    }
}
```

### 6.2 Validación

```rust
fn validate_bulk_selection(
    selection: &BulkSelection,
    oven_index: &OvenIndex,
) -> Vec<String> {
    let mut errors = Vec::new();

    if selection.action == BulkAction::None {
        return vec!["No hay acción seleccionada".into()];
    }

    // Check at least one oven selected
    if selection.select_all && oven_index.0.is_empty() {
        return vec!["No hay hornos para seleccionar".into()];
    }

    if !selection.select_all {
        if selection.from_index > selection.to_index {
            errors.push("Rango inválido: from debe ser <= to".into());
        }
        // Rango siempre válido con u32 (no negativos)
    }

    // Temperature validation: check against the minimum max_temp of selected ovens
    if selection.action == BulkAction::ApplyTemperature || selection.action == BulkAction::EnableAndApplyTemperature {
        if selection.target_temp < 0.0 {
            errors.push("Temperatura debe ser >= 0".into());
        }
        // Note: exact max_temp check requires querying components,
        // which is more complex. For v1, we validate against a fixed max (300°C).
        // The RPi will reject out-of-range values anyway.
    }

    errors
}
```

## 7. Event Log Behavior

### 7.1 Log entries for bulk commands

Cada comando individual en bloque se registra con prefijo `[BULK]` en el `log_capture_events`:

```rust
// En log_capture_events, al leer OutboundProtocolQueue:
for envelope in &outbound.0 {
    let is_bulk = envelope.payload.to_string().starts_with("SetOvenEnabled")
        || envelope.payload.to_string().starts_with("SetTargetTemperature");
    // ... format summary with "[BULK]" prefix if bulk
}
```

### 7.2 Sin spam excesivo

El log ya tiene límite de 200 entradas (FIFO). Para demos con 100+ ovens, esto significa que las entradas más antiguas se pierden. No se agrega deduplicación ni resumen — cada comando individual se registra. Si el log se llena, las primeras entradas de un batch grande pueden desplazarse fuera del window visible, pero esto es aceptable para una demo académica.

**Alternativa futura**: Agregar un contador "[BULK] 100 comandos enviados" como entrada única en vez de 100 entradas individuales. Out of scope para v1.

## 8. Schedule de Sistemas

```
┌─────────────────────────────────────────────────────────────┐
│                    Update (50ms)                             │
│                                                              │
│  [1] ingest_inbound_protocol (existing)                     │
│  [2] log_capture_events (existing — reads OutboundQueue     │
│       para registrar entradas [BULK])                       │
│  [3] ui_command_dispatch (MODIFIED — procesa BulkSelection) │
│  [4] connection_monitor (existing)                          │
│                                                              │
│  → bridge_outbound_to_transport (existing, vacía queue)     │
└─────────────────────────────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────┐
│              EguiPrimaryContextPass                          │
│                                                              │
│  [5] ui_render (MODIFIED — renderiza bulk panel)            │
└─────────────────────────────────────────────────────────────┘
```

## 9. Files Affected

| Archivo | Estado | Descripción |
|---------|--------|-------------|
| `pc-app/src/resources.rs` | Modified | Agregar `BulkSelection`, `BulkAction`, `BulkValidation` |
| `pc-app/src/ui/panels.rs` | Modified | Agregar función `render_bulk_panel` |
| `pc-app/src/systems/ui/render.rs` | Modified | Integrar `render_bulk_panel` en layout como `SidePanel::left` |
| `pc-app/src/systems/ui/dispatch.rs` | Modified | Procesar `BulkSelection` en `ui_command_dispatch` |
| `pc-app/src/ui/styles.rs` | Modified | Agregar colores/estilos para panel bulk |
| `pc-app/src/lib.rs` | Unchanged | No changes needed |

## 10. Decisiones de Diseño

| Decisión | Alternativa | Razón |
|----------|-------------|-------|
| `SidePanel::left` para bulk panel | Sección en `CentralPanel` | Panel dedicado, no compite con oven cards, fácil de encontrar |
| `BulkSelection` como Resource | `UiIntent` extendido | Separa bulk intent del intent individual; más fácil de validar y resetear |
| Parsing por `strip_prefix("oven-")` | Regex | Simpler, más rápido, formato conocido del demo |
| Orden ascendente por índice | Orden de detección | Determinístico, reproducible, fácil de razonar |
| Validación fija 0-300°C | Consulta componentes ECS | Más simple para v1; RPi rechaza valores inválidos de todos modos |
| Cada comando en log con [BULK] | Resumen único [BULK] | Trazabilidad completa; log ya tiene límite de 200 |

## 11. Estimación de Líneas

| Archivo | Líneas añadidas |
|---------|-----------------|
| `resources.rs` | ~50 |
| `panels.rs` | ~80 |
| `render.rs` | ~20 |
| `dispatch.rs` | ~60 |
| `styles.rs` | ~15 |
| **Total** | **~225 líneas** |

Estimado conservador con tests: ~300-350 líneas. Dentro del presupuesto de 400 líneas.
