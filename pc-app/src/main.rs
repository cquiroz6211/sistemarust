//! pc-app entry point — headless Bevy ECS application.
//!
//! Usage:
//! ```
//! cargo run --package pc-app
//! cargo run --package pc-app -- --connect 127.0.0.1:7000
//! cargo run --package pc-app -- --connect 127.0.0.1:7000 --demo
//! ```
//!
//! The app runs headlessly with `MinimalPlugins` + `ScheduleRunnerPlugin` at 50ms.
//! Without `--connect`, queues are in-memory Resources (no transport).
//! With `--connect`, a TCP transport bridges queues to a remote process.

use std::net::SocketAddr;
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::{App, DefaultPlugins, IntoSystemConfigs, Local, MinimalPlugins, PluginGroup, Res, ResMut, Update};

use pc_app::resources::{InboundProtocolQueue, OvenIndex, OutboundProtocolQueue};
use pc_app::systems::commands::{
    author_request_status_command, author_set_target_temperature_command,
};
use pc_app::PcAppPlugin;
use protocol::RequestScope;
use transport::{InboundReceiver, OutboundSender, TransportConfig, TransportPlugin};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let headless = args.iter().any(|a| a == "--headless");

    let connect_addr: Option<SocketAddr> = if let Some(idx) =
        args.iter().position(|a| a == "--connect")
    {
        args.get(idx + 1)
            .and_then(|s| s.parse::<SocketAddr>().ok())
    } else {
        None
    };

    let demo_mode = args.iter().any(|a| a == "--demo");

    let mut app = App::new();

    if headless {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(50),
        )));
    } else {
        app.add_plugins(DefaultPlugins);
    }

    app.add_plugins(PcAppPlugin);

    if let Some(addr) = connect_addr {
        app.add_plugins(TransportPlugin {
            config: TransportConfig::client(addr),
        });

        // Bridge systems: connect app queues to transport channels
        app.add_systems(
            Update,
            bridge_inbound_from_transport.before(pc_app::systems::ingest::ingest_inbound_protocol),
        );
        app.add_systems(Update, bridge_outbound_to_transport);

        if demo_mode {
            app.add_systems(Update, demo_system.before(bridge_outbound_to_transport));
            eprintln!("[DEMO] Demo mode active — will auto-send commands");
        }
    }

    // UI plugins — skipped in headless mode
    if !headless {
        app.add_plugins(bevy_egui::EguiPlugin);
        app.add_plugins(pc_app::plugins::ui::UiPlugin);
    }

    app.run();
}

/// Bridge: drains `OutboundProtocolQueue` → sends batch through transport channel.
fn bridge_outbound_to_transport(
    mut outbound: ResMut<OutboundProtocolQueue>,
    sender: Res<OutboundSender>,
) {
    let batch = std::mem::take(&mut outbound.0);
    if !batch.is_empty() {
        if let Err(e) = sender.0.try_send(batch) {
            eprintln!("[WARN] Transport outbound channel full or closed: {}", e);
        }
    }
}

/// Bridge: receives deserialized envelopes from transport channel → pushes to `InboundProtocolQueue`.
fn bridge_inbound_from_transport(
    mut inbound: ResMut<InboundProtocolQueue>,
    receiver: Res<InboundReceiver>,
) {
    let mut rx = receiver.0.lock().unwrap();
    while let Ok(batch) = rx.try_recv() {
        inbound.0.extend(batch);
    }
}

/// Demo system: auto-sends commands after ovens are detected.
#[derive(Default)]
struct DemoState {
    step: u32,
    ticks: u32,
}

fn demo_system(
    mut outbound: ResMut<OutboundProtocolQueue>,
    oven_index: Res<OvenIndex>,
    mut state: Local<DemoState>,
) {
    match state.step {
        0 => {
            if !oven_index.0.is_empty() {
                let oven_id = oven_index.0.keys().next().unwrap().clone();
                author_set_target_temperature_command(oven_id.clone(), 250.0, &mut outbound);
                eprintln!("[DEMO] Sent SetTargetTemperature({}) → 250°C", oven_id);
                state.step = 1;
            }
        }
        1 => {
            state.ticks += 1;
            if state.ticks >= 40 {
                author_request_status_command(None, RequestScope::All, &mut outbound);
                eprintln!("[DEMO] Sent RequestStatus(All)");
                state.step = 2;
            }
        }
        _ => {
            // Demo cycle complete
        }
    }
}
