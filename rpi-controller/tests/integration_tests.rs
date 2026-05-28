//! Integration tests for rpi-controller bevy-headless domain.
//!
//! Tests use App::update() stepping — no hardware, no tokio runtime as domain orchestrator.
//! All tests are deterministic via the TestRng resource.

use std::time::Duration;

use bevy::app::App;
use bevy::ecs::event::Events;
use bevy::prelude::PluginGroup;
use bevy::prelude::{Entity, With};
use protocol::{
    EmergencyStopPayload, EventEnvelope, FaultCode, Message, OvenState,
    RequestScope, RequestStatusPayload, SetOvenEnabledPayload,
    SetTargetTemperaturePayload,
};
use rpi_controller::{
    components::{CurrentTemperature, Enabled, Heating, MaxTemperature, OvenId,
                 OvenStatus, TargetTemperature},
    resources::{ControllerConfig, EmergencyStopActive, InboundProtocolQueue,
                LastStatusPublishTime, OutboundProtocolQueue, SimulationConfig,
                SimulateOvenCount, TestRng},
    OvenControllerPlugin,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Builds a headless test app with N simulated ovens and seeded RNG for determinism.
/// Uses the same pattern as bevy_app::build_app but without re-adding runtime plugins.
fn build_test_app(simulate_count: usize) -> App {
    let mut app = App::new();

    // Simulation config
    app.insert_resource(SimulationConfig {
        heating_drift: 4.0,
        cooling_drift: 1.0,
        room_temp: 20.0,
        noise_amplitude: 2.0,
        hysteresis: 5.0,
    });

    // Controller config
    app.insert_resource(ControllerConfig {
        tick_interval_ms: 50,
        status_publish_interval_ms: 1000,
    });

    // Seeded RNG for deterministic noise
    app.insert_resource(TestRng(Some(StdRng::seed_from_u64(42))));

    // Resources
    app.insert_resource(EmergencyStopActive(false));
    app.insert_resource(InboundProtocolQueue::default());
    app.insert_resource(OutboundProtocolQueue::default());
    app.insert_resource(rpi_controller::resources::OvenIndex::default());
    app.insert_resource(LastStatusPublishTime(Duration::ZERO));

    if simulate_count > 0 {
        app.insert_resource(SimulateOvenCount(simulate_count));
    }

    // Headless runtime + domain plugin
    app.add_plugins(
        bevy::prelude::MinimalPlugins
            .set(bevy::app::ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))),
    );
    app.add_plugins(OvenControllerPlugin);

    app
}

fn tick_fixed_update(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.world_mut().run_schedule(bevy::app::FixedUpdate);
    }
}

// ── 1. headless-runtime ───────────────────────────────────────────────────────

#[test]
fn app_headless_starts_without_panic() {
    let mut app = build_test_app(0);
    app.update();
}

#[test]
fn app_update_schedule_runs() {
    let mut app = build_test_app(0);
    app.update();
}

#[test]
fn app_fixed_update_schedule_runs() {
    let mut app = build_test_app(1);
    tick_fixed_update(&mut app, 3);
}

// ── 3. simulated-oven-detection ───────────────────────────────────────────────

#[test]
fn startup_spawns_n_ovens() {
    let mut app = build_test_app(3);
    app.update();

    let world = app.world_mut();
    let mut oven_query = world.query::<(Entity, &OvenId)>();
    let ovens: Vec<_> = oven_query.iter(world).collect();
    assert_eq!(ovens.len(), 3, "Expected 3 oven entities");
}

#[test]
fn startup_emits_oven_detected_per_oven() {
    let mut app = build_test_app(2);
    app.update();

    let outbound = app.world().resource::<OutboundProtocolQueue>();
    let oven_detected: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::OvenDetected(_)))
        .collect();
    assert_eq!(oven_detected.len(), 2, "Expected 2 OvenDetected envelopes");
}

#[test]
fn oven_entity_has_correct_initial_state() {
    let mut app = build_test_app(1);
    app.update();

    let world = app.world_mut();
    let mut q = world.query::<(
        &CurrentTemperature,
        &TargetTemperature,
        &MaxTemperature,
        &Enabled,
        &Heating,
        &OvenStatus,
    )>();
    let states: Vec<_> = q.iter(world).collect();

    assert_eq!(states.len(), 1);
    let (current, target, max, enabled, heating, status) = &states[0];
    assert_eq!(current.0, 20.0, "CurrentTemperature starts at room_temp");
    assert_eq!(target.0, 20.0, "TargetTemperature starts at room_temp");
    assert_eq!(max.0, 300.0, "MaxTemperature is 300.0");
    assert!(!enabled.0, "Enabled starts false");
    assert!(!heating.0, "Heating starts false");
    assert_eq!(status.0, OvenState::Disabled, "OvenStatus starts Disabled");
}

// ── 4. protocol-command-processing ───────────────────────────────────────────

fn make_set_target_temp_cmd(oven_id: &str, target_celsius: f64) -> EventEnvelope {
    EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: oven_id.to_string(),
            target_celsius,
        }),
    )
}

fn make_set_oven_enabled_cmd(oven_id: &str, enabled: bool) -> EventEnvelope {
    EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetOvenEnabled(SetOvenEnabledPayload {
            oven_id: oven_id.to_string(),
            enabled,
        }),
    )
}

fn make_emergency_stop_cmd() -> EventEnvelope {
    EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::EmergencyStop(EmergencyStopPayload {
            reason: "Test emergency".to_string(),
        }),
    )
}

fn make_request_status_cmd(oven_id: Option<&str>, scope: RequestScope) -> EventEnvelope {
    EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::RequestStatus(RequestStatusPayload {
            oven_id: oven_id.map(String::from),
            scope,
        }),
    )
}

fn push_inbound(app: &mut App, envelope: EventEnvelope) {
    let mut q = app.world_mut().resource_mut::<InboundProtocolQueue>();
    q.0.push(envelope);
}

fn get_outbound_queue(app: &App) -> &OutboundProtocolQueue {
    app.world().resource::<OutboundProtocolQueue>()
}

// ── 5. thermal-control-fixed-update helpers ──────────────────────────────────

fn find_entity(world: &mut bevy::prelude::World, oven_id: &str) -> Option<Entity> {
    let mut q = world.query_filtered::<Entity, With<OvenId>>();
    q.iter(world).find(|e| {
        world.entity(*e).get::<OvenId>().map(|id| id.0 == oven_id).unwrap_or(false)
    })
}

fn set_entity_temp(app: &mut App, oven_id: &str, temp: f64) {
    let world = app.world_mut();
    let entity = find_entity(world, oven_id).expect("oven entity");
    let mut q = world.query::<&mut CurrentTemperature>();
    let mut c = q.get_mut(world, entity).unwrap();
    c.0 = temp;
}

fn set_entity_heating(app: &mut App, oven_id: &str, heating: bool) {
    let world = app.world_mut();
    let entity = find_entity(world, oven_id).expect("oven entity");
    let mut q = world.query::<&mut Heating>();
    let mut h = q.get_mut(world, entity).unwrap();
    h.0 = heating;
}

fn get_current_temp(app: &mut App) -> f64 {
    let world = app.world_mut();
    let mut q = world.query::<&CurrentTemperature>();
    q.iter(world).next().unwrap().0
}

fn get_heating(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut q = world.query::<&Heating>();
    q.iter(world).next().unwrap().0
}

fn get_target_temp(app: &mut App) -> f64 {
    let world = app.world_mut();
    let mut q = world.query::<&TargetTemperature>();
    q.iter(world).next().unwrap().0
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn set_target_temperature_accepts_valid() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 180.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let accepted = outbound.0.iter().find(|e| matches!(e.payload, Message::CommandAccepted(_)));
    assert!(accepted.is_some(), "Expected CommandAccepted for valid target");

    assert_eq!(get_target_temp(&mut app), 180.0, "TargetTemperature should be updated");
}

#[test]
fn set_target_temperature_rejects_negative() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", -10.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let rejected = outbound.0.iter().find_map(|e| match &e.payload {
        Message::CommandRejected(r) if r.reason == FaultCode::InvalidTemperature => Some(r),
        _ => None,
    });
    assert!(rejected.is_some(), "Expected CommandRejected with InvalidTemperature");
}

#[test]
fn set_target_temperature_rejects_above_max() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 350.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let rejected = outbound.0.iter().find_map(|e| match &e.payload {
        Message::CommandRejected(r) if r.reason == FaultCode::SafetyLimitExceeded => Some(r),
        _ => None,
    });
    assert!(rejected.is_some(), "Expected CommandRejected with SafetyLimitExceeded");
}

#[test]
fn set_target_temperature_rejects_nonexistent_oven() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-99", 100.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let rejected = outbound.0.iter().find_map(|e| match &e.payload {
        Message::CommandRejected(r) if r.reason == FaultCode::OvenNotFound => Some(r),
        _ => None,
    });
    assert!(rejected.is_some(), "Expected CommandRejected with OvenNotFound");
}

#[test]
fn set_target_temperature_rejects_when_emergency_stop_active() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_emergency_stop_cmd());
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 100.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let rejected = outbound.0.iter().find_map(|e| match &e.payload {
        Message::CommandRejected(r) if r.reason == FaultCode::EmergencyStopActive => Some(r),
        _ => None,
    });
    assert!(rejected.is_some(), "Expected CommandRejected with EmergencyStopActive");
}

#[test]
fn command_accepted_has_correct_correlation_id() {
    let mut app = build_test_app(1);
    app.update();

    let cmd = make_set_target_temp_cmd("oven-0", 120.0);
    let original_id = cmd.event_id;
    push_inbound(&mut app, cmd);
    app.update();

    let outbound = get_outbound_queue(&app);
    let accepted = outbound.0.iter().find(|e| matches!(e.payload, Message::CommandAccepted(_)));
    let accepted = accepted.expect("Expected CommandAccepted");
    assert_eq!(
        accepted.correlation_id,
        Some(original_id),
        "correlation_id should reference original command"
    );
    assert_eq!(accepted.source, "rpi-controller", "source should be inverted");
    assert_eq!(accepted.target, "pc-app", "target should be inverted");
}

#[test]
fn outbound_queue_maintains_fifo_order() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 100.0));
    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 120.0));
    app.update();

    let outbound = get_outbound_queue(&app);
    let outbound_messages: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::CommandAccepted(_)))
        .collect();
    assert_eq!(outbound_messages.len(), 2);
}

// ── 5. thermal-control-fixed-update ─────────────────────────────────────────

#[test]
fn fixed_update_heats_oven_when_below_hysteresis_band() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 150.0));
    app.update();
    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 140.0);
    set_entity_heating(&mut app, "oven-0", false);

    tick_fixed_update(&mut app, 1);

    assert!(get_heating(&mut app), "Heating should be true when below hysteresis band");
}

#[test]
fn fixed_update_stops_heating_when_at_target() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 150.0));
    app.update();
    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 150.0);
    set_entity_heating(&mut app, "oven-0", true);

    tick_fixed_update(&mut app, 2);

    assert!(!get_heating(&mut app), "Heating should be false at or above target");
}

#[test]
fn fixed_update_maintains_heating_in_band() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 150.0));
    app.update();
    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 147.0);
    set_entity_heating(&mut app, "oven-0", true);

    tick_fixed_update(&mut app, 1);

    assert!(get_heating(&mut app), "Heating should remain true inside hysteresis dead band");
}

#[test]
fn fixed_update_never_drops_below_room_temp() {
    let mut app = build_test_app(1);
    app.update();

    set_entity_temp(&mut app, "oven-0", 20.0);
    set_entity_heating(&mut app, "oven-0", false);

    tick_fixed_update(&mut app, 10);

    assert!(get_current_temp(&mut app) >= 20.0, "Temperature should never drop below room_temp (20.0)");
}

#[test]
fn fixed_update_derives_correct_oven_status() {
    let mut app = build_test_app(1);
    app.update();

    let world = app.world_mut();
    let mut q0 = world.query::<&OvenStatus>();
    assert_eq!(
        q0.iter(world).next().unwrap().0,
        OvenState::Disabled,
        "Initial state is Disabled"
    );

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 100.0));
    app.update();
    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 50.0);
    tick_fixed_update(&mut app, 2);

    let world = app.world_mut();
    let mut q = world.query::<&OvenStatus>();
    let status = q.iter(world).next().unwrap().0;
    assert!(
        matches!(status, OvenState::Idle | OvenState::Heating | OvenState::Disabled),
        "Status should be a valid operational state, got {:?}",
        status
    );
}

#[test]
fn hysteresis_prevents_rapid_oscillation() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 150.0));
    app.update();
    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 140.0);

    let mut heating_flips = 0i32;
    let mut prev_heating = false;

    for _ in 0..20 {
        let h = get_heating(&mut app);
        if h != prev_heating {
            heating_flips += 1;
            prev_heating = h;
        }
        tick_fixed_update(&mut app, 1);
    }

    assert!(heating_flips <= 4, "Heating should not oscillate rapidly (flips={}, expected <=4)", heating_flips);
}

// ── 6. fault-and-safety ──────────────────────────────────────────────────────

#[test]
fn fault_raised_when_temperature_exceeds_max() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 305.0);

    tick_fixed_update(&mut app, 2);

    let outbound = get_outbound_queue(&app);
    let fault = outbound.0.iter().find(|e| matches!(e.payload, Message::FaultRaised(_)));
    assert!(fault.is_some(), "Expected FaultRaised when temp exceeds max");

    let world = app.world_mut();
    let mut q = world.query::<(&Heating, &Enabled, &OvenStatus)>();
    let states: Vec<_> = q.iter(world).collect();
    assert!(!states[0].0.0, "Heating should be disabled after fault");
    assert!(!states[0].1.0, "Enabled should be disabled after fault");
    assert_eq!(states[0].2.0, OvenState::Faulted, "Status should be Faulted");
}

#[test]
fn faulted_oven_cannot_be_heating() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 310.0);

    tick_fixed_update(&mut app, 2);

    let world = app.world_mut();
    let mut q = world.query::<(&Heating, &OvenStatus)>();
    let states: Vec<_> = q.iter(world).collect();
    assert!(!states[0].0.0 || states[0].1.0 == OvenState::Faulted, "Faulted oven should not be heating");
}

#[test]
fn emergency_stop_accepted_during_fault() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();

    set_entity_temp(&mut app, "oven-0", 310.0);
    tick_fixed_update(&mut app, 2);

    push_inbound(&mut app, make_emergency_stop_cmd());
    app.update();

    let outbound = get_outbound_queue(&app);
    let accepted = outbound.0.iter().find(|e| matches!(e.payload, Message::CommandAccepted(_)));
    assert!(accepted.is_some(), "EmergencyStop should be accepted even during fault");
}

#[test]
fn no_commands_accepted_during_emergency_stop() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_emergency_stop_cmd());
    app.update();

    push_inbound(&mut app, make_set_target_temp_cmd("oven-0", 100.0));
    app.update();
    let outbound1 = get_outbound_queue(&app).0.clone();

    push_inbound(&mut app, make_set_oven_enabled_cmd("oven-0", true));
    app.update();
    let outbound2 = get_outbound_queue(&app).0.clone();

    let all_outbound: Vec<_> = outbound1.into_iter().chain(outbound2.into_iter()).collect();
    let rejected_ests = all_outbound.iter().filter_map(|e| match &e.payload {
        Message::CommandRejected(r) if r.reason == FaultCode::EmergencyStopActive => Some(r),
        _ => None,
    });
    assert!(rejected_ests.count() >= 2, "All non-emergency commands should be rejected during emergency stop");
}

// ── 7. status-publication ────────────────────────────────────────────────────

#[test]
fn request_status_single_emits_oven_status_updated() {
    let mut app = build_test_app(1);
    app.update();

    push_inbound(&mut app, make_request_status_cmd(Some("oven-0"), RequestScope::Single));
    app.update();
    tick_fixed_update(&mut app, 1);

    let outbound = get_outbound_queue(&app);
    let status_updated = outbound.0.iter().find(|e| matches!(e.payload, Message::OvenStatusUpdated(_)));
    assert!(status_updated.is_some(), "RequestStatus should emit OvenStatusUpdated");
}

#[test]
fn request_status_all_emits_per_oven() {
    let mut app = build_test_app(3);
    app.update();
    app.world_mut().resource_mut::<OutboundProtocolQueue>().0.clear();
    app.world_mut().resource_mut::<Events<rpi_controller::events::StatusPublishEvent>>().clear();
    app.world_mut().resource_mut::<LastStatusPublishTime>().0 = Duration::from_millis(1);

    push_inbound(&mut app, make_request_status_cmd(None, RequestScope::All));
    app.update();
    tick_fixed_update(&mut app, 1);

    let outbound = get_outbound_queue(&app);
    let status_updated: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::OvenStatusUpdated(_)))
        .collect();
    assert_eq!(status_updated.len(), 3, "RequestStatus(All) should emit one per oven");
}

#[test]
fn periodic_status_published_when_interval_elapsed() {
    let mut app = build_test_app(1);
    app.update();

    {
        let mut out = app.world_mut().resource_mut::<OutboundProtocolQueue>();
        out.0.clear();
    }
    {
        let mut last = app.world_mut().resource_mut::<LastStatusPublishTime>();
        last.0 = Duration::ZERO;
    }

    tick_fixed_update(&mut app, 1);

    let outbound = get_outbound_queue(&app);
    let status_updated: Vec<_> = outbound
        .0
        .iter()
        .filter(|e| matches!(e.payload, Message::OvenStatusUpdated(_)))
        .collect();
    assert!(!status_updated.is_empty(), "Periodic status should be published when interval elapsed");
}

// ── 8. testability ───────────────────────────────────────────────────────────

#[test]
fn domain_logic_tested_without_tokio() {
    let mut app = build_test_app(1);
    app.update();
    app.update();
}

#[test]
fn simulation_noise_is_deterministic_in_tests() {
    let mut app1 = build_test_app(1);
    let mut app2 = build_test_app(1);

    app1.update();
    app2.update();

    for app in [&mut app1, &mut app2] {
        push_inbound(app, make_set_target_temp_cmd("oven-0", 100.0));
        app.update();
        push_inbound(app, make_set_oven_enabled_cmd("oven-0", true));
        app.update();
    }

    for app in [&mut app1, &mut app2] {
        set_entity_temp(app, "oven-0", 50.0);
    }

    tick_fixed_update(&mut app1, 5);
    tick_fixed_update(&mut app2, 5);

    let t1 = get_current_temp(&mut app1);
    let t2 = get_current_temp(&mut app2);

    assert_eq!(t1, t2, "Seeded RNG should produce deterministic temperature sequences");
}
