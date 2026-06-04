# Design: ECS Demo Monitor for pc-app

## Intent

Retroactive technical design for the **ECS Demo Monitor** — a pedagogical panel that displays real-time metrics about how Bevy ECS processes oven entities. This is **app-level metrics only**, explicitly not a visualization of Bevy's internal scheduler.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    UiPlugin (ui.rs)                      │
│                                                          │
│  Update Schedule:                                        │
│  ┌──────────────────┐    ┌──────────────────────────┐   │
│  │ ui_command_      │ →  │ update_ecs_demo_metrics  │   │
│  │ dispatch         │    │ (monitor.rs)             │   │
│  │ (dispatch.rs)    │    │                          │   │
│  │                  │    │ Reads:                   │   │
│  │  - UiIntent      │    │   - Query<oven entities> │   │
│  │  - BulkSelection │    │   - Res<Time>            │   │
│  │  - EcsDemoMetrics│    │   - Res<EventLog>        │   │
│  │    (writes)      │    │                          │   │
│  └────────┬─────────┘    └──────────┬───────────────┘   │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ui_render (render.rs) → EguiPrimaryContextPass  │   │
│  │   → render_ecs_demo_monitor (panels.rs)         │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Component Interactions

### Data Flow

```
[Oven entities in ECS world]
        │
        │ Query<(&OvenId, &Enabled, &Heating, &FaultState)>
        ▼
┌─────────────────────────┐
│ update_ecs_demo_metrics │  ← Update schedule, runs every frame
│ (monitor.rs:8-36)       │
│                         │
│ - Counts oven groups    │
│ - Computes FPS/frame    │
│ - Derives events/sec    │
│ - Writes EcsDemoMetrics │
└───────────┬─────────────┘
            │ ResMut<EcsDemoMetrics>
            ▼
┌─────────────────────────┐
│ ui_command_dispatch     │  ← Update schedule, runs BEFORE monitor
│ (dispatch.rs:22-125)    │
│                         │
│ - Processes UiIntent    │
│ - Processes BulkSelection│
│ - Writes bulk metrics:  │
│   last_bulk_operation   │
│   last_commands_generated│
│   last_dispatch_micros  │
└─────────────────────────┘
            │
            ▼
┌─────────────────────────┐
│ ui_render               │  ← EguiPrimaryContextPass
│ (render.rs:21-132)      │
│                         │
│ - Reads Res<EcsDemoMetrics>│
│ - Calls render_ecs_     │
│   demo_monitor(panels)  │
└─────────────────────────┘
```

### Execution Order Guarantee

The test file (`ui_integration.rs:48-51`) explicitly orders the systems:

```rust
app.add_systems(Update, ui_command_dispatch);
app.add_systems(Update, update_ecs_demo_metrics.after(ui_command_dispatch));
```

This ensures `EcsDemoMetrics` reflects the bulk dispatch results from the same frame.

## Resource Design

### `EcsDemoMetrics` (resources.rs:159-191)

```rust
pub struct EcsDemoMetrics {
    // Oven group counts — computed from Query per frame
    pub total_ovens: usize,
    pub enabled_ovens: usize,
    pub heating_ovens: usize,
    pub faulted_ovens: usize,

    // Bulk operation metrics — written by ui_command_dispatch
    pub last_bulk_operation: String,
    pub last_commands_generated: usize,
    pub last_dispatch_micros: u128,

    // Performance metrics — computed from Time resource
    pub fps: f32,
    pub frame_time_ms: f32,

    // Event log metrics — computed from EventLog resource
    pub logged_events_per_second: f32,
    pub observed_log_entries: usize,
}
```

**Design decisions:**

1. **Typed fields, not a generic map** — Each metric has a named field for self-documentation and type safety.
2. **`u128` for dispatch time** — Microsecond precision on 64-bit systems; `Instant::elapsed().as_micros()` returns `u128`.
3. **`observed_log_entries` as delta counter** — Stores last frame's count to compute per-second rate without storing timestamps.
4. **Default initializes to zero** — Safe display even before first frame populates values.

### System Dependencies

| System | Depends On | Runs In |
|--------|-----------|---------|
| `update_ecs_demo_metrics` | `Time`, `EventLog`, `EcsDemoMetrics`, oven Query | `Update` |
| `ui_command_dispatch` | `UiIntent`, `BulkSelection`, `EcsDemoMetrics`, `OutboundProtocolQueue`, `OvenIndex` | `Update` |
| `ui_render` | `EcsDemoMetrics`, all UI resources, oven Query | `EguiPrimaryContextPass` (via `Update`) |

## System Scheduling

All three systems run in the `Update` schedule (Egui render happens after `BeginPass` in `PreUpdate`):

1. **`ui_command_dispatch`** — First in Update. Processes pending UI intents and bulk operations. Writes bulk metrics to `EcsDemoMetrics`.
2. **`update_ecs_demo_metrics`** — Runs `.after(ui_command_dispatch)`. Reads oven Query, computes frame-level metrics, overwrites `EcsDemoMetrics` with fresh counts.
3. **`ui_render`** — Runs in `Update` (after egui BeginPass). Reads `EcsDemoMetrics` and renders the panel.

The ordering ensures that when the panel renders, `EcsDemoMetrics` contains:
- Oven counts from the current frame's Query
- Bulk metrics from the current frame's dispatch
- FPS/frame time from the current frame's `Time::delta()`

## Panel Layout

The monitor panel renders inside the left `SidePanel` (`bulk_panel`), **below** the bulk operations controls:

```
┌─────────────────────┐
│ Bulk Operations     │
│ [controls...]       │
│                     │
│ ─────────────────── │
│ ECS Demo Monitor    │  ← Pedagogical header
│ Systems process...  │  ← Pedagogical note
│ Not Bevy scheduler  │  ← Honest disclaimer
│                     │
│ ┌────────┬────────┐ │
│ │ Total  │   3    │ │  ← 2-column egui Grid
│ │ Enabled│   2    │ │
│ │ Heating│   1    │ │
│ │ Faulted│   0    │ │
│ │ Last   │ Enable │ │
│ │ Cmds   │   6    │ │
│ │ Dispatch│ 142us │ │
│ │ FPS    │  60.0  │ │
│ │ Frame  │ 16.67  │ │
│ │ Events │  2.5/s │ │
│ └────────┴────────┘ │
└─────────────────────┘
```

The `metric_row` helper renders each key-value pair as a label + bold value, using `egui::Grid` with 2 columns and tight spacing (`[8.0, 4.0]`).

## Test Strategy

### Headless Integration Tests (ui_integration.rs)

The test harness (`build_test_app()`) constructs a minimal `App` with:
- `MinimalPlugins` (no GPU/render plugins)
- `PcAppPlugin` (core ECS systems)
- Manual insertion of all UI resources
- Only data-flow systems: `log_capture_events`, `ui_command_dispatch`, `update_ecs_demo_metrics`

### Monitor-Specific Tests

**`ecs_demo_metrics_count_oven_groups`** (line 329):
- Spawns 2 ovens via protocol messages
- Sends `OvenStatusUpdated` to set one oven as enabled+heating
- Asserts: `total_ovens=2`, `enabled_ovens=1`, `heating_ovens=1`, `faulted_ovens=0`
- Proves the Query correctly filters by component state

**`ecs_demo_metrics_record_bulk_dispatch`** (line 357):
- Spawns 2 ovens, sets `BulkSelection` for EnableAndApplyTemperature on range 1-2
- Ticks once (triggers dispatch → monitor)
- Asserts: `last_bulk_operation="Enable and apply temperature"`, `last_commands_generated=4`
- Proves the dispatch→metrics pipeline (2 ovens × 2 commands each = 4)

### Test Coverage Gaps

- FPS/frame time tests would require controlling `Time` — not feasible in headless tests without `TimePlugin`
- Logged events/sec tests would require controlling `Time::delta()` — same limitation
- These metrics are implicitly tested via the integration of `update_ecs_demo_metrics` with `Time` and `EventLog`

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Operator confuses panel with Bevy scheduler | Low | Explicit pedagogical disclaimer in panel header |
| Query iteration overhead | Negligible | Query already runs in `ui_render`; monitor runs same filter |
| Multiple `ovens.iter()` calls in monitor | Low | 4 separate iter calls; acceptable for demo scale (<100 ovens) |
| FPS inaccurate at non-60fps | Low | Honest calculation from `Time::delta()` — no smoothing |

## Files

| File | Role |
|------|------|
| `pc-app/src/resources.rs:159-191` | `EcsDemoMetrics` resource definition |
| `pc-app/src/systems/ui/monitor.rs` | `update_ecs_demo_metrics` system |
| `pc-app/src/systems/ui/dispatch.rs:53-124` | Bulk metrics recording in `ui_command_dispatch` |
| `pc-app/src/systems/ui/render.rs:76` | Panel integration point |
| `pc-app/src/systems/ui/mod.rs:4` | Module export |
| `pc-app/src/plugins/ui.rs:18,34,52` | System and resource registration |
| `pc-app/src/ui/panels.rs:394-438` | `render_ecs_demo_monitor` + `metric_row` |
| `pc-app/tests/ui_integration.rs:328-375` | Headless metric tests |
