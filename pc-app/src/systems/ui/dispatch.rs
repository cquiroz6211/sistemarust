//! `Update` schedule system — UI command dispatch.
//!
//! Reads `UiIntent` and translates it into protocol commands using the
//! existing authoring functions. Also processes `BulkSelection` for batch
//! operations across multiple ovens. Resets both resources after processing.

use bevy::prelude::{Res, ResMut};
use std::time::Instant;

use protocol::RequestScope;

use crate::resources::{
    BulkAction, BulkSelection, BulkValidation, EcsDemoMetrics, OutboundProtocolQueue, OvenIndex,
    UiIntent,
};
use crate::resources::{resolve_selected_ovens, validate_bulk_selection};
use crate::systems::commands::{
    author_emergency_stop_command, author_request_status_command,
    author_set_oven_enabled_command, author_set_target_temperature_command,
};

pub fn ui_command_dispatch(
    mut intent: ResMut<UiIntent>,
    mut bulk_selection: ResMut<BulkSelection>,
    mut bulk_validation: ResMut<BulkValidation>,
    mut metrics: ResMut<EcsDemoMetrics>,
    mut outbound: ResMut<OutboundProtocolQueue>,
    oven_index: Res<OvenIndex>,
) {
    // ── Individual command dispatch ───────────────────────────────────────
    if let Some((oven_id, enabled)) = intent.set_enabled.take() {
        author_set_oven_enabled_command(oven_id, enabled, &mut outbound);
    }

    if let Some((oven_id, temp)) = intent.set_temperature.take() {
        author_set_target_temperature_command(oven_id, temp, &mut outbound);
    }

    if let Some(maybe_oven_id) = intent.request_status.take() {
        let scope = if maybe_oven_id.is_some() {
            RequestScope::Single
        } else {
            RequestScope::All
        };
        author_request_status_command(maybe_oven_id, scope, &mut outbound);
    }

    if intent.emergency_stop {
        intent.emergency_stop = false;
        author_emergency_stop_command("Activado desde UI".to_string(), &mut outbound);
    }

    // ── Bulk command dispatch ─────────────────────────────────────────────
    if bulk_selection.action != BulkAction::None {
        let dispatch_start = Instant::now();
        let action = bulk_selection.action;

        // 1. Validate
        let errors = validate_bulk_selection(&bulk_selection, &oven_index);
        if !errors.is_empty() {
            bulk_validation.errors = errors;
            metrics.last_bulk_operation = format!("{} (validation failed)", action.label());
            metrics.last_commands_generated = 0;
            metrics.last_dispatch_micros = dispatch_start.elapsed().as_micros();
            bulk_selection.action = BulkAction::None;
            return;
        }
        bulk_validation.errors.clear();

        // 2. Resolve selected ovens
        let selected = resolve_selected_ovens(&oven_index, &bulk_selection);

        if selected.is_empty() {
            bulk_validation.errors = vec!["No hay hornos en el rango seleccionado".into()];
            metrics.last_bulk_operation = format!("{} (no matching ovens)", action.label());
            metrics.last_commands_generated = 0;
            metrics.last_dispatch_micros = dispatch_start.elapsed().as_micros();
            bulk_selection.action = BulkAction::None;
            return;
        }

        // 3. Dispatch commands per oven, sorted by index
        let outbound_before = outbound.0.len();
        for (oven_id, _idx) in &selected {
            match action {
                BulkAction::Enable => {
                    author_set_oven_enabled_command(oven_id.to_string(), true, &mut outbound);
                }
                BulkAction::Disable => {
                    author_set_oven_enabled_command(oven_id.to_string(), false, &mut outbound);
                }
                BulkAction::ApplyTemperature => {
                    author_set_target_temperature_command(
                        oven_id.to_string(),
                        bulk_selection.target_temp,
                        &mut outbound,
                    );
                }
                BulkAction::EnableAndApplyTemperature => {
                    author_set_oven_enabled_command(oven_id.to_string(), true, &mut outbound);
                    author_set_target_temperature_command(
                        oven_id.to_string(),
                        bulk_selection.target_temp,
                        &mut outbound,
                    );
                }
                BulkAction::RequestStatus => {
                    author_request_status_command(
                        Some(oven_id.to_string()),
                        RequestScope::Single,
                        &mut outbound,
                    );
                }
                BulkAction::None => {}
            }
        }

        metrics.last_bulk_operation = action.label().to_string();
        metrics.last_commands_generated = outbound.0.len().saturating_sub(outbound_before);
        metrics.last_dispatch_micros = dispatch_start.elapsed().as_micros();

        // 4. Reset selection action
        bulk_selection.action = BulkAction::None;
    }
}
