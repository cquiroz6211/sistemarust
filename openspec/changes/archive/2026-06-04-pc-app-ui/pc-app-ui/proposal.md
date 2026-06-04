# Proposal: Interfaz de usuario visual en pc-app con bevy_egui

## Intent

Agregar una interfaz gráfica al `pc-app` para visualizar el estado de los hornos en tiempo real y enviar comandos de control. Actualmente el `pc-app` es headless: procesa eventos del protocolo y mantiene un read model en ECS, pero no tiene ninguna forma de interactuar con un operador. Esto impide la demostración académica del sistema completo y limita la capacidad de diagnóstico en producción.

La decisión de usar `bevy_egui` está documentada en ADR 004 (`docs/adr/004-bevy-egui-para-ui.md`).

## Scope

### In Scope
- Integrar `bevy_egui` en el `pc-app` como dependencia de `Cargo.toml`
- Panel principal con tabla/grilla de todos los hornos mostrando: ID, temperatura actual, temperatura objetivo, estado (enabled/disabled), estado de calentamiento, y fallos activos
- Controles por horno: botón encender/apagar, slider + campo numérico para temperatura objetivo, botón solicitar estado
- Botón global de emergency stop
- Panel lateral con log de eventos (ingresados y enviados)
- Indicador visual de conexión con la Raspberry Pi (conectado/desconectado)
- Lectura de datos desde los componentes ECS existentes (`OvenId`, `CurrentTemperature`, `TargetTemperature`, `OvenStatus`, `FaultState`, etc.)
- Envío de comandos usando las funciones de command authoring existentes

### Out of Scope
- Gráficos históricos de temperatura (futuro: gráfico de líneas temporal)
- Autenticación o control de permisos
- Configuración de red desde la UI (hostname, puerto)
- Soporte para temas oscuros/claros o personalización visual
- Aplicación standalone fuera de Bevy
- Pruebas E2E de la interfaz
- Soporte multi-monitor o ventanas múltiples

## Capabilities

### New Capabilities
- `pc-app-ui`: Interfaz gráfica con bevy_egui para visualización de estado de hornos, envío de comandos (enable/disable, temperatura, status request, emergency stop), log de eventos, e indicador de conexión.

### Modified Capabilities
- Ninguno. Los specs existentes (`pc-oven-read-model`, `pc-command-authoring`, `pc-protocol-ingestion`) no cambian en su comportamiento; la UI solo consume sus recursos y componentes existentes.

## Approach

1. **Dependencia**: Agregar `bevy_egui` como dependencia en `pc-app/Cargo.toml`. Version compatible con Bevy 0.15+.

2. **Plugin**: Crear un `EguiPlugin` dentro de `pc-app/src/` que configure `EguiPlugin` de `bevy_egui` y registre las systems de UI.

3. **Sistema de UI principal**: Un sistema `ui_system` que:
   - Reciba `Res<OvenStore>` (o los componentes de horno directamente via `Query`)
   - Reciba `Res<OutboundProtocolQueue>` para enviar comandos
   - Reciba `Res<InboundProtocolQueue>` para mostrar eventos
   - Use `EguiContext` via `EguiContexts` para renderizar

4. **Layout propuesto**:
   ```
   ┌─────────────────────────────────────────────────────┐
   │  Barra superior: Título + Estado de conexión        │
   ├──────────────────────┬──────────────────────────────┤
   │  Tabla de hornos     │  Panel de log de eventos     │
   │  (Grid de egui)      │  (Scrolling text)            │
   │  ┌────┬───────┬─────┐ │                              │
   │  │ID  │Temp   │Ctrl │ │                              │
   │  ├────┼───────┼─────┤ │                              │
   │  │oven1│185°C │[On] │ │                              │
   │  └────┴───────┴─────┘ │                              │
   │                        │                              │
   └────────────────────────┴──────────────────────────────┘
   ```

5. **Comandos**: Al hacer clic en un botón de UI, el sistema construirá el `EventEnvelope` correspondiente usando las funciones de authoring existentes (`author_set_target_temperature_command`, etc.) y lo encolará en `OutboundProtocolQueue`.

6. **Lectura de estado**: La UI leerá directamente los componentes ECS vía `Query` para mostrar el estado actualizado cada frame.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `pc-app/Cargo.toml` | Modified | Agregar dependencia `bevy_egui` |
| `pc-app/src/main.rs` | Modified | Integrar `EguiPlugin`, configurar app |
| `pc-app/src/ui/` | New | Módulo de UI (panel principal, controles, log) |
| `pc-app/src/lib.rs` | Modified | Exponer recursos/queries necesarios para UI |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `bevy_egui` incompatibilidad con versión de Bevy | Low | Verificar compatibilidad antes de integrar; usar versión estable del crate |
| UI bloquea el loop ECS (render en thread principal) | Low | La UI es liviana (< 1000 widgets); Bevy maneja render en main thread sin problemas |
| Demasiada complejidad para demostración académica | Low | Mantener UI simple: tabla + botones + log. Sin animaciones ni gráficos |
| Dependencia externa añade superficie de bug | Low | `bevy_egui` es estable y activamente mantenido; no hay código propio de UI que mantener |

## Rollback Plan

1. Remover `bevy_egui` de `Cargo.toml`
2. Eliminar módulo `ui/` y referencias a `EguiPlugin` en `main.rs`
3. El `pc-app` vuelve a su estado headless anterior sin cambios en la lógica de protocolo o ECS

## Dependencies

- `bevy_egui` crate (compatible con Bevy 0.15+)
- Los recursos ECS existentes (`InboundProtocolQueue`, `OutboundProtocolQueue`, componentes de horno)

## Success Criteria

- [ ] `pc-app` compila con `bevy_egui` integrado
- [ ] La UI muestra todos los hornos detectados con su estado actualizado en tiempo real
- [ ] Los botones de encender/apagar envían comandos exitosamente a la cola de salida
- [ ] El slider de temperatura actualiza `TargetTemperature` y envía el comando correspondiente
- [ ] El botón de emergency stop funciona y muestra confirmación visual
- [ ] El log de eventos muestra eventos entrantes y comandos salientes
- [ ] La aplicación mantiene su funcionalidad headless original (los sistemas ECS siguen operando)
- [ ] Demostración académica funcional: un operador puede monitorear y controlar hornos desde la UI
