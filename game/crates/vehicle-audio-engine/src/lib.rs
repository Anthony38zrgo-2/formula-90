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
//! - [`mixer`]: real-time sample-accurate synthesis + limiter (runtime core).
//! - [`ffi`]: `extern "C"` ABI surface consumed by the C++ GDExtension.
//!
//! In the live game the C++ `VehicleAudioControllerNative` node loads the Rust
//! `cdylib`, calls [`ffi::vehicle_audio_set_state`] from vehicle telemetry, and
//! pulls samples via [`ffi::vehicle_audio_render`] into an `AudioStreamGenerator`.
//! The GDScript `vehicle_audio_controller.gd` is a fallback that mirrors the
//! same rules.

pub mod adapter;
pub mod bank;
pub mod config;
pub mod dsp;
pub mod ffi;
pub mod mixer;
pub mod state;
pub mod telemetry;
pub mod tire_scrub;

pub use adapter::{surface_token, GevpTelemetry};
pub use bank::{BankError, Sample, VehicleSoundBank};
pub use config::{SoundMixerConfig, MIXER_CONFIG_FILENAME};
pub use ffi::{vehicle_audio_abi_version, VEHICLE_AUDIO_ABI_VERSION};
pub use mixer::{AudioConfig, VehicleAudioEngine};
pub use state::{engine_weights, mix, Mix, Trigger, VehicleAudioState};

/// The default relative path to the v10_vehicle bank from the game/ directory.
pub const DEFAULT_BANK_REL: &str = "sounds/banks/v10_vehicle";
