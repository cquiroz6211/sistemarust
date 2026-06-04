# Proposal: ECS Demo Monitor for pc-app

## Intent

Pedagogical panel showing real-time ECS metrics: oven counts, bulk op stats, dispatch time, FPS, frame time, logged events/sec. App-level metrics, NOT Bevy internal scheduler.

## Scope

### In Scope
- Panel "ECS Demo Monitor" in left sidebar under bulk operations
- `EcsDemoMetrics` resource
- `update_ecs_demo_metrics` system reading ECS components per frame
- Grid render in `panels.rs` with pedagogical disclaimer
- Bulk command metrics in `ui_command_dispatch`
- Headless ECS count and bulk metric tests

### Out of Scope
- Bevy scheduler thread visualization
- Metric charts or history
- Performance alerts or thresholds
- ECS schedule modifications

## Capabilities

### New Capabilities
- `pc-app-ecs-demo-monitor`: Pedagogical panel showing real-time ECS metrics (oven counts, bulk stats, dispatch time, FPS, frame time, logged events/sec).

### Modified Capabilities
- None.

## Approach

1. **`EcsDemoMetrics` resource** in `resources.rs` — default-initialized, typed fields.
2. **`update_ecs_demo_metrics` system** — reads oven `Query`, calculates counts from `Time` and `EventLog`.
3. **`render_ecs_demo_monitor`** — 2-column egui grid with pedagogical note.
4. **Dispatch integration** — `ui_command_dispatch` records bulk op name, command count (queue delta), and dispatch time (elapsed).
5. **Headless tests** — ECS counts and bulk metrics in `ui_integration.rs`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `pc-app/src/resources.rs` | Modified | Add `EcsDemoMetrics` resource |
| `pc-app/src/systems/ui/monitor.rs` | New | `update_ecs_demo_metrics` system |
| `pc-app/src/systems/ui/dispatch.rs` | Modified | Register bulk command metrics |
| `pc-app/src/systems/ui/render.rs` | Modified | Integrate monitor panel |
| `pc-app/src/systems/ui/mod.rs` | Modified | Export monitor system |
| `pc-app/src/plugins/ui.rs` | Modified | Register monitor system |
| `pc-app/src/ui/panels.rs` | Modified | Add `render_ecs_demo_monitor` |
| `pc-app/tests/ui_integration.rs` | New | Headless metric tests |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|-------------|
| Operator confuses panel with Bevy scheduler | Low | Explicit pedagogical disclaimer |
| ECS read overhead | Negligible | Query already runs; only adds filters |
| FPS inaccurate if UI not at 60fps | Low | Calculated from `Time::delta()` — honest |

## Rollback Plan

1. Remove `EcsDemoMetrics` from `resources.rs`
2. Remove `monitor.rs` and its export
3. Remove `render_ecs_demo_monitor` from `panels.rs`
4. Remove metrics registration from `dispatch.rs`
5. Remove system registration from `ui.rs`
6. Remove metric tests from `ui_integration.rs`

## Dependencies

- `pc-app-bulk-oven-controls` — references bulk op metrics
- `EventLog` resource — for events/sec
- `Time` resource (Bevy) — for FPS/frame time

## Success Criteria

- [ ] Panel visible under bulk operations
- [ ] Counts reflect real ECS state
- [ ] Bulk op metrics update after each action
- [ ] Dispatch time measurable in microseconds
- [ ] FPS and frame time correct
- [ ] Logged events/sec calculated from EventLog
- [ ] 17 ui_integration tests pass
- [ ] Pedagogical disclaimer present
