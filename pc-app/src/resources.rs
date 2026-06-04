//! Bevy resources for the PC-side application.
//!
//! Two in-memory protocol queues mediate I/O between PC and RPi, mirroring the
//! `rpi-controller` pattern. `OvenIndex` provides O(1) lookup from protocol
//! oven_id strings to Bevy entities.

use std::collections::HashMap;

use bevy::prelude::{Entity, Resource};
use protocol::EventEnvelope;

use crate::components::FaultInfo;

/// Queue of inbound protocol envelopes (RPi → PC).
/// Drained each tick by `ingest_inbound_protocol`.
#[derive(Debug, Clone, Resource, Default)]
pub struct InboundProtocolQueue(pub Vec<EventEnvelope>);

/// Queue of outbound protocol envelopes (PC → RPi).
/// Command authoring systems push here; transport layer drains it.
#[derive(Debug, Clone, Resource, Default)]
pub struct OutboundProtocolQueue(pub Vec<EventEnvelope>);

/// Fast lookup index: `oven_id` → Bevy `Entity`.
/// Maintained by `apply_oven_detected`; consumed by all state-mutation systems.
#[derive(Debug, Clone, Resource, Default)]
pub struct OvenIndex(pub HashMap<String, Entity>);

/// Global system-level fault (when `FaultRaised` arrives with `oven_id: None`).
/// Oven entities are never modified by global faults.
#[derive(Debug, Clone, Resource, Default)]
pub struct GlobalFault(pub Option<FaultInfo>);

// ── UI resources ─────────────────────────────────────────────────────────────

/// Direction of a log entry: inbound (RPi → PC) or outbound (PC → RPi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDirection {
    In,
    Out,
}

impl LogDirection {
    pub fn label(&self) -> &'static str {
        match self {
            LogDirection::In => "IN",
            LogDirection::Out => "OUT",
        }
    }
}

/// A single entry in the event log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: LogDirection,
    pub message_type: String,
    pub summary: String,
}

/// Scrollable event log — FIFO with a 200-entry limit.
#[derive(Debug, Resource)]
pub struct EventLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

impl Default for EventLog {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 200,
        }
    }
}

impl EventLog {
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
}

/// Connection state heuristic — reflects transport channel availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// UI intent captured by egui controls each frame.
/// Processed by `ui_command_dispatch` in the next Update tick.
#[derive(Debug, Resource, Default)]
pub struct UiIntent {
    pub set_enabled: Option<(String, bool)>,
    pub set_temperature: Option<(String, f64)>,
    /// `Some(None)` = request all, `Some(Some(id))` = request single oven.
    pub request_status: Option<Option<String>>,
    pub emergency_stop: bool,
}

/// Tracks whether the Emergency Stop confirmation dialog is open.
#[derive(Debug, Resource, Default)]
pub struct EmergencyStopConfirm {
    pub open: bool,
}

/// Per-oven temperature validation message (shown when out of range).
#[derive(Debug, Resource, Default)]
pub struct TemperatureValidation {
    /// oven_id → optional error message to display.
    pub errors: std::collections::HashMap<String, String>,
}

// ── Bulk operations resources ────────────────────────────────────────────────

/// Action requested by the bulk operations panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkAction {
    #[default]
    None,
    Enable,
    Disable,
    ApplyTemperature,
    EnableAndApplyTemperature,
    RequestStatus,
}

/// Selection of ovens for bulk operations.
/// Processed by `ui_command_dispatch` on the next frame.
#[derive(Debug, Clone, Resource, Default)]
pub struct BulkSelection {
    /// Minimum index of the range (inclusive).
    pub from_index: u32,
    /// Maximum index of the range (inclusive).
    pub to_index: u32,
    /// If true, select all detected ovens regardless of range.
    pub select_all: bool,
    /// Target temperature for bulk apply.
    pub target_temp: f64,
    /// Action to execute on selected ovens.
    pub action: BulkAction,
}

/// Validation messages for the bulk operations panel.
#[derive(Debug, Resource, Default)]
pub struct BulkValidation {
    pub errors: Vec<String>,
}

/// Pedagogical metrics for demonstrating how Bevy ECS processes oven entities.
#[derive(Debug, Clone, Resource)]
pub struct EcsDemoMetrics {
    pub total_ovens: usize,
    pub enabled_ovens: usize,
    pub heating_ovens: usize,
    pub faulted_ovens: usize,
    pub last_bulk_operation: String,
    pub last_commands_generated: usize,
    pub last_dispatch_micros: u128,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub logged_events_per_second: f32,
    pub observed_log_entries: usize,
}

impl Default for EcsDemoMetrics {
    fn default() -> Self {
        Self {
            total_ovens: 0,
            enabled_ovens: 0,
            heating_ovens: 0,
            faulted_ovens: 0,
            last_bulk_operation: "None yet".to_string(),
            last_commands_generated: 0,
            last_dispatch_micros: 0,
            fps: 0.0,
            frame_time_ms: 0.0,
            logged_events_per_second: 0.0,
            observed_log_entries: 0,
        }
    }
}

impl BulkAction {
    pub fn label(&self) -> &'static str {
        match self {
            BulkAction::None => "None",
            BulkAction::Enable => "Enable selected",
            BulkAction::Disable => "Disable selected",
            BulkAction::ApplyTemperature => "Apply temperature",
            BulkAction::EnableAndApplyTemperature => "Enable and apply temperature",
            BulkAction::RequestStatus => "Request status",
        }
    }
}

/// Per-oven local edit state for temperature adjustment.
/// The operator edits a local value, then explicitly clicks "Apply" to send.
#[derive(Debug, Clone, Resource, Default)]
pub struct OvenEditStates {
    /// oven_id → local edit value (the temperature the user is adjusting).
    pub values: std::collections::HashMap<String, OvenEditEntry>,
}

/// Local editing state for a single oven's target temperature.
#[derive(Debug, Clone)]
pub struct OvenEditEntry {
    /// Local target temperature the user is adjusting before applying.
    pub local_target: f64,
    /// Maximum allowed temperature (synced from MaxTemperature component).
    pub max_temp: f64,
}

impl OvenEditStates {
    /// Get or create an edit entry for the given oven, syncing max_temp if needed.
    pub fn get_or_create(&mut self, oven_id: &str, max_temp: f64) -> &mut OvenEditEntry {
        self.values
            .entry(oven_id.to_string())
            .or_insert_with(|| OvenEditEntry {
                local_target: 0.0,
                max_temp,
            })
    }

    /// Sync the local target to the actual target from the ECS (only on first appearance
    /// or if the entry doesn't exist yet).
    pub fn sync_from_ecs(&mut self, oven_id: &str, actual_target: f64, max_temp: f64) {
        let entry = self.get_or_create(oven_id, max_temp);
        entry.max_temp = max_temp;
        // Only overwrite local_target if this is a fresh entry (max_temp == 0.0 means new)
        // or if the user hasn't started editing (we use a sentinel: if local_target == 0.0
        // and actual_target != 0.0, sync).
        if entry.local_target == 0.0 && actual_target != 0.0 {
            entry.local_target = actual_target;
        }
    }
}

// ── Bulk operations helpers ──────────────────────────────────────────────────

/// Extract the numeric suffix from an `oven_id` like `"oven-42"`.
/// Returns `None` if the format does not match `"oven-N"`.
pub fn parse_oven_index(oven_id: &str) -> Option<u32> {
    oven_id
        .strip_prefix("oven-")
        .and_then(|s| s.trim().parse::<u32>().ok())
}

/// Resolve which oven IDs match the given bulk selection, sorted by numeric index.
///
/// - If `select_all` is true, returns all ovens sorted ascending by index.
/// - Otherwise, filters by the `[from_index, to_index]` range.
pub fn resolve_selected_ovens<'a>(
    oven_index: &'a OvenIndex,
    selection: &BulkSelection,
) -> Vec<(&'a String, u32)> {
    let mut candidates: Vec<_> = oven_index
        .0
        .keys()
        .filter_map(|id| parse_oven_index(id).map(|idx| (id, idx)))
        .collect();

    if selection.select_all {
        candidates.sort_by_key(|(_, idx)| *idx);
        candidates
    } else {
        candidates
            .into_iter()
            .filter(|(_, idx)| *idx >= selection.from_index && *idx <= selection.to_index)
            .collect()
    }
}

/// Validate a bulk selection before dispatching commands.
pub fn validate_bulk_selection(
    selection: &BulkSelection,
    oven_index: &OvenIndex,
) -> Vec<String> {
    let mut errors = Vec::new();

    if selection.action == BulkAction::None {
        return errors;
    }

    // Check at least one oven exists when select_all
    if selection.select_all && oven_index.0.is_empty() {
        errors.push("No hay hornos para seleccionar".into());
        return errors;
    }

    if !selection.select_all {
        if selection.from_index > selection.to_index {
            errors.push("Rango inválido: from debe ser <= to".into());
        }
    }

    // Temperature validation
    if selection.action == BulkAction::ApplyTemperature
        || selection.action == BulkAction::EnableAndApplyTemperature
    {
        if selection.target_temp < 0.0 {
            errors.push("Temperatura debe ser >= 0".into());
        }
        // Fixed max of 300.0°C — RPi rejects out-of-range values anyway
        if selection.target_temp > 300.0 {
            errors.push("Temperatura excede límite (300.0 °C)".into());
        }
    }

    errors
}
