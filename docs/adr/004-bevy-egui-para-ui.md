# ADR 004: bevy_egui como framework de UI para pc-app

**Estado**: Aceptado  
**Fecha**: 2026-06-04  
**Decisor**: Equipo del proyecto  
**Contexto**: Sistema de Control de Hornos - Interfaz de usuario en PC

---

## Contexto

El sistema de control de hornos requiere una interfaz gráfica en la PC para que los operadores puedan:

- Visualizar el estado de múltiples hornos en tiempo real
- Enviar comandos (encender, apagar, ajustar temperatura)
- Recibir alertas de fallas y emergencias
- Observar el flujo de datos entre PC y Raspberry Pi

Actualmente, `pc-app` es una aplicación Bevy headless sin interfaz visual. Necesitamos agregar UI que se integre con la arquitectura ECS existente.

## Decisione evaluadas

### Opción A: `bevy_ui` (UI nativa de Bevy)

**Filosofía**: Retained mode — nodos persisten en el ECS como entities.

**Ventajas**:
- Integración nativa con ECS (cada nodo es una Entity)
- Sin dependencias externas
- Control fino sobre layout y rendering
- Performance óptima para UIs estáticas

**Desventajas**:
- API verbosa y boilerplate significativo
- Curva de aprendizaje empinada
- Pocos widgets pre-construidos
- Desarrollo lento para prototipos

**Ejemplo de código**:
```rust
// Un simple botón requiere ~30 líneas
commands.spawn(NodeBundle { ... })
    .with_children(|parent| {
        parent.spawn((ButtonBundle { ... },))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section("Botón", ...));
            });
    });
```

### Opción B: `bevy_egui` (Immediate mode via egui)

**Filosofía**: Immediate mode — se describe la UI cada frame, egui la renderiza.

**Ventajas**:
- Desarrollo rápido y expressivo
- Widgets ricos: tablas, sliders, gráficos, paneles
- Hot-reloading natural
- Comunidad activa, bien documentado
- Ideal para dashboards y herramientas

**Desventajas**:
- Dependencia externa (egui)
- UI no se integra directamente como ECS entities
- Menos control fino sobre rendering custom
- Performance puede degradar con UIs muy complejas

**Ejemplo de código**:
```rust
// Un panel completo con botones: ~15 líneas
egui::CentralPanel::default().show(ctx, |ui| {
    ui.heading("Horno 1");
    ui.label("Temperatura: 185°C");
    if ui.button("Apagar").clicked() {
        // enviar comando
    }
});
```

### Opción C: Otras alternativas (no evaluadas en profundidad)

- `egui` standalone (sin integración Bevy)
- `iced` (framework UI independiente)
- `gpui` (nuevo, experimental)

## Decision

**Elegimos `bevy_egui`** como framework de UI para pc-app.

## Justificacion

### 1. Alineación con objetivos del proyecto

El proyecto es **académico y de demostración**. Necesitamos:

- Prototipado rápido para mostrar conceptos
- Código legible para estudiantes
- Resultados observables inmediatos

`bevy_egui` permite crear una UI funcional en horas, no días.

### 2. Velocidad de desarrollo

Comparación para implementar un panel de control básico:

| Aspecto | bevy_ui | bevy_egui |
|---------|---------|-----------|
| Código para un botón | ~30 líneas | ~5 líneas |
| Tiempo estimado UI completa | 2-3 días | 4-6 horas |
| Complejidad de learning curve | Alta | Baja |

### 3. Widgets pre-construidos

`bevy_egui` ofrece widgets que necesitamos directamente:

- `egui::Grid` → tabla de estado de hornos
- `egui::Slider` → ajuste de temperatura
- `egui::ProgressBar` → barra de progreso de calentamiento
- `egui::CentralPanel` → layout principal
- `egui::TopBottomPanel` → barra de estado y controles

### 4. Integración con ECS existente

Aunque la UI no son entities ECS, podemos:

```rust
// Leer recursos ECS desde la UI
fn ui_system(
    mut contexts: EguiContexts,
    oven_store: Res<OvenStore>,  // ← acceso directo
    transport: Res<TransportState>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut()?, |ui| {
        for (id, oven) in oven_store.iter() {
            ui.label(format!("{}: {}°C", id, oven.current_temp));
        }
    });
}

// Enviar comandos desde la UI
if ui.button("Encender").clicked() {
    commands.send(WriteEvent::SetOvenEnabled { oven_id, enabled: true });
}
```

### 5. Rendimiento adecuado

Para nuestro caso de uso (4-8 hornos, updates cada 500ms):

- `bevy_egui` maneja easily < 1000 widgets sin issues
- El overhead de immediate mode es negligible para UIs de control
- No necesitamos 60 FPS para un panel de industrial

### 6. Comunidad y soporte

- `bevy_egui`: 40+ code snippets en Context7, bien documentado
- Activo mantenimiento (compatible con Bevy 0.15+)
- Ejemplos abundantes de dashboards y herramientas

## Consecuencias

### Positivas

- ✅ UI funcional en menos de un día
- ✅ Código legible y mantenible
- ✅ Fácil de extender con nuevos widgets
- ✅ Hot-reloading para desarrollo iterativo
- ✅ Students pueden enfocarse en lógica, no en boilerplate de UI

### Negativas

- ⚠️ Dependencia externa (mitigado: crate estable, activamente mantenido)
- ⚠️ UI no se integra como ECS entities (mitigado: acceso via Res<>)
- ⚠️ Menos control sobre rendering custom (mitigado: no lo necesitamos)

### Riesgos

- **Riesgo bajo**: Si necesitamos UI altamente custom en el futuro, podemos migrar a `bevy_ui` gradualmente
- **Riesgo bajo**: Performance no será issue para nuestro volumen de datos

## Alternativas futuras

Si el proyecto evoluciona hacia:

- **UI para móvil**: Evaluar `egui` standalone o `iced`
- **UI altamente custom**: Considerar migración parcial a `bevy_ui`
- **Solo debug/tools**: `bevy_egui` sigue siendo ideal

## Referencias

- [bevy_egui docs](https://github.com/vladbat00/bevy_egui)
- [egui documentation](https://docs.rs/egui/latest/egui/)
- [Bevy UI cookbook](https://bevy-cheatbook.github.io/)
- ADR 001: Protocolo eventos Rust enum
- ADR 002: Histéresis control térmico
- ADR 003: Tokio como adaptador de transporte

---

**Aprobado por**: [Pendiente de revisión]  
**Fecha de implementación**: Pendiente  
**Crate afectado**: `pc-app`
