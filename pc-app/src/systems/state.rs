//! `FixedUpdate` schedule systems — state mutation.
//!
//! These systems read internal ECS events emitted by `ingest_inbound_protocol`
//! and mutate the oven read model (spawn/update entities, set components).

use bevy::prelude::{
    Commands, EventReader, Query, Res, ResMut,
};

use crate::components::{
    CommandResult, CurrentTemperature, Enabled, FaultInfo, FaultState, Heating, LastCommandResult,
    MaxTemperature, OvenId, OvenStatus, OutputRef, SensorRef, TargetTemperature,
};
use crate::events::{
    CommandAcceptedReceived, CommandRejectedReceived, FaultReceived, OvenDiscovered,
    OvenStatusReceived,
};
use crate::resources::{GlobalFault, OvenIndex};

/// Spawns a new oven entity on first detection, or updates components on redetection.
///
/// Uses `OvenIndex` for O(1) lookup. If the entity already exists, only
/// `MaxTemperature`, `SensorRef`, and `OutputRef` are updated (identity fields
/// may change on hardware re-detection).
pub fn apply_oven_detected(
    mut ev: EventReader<OvenDiscovered>,
    mut commands: Commands,
    mut oven_index: ResMut<OvenIndex>,
    mut query: Query<(
        &mut SensorRef,
        &mut OutputRef,
        &mut MaxTemperature,
    )>,
) {
    for event in ev.read() {
        if let Some(&entity) = oven_index.0.get(&event.oven_id) {
            // Redetection: update existing entity
            if let Ok((mut sensor, mut output, mut max_temp)) = query.get_mut(entity) {
                sensor.0 = event.sensor_ref.clone();
                output.0 = event.output_ref.clone();
                max_temp.0 = event.max_celsius;
            }
        } else {
            // First detection: spawn new entity with all components
            let entity = commands.spawn((
                OvenId(event.oven_id.clone()),
                SensorRef(event.sensor_ref.clone()),
                OutputRef(event.output_ref.clone()),
                CurrentTemperature::default(),
                TargetTemperature::default(),
                MaxTemperature(event.max_celsius),
                Enabled::default(),
                Heating::default(),
                OvenStatus(protocol::OvenState::Disabled),
                FaultState::default(),
                LastCommandResult::default(),
            )).id();
            oven_index.0.insert(event.oven_id.clone(), entity);
        }
    }
}

/// Updates oven components from `OvenStatusReceived` events.
///
/// Only processes events for ovens that exist in the `OvenIndex` (known ovens).
/// Unknown ovens are silently ignored — no new entity is spawned.
///
/// **Does NOT clear `FaultState`** — faults persist across status updates.
pub fn apply_oven_status_updated(
    mut ev: EventReader<OvenStatusReceived>,
    oven_index: Res<OvenIndex>,
    mut query: Query<(
        &OvenId,
        &mut CurrentTemperature,
        &mut TargetTemperature,
        &mut Enabled,
        &mut Heating,
        &mut OvenStatus,
    )>,
) {
    for event in ev.read() {
        if let Some(&entity) = oven_index.0.get(&event.oven_id) {
            if let Ok((
                _,
                mut current_temp,
                mut target_temp,
                mut enabled,
                mut heating,
                mut status,
            )) = query.get_mut(entity)
            {
                current_temp.0 = event.current_celsius;
                target_temp.0 = event.target_celsius;
                enabled.0 = event.enabled;
                heating.0 = event.heating;
                status.0 = event.state;
            }
        }
        // Unknown oven: silently ignored (no entity spawned)
    }
}

/// Sets `FaultState` on oven entities or stores global faults.
///
/// - If `oven_id` is `Some(id)`: sets `FaultState` on the matching entity.
/// - If `oven_id` is `None`: stores the fault in `GlobalFault` resource.
///
/// `FaultState` is NEVER cleared by status updates — only explicit fault
/// clearance (future) or entity removal clears it.
pub fn apply_fault_raised(
    mut ev: EventReader<FaultReceived>,
    oven_index: Res<OvenIndex>,
    mut global_fault: ResMut<GlobalFault>,
    mut query: Query<&mut FaultState>,
) {
    for event in ev.read() {
        let fault_info = FaultInfo {
            fault_code: event.fault_code,
            severity: event.severity,
            message: event.message.clone(),
        };

        match &event.oven_id {
            Some(oven_id) => {
                if let Some(&entity) = oven_index.0.get(oven_id) {
                    if let Ok(mut fault_state) = query.get_mut(entity) {
                        fault_state.0 = Some(fault_info);
                    }
                }
            }
            None => {
                // Global fault — stored in resource, not on any entity
                global_fault.0 = Some(fault_info);
            }
        }
    }
}

/// Records the last command result (accepted or rejected) on the matching oven entity.
///
/// Only processes events with `oven_id: Some(id)`. Events without an oven_id
/// (e.g., global `EmergencyStop` acceptances) are ignored for per-oven tracking.
pub fn record_command_result(
    mut ev_accepted: EventReader<CommandAcceptedReceived>,
    mut ev_rejected: EventReader<CommandRejectedReceived>,
    oven_index: Res<OvenIndex>,
    mut query: Query<&mut LastCommandResult>,
) {
    for event in ev_accepted.read() {
        if let Some(ref oven_id) = event.oven_id {
            if let Some(&entity) = oven_index.0.get(oven_id) {
                if let Ok(mut result) = query.get_mut(entity) {
                    result.0 = Some(CommandResult::Accepted {
                        accepted_type: event.accepted_type.clone(),
                        message: event.message.clone(),
                    });
                }
            }
        }
    }

    for event in ev_rejected.read() {
        if let Some(ref oven_id) = event.oven_id {
            if let Some(&entity) = oven_index.0.get(oven_id) {
                if let Ok(mut result) = query.get_mut(entity) {
                    result.0 = Some(CommandResult::Rejected {
                        rejected_type: event.rejected_type.clone(),
                        reason: event.reason,
                        message: event.message.clone(),
                    });
                }
            }
        }
    }
}
