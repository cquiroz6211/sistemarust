# Proposal: Bulk Oven Controls for pc-app

## Intent

Agregar controles de operaciones en bloque al `pc-app` existente para que el operador pueda seleccionar rangos de hornos simulados y enviar comandos `SetOvenEnabled` y `SetTargetTemperature` a grupos enteros de una vez. El objetivo es demostrar que el sistema Bevy ECS + protocolo PC↔RPi escala a decenas o cientos de hornos simultáneos, algo imposible con los controles individuales actuales.

## Scope

### In Scope
- Panel de operaciones en bloque en la UI existente (integrado en `render.rs` como panel lateral izquierdo o sección en `CentralPanel`)
- Selección por rango de índices: `oven-0` a `oven-29`, `oven-30` a `oven-99`, etc.
- Seleccionar todos los hornos detectados
- Acciones:
  - Encender seleccionados (`SetOvenEnabled { enabled: true }`)
  - Apagar seleccionados (`SetOvenEnabled { enabled: false }`)
  - Aplicar temperatura (`SetTargetTemperature`)
  - Encender + aplicar temperatura (ambos comandos en secuencia)
  - Opcional: solicitar estado de seleccionados (`RequestStatus`)
- Validación de rangos (no negativos, no exceden hornos detectados)
- Validación de temperatura (rango `[0.0, max_temp]` del horno menor en la selección)
- Reutilización de funciones `author_set_oven_enabled_command` y `author_set_target_temperature_command` existentes
- Preservación total de controles individuales existentes

### Out of Scope
- Modificar el protocolo (no se agregan nuevos tipos de mensaje)
- Modificar el transport layer
- Modificar el rpi-controller
- Gráficos o historiales de temperatura
- Persistencia de configuraciones de batch
- Undo/redo de operaciones en bloque

## Capabilities

### New Capabilities
- `pc-app-bulk-oven-controls`: Panel de operaciones en bloque que permite seleccionar rangos de hornos por índice numérico o seleccionar todos, y enviar comandos `SetOvenEnabled` y `SetTargetTemperature` a todos los seleccionados en un solo clic.

### Modified Capabilities
- `pc-app-ui`: Se agrega un panel de bulk operations junto a los controles individuales existentes. Los specs de `pc-app-ui` no cambian — los controles individuales permanecen intactos.

## Approach

1. **Recurso `BulkSelection`**: Nuevo recurso que almacena los parámetros de selección del operador: rango (`from_index`, `to_index`), flag `select_all`, temperatura objetivo, y acción solicitada.

2. **Panel de UI**: Sección en el `CentralPanel` (arriba de la lista de oven cards o en un SidePanel izquierdo) con:
   - Inputs numéricos "From" / "To" para rango de índices
   - Checkbox "Select All"
   - Input numérico para temperatura
   - Botones de acción: "Encender seleccionados", "Apagar seleccionados", "Aplicar temperatura", "Encender + aplicar temperatura"

3. **Parsing de IDs**: Los hornos se nombran `oven-N` donde N es un entero. Para bulk, se parsea el sufijo numérico de cada `OvenId` y se ordenan determinísticamente. La función `parse_oven_index(oven_id: &str) -> Option<u32>` extrae el N.

4. **Dispatch de batch**: Al accionar un botón, se iteran los oven IDs seleccionados y se llama `author_set_oven_enabled_command` y/o `author_set_target_temperature_command` para cada uno. Los comandos se encolan secuencialmente en `OutboundProtocolQueue`.

5. **Validación**:
   - Rangos inválidos (from > to, índices negativos) → mensaje de error en UI, no se envía nada
   - Temperatura fuera de rango → mensaje de error
   - Rangos que exceden hornos disponibles → truncar al máximo existente o mostrar error

6. **Event log**: Cada comando individual en bloque se registra en el log para trazabilidad.

## Why This Matters

Este change es central para la demostración académica. Sin bulk controls, el operador solo puede controlar un horno a la vez, lo que hace imposible demostrar:
- Cómo Bevy ECS maneja decenas/hundreds de entities
- Cómo el protocolo escala con múltiples comandos encolados
- Cómo el RPi procesa actualizaciones masivas de estado

El bulk panel permite escenarios como: "Encender 100 hornos en 2 grupos con diferentes temperaturas", demostrando que el pipeline completo (PC→protocolo→TCP→RPi→ECS→protocolo→TCP→PC) funciona bajo carga.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `pc-app/src/resources.rs` | Modified | Agregar `BulkSelection` resource |
| `pc-app/src/ui/panels.rs` | Modified | Agregar panel de bulk operations |
| `pc-app/src/systems/ui/render.rs` | Modified | Integrar panel bulk en render |
| `pc-app/src/systems/ui/dispatch.rs` | Modified | Procesar intentos de bulk commands |
| `pc-app/src/ui/styles.rs` | Modified | Estilos para panel bulk |
| `pc-app/src/lib.rs` | Unchanged | No changes needed |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Log spam con 100+ comandos | Medium | El log ya tiene límite de 200 entradas; agregar opción de "bulk summary" en vez de cada comando individual |
| Rendimiento UI con muchos hornos | Low | El panel bulk es independiente de las oven cards; solo se iteran IDs en memoria |
| Confusión operador entre bulk y controles individuales | Low | Panel bulk visualmente separado con borde/colores distintos |

## Rollback Plan

1. Remover panel de bulk operations de `panels.rs`
2. Remover `BulkSelection` resource de `resources.rs`
3. Remover dispatch de bulk de `dispatch.rs`
4. El `pc-app` vuelve a su estado con solo controles individuales

## Dependencies

- `pc-app-ui` (ya implementado y archivado)
- Funciones `author_set_oven_enabled_command` y `author_set_target_temperature_command` existentes

## Success Criteria

- [ ] Panel bulk visible en UI (modo no headless)
- [ ] Selección por rango funciona correctamente
- [ ] "Select All" selecciona todos los hornos detectados
- [ ] Cada acción bulk encola los comandos individuales correctos
- [ ] Validación de rangos y temperatura previene envío de comandos inválidos
- [ ] Controles individuales siguen funcionando sin cambios
- [ ] Log registra comandos enviados en bloque
- [ ] Demo demostrable: 100 ovens, 2 grupos con diferentes temperaturas
