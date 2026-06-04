# Tasks: ECS Demo Monitor for pc-app

## Change
`pc-app-ecs-demo-monitor`

## Summary
Implement a pedagogical panel showing real-time app-level ECS metrics: oven entity counts by component state, bulk operation dispatch stats, frame timing, and event throughput. Explicitly NOT a Bevy scheduler visualization.

## Work Unit 1 — Resource definition

> **Goal:** Define the `EcsDemoMetrics` resource with typed fields and safe defaults.

- [ ] **T1.1** Add `EcsDemoMetrics` struct to `pc-app/src/resources.rs` (after line 158)
  - Fields: `total_ovens`, `enabled_ovens`, `heating_ovens`, `faulted_ovens`, `last_bulk_operation`, `last_commands_generated`, `last_dispatch_micros`, `fps`, `frame_time_ms`, `logged_events_per_second`, `observed_log_entries`
  - Add `#[derive(Debug, Clone, Resource)]`
  - Implement `Default` with zero counts and `"None yet"` for `last_bulk_operation`
  - **Where:** `pc-app/src/resources.rs:159-191`

## Work Unit 2 — Monitor system

> **Goal:** Create `update_ecs_demo_metrics` system that reads ECS state per frame.

- [ ] **T2.1** Create `pc-app/src/systems/ui/monitor.rs`
  - System `update_ecs_demo_metrics` with deps: `Res<Time>`, `Res<EventLog>`, `ResMut<EcsDemoMetrics>`, `Query<(&OvenId, &Enabled, &Heating, &FaultState)>`
  - Counts oven groups: total (all), enabled (`Enabled.0 == true`), heating (`Heating.0 == true`), faulted (`FaultState.0.is_some()`)
  - Computes `frame_time_ms` from `Time::delta().as_secs_f32() * 1000.0`
  - Computes `fps` as `1.0 / delta_seconds` (0.0 if delta is zero)
  - Computes `logged_events_per_second` from `EventLog::entries.len()` delta
  - Updates `observed_log_entries` for next-frame delta tracking
  - **Where:** `pc-app/src/systems/ui/monitor.rs` (36 lines)

- [ ] **T2.2** Export monitor module in `pc-app/src/systems/ui/mod.rs`
  - Add `pub mod monitor;`
  - **Where:** `pc-app/src/systems/ui/mod.rs:4`

## Work Unit 3 — Bulk command metrics integration

> **Goal:** Record bulk operation metrics in `ui_command_dispatch`.

- [ ] **T3.1** Modify `pc-app/src/systems/ui/dispatch.rs` — bulk dispatch section
  - Import `Instant` for timing measurement
  - Wrap bulk dispatch block with `dispatch_start = Instant::now()`
  - On validation failure: record `last_bulk_operation` with `"(validation failed)"` suffix, `last_commands_generated = 0`, `last_dispatch_micros`
  - On no matching ovens: record `last_bulk_operation` with `"(no matching ovens)"` suffix, `last_commands_generated = 0`, `last_dispatch_micros`
  - On success: record `last_bulk_operation = action.label()`, `last_commands_generated = outbound.len() - outbound_before`, `last_dispatch_micros`
  - **Where:** `pc-app/src/systems/ui/dispatch.rs:53-124`

## Work Unit 4 — Panel rendering

> **Goal:** Render the ECS Demo Monitor panel in the left sidebar.

- [ ] **T4.1** Add `render_ecs_demo_monitor` function to `pc-app/src/ui/panels.rs`
  - Function signature: `pub fn render_ecs_demo_monitor(ui: &mut egui::Ui, metrics: &EcsDemoMetrics)`
  - Adds separator, heading "ECS Demo Monitor" with `bulk_header_color()`
  - Pedagogical note: "Systems process groups of oven entities each frame."
  - Disclaimer: "This panel shows app-level metrics, not Bevy scheduler threads."
  - 2-column `egui::Grid` with `[8.0, 4.0]` spacing showing all metric fields
  - Add `metric_row` helper function for label-value rows
  - **Where:** `pc-app/src/ui/panels.rs:394-438`

- [ ] **T4.2** Import `EcsDemoMetrics` and `render_ecs_demo_monitor` in `pc-app/src/ui/panels.rs`
  - Add `EcsDemoMetrics` to resources import
  - **Where:** `pc-app/src/ui/panels.rs:6`

## Work Unit 5 — Panel integration into render system

> **Goal:** Wire the monitor panel into the main render pipeline.

- [ ] **T5.1** Modify `pc-app/src/systems/ui/render.rs`
  - Import `EcsDemoMetrics` from resources
  - Import `render_ecs_demo_monitor` from `crate::ui::panels`
  - Call `render_ecs_demo_monitor(ui, &metrics)` in the bulk panel section, after bulk operations controls
  - **Where:** `pc-app/src/systems/ui/render.rs:13,18,76`

## Work Unit 6 — Plugin registration

> **Goal:** Register the monitor system and resource in the UiPlugin.

- [ ] **T6.1** Modify `pc-app/src/plugins/ui.rs`
  - Add `EcsDemoMetrics` to resources import
  - Add `use crate::systems::ui::monitor::update_ecs_demo_metrics;`
  - Insert `EcsDemoMetrics::default()` as a resource
  - Register `update_ecs_demo_metrics` in the Update schedule
  - Update file header comment to document the new system
  - **Where:** `pc-app/src/plugins/ui.rs:12,18,34,52`

## Work Unit 7 — Headless integration tests

> **Goal:** Write headless tests for ECS metrics without GPU.

- [ ] **T7.1** Add tests to `pc-app/tests/ui_integration.rs`
  - `ecs_demo_metrics_count_oven_groups`: Spawn 2 ovens, set one enabled+heating, assert counts (total=2, enabled=1, heating=1, faulted=0)
  - `ecs_demo_metrics_record_bulk_dispatch`: Spawn 2 ovens, set bulk selection for range [1,2], tick, assert `last_bulk_operation` and `last_commands_generated = 4`
  - Helper functions: `spawn_oven`, `push_envelope`, `tick` (verify existing or add)
  - **Where:** `pc-app/tests/ui_integration.rs:328-375`

## Verification

```bash
cargo test --package pc-app --test ui_integration
```

Expected: 17/17 ui_integration tests pass.

## Work Unit Dependencies

```
Unit 1 (Resource) ──┬──> Unit 2 (Monitor system) ──┬──> Unit 6 (Plugin registration)
                    │                                │
                    └────────────────────────────────┘
Unit 3 (Dispatch metrics) ──┐
                            ├──> Unit 6 (Plugin registration)
Unit 4 (Panel rendering) ───┤
                            ├──> Unit 5 (Render integration) ──> Unit 6
Unit 7 (Tests) ─────────────┴──> Runs after all units complete
```

## Review Checklist

- [ ] `EcsDemoMetrics` has all 11 fields from the spec, typed correctly
- [ ] Default impl matches spec: zero counts, `"None yet"` for last bulk op
- [ ] Monitor system queries ALL ovens for total, then FILTERS for each subgroup (4 separate iter calls)
- [ ] FPS calculation handles zero-delta edge case
- [ ] Events/sec uses saturating_sub to avoid underflow
- [ ] Dispatch metrics record on ALL paths: success, validation failure, no matching ovens
- [ ] Pedagogical disclaimer text matches spec exactly
- [ ] Grid uses 2 columns with `[8.0, 4.0]` spacing
- [ ] System ordering: `ui_command_dispatch` → `update_ecs_demo_metrics` → `ui_render`
- [ ] All 17 ui_integration tests pass
