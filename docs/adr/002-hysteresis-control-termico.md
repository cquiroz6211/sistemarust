# ADR 002 — Hysteresis de 5°C en Control Térmico del rpi-controller

## Estado

Aceptado

## Contexto

El `rpi-controller` debe decidir cuándo activar o desactivar la salida de calor de cada horno. La decisión más simple es un control de umbral puro:

```
si current < target → heating = true
si current >= target → heating = false
```

Este enfoque, aunque funcional en papel, presenta un problema grave en sistemas de control térmico reales: la temperatura oscila naturalmente alrededor del valor objetivo, lo que provoca ciclos rápidos de encendido/apagado del actuador (relay o resistencia).

Se evaluaron tres estrategias de control térmico para la v1 del `rpi-controller`:

1. **Threshold puro** (sin hysteresis): Encender/apagar exactamente al cruzar el target.
2. **Hysteresis (bang-bang)**: Usar una banda muerta alrededor del target.
3. **PID (Proporcional-Integral-Derivativo)**: Control continuo de potencia.

## Decisión

Se adopta la **Opción 2: Control por Hysteresis con banda de ±5°C**.

### Regla de control

```
Banda de hysteresis = 5°C

Si current_celsius < (target_celsius - 5.0) → heating = true
Si current_celsius >= target_celsius → heating = false
Entre (target - 5.0) y target → mantener estado anterior (no cambiar)
```

### Ejemplo con target = 150°C

| Temperatura actual | Acción | Estado heating |
|---|---|---|
| 140°C | Prender | `true` |
| 145°C | Prender | `true` |
| 147°C | Mantener | `true` (estaba prendido) |
| 150°C | Apagar | `false` |
| 148°C | Mantener | `false` (estaba apagado) |
| 144°C | Prender | `true` |

## Consecuencias

### Positivas

- **Protección del hardware**: Reduce los ciclos de encendido/apagado del relay de "cada pocos segundos" a "cada varios minutos". Los relays electromecánicos tienen vida útil limitada por número de ciclos (~100,000 operaciones).
- **Menor estrés en resistencia calefactora**: Cada encendido genera un pico de corriente (inrush) que degrada la resistencia con el tiempo.
- **Estándar de la industria**: Termostatos domésticos (Nest, Honeywell), controladores industriales, y librerías de Arduino usan hysteresis como buena práctica documentada.
- **Trivial de implementar**: Solo requiere dos comparaciones y un estado previo.
- **No afecta el flujo de eventos**: La PC sigue recibiendo `OvenStatusUpdated` con la misma frecuencia. Solo cambia la lógica interna de decisión.

### Negativas

- **Menor precisión**: El horno opera en una banda de ±5°C alrededor del target, no exactamente en el target.
- **Oscilación visible**: El operador verá que la temperatura "rebota" entre `target - 5` y `target`.
- **No escalable a control fino**: Para procesos que requieren ±0.5°C, se necesitará PID en el futuro.

## Alternativas consideradas

### Opción A: Threshold puro (sin hysteresis)

```rust
if current < target { heating = true } else { heating = false }
```

**Rechazada porque**: En un sistema real, la temperatura oscila por inercia térmica y ruido de sensor. Sin hysteresis, el relay haría click cada 1-2 segundos cuando la temperatura está cerca del target, destruyendo el hardware en días.

### Opción C: PID (Proporcional-Integral-Derivativo)

Control continuo que calcula un nivel de potencia (0-100%) en lugar de prender/apagar.

**Rechazada porque**:
- El PRD explícitamente lista "Tuning avanzado de PID" como **out of scope**.
- Requiere 3 constantes (Kp, Ki, Kd) que necesitan calibración empírica.
- Agrega complejidad sin valor para la v1, cuyo objetivo es demostrar el flujo de eventos PC↔Raspberry.
- Incluso con PID, si el actuador final es un relay (on/off), se suele agregar una capa de hysteresis al final de todos modos.

## Notas

- El valor de 5°C fue elegido como un compromiso entre protección del hardware y precisión aceptable para un horno industrial típico. Este valor puede ajustarse en el futuro sin romper el protocolo (es una constante interna del controller).
- Cuando se implemente PID en una etapa posterior, la hysteresis puede mantenerse como capa de "output stage" que traduce el valor continuo del PID a señal discreta on/off, protegiendo el relay.
- La simulación de temperatura en v1 debe incluir este comportamiento: cuando `heating = true`, la temperatura sube; cuando `heating = false`, baja por disipación. La tasa de cambio debe ser lenta (inercia térmica) para que la hysteresis sea observable y testeable.

## Referencias

- `docs/prd-sistema-control-hornos.md` — PRD (sección Out of Scope: PID avanzado)
- `docs/panorama-app-ecs.md` — Visión ECS
- `openspec/changes/rpi-controller/proposal.md` — Propuesta del change
- Context7: "HAL Design Patterns" — The Embedded Rust Book
- Context7: "Command Pattern" — Rust Patterns
- Práctica estándar en termostatos IoT y controladores industriales
