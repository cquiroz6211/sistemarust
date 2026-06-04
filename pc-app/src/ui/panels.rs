//! UI panel rendering functions.

use bevy_egui::egui;

use crate::resources::{BulkAction, BulkSelection, BulkValidation, ConnectionState, EmergencyStopConfirm, OvenEditStates, TemperatureValidation, UiIntent};
use crate::ui::controls::{render_enabled_toggle, render_status_request, render_temperature_editor};
use crate::ui::styles::{
    bulk_header_color, card_background, card_border, disconnected_color, format_temp, oven_state_color, oven_state_label,
};
use crate::components::{
    CurrentTemperature, Enabled, FaultState, Heating, MaxTemperature, OvenId, OvenStatus,
    TargetTemperature,
};

/// Renders the top header panel with title, connection indicator, and global controls.
pub fn render_header(
    ctx: &egui::Context,
    connection_state: &mut ConnectionState,
    intent: &mut UiIntent,
    confirm: &mut EmergencyStopConfirm,
) {
    egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Control de Hornos");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Global controls on the right
                if ui.button("Refresh All").clicked() {
                    intent.request_status = Some(None);
                }

                if ui
                    .button(
                        egui::RichText::new("Emergency Stop").color(egui::Color32::RED),
                    )
                    .clicked()
                {
                    confirm.open = true;
                }

                // Connection indicator
                match connection_state {
                    ConnectionState::Connected => {
                        ui.colored_label(
                            crate::ui::styles::connected_color(),
                            "Conectado",
                        );
                    }
                    ConnectionState::Disconnected => {
                        ui.colored_label(disconnected_color(), "Desconectado");
                    }
                }
            });
        });

        // Disconnected warning banner
        if *connection_state == ConnectionState::Disconnected {
            ui.colored_label(
                disconnected_color(),
                "⚠ Sin conexión — los comandos no llegarán al RPi",
            );
        }
    });

    // ── Emergency Stop confirmation dialog ──
    if confirm.open {
        egui::Window::new("Confirmar Emergency Stop")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 50, 50),
                    "¿Está seguro que desea activar Emergency Stop?",
                );
                ui.label("Esto detendrá inmediatamente todos los hornos.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("Confirmar").strong().color(egui::Color32::RED))
                        .clicked()
                    {
                        intent.emergency_stop = true;
                        confirm.open = false;
                    }
                    if ui.button("Cancelar").clicked() {
                        confirm.open = false;
                    }
                });
            });
    }
}

/// Renders a single oven card with all operator-relevant information.
pub fn render_oven_card(
    ui: &mut egui::Ui,
    oven_id: &OvenId,
    current: &CurrentTemperature,
    target: &TargetTemperature,
    enabled: &Enabled,
    heating: &Heating,
    status: &OvenStatus,
    fault: &FaultState,
    max_temp: &MaxTemperature,
    edit_states: &mut OvenEditStates,
    intent: &mut UiIntent,
    connection_state: &ConnectionState,
    validation: &mut TemperatureValidation,
) {
    let is_emergency = status.0 == protocol::OvenState::EmergencyStopped;
    let is_faulted = fault.0.is_some();
    let is_off = status.0 == protocol::OvenState::Disabled;
    let controls_disabled = is_emergency || is_faulted;

    // Card frame using Frame::none() + manual styling (egui 0.29 compatible)
    egui::Frame::none()
        .fill(card_background())
        .rounding(8.0)
        .inner_margin(12.0)
        .outer_margin(4.0)
        .stroke(egui::Stroke::new(1.0, card_border()))
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_min_width(ui.available_width());

            // ── Title row: Oven ID + state badge ──
            ui.horizontal(|ui: &mut egui::Ui| {
                ui.heading(&oven_id.0);

                let state_text = oven_state_label(&status.0);
                let state_color = oven_state_color(&status.0);
                ui.label(
                    egui::RichText::new(state_text)
                        .color(state_color)
                        .strong()
                        .size(14.0),
                );

                if heating.0 {
                    ui.label(
                        egui::RichText::new("🔥 Calentando")
                            .color(egui::Color32::from_rgb(220, 180, 30))
                            .size(12.0),
                    );
                }
            });

            ui.separator();

            // ── Temperature display ──
            ui.horizontal(|ui: &mut egui::Ui| {
                // Current temperature — prominent
                ui.label(
                    egui::RichText::new(format!("Actual: {}", format_temp(current.0)))
                        .size(18.0)
                        .strong(),
                );

                ui.separator();

                // Target temperature
                ui.label(
                    egui::RichText::new(format!("Objetivo: {}", format_temp(target.0)))
                        .size(16.0),
                );

                ui.separator();

                // Max temperature
                ui.label(
                    egui::RichText::new(format!("Máx: {}", format_temp(max_temp.0)))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(160, 160, 160)),
                );
            });

            // ── Emergency warning ──
            if is_emergency {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 80, 80),
                    "⚠ Bloqueado por emergencia — Reinicie el controlador RPi para salir de emergencia",
                );
            }

            // ── Fault details ──
            if let Some(info) = &fault.0 {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 50, 50),
                    format!("Falla: {:?} ({:?}) — {}", info.fault_code, info.severity, info.message),
                );
            }

            ui.separator();

            // ── Controls ──
            ui.horizontal(|ui: &mut egui::Ui| {
                // Enable/disable toggle
                render_enabled_toggle(
                    ui,
                    &oven_id.0,
                    enabled.0,
                    controls_disabled || *connection_state == ConnectionState::Disconnected,
                    intent,
                );

                ui.separator();

                // Status request
                render_status_request(ui, &oven_id.0, intent);
            });

            // ── Temperature editor ──
            if !is_off {
                render_temperature_editor(
                    ui,
                    &oven_id.0,
                    target.0,
                    max_temp.0,
                    edit_states,
                    intent,
                    validation,
                    controls_disabled || *connection_state == ConnectionState::Disconnected,
                );
            } else {
                ui.label(
                    egui::RichText::new("Encienda el horno para ajustar la temperatura")
                        .italics()
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
            }
        });
}

/// Renders instructions when no ovens are detected.
pub fn render_empty_state(ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.heading("Sin hornos detectados");
            ui.add_space(12.0);
            ui.label("Para iniciar, ejecute en terminales separadas:");
            ui.add_space(8.0);

            // RPi simulator command
            ui.label(
                egui::RichText::new("RPi (simulador):")
                    .strong(),
            );
            ui.label(
                egui::RichText::new("cargo run --package rpi-controller -- --simulate 3 --listen 127.0.0.1:7000")
                    .monospace()
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );

            ui.add_space(8.0);

            // PC command
            ui.label(
                egui::RichText::new("PC (esta UI):")
                    .strong(),
            );
            ui.label(
                egui::RichText::new("cargo run --package pc-app -- --connect 127.0.0.1:7000")
                    .monospace()
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );

            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("Los hornos aparecerán automáticamente cuando el RPi los detecte.")
                    .italics()
                    .color(egui::Color32::from_rgb(160, 160, 160)),
            );
        });
    });
}

/// Renders the bulk operations side panel with range selection, temperature
/// input, and batch action buttons.
pub fn render_bulk_panel(
    ui: &mut egui::Ui,
    oven_count: usize,
    selection: &mut BulkSelection,
    validation: &mut BulkValidation,
    connection_state: &ConnectionState,
) {
    ui.heading(
        egui::RichText::new("Operaciones en Bloque")
            .color(bulk_header_color())
            .strong(),
    );
    ui.separator();

    let disabled = *connection_state == ConnectionState::Disconnected;

    // ── Select All checkbox ──
    ui.checkbox(&mut selection.select_all, "Seleccionar todos");
    ui.add_space(4.0);

    // ── Range inputs (disabled when select_all is active) ──
    let range_disabled = disabled || selection.select_all;
    ui.label("Índice desde:");
    ui.add_enabled(
        !range_disabled,
        egui::DragValue::new(&mut selection.from_index).speed(1.0),
    );

    ui.label("Índice hasta:");
    ui.add_enabled(
        !range_disabled,
        egui::DragValue::new(&mut selection.to_index).speed(1.0),
    );

    ui.add_space(4.0);

    // ── Temperature input ──
    ui.label("Temperatura objetivo (°C):");
    ui.add_enabled(
        !disabled,
        egui::DragValue::new(&mut selection.target_temp)
            .speed(1.0)
            .range(0.0..=300.0),
    );

    ui.add_space(4.0);
    ui.separator();

    // ── Selection summary ──
    let summary = if selection.select_all {
        format!("{} hornos detectados", oven_count)
    } else {
        format!("Rango {}-{}", selection.from_index, selection.to_index)
    };
    ui.label(
        egui::RichText::new(&summary)
            .small()
            .color(egui::Color32::from_rgb(160, 160, 160)),
    );

    ui.add_space(4.0);

    // ── Action buttons ──
    let btn = |label: &str| {
        egui::RichText::new(label).color(egui::Color32::WHITE)
    };

    if ui
        .add_enabled(!disabled, egui::Button::new(btn("Encender seleccionados")))
        .clicked()
    {
        selection.action = BulkAction::Enable;
    }
    if ui
        .add_enabled(!disabled, egui::Button::new(btn("Apagar seleccionados")))
        .clicked()
    {
        selection.action = BulkAction::Disable;
    }
    if ui
        .add_enabled(!disabled, egui::Button::new(btn("Aplicar temperatura")))
        .clicked()
    {
        selection.action = BulkAction::ApplyTemperature;
    }
    if ui
        .add_enabled(
            !disabled,
            egui::Button::new(btn("Encender + aplicar temp.")),
        )
        .clicked()
    {
        selection.action = BulkAction::EnableAndApplyTemperature;
    }
    if ui
        .add_enabled(!disabled, egui::Button::new(btn("Solicitar estado")))
        .clicked()
    {
        selection.action = BulkAction::RequestStatus;
    }

    // ── Validation errors ──
    if !validation.errors.is_empty() {
        ui.add_space(4.0);
        for error in &validation.errors {
            ui.colored_label(egui::Color32::from_rgb(220, 50, 50), error);
        }
    }
}
