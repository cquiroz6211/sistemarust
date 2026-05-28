//! Comprehensive tests for the pc-app Bevy ECS read model and command authoring.
//!
//! All tests use `tick()` (Update + FixedUpdate) to step deterministically.
//! Mock data is pushed directly into `InboundProtocolQueue`, and component state
//! is asserted after the tick.

use std::time::Duration;

use bevy::app::{FixedUpdate, ScheduleRunnerPlugin};
use bevy::prelude::{App, Entity, MinimalPlugins, PluginGroup};

use protocol::{
    CommandAcceptedPayload, CommandRejectedPayload, EventEnvelope, FaultCode,
    FaultRaisedPayload, Message, OvenDetectedPayload, OvenState, OvenStatusUpdatedPayload,
    RequestScope, Severity, SetTargetTemperaturePayload,
};

use pc_app::components::{
    CommandResult, CurrentTemperature, FaultState, Heating, LastCommandResult, MaxTemperature,
    OvenId, OvenStatus, TargetTemperature,
};
use pc_app::plugins::pc_app::PcAppPlugin;
use pc_app::resources::{GlobalFault, InboundProtocolQueue, OvenIndex, OutboundProtocolQueue};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))));
    app.add_plugins(PcAppPlugin);
    app
}

/// Runs Update (ingestion) then manually triggers FixedUpdate (state mutation).
/// This is necessary because FixedUpdate is time-gated and a single app.update()
/// in a test won't accumulate enough time to trigger it automatically.
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

fn push_envelopes(app: &mut App, messages: Vec<Message>) {
    let mut queue = app.world_mut().resource_mut::<InboundProtocolQueue>();
    for msg in messages {
        queue.0.push(EventEnvelope::new("rpi-controller", "pc-app", msg));
    }
}

fn spawn_oven(app: &mut App, oven_id: &str) {
    app.world_mut()
        .resource_mut::<InboundProtocolQueue>()
        .0
        .push(EventEnvelope::new(
            "rpi-controller",
            "pc-app",
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: oven_id.to_string(),
                sensor_ref: "temp0".to_string(),
                output_ref: "relay0".to_string(),
                max_celsius: 300.0,
            }),
        ));
    tick(app);
}

fn find_oven_entity(app: &App, oven_id: &str) -> Option<Entity> {
    app.world().resource::<OvenIndex>().0.get(oven_id).copied()
}

// ── 3.1 ingest_inbound_protocol tests ────────────────────────────────────────

#[test]
fn ingest_drains_queue_on_tick() {
    let mut app = build_test_app();

    push_envelopes(
        &mut app,
        vec![
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven1".into(),
                sensor_ref: "s1".into(),
                output_ref: "o1".into(),
                max_celsius: 300.0,
            }),
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven2".into(),
                sensor_ref: "s2".into(),
                output_ref: "o2".into(),
                max_celsius: 350.0,
            }),
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven3".into(),
                sensor_ref: "s3".into(),
                output_ref: "o3".into(),
                max_celsius: 400.0,
            }),
        ],
    );

    tick(&mut app);

    let queue = app.world().resource::<InboundProtocolQueue>();
    assert!(queue.0.is_empty(), "Queue should be empty after tick");
}

#[test]
fn ingest_empty_queue_is_noop() {
    let mut app = build_test_app();
    tick(&mut app);

    let queue = app.world().resource::<InboundProtocolQueue>();
    assert!(queue.0.is_empty());
}

#[test]
fn ingest_command_variant_ignored_in_inbound() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        }),
    );

    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let temp = app.world().get::<CurrentTemperature>(entity).unwrap();
    assert_eq!(temp.0, 0.0, "Command in inbound queue should be ignored");
}

#[test]
fn ingest_all_five_event_variants_dispatch() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelopes(
        &mut app,
        vec![
            Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "oven1".into(),
                current_celsius: 180.5,
                target_celsius: 250.0,
                enabled: true,
                heating: true,
                output_level: Some(75.0),
                state: OvenState::Heating,
            }),
            Message::FaultRaised(FaultRaisedPayload {
                oven_id: Some("oven1".into()),
                fault_code: FaultCode::SafetyLimitExceeded,
                severity: Severity::High,
                message: "Limit exceeded".into(),
            }),
            Message::CommandAccepted(CommandAcceptedPayload {
                accepted_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                message: "OK".into(),
            }),
            Message::CommandRejected(CommandRejectedPayload {
                rejected_type: "SetOvenEnabled".into(),
                oven_id: Some("oven1".into()),
                reason: FaultCode::OvenNotFound,
                message: "Not found".into(),
            }),
        ],
    );

    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();

    let temp = app.world().get::<CurrentTemperature>(entity).unwrap();
    assert_eq!(temp.0, 180.5);

    let fault = app.world().get::<FaultState>(entity).unwrap();
    assert!(fault.0.is_some());

    let result = app.world().get::<LastCommandResult>(entity).unwrap();
    assert!(result.0.is_some());
}

// ── 3.2 apply_oven_detected tests ────────────────────────────────────────────

#[test]
fn first_detection_spawns_entity_with_correct_components() {
    let mut app = build_test_app();

    push_envelope(
        &mut app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        }),
    );

    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1");
    assert!(entity.is_some(), "Entity should exist in OvenIndex");

    let entity = entity.unwrap();
    let oven_id = app.world().get::<OvenId>(entity).unwrap();
    assert_eq!(oven_id.0, "oven1");

    let max_temp = app.world().get::<MaxTemperature>(entity).unwrap();
    assert_eq!(max_temp.0, 300.0);

    let current = app.world().get::<CurrentTemperature>(entity).unwrap();
    assert_eq!(current.0, 0.0);
}

#[test]
fn redetection_updates_max_temperature_no_duplicate() {
    let mut app = build_test_app();

    push_envelope(
        &mut app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        }),
    );
    tick(&mut app);

    push_envelope(
        &mut app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 350.0,
        }),
    );
    tick(&mut app);

    let index = app.world().resource::<OvenIndex>();
    assert!(index.0.contains_key("oven1"));
    assert_eq!(index.0.len(), 1, "No duplicate entity");

    let entity = index.0.get("oven1").unwrap();
    let max_temp = app.world().get::<MaxTemperature>(*entity).unwrap();
    assert_eq!(max_temp.0, 350.0);
}

#[test]
fn oven_exists_in_index_after_detection() {
    let mut app = build_test_app();

    push_envelope(
        &mut app,
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "my-oven".into(),
            sensor_ref: "s1".into(),
            output_ref: "o1".into(),
            max_celsius: 500.0,
        }),
    );
    tick(&mut app);

    let index = app.world().resource::<OvenIndex>();
    assert!(index.0.contains_key("my-oven"));
    assert_eq!(index.0.len(), 1);
}

// ── 3.3 apply_oven_status_updated tests ──────────────────────────────────────

#[test]
fn known_oven_gets_updated_components() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 180.5,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        }),
    );
    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();

    let temp = app.world().get::<CurrentTemperature>(entity).unwrap();
    assert_eq!(temp.0, 180.5);

    let target = app.world().get::<TargetTemperature>(entity).unwrap();
    assert_eq!(target.0, 250.0);

    let heating = app.world().get::<Heating>(entity).unwrap();
    assert!(heating.0);

    let status = app.world().get::<OvenStatus>(entity).unwrap();
    assert_eq!(status.0, OvenState::Heating);
}

#[test]
fn unknown_oven_status_ignored() {
    let mut app = build_test_app();

    push_envelope(
        &mut app,
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "unknown-oven".into(),
            current_celsius: 200.0,
            target_celsius: 250.0,
            enabled: true,
            heating: false,
            output_level: None,
            state: OvenState::Idle,
        }),
    );
    tick(&mut app);

    let index = app.world().resource::<OvenIndex>();
    assert!(
        !index.0.contains_key("unknown-oven"),
        "No entity should be spawned for unknown oven"
    );
}

#[test]
fn fault_state_survives_status_update() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    // Set a fault
    push_envelope(
        &mut app,
        Message::FaultRaised(FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::SafetyLimitExceeded,
            severity: Severity::High,
            message: "Limit exceeded".into(),
        }),
    );
    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let fault = app.world().get::<FaultState>(entity).unwrap();
    assert!(fault.0.is_some(), "Fault should be set");

    // Push status update
    push_envelope(
        &mut app,
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 180.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(50.0),
            state: OvenState::Heating,
        }),
    );
    tick(&mut app);

    // Fault should still be present
    let fault = app.world().get::<FaultState>(entity).unwrap();
    assert!(
        fault.0.is_some(),
        "FaultState must survive OvenStatusUpdated"
    );
    let info = fault.0.as_ref().unwrap();
    assert_eq!(info.fault_code, FaultCode::SafetyLimitExceeded);
}

// ── 3.4 apply_fault_raised tests ─────────────────────────────────────────────

#[test]
fn fault_recorded_on_entity() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::FaultRaised(FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::SensorUnavailable,
            severity: Severity::Medium,
            message: "Sensor offline".into(),
        }),
    );
    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let fault = app.world().get::<FaultState>(entity).unwrap();
    let info = fault.0.as_ref().expect("Fault should be recorded");

    assert_eq!(info.fault_code, FaultCode::SensorUnavailable);
    assert_eq!(info.severity, Severity::Medium);
    assert_eq!(info.message, "Sensor offline");
}

#[test]
fn global_fault_stored_in_resource_not_on_entities() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::FaultRaised(FaultRaisedPayload {
            oven_id: None,
            fault_code: FaultCode::EmergencyStopActive,
            severity: Severity::High,
            message: "System emergency".into(),
        }),
    );
    tick(&mut app);

    let global = app.world().resource::<GlobalFault>();
    assert!(global.0.is_some(), "GlobalFault should be set");
    let info = global.0.as_ref().unwrap();
    assert_eq!(info.fault_code, FaultCode::EmergencyStopActive);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let fault = app.world().get::<FaultState>(entity).unwrap();
    assert!(
        fault.0.is_none(),
        "Oven entity FaultState should not be modified by global fault"
    );
}

// ── 3.5 record_command_result tests ──────────────────────────────────────────

#[test]
fn accepted_command_stores_result() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::CommandAccepted(CommandAcceptedPayload {
            accepted_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            message: "Target set to 250°C".into(),
        }),
    );
    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let result = app.world().get::<LastCommandResult>(entity).unwrap();
    let cmd = result.0.as_ref().expect("Should have a result");

    match cmd {
        CommandResult::Accepted { accepted_type, message } => {
            assert_eq!(accepted_type, "SetTargetTemperature");
            assert_eq!(message, "Target set to 250°C");
        }
        CommandResult::Rejected { .. } => panic!("Expected Accepted, got Rejected"),
    }
}

#[test]
fn rejected_command_stores_result() {
    let mut app = build_test_app();
    spawn_oven(&mut app, "oven1");

    push_envelope(
        &mut app,
        Message::CommandRejected(CommandRejectedPayload {
            rejected_type: "SetOvenEnabled".into(),
            oven_id: Some("oven1".into()),
            reason: FaultCode::OutputUnavailable,
            message: "Oven in fault state".into(),
        }),
    );
    tick(&mut app);

    let entity = find_oven_entity(&app, "oven1").unwrap();
    let result = app.world().get::<LastCommandResult>(entity).unwrap();
    let cmd = result.0.as_ref().expect("Should have a result");

    match cmd {
        CommandResult::Accepted { .. } => panic!("Expected Rejected, got Accepted"),
        CommandResult::Rejected { rejected_type, reason, message } => {
            assert_eq!(rejected_type, "SetOvenEnabled");
            assert_eq!(*reason, FaultCode::OutputUnavailable);
            assert_eq!(message, "Oven in fault state");
        }
    }
}

// ── 3.6 command authoring tests ──────────────────────────────────────────────

#[test]
fn author_set_target_temperature_envelope() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_set_target_temperature_command(
            "oven1".to_string(),
            250.0,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 1);

    let envelope = &outbound.0[0];
    assert_eq!(envelope.source, "pc-app");
    assert_eq!(envelope.target, "rpi-controller");

    match &envelope.payload {
        Message::SetTargetTemperature(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert_eq!(p.target_celsius, 250.0);
        }
        other => panic!("Expected SetTargetTemperature, got {:?}", other),
    }
}

#[test]
fn author_set_oven_enabled_true() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_set_oven_enabled_command(
            "oven1".to_string(),
            true,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 1);

    match &outbound.0[0].payload {
        Message::SetOvenEnabled(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert!(p.enabled);
        }
        other => panic!("Expected SetOvenEnabled, got {:?}", other),
    }
}

#[test]
fn author_set_oven_enabled_false() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_set_oven_enabled_command(
            "oven1".to_string(),
            false,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    match &outbound.0[0].payload {
        Message::SetOvenEnabled(p) => {
            assert!(!p.enabled);
        }
        other => panic!("Expected SetOvenEnabled, got {:?}", other),
    }
}

#[test]
fn author_request_status_single() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_request_status_command(
            Some("oven1".to_string()),
            RequestScope::Single,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 1);

    match &outbound.0[0].payload {
        Message::RequestStatus(p) => {
            assert_eq!(p.oven_id, Some("oven1".to_string()));
            assert_eq!(p.scope, RequestScope::Single);
        }
        other => panic!("Expected RequestStatus, got {:?}", other),
    }
}

#[test]
fn author_request_status_all() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_request_status_command(
            None,
            RequestScope::All,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    match &outbound.0[0].payload {
        Message::RequestStatus(p) => {
            assert!(p.oven_id.is_none());
            assert_eq!(p.scope, RequestScope::All);
        }
        other => panic!("Expected RequestStatus, got {:?}", other),
    }
}

#[test]
fn author_emergency_stop_envelope() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_emergency_stop_command(
            "Overheating detected".to_string(),
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 1);

    match &outbound.0[0].payload {
        Message::EmergencyStop(p) => {
            assert_eq!(p.reason, "Overheating detected");
        }
        other => panic!("Expected EmergencyStop, got {:?}", other),
    }
}

// ── 3.7 integration tests ────────────────────────────────────────────────────

#[test]
fn full_inbound_flow_envelope_to_entity_state() {
    let mut app = build_test_app();

    push_envelopes(
        &mut app,
        vec![
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven1".into(),
                sensor_ref: "temp0".into(),
                output_ref: "relay0".into(),
                max_celsius: 300.0,
            }),
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven2".into(),
                sensor_ref: "temp1".into(),
                output_ref: "relay1".into(),
                max_celsius: 500.0,
            }),
        ],
    );
    tick(&mut app);

    let index = app.world().resource::<OvenIndex>();
    assert_eq!(index.0.len(), 2);

    // Status update for oven1, fault on oven2
    push_envelopes(
        &mut app,
        vec![
            Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "oven1".into(),
                current_celsius: 180.5,
                target_celsius: 250.0,
                enabled: true,
                heating: true,
                output_level: Some(75.0),
                state: OvenState::Heating,
            }),
            Message::FaultRaised(FaultRaisedPayload {
                oven_id: Some("oven2".into()),
                fault_code: FaultCode::SensorUnavailable,
                severity: Severity::High,
                message: "Sensor died".into(),
            }),
        ],
    );
    tick(&mut app);

    // oven1: heated, no fault
    let e1 = find_oven_entity(&app, "oven1").unwrap();
    assert_eq!(app.world().get::<CurrentTemperature>(e1).unwrap().0, 180.5);
    assert_eq!(app.world().get::<OvenStatus>(e1).unwrap().0, OvenState::Heating);
    assert!(app.world().get::<FaultState>(e1).unwrap().0.is_none());

    // oven2: fault recorded
    let e2 = find_oven_entity(&app, "oven2").unwrap();
    let fault2 = app.world().get::<FaultState>(e2).unwrap();
    assert!(fault2.0.is_some());
    assert_eq!(fault2.0.as_ref().unwrap().fault_code, FaultCode::SensorUnavailable);

    // Command result for oven1
    push_envelope(
        &mut app,
        Message::CommandAccepted(CommandAcceptedPayload {
            accepted_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            message: "OK".into(),
        }),
    );
    tick(&mut app);

    let result = app.world().get::<LastCommandResult>(e1).unwrap();
    assert!(result.0.is_some());
}

#[test]
fn full_outbound_flow_command_to_envelope() {
    let mut app = build_test_app();

    {
        let mut outbound = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        pc_app::systems::commands::author_set_target_temperature_command(
            "oven1".to_string(),
            250.0,
            &mut outbound,
        );
        pc_app::systems::commands::author_request_status_command(
            None,
            RequestScope::All,
            &mut outbound,
        );
    }

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    assert_eq!(outbound.0.len(), 2);

    match &outbound.0[0].payload {
        Message::SetTargetTemperature(p) => {
            assert_eq!(p.oven_id, "oven1");
            assert_eq!(p.target_celsius, 250.0);
        }
        other => panic!("Expected SetTargetTemperature, got {:?}", other),
    }

    for env in &outbound.0 {
        assert_eq!(env.source, "pc-app");
        assert_eq!(env.target, "rpi-controller");
    }

    match &outbound.0[1].payload {
        Message::RequestStatus(p) => {
            assert!(p.oven_id.is_none());
            assert_eq!(p.scope, RequestScope::All);
        }
        other => panic!("Expected RequestStatus, got {:?}", other),
    }
}

#[test]
fn multiple_ovens_independent_state() {
    let mut app = build_test_app();

    push_envelopes(
        &mut app,
        vec![
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven1".into(),
                sensor_ref: "s1".into(),
                output_ref: "o1".into(),
                max_celsius: 300.0,
            }),
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven2".into(),
                sensor_ref: "s2".into(),
                output_ref: "o2".into(),
                max_celsius: 400.0,
            }),
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: "oven3".into(),
                sensor_ref: "s3".into(),
                output_ref: "o3".into(),
                max_celsius: 500.0,
            }),
        ],
    );
    tick(&mut app);

    push_envelopes(
        &mut app,
        vec![
            Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "oven1".into(),
                current_celsius: 100.0,
                target_celsius: 200.0,
                enabled: true,
                heating: true,
                output_level: Some(50.0),
                state: OvenState::Heating,
            }),
            Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "oven3".into(),
                current_celsius: 50.0,
                target_celsius: 150.0,
                enabled: true,
                heating: false,
                output_level: None,
                state: OvenState::Idle,
            }),
        ],
    );
    tick(&mut app);

    let e1 = find_oven_entity(&app, "oven1").unwrap();
    assert_eq!(app.world().get::<CurrentTemperature>(e1).unwrap().0, 100.0);
    assert!(app.world().get::<Heating>(e1).unwrap().0);

    let e2 = find_oven_entity(&app, "oven2").unwrap();
    assert_eq!(app.world().get::<CurrentTemperature>(e2).unwrap().0, 0.0);

    let e3 = find_oven_entity(&app, "oven3").unwrap();
    assert_eq!(app.world().get::<CurrentTemperature>(e3).unwrap().0, 50.0);
    assert_eq!(app.world().get::<OvenStatus>(e3).unwrap().0, OvenState::Idle);
}
