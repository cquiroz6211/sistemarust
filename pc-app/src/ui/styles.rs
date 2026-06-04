//! Visual styles and formatting helpers for the UI.

use bevy_egui::egui::Color32;

pub fn connected_color() -> Color32 {
    Color32::from_rgb(40, 180, 40)
}

pub fn disconnected_color() -> Color32 {
    Color32::from_rgb(200, 50, 50)
}

pub fn heating_color() -> Color32 {
    Color32::from_rgb(220, 180, 30)
}

pub fn fault_color() -> Color32 {
    Color32::from_rgb(220, 50, 50)
}

pub fn disabled_color() -> Color32 {
    Color32::from_rgb(140, 140, 140)
}

pub fn emergency_color() -> Color32 {
    Color32::from_rgb(200, 60, 60)
}

pub fn idle_color() -> Color32 {
    Color32::from_rgb(100, 180, 240)
}

pub fn off_color() -> Color32 {
    Color32::from_rgb(140, 140, 140)
}

pub fn card_background() -> Color32 {
    Color32::from_rgba_premultiplied(30, 30, 40, 200)
}

pub fn card_border() -> Color32 {
    Color32::from_rgb(80, 80, 100)
}

/// Format temperature to 1 decimal with °C suffix.
pub fn format_temp(celsius: f64) -> String {
    format!("{:.1} °C", celsius)
}

/// Map an OvenState to a human-friendly Spanish label.
pub fn oven_state_label(state: &protocol::OvenState) -> &'static str {
    match state {
        protocol::OvenState::Disabled => "Apagado",
        protocol::OvenState::Idle => "Encendido",
        protocol::OvenState::Heating => "Calentando",
        protocol::OvenState::Faulted => "Falla",
        protocol::OvenState::EmergencyStopped => "Emergencia",
    }
}

/// Color associated with each oven state.
pub fn oven_state_color(state: &protocol::OvenState) -> Color32 {
    match state {
        protocol::OvenState::Disabled => off_color(),
        protocol::OvenState::Idle => idle_color(),
        protocol::OvenState::Heating => heating_color(),
        protocol::OvenState::Faulted => fault_color(),
        protocol::OvenState::EmergencyStopped => emergency_color(),
    }
}
