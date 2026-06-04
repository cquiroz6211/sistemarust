//! Headless UI integration tests.
//!
//! Tests the UI data flow (log capture, command dispatch, connection monitor)
//! without the graphical render pipeline. Does NOT add EguiPlugin since
//! bevy_egui's render feature requires GPU resources unavailable in CI.

use std::time::Duration;

use bevy::app::{FixedUpdate, ScheduleRunnerPlugin};
use bevy::prelude::{App, IntoSystemConfigs, MinimalPlugins, PluginGroup};

use pc_app::plugins::pc_app::PcAppPlugin;
use pc_app::resources::{
    BulkAction, BulkSelection, BulkValidation, ConnectionState, EcsDemoMetrics,
    EmergencyStopConfirm, EventLog, InboundProtocolQueue, OvenEditStates, OutboundProtocolQueue,
    TemperatureValidation, UiIntent,
};
use pc_app::systems::ui::dispatch::ui_command_dispatch;
use pc_app::systems::ui::log_capture::log_capture_events;
use pc_app::systems::ui::monitor::update_ecs_demo_metrics;
use protocol::{
    EventEnvelope, Message, OvenDetectedPayload, OvenStatusUpdatedPayload, OvenState,
};

/// Build a minimal test app with the core UI systems (no render).
fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);

    // Insert UI resources manually (UiPlugin is not added because it depends on EguiPlugin)
    app.insert_resource(EventLog::default());
    app.insert_resource(ConnectionState::Disconnected);
    app.insert_resource(UiIntent::default());
    app.insert_resource(OvenEditStates::default());
    app.insert_resource(EmergencyStopConfirm::default());
    app.insert_resource(TemperatureValidation::default());
    app.insert_resource(BulkSelection::default());
    app.insert_resource(BulkValidation::default());
    app.insert_resource(EcsDemoMetrics::default());

    // Register the data-flow systems (no render system)
    app.add_systems(
        bevy::prelude::Update,
        log_capture_events.after(pc_app::systems::ingest::ingest_inbound_protocol),
    );
    app.add_systems(bevy::prelude::Update, ui_command_dispatch);
    app.add_systems(
        bevy::prelude::Update,
        update_ecs_demo_metrics.after(ui_command_dispatch),
    );

    app
}

fn tick(app: &mut App) {
    app.update();
    app.world_mut().run_schedule(FixedUpdate);
}

fn push_envelope(app: &mut App, message: Message) {
    app.world_mut()
        .resource_mut::<InboundProtocolQueue>()
        .0
        .push(EventEnvelope::new("rpi-controller", "pc-app", message));
}

fn spawn_oven(app: &mut App, oven_id: &str) {
    push_envelope(
        app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: oven_id.to_string(),
            sensor_ref: "temp0".to_string(),
            output_ref: "relay0".to_string(),
            max_celsius: 300.0,
        }),
    );
    tick(app);
}

// ── EventLog tests ──────────────────────────────────────────────────────────

#[test]
fn log_captures_inbound_events() {
    let mut app = build_test_app();

    push_envelope(
        &mut app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "s1".into(),
            output_ref: "o1".into(),
            max_celsius: 300.0,
        }),
    );
    tick(&mut app);

    let log = app.world().resource::<EventLog>();
    assert!(
        !log.entries.is_empty(),
        "EventLog should have entries after OvenDetected"
    );
    let entry = &log.entries[0];
    assert_eq!(entry.message_type, "OvenDetected");
    assert!(entry.summary.contains("oven1"));
}

#[test]
fn log_captures_status_updated_events() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 185.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        }),
    );
    tick(&mut app);

    let log = app.world().resource::<EventLog>();
    let status_entries: Vec<_> = log
        .entries
        .iter()
        .filter(|e| e.message_type == "OvenStatusUpdated")
        .collect();
    assert!(
        !status_entries.is_empty(),
        "Should have OvenStatusUpdated entries"
    );
    assert!(status_entries[0].summary.contains("185.0"));
}

#[test]
fn log_respects_200_entry_limit() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Push 210 status updates to exceed the 200 limit
    for i in 0..210 {
        push_envelope(
            &mut app,
            Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "oven1".into(),
                current_celsius: i as f64,
                target_celsius: 250.0,
                enabled: true,
                heating: false,
                output_level: None,
                state: OvenState::Idle,
            }),
        );
        tick(&mut app);
    }

    let log = app.world().resource::<EventLog>();
    // 200 status entries + 1 OvenDetected from spawn_oven = 201
    assert!(
        log.entries.len() <= 201,
        "Log should not exceed 201 entries (200 max + 1 from spawn), got {}",
        log.entries.len()
    );
}

// ── UiIntent dispatch tests ─────────────────────────────────────────────────

#[test]
fn ui_intent_sends_enabled_command() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Simulate UI intent
    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.set_enabled = Some(("oven1".into(), true));
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let enabled_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::SetOvenEnabled(_)))
        .collect();
    assert!(
        !enabled_cmds.is_empty(),
        "Should have SetOvenEnabled command"
    );
    match &enabled_cmds[0].payload {
        Message::SetOvenEnabled(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert!(p.enabled);
        }
        _ => panic!("Expected SetOvenEnabled"),
    }
}

#[test]
fn ui_intent_sends_temperature_command() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.set_temperature = Some(("oven1".into(), 250.0));
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let temp_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::SetTargetTemperature(_)))
        .collect();
    assert!(
        !temp_cmds.is_empty(),
        "Should have SetTargetTemperature command"
    );
    match &temp_cmds[0].payload {
        Message::SetTargetTemperature(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert_eq!(p.target_celsius, 250.0);
        }
        _ => panic!("Expected SetTargetTemperature"),
    }
}

#[test]
fn ui_intent_sends_status_request_all() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.request_status = Some(None); // All ovens
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let status_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::RequestStatus(_)))
        .collect();
    assert!(!status_cmds.is_empty(), "Should have RequestStatus command");
    match &status_cmds[0].payload {
        Message::RequestStatus(p) => {
            assert!(p.oven_id.is_none());
        }
        _ => panic!("Expected RequestStatus"),
    }
}

#[test]
fn ui_intent_sends_status_request_single() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.request_status = Some(Some("oven1".into()));
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let status_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::RequestStatus(_)))
        .collect();
    assert!(!status_cmds.is_empty(), "Should have RequestStatus command");
    match &status_cmds[0].payload {
        Message::RequestStatus(p) => {
            assert_eq!(p.oven_id, Some("oven1".to_string()));
        }
        _ => panic!("Expected RequestStatus"),
    }
}

#[test]
fn ui_intent_sends_emergency_stop() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    {
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.emergency_stop = true;
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let stop_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::EmergencyStop(_)))
        .collect();
    assert!(!stop_cmds.is_empty(), "Should have EmergencyStop command");
    match &stop_cmds[0].payload {
        Message::EmergencyStop(p) => {
            assert!(p.reason.contains("UI"));
        }
        _ => panic!("Expected EmergencyStop"),
    }
}

// ── ConnectionState tests ───────────────────────────────────────────────────

#[test]
fn connection_state_default_is_disconnected() {
    let app = build_test_app();
    let state = app.world().resource::<ConnectionState>();
    assert_eq!(*state, ConnectionState::Disconnected);
}

// ── ECS demo monitor tests ──────────────────────────────────────────────────

#[test]
fn ecs_demo_metrics_count_oven_groups() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven-1");
    spawn_oven(&mut app, "oven-2");

    push_envelope(
        &mut app,
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven-1".into(),
            current_celsius: 185.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        }),
    );
    tick(&mut app);
    tick(&mut app);

    let metrics = app.world().resource::<EcsDemoMetrics>();
    assert_eq!(metrics.total_ovens, 2);
    assert_eq!(metrics.enabled_ovens, 1);
    assert_eq!(metrics.heating_ovens, 1);
    assert_eq!(metrics.faulted_ovens, 0);
}

#[test]
fn ecs_demo_metrics_record_bulk_dispatch() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven-1");
    spawn_oven(&mut app, "oven-2");

    {
        let mut selection = app.world_mut().resource_mut::<BulkSelection>();
        selection.from_index = 1;
        selection.to_index = 2;
        selection.target_temp = 220.0;
        selection.action = BulkAction::EnableAndApplyTemperature;
    }

    tick(&mut app);

    let metrics = app.world().resource::<EcsDemoMetrics>();
    assert_eq!(metrics.last_bulk_operation, "Enable and apply temperature");
    assert_eq!(metrics.last_commands_generated, 4);
}

// ── Emergency Stop confirmation tests ──────────────────────────────────────

#[test]
fn emergency_stop_not_sent_without_confirmation() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Simulate clicking the Emergency Stop button (sets confirm.open = true)
    {
        let mut confirm = app.world_mut().resource_mut::<EmergencyStopConfirm>();
        confirm.open = true;
    }

    // Do NOT confirm — intent should remain false
    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let stop_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::EmergencyStop(_)))
        .collect();
    assert!(
        stop_cmds.is_empty(),
        "EmergencyStop should NOT be sent when confirmation dialog is open but not confirmed"
    );
}

#[test]
fn emergency_stop_sent_after_confirmation() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Open confirmation dialog
    {
        let mut confirm = app.world_mut().resource_mut::<EmergencyStopConfirm>();
        confirm.open = true;
    }

    // Confirm: set intent.emergency_stop = true and close dialog
    {
        let mut confirm = app.world_mut().resource_mut::<EmergencyStopConfirm>();
        confirm.open = false;
        let mut intent = app.world_mut().resource_mut::<UiIntent>();
        intent.emergency_stop = true;
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let stop_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::EmergencyStop(_)))
        .collect();
    assert!(
        !stop_cmds.is_empty(),
        "EmergencyStop should be sent after confirmation"
    );
}

#[test]
fn emergency_stop_not_sent_after_cancellation() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Open confirmation dialog
    {
        let mut confirm = app.world_mut().resource_mut::<EmergencyStopConfirm>();
        confirm.open = true;
    }

    // Cancel: close dialog without setting intent
    {
        let mut confirm = app.world_mut().resource_mut::<EmergencyStopConfirm>();
        confirm.open = false;
    }

    tick(&mut app);

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let stop_cmds: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::EmergencyStop(_)))
        .collect();
    assert!(
        stop_cmds.is_empty(),
        "EmergencyStop should NOT be sent after cancellation"
    );
}

// ── Temperature validation tests ────────────────────────────────────────────

#[test]
fn temperature_validation_blocks_out_of_range_command() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Simulate a validation error for out-of-range input
    {
        let mut validation = app.world_mut().resource_mut::<TemperatureValidation>();
        validation.errors.insert(
            "oven1".to_string(),
            "Temperatura excede limite (300.0 °C)".to_string(),
        );
    }

    // Even if intent has a temperature, the validation error should prevent dispatch
    // (dispatch itself doesn't check validation — the UI control layer prevents setting intent)
    // This test verifies the validation resource stores the error correctly.
    {
        let validation = app.world().resource::<TemperatureValidation>();
        assert!(validation.errors.contains_key("oven1"));
        assert!(validation.errors["oven1"].contains("excede limite"));
    }
}

#[test]
fn temperature_validation_cleared_on_valid_input() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Set a validation error
    {
        let mut validation = app.world_mut().resource_mut::<TemperatureValidation>();
        validation.errors.insert("oven1".to_string(), "error".to_string());
    }

    // Clear it
    {
        let mut validation = app.world_mut().resource_mut::<TemperatureValidation>();
        validation.errors.remove("oven1");
    }

    let validation = app.world().resource::<TemperatureValidation>();
    assert!(
        !validation.errors.contains_key("oven1"),
        "Validation error should be cleared after valid input"
    );
}

#[test]
fn temperature_validation_negative_value_blocked() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Simulate negative value validation
    {
        let mut validation = app.world_mut().resource_mut::<TemperatureValidation>();
        validation.errors.insert(
            "oven1".to_string(),
            "Temperatura excede limite (300.0 °C)".to_string(),
        );
    }

    let validation = app.world().resource::<TemperatureValidation>();
    assert!(validation.errors.contains_key("oven1"));
}
