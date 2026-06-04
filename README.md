# Sistema de Control de Hornos

Proyecto Rust + Bevy para controlar hornos desde una aplicación de PC y una Raspberry Pi simulada. La PC muestra una UI de operación, mientras la Raspberry mantiene el estado de los hornos y responde por TCP.

## Inicio rápido

Ejecutá estos comandos desde la raíz del proyecto.

### 1. Levantar la Raspberry Pi simulada

En una terminal:

```powershell
cargo run --package rpi-controller -- --simulate 3 --listen 127.0.0.1:7000
```

Esto crea 3 hornos simulados y deja la Raspberry escuchando conexiones TCP en `127.0.0.1:7000`.

### 2. Levantar la PC con interfaz gráfica

En otra terminal:

```powershell
cargo run --package pc-app -- --connect 127.0.0.1:7000
```

La UI debería mostrar estado `Connected`, las tarjetas de hornos, controles de temperatura y el log de eventos.

## Probar el flujo completo

1. Iniciá primero `rpi-controller`.
2. Iniciá después `pc-app` con `--connect`.
3. En la UI de PC:
   - Usá `Encender horno` / `Apagar horno` para habilitar o deshabilitar un horno.
   - Ajustá temperatura con `-10`, `-1`, `+1`, `+10` o el slider.
   - Presioná `Aplicar temperatura` para enviar el nuevo objetivo.
   - Usá `Solicitar estado` o `Refresh All` para pedir estado actualizado.

## Modo headless

Para ejecutar la PC sin ventana gráfica:

```powershell
cargo run --package pc-app -- --headless
```

Este modo mantiene los sistemas ECS sin iniciar la UI.

## Tests

```powershell
cargo test --workspace
```

Para validar solo la app de PC:

```powershell
cargo test --package pc-app
```

## Notas importantes

- Si la UI muestra `Disconnected`, verificá que `pc-app` se inició con `--connect 127.0.0.1:7000`.
- Si no aparecen hornos, verificá que `rpi-controller` se inició con `--simulate N` y `--listen 127.0.0.1:7000`.
- Si activás `Emergency Stop`, el protocolo actual no tiene comando de rearme. Para salir de emergencia, reiniciá el `rpi-controller`.

## Crates principales

| Crate | Rol |
|---|---|
| `protocol` | Mensajes compartidos entre PC y Raspberry |
| `transport` | Transporte TCP localhost entre procesos |
| `rpi-controller` | Controlador Bevy headless con hornos simulados |
| `pc-app` | Aplicación PC con UI Bevy + egui |
