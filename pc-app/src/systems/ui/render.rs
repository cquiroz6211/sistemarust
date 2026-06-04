//! Main UI render system.
//!
//! Renders the full UI layout: header, bulk operations panel, oven cards, and event log panel.

use bevy::prelude::{Query, Res, ResMut};
use bevy_egui::{egui, EguiContexts};

use crate::components::{
    CurrentTemperature, Enabled, FaultState, Heating, MaxTemperature, OvenId, OvenStatus,
    TargetTemperature,
};
use crate::resources::{
    BulkSelection, BulkValidation, ConnectionState, EmergencyStopConfirm, EventLog, OvenEditStates,
    OvenIndex, TemperatureValidation, UiIntent,
};
use crate::ui::log_panel::render_event_log;
use crate::ui::panels::{render_bulk_panel, render_empty_state, render_header, render_oven_card};

pub fn ui_render(
    mut contexts: EguiContexts,
    oven_index: Res<OvenIndex>,
    mut connection_state: ResMut<ConnectionState>,
    event_log: Res<EventLog>,
    mut intent: ResMut<UiIntent>,
    mut edit_states: ResMut<OvenEditStates>,
    mut confirm: ResMut<EmergencyStopConfirm>,
    mut validation: ResMut<TemperatureValidation>,
    mut bulk_selection: ResMut<BulkSelection>,
    mut bulk_validation: ResMut<BulkValidation>,
    query: Query<(
        &OvenId,
        &CurrentTemperature,
        &TargetTemperature,
        &Enabled,
        &Heating,
        &OvenStatus,
        &FaultState,
        &MaxTemperature,
    )>,
) {
    let ctx = contexts.ctx_mut();

    // ── Header ────────────────────────────────────────────────────────────
    render_header(ctx, &mut connection_state, &mut intent, &mut confirm);

    // ── Event log side panel ──────────────────────────────────────────────
    egui::SidePanel::right("event_log_panel")
        .default_width(320.0)
        .show(ctx, |ui| {
            render_event_log(ui, &event_log);
        });

    // ── Check for empty state ─────────────────────────────────────────────
    let oven_ids: Vec<_> = oven_index.0.keys().cloned().collect();

    if oven_ids.is_empty() {
        render_empty_state(ctx);
        return;
    }

    // ── Bulk operations side panel (left) ─────────────────────────────────
    egui::SidePanel::left("bulk_panel")
        .default_width(240.0)
        .show(ctx, |ui| {
            render_bulk_panel(
                ui,
                oven_ids.len(),
                &mut bulk_selection,
                &mut bulk_validation,
                &connection_state,
            );
        });

    // ── Central panel: oven cards ─────────────────────────────────────────
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for oven_id_str in &oven_ids {
                    if let Some(entity) = oven_index.0.get(oven_id_str) {
                        if let Ok((
                            id,
                            current,
                            target,
                            enabled,
                            heating,
                            status,
                            fault,
                            max_temp,
                        )) = query.get(*entity)
                        {
                            render_oven_card(
                                ui,
                                id,
                                current,
                                target,
                                enabled,
                                heating,
                                status,
                                fault,
                                max_temp,
                                &mut edit_states,
                                &mut intent,
                                &connection_state,
                                &mut validation,
                            );
                        }
                    }
                }

                // ── Global controls at bottom ─────────────────────────────
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("Emergency Stop").color(egui::Color32::RED))
                        .clicked()
                    {
                        confirm.open = true;
                    }

                    if ui.button("Solicitar todos los estados").clicked() {
                        intent.request_status = Some(None);
                    }
                });
            });
    });
}
