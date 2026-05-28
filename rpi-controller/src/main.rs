//! rpi-controller entry point — headless Bevy ECS application.
//!
//! Usage:
//! ```
//! cargo run --package rpi-controller -- --simulate N
//! cargo run --package rpi-controller -- --simulate N --listen 127.0.0.1:7000
//! ```
//!
//! No `--port` flag — MockTransport replaced by Inbound/OutboundProtocolQueue for v1.

use std::net::SocketAddr;

use bevy::prelude::{IntoSystemConfigs, Res, ResMut, Update};

use rpi_controller::resources::{InboundProtocolQueue, OutboundProtocolQueue};
use rpi_controller::systems::update;
use transport::{InboundReceiver, OutboundSender, TransportConfig, TransportPlugin};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let simulate_count = if let Some(idx) = args.iter().position(|a| a == "--simulate") {
        args.get(idx + 1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
    } else {
        0
    };

    let listen_addr: Option<SocketAddr> = if let Some(idx) =
        args.iter().position(|a| a == "--listen")
    {
        args.get(idx + 1)
            .and_then(|s| s.parse::<SocketAddr>().ok())
    } else {
        None
    };

    if simulate_count > 0 {
        eprintln!("rpi-controller starting with {} simulated ovens", simulate_count);
    } else {
        eprintln!("rpi-controller starting (no simulated ovens)");
    }

    let mut app = rpi_controller::bevy_app::build_app(simulate_count);

    if let Some(addr) = listen_addr {
        app.add_plugins(TransportPlugin {
            config: TransportConfig::server(addr),
        });

        // Bridge systems: connect app queues to transport channels
        app.add_systems(
            Update,
            bridge_inbound_from_transport.before(update::ingest_commands),
        );
        app.add_systems(
            Update,
            bridge_outbound_to_transport.after(update::emit_protocol_responses),
        );
    }

    // Run the headless Bevy app — ScheduleRunnerPlugin drives the loop
    app.run();

    eprintln!("rpi-controller shutting down");
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
