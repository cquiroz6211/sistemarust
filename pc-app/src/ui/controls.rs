//! Per-oven interactive controls: toggle, temperature editor, status request.

use bevy_egui::egui;

use crate::resources::{OvenEditStates, TemperatureValidation, UiIntent};
use crate::ui::styles::format_temp;

/// Renders the enabled/disabled toggle button for an oven with explicit Spanish labels.
pub fn render_enabled_toggle(
    ui: &mut egui::Ui,
    oven_id: &str,
    enabled: bool,
    disabled: bool,
    intent: &mut UiIntent,
) {
    let (label, color) = if enabled {
        ("Apagar horno", egui::Color32::from_rgb(220, 80, 80))
    } else {
        ("Encender horno", egui::Color32::from_rgb(40, 180, 40))
    };

    let button = egui::Button::new(egui::RichText::new(label).color(color));
    let button = if disabled {
        button.sense(egui::Sense::hover())
    } else {
        button
    };

    if ui.add(button).clicked() && !disabled {
        intent.set_enabled = Some((oven_id.to_string(), !enabled));
    }
}

/// Renders the temperature editor with local editing, quick buttons, text input, and explicit apply.
///
/// The operator edits a local value via slider or text input. Quick buttons adjust
/// by -10, -1, +1, +10. If the text input exceeds [0.0, max_temp], a validation
/// message is shown and the command is not sent. The "Apply" button sends
/// `SetTargetTemperature`.
pub fn render_temperature_editor(
    ui: &mut egui::Ui,
    oven_id: &str,
    actual_target: f64,
    max_temp: f64,
    edit_states: &mut OvenEditStates,
    intent: &mut UiIntent,
    validation: &mut TemperatureValidation,
    disabled: bool,
) {
    // Ensure edit entry exists and is synced
    edit_states.sync_from_ecs(oven_id, actual_target, max_temp);
    let entry = edit_states.get_or_create(oven_id, max_temp);
    let local = &mut entry.local_target;
    let max = entry.max_temp;

    // ── Quick adjustment buttons ──
    ui.horizontal(|ui| {
        if ui.small_button("-10").clicked() && !disabled {
            *local = (*local - 10.0).max(0.0).min(max);
            validation.errors.remove(oven_id);
        }
        if ui.small_button("-1").clicked() && !disabled {
            *local = (*local - 1.0).max(0.0).min(max);
            validation.errors.remove(oven_id);
        }
        if ui.small_button("+1").clicked() && !disabled {
            *local = (*local + 1.0).max(0.0).min(max);
            validation.errors.remove(oven_id);
        }
        if ui.small_button("+10").clicked() && !disabled {
            *local = (*local + 10.0).max(0.0).min(max);
            validation.errors.remove(oven_id);
        }
    });

    // ── Slider for direct editing (always in range) ──
    ui.add(
        egui::Slider::new(local, 0.0..=max)
            .suffix(" °C"),
    );

    // ── Numeric text input (can exceed range → triggers validation) ──
    let mut text_buf = format!("{:.1}", *local);
    let text_response = ui.add(
        egui::TextEdit::singleline(&mut text_buf)
            .desired_width(100.0)
            .hint_text("Valor manual"),
    );

    // Parse and validate the text input
    if text_response.changed() {
        if let Ok(val) = text_buf.trim().parse::<f64>() {
            if val < 0.0 || val > max {
                validation.errors.insert(
                    oven_id.to_string(),
                    format!("Temperatura excede limite ({:.1} °C)", max),
                );
            } else {
                *local = val;
                validation.errors.remove(oven_id);
            }
        }
    }

    // ── Show validation error if present ──
    if let Some(err) = validation.errors.get(oven_id) {
        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), err.as_str());
    }

    // ── Apply button ──
    let changed = (*local - actual_target).abs() > 0.01;
    let has_validation_error = validation.errors.contains_key(oven_id);
    let apply_label = if changed {
        format!("Aplicar temperatura ({} → {})", format_temp(actual_target), format_temp(*local))
    } else {
        "Aplicar temperatura".to_string()
    };

    let apply_button = egui::Button::new(
        egui::RichText::new(&apply_label).strong(),
    );
    let apply_button = if disabled || !changed || has_validation_error {
        apply_button.sense(egui::Sense::hover())
    } else {
        apply_button.fill(egui::Color32::from_rgb(50, 140, 220))
    };

    if ui.add(apply_button).clicked() && !disabled && changed && !has_validation_error {
        intent.set_temperature = Some((oven_id.to_string(), *local));
    }
}

/// Renders the "Request Status" button for an oven with explicit Spanish label.
pub fn render_status_request(
    ui: &mut egui::Ui,
    oven_id: &str,
    intent: &mut UiIntent,
) {
    if ui.small_button("Solicitar estado").clicked() {
        intent.request_status = Some(Some(oven_id.to_string()));
    }
}
