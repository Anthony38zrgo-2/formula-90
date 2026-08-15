//! Formula-90 vehicle audio runtime core (pure Rust).
//!
//! Consumes the approved `v10_vehicle` sample bank and produces deterministic
//! mixing decisions. No Godot, no GEVP, no synthesis in this core.
//!
//! Modules:
//! - [`state`]: deterministic mix state + weights + gains.
//! - [`bank`]: `VehicleSoundBank` loader/validator.
//! - [`dsp`]: loop crossfade helpers.
//! - [`adapter`]: GEVP telemetry -> state mapping.
//!
//! The thin Godot controller (`vehicle_audio_controller.gd`) reads GEVP
//! telemetry and drives `AudioStreamWAV` players; all audio rules live here.

pub mod adapter;
pub mod bank;
pub mod dsp;
pub mod state;
pub mod telemetry;

pub use adapter::{surface_token, GevpTelemetry};
pub use bank::{Sample, VehicleSoundBank};
pub use state::{engine_weights, mix, Mix, Trigger, VehicleAudioState};

/// The default relative path to the v10_vehicle bank from the game/ directory.
pub const DEFAULT_BANK_REL: &str = "sounds/banks/v10_vehicle";
