//! Event log panel rendering.

use bevy_egui::egui;

use crate::resources::{EventLog, LogDirection};
use crate::ui::styles::{connected_color, disconnected_color};

/// Renders the scrollable event log panel.
pub fn render_event_log(ui: &mut egui::Ui, event_log: &EventLog) {
    ui.heading("Event Log");
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if event_log.entries.is_empty() {
                ui.label("No events yet");
                return;
            }

            for entry in &event_log.entries {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&entry.timestamp).monospace().small(),
                    );

                    let color = match entry.direction {
                        LogDirection::In => connected_color(),
                        LogDirection::Out => disconnected_color(),
                    };
                    ui.label(
                        egui::RichText::new(entry.direction.label())
                            .color(color)
                            .strong()
                            .monospace()
                            .small(),
                    );

                    ui.label(
                        egui::RichText::new(&entry.message_type)
                            .monospace()
                            .small(),
                    );

                    ui.label(
                        egui::RichText::new(&entry.summary).small(),
                    );
                });
            }
        });
}
