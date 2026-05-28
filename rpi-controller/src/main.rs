//! rpi-controller entry point — headless Bevy ECS application.
//!
//! Usage:
//! ```
//! cargo run --package rpi-controller -- --simulate N   # Creates N simulated ovens at startup
//! ```
//!
//! No `--port` flag — MockTransport replaced by Inbound/OutboundProtocolQueue for v1.

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let simulate_count = if let Some(idx) = args.iter().position(|a| a == "--simulate") {
        args.get(idx + 1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
    } else {
        0
    };

    if simulate_count > 0 {
        eprintln!("rpi-controller starting with {} simulated ovens", simulate_count);
    } else {
        eprintln!("rpi-controller starting (no simulated ovens)");
    }

    let mut app = rpi_controller::bevy_app::build_app(simulate_count);

    // Run the headless Bevy app — ScheduleRunnerPlugin drives the loop
    app.run();

    eprintln!("rpi-controller shutting down");
}