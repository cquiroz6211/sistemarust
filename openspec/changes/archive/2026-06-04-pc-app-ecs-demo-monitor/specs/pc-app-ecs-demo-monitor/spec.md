# pc-app-ecs-demo-monitor Specification

## Purpose

Pedagogical real-time panel showing app-level ECS metrics — oven entity counts grouped by component state, bulk operation dispatch stats, frame timing, and event throughput. Explicitly NOT a Bevy scheduler visualization.

## Requirements

### Requirement: EcsDemoMetrics resource

The system MUST provide an `EcsDemoMetrics` Bevy resource with typed fields for: `total_ovens`, `enabled_ovens`, `heating_ovens`, `faulted_ovens`, `last_bulk_operation`, `last_commands_generated`, `last_dispatch_micros`, `fps`, `frame_time_ms`, `logged_events_per_second`, and `observed_log_entries`. The resource MUST be default-initialized with zero counts and `"None yet"` for the last bulk operation.

#### Scenario: Default initialization

- GIVEN a fresh ECS world
- WHEN `EcsDemoMetrics::default()` is inserted
- THEN `total_ovens` = 0, `enabled_ovens` = 0, `heating_ovens` = 0, `faulted_ovens` = 0
- AND `last_bulk_operation` = "None yet", `last_commands_generated` = 0, `last_dispatch_micros` = 0
- AND `fps` = 0.0, `frame_time_ms` = 0.0, `logged_events_per_second` = 0.0

### Requirement: Per-frame oven entity counting

The system MUST run `update_ecs_demo_metrics` each Update tick. It MUST query all entities with `(OvenId, Enabled, Heating, FaultState)` and populate `total_ovens` (all), `enabled_ovens` (where `Enabled.0 == true`), `heating_ovens` (where `Heating.0 == true`), and `faulted_ovens` (where `FaultState.0.is_some()`).

#### Scenario: Counts reflect real ECS state

- GIVEN 2 oven entities exist, one enabled+heating, one disabled
- WHEN `update_ecs_demo_metrics` runs
- THEN `total_ovens` = 2, `enabled_ovens` = 1, `heating_ovens` = 1, `faulted_ovens` = 0

#### Scenario: Zero ovens

- GIVEN no oven entities exist
- WHEN `update_ecs_demo_metrics` runs
- THEN all count fields are 0

### Requirement: Frame timing from Time resource

The system MUST calculate `frame_time_ms` as `Time::delta().as_secs_f32() * 1000.0` and `fps` as `1.0 / delta_seconds` (or 0.0 if delta is zero). These are app-level frame timings from Bevy's `Time` resource, NOT scheduler internals.

#### Scenario: FPS from non-zero delta

- GIVEN `Time::delta()` returns 16.67ms
- WHEN metrics update
- THEN `fps` ≈ 60.0, `frame_time_ms` ≈ 16.67

#### Scenario: Zero delta on first frame

- GIVEN `Time::delta()` returns 0.0
- WHEN metrics update
- THEN `fps` = 0.0, `frame_time_ms` = 0.0

### Requirement: Logged events per second

The system MUST track `observed_log_entries` from `EventLog::entries.len()` each frame. It MUST compute `logged_events_per_second` as `(current_entries - observed_log_entries) / delta_seconds` (or 0.0 if delta is zero). `observed_log_entries` MUST be updated to `current_entries` after each calculation.

#### Scenario: Events arriving mid-run

- GIVEN 10 new log entries arrived during a 0.05s delta
- WHEN metrics update
- THEN `logged_events_per_second` = 200.0

#### Scenario: No new events

- GIVEN no new log entries since last tick
- WHEN metrics update
- THEN `logged_events_per_second` = 0.0

### Requirement: Bulk dispatch metrics recording

The `ui_command_dispatch` system MUST record bulk operation metrics into `EcsDemoMetrics` on every bulk action dispatch: `last_bulk_operation` set to `BulkAction::label()`, `last_commands_generated` as `outbound_queue.len() - outbound_before`, and `last_dispatch_micros` as the elapsed `Instant::now()` duration. Failed validations and empty selections MUST also record metrics with descriptive suffixes and 0 commands.

#### Scenario: Successful bulk dispatch

- GIVEN 2 ovens selected with `EnableAndApplyTemperature` at 220.0°C
- WHEN dispatch completes
- THEN `last_bulk_operation` = "Enable and apply temperature", `last_commands_generated` = 4

#### Scenario: Validation failure records metrics

- GIVEN bulk selection with invalid range
- WHEN dispatch runs
- THEN `last_bulk_operation` contains "(validation failed)", `last_commands_generated` = 0

#### Scenario: No matching ovens

- GIVEN valid range but no ovens exist in it
- WHEN dispatch runs
- THEN `last_bulk_operation` contains "(no matching ovens)", `last_commands_generated` = 0

### Requirement: ECS Demo Monitor panel rendering

The system MUST render a panel titled "ECS Demo Monitor" inside the left sidebar below bulk operations. The panel MUST display a 2-column egui grid with all `EcsDemoMetrics` fields as label-value rows. The panel MUST include a pedagogical disclaimer stating these are app-level metrics, NOT Bevy scheduler internals.

#### Scenario: Panel shows all metrics

- GIVEN `EcsDemoMetrics` with non-zero values
- WHEN the UI renders the left sidebar
- THEN the grid shows rows for: Total ovens, Enabled, Heating, Faulted, Last bulk op, Commands, Dispatch (us), FPS, Frame time (ms), Logged events/sec

#### Scenario: Pedagogical disclaimer visible

- GIVEN the ECS Demo Monitor panel is rendered
- THEN the panel displays "This panel shows app-level metrics, not Bevy scheduler threads."

### Requirement: Headless ECS metric tests

The system MUST support headless testing of `EcsDemoMetrics` without GPU. Tests MUST spawn ovens via protocol ingestion, run `update_ecs_demo_metrics`, and assert count fields match ECS state. Bulk metric tests MUST verify `last_bulk_operation` and `last_commands_generated` after dispatch.

#### Scenario: Headless oven count test passes

- GIVEN 2 ovens spawned via `OvenDetected` envelopes, one with `OvenStatusUpdated` (enabled, heating)
- WHEN `update_ecs_demo_metrics` runs
- THEN `total_ovens` = 2, `enabled_ovens` = 1, `heating_ovens` = 1, `faulted_ovens` = 0

#### Scenario: Headless bulk metric test passes

- GIVEN 2 ovens spawned with range [1, 2] and `EnableAndApplyTemperature` at 220.0
- WHEN `ui_command_dispatch` runs
- THEN `last_bulk_operation` = "Enable and apply temperature", `last_commands_generated` = 4
