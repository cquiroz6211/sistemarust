//! UI-facing ECS demo metrics.

use bevy::prelude::{Query, Res, ResMut, Time};

use crate::components::{Enabled, FaultState, Heating, OvenId};
use crate::resources::{EcsDemoMetrics, EventLog};

pub fn update_ecs_demo_metrics(
    time: Res<Time>,
    event_log: Res<EventLog>,
    mut metrics: ResMut<EcsDemoMetrics>,
    ovens: Query<(&OvenId, &Enabled, &Heating, &FaultState)>,
) {
    let delta_seconds = time.delta().as_secs_f32();

    metrics.total_ovens = ovens.iter().count();
    metrics.enabled_ovens = ovens.iter().filter(|(_, enabled, _, _)| enabled.0).count();
    metrics.heating_ovens = ovens.iter().filter(|(_, _, heating, _)| heating.0).count();
    metrics.faulted_ovens = ovens.iter().filter(|(_, _, _, fault)| fault.0.is_some()).count();

    metrics.frame_time_ms = delta_seconds * 1_000.0;
    metrics.fps = if delta_seconds > 0.0 {
        1.0 / delta_seconds
    } else {
        0.0
    };

    let current_entries = event_log.entries.len();
    let new_entries = current_entries.saturating_sub(metrics.observed_log_entries);
    metrics.logged_events_per_second = if delta_seconds > 0.0 {
        new_entries as f32 / delta_seconds
    } else {
        0.0
    };
    metrics.observed_log_entries = current_entries;
}
