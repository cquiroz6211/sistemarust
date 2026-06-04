//! pc-app — PC-side Bevy ECS application for oven read model and command authoring.
//!
//! This crate consumes Raspberry Pi protocol events and maintains a local read
//! model of oven state. v1 is headless/testable — no visual UI, no real transport.

pub mod components;
pub mod events;
pub mod plugins;
pub mod resources;
pub mod systems;
pub mod ui;

pub use plugins::pc_app::PcAppPlugin;
