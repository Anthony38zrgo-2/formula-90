#![deny(unsafe_op_in_unsafe_fn)]
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
pub mod dsp_contract;
pub mod dsp_runtime;
pub mod ffi;
pub mod mixer;
pub mod powertrain;
pub mod state;
pub mod synth;
pub mod telemetry;
pub mod tire_scrub;

pub use adapter::{surface_token, GevpTelemetry};
pub use bank::{BankError, Sample, VehicleSoundBank};
pub use config::{SoundMixerConfig, MIXER_CONFIG_FILENAME};
pub use ffi::{vehicle_audio_abi_version, VEHICLE_AUDIO_ABI_VERSION};
pub use mixer::{
    AudioConfig, ContinuousDiagnostics, ContinuousSourceKind, DiagnosticMode, V10LayerTuning,
    VehicleAudioEngine,
};
pub use powertrain::AudioPowertrainSynthesis;
pub use state::{engine_weights, mix, EngineBandProfile, Mix, Trigger, VehicleAudioState};

/// The default relative path to the v10_vehicle bank from the game/ directory.
pub const DEFAULT_BANK_REL: &str = "sounds/banks/v10_vehicle";

#[cfg(test)]
pub(crate) mod allocation_probe {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct TrackingAllocator;
    thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; }
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ACTIVE.with(|active| {
                if active.get() {
                    ALLOCS.fetch_add(1, Ordering::Relaxed);
                }
            });
            // SAFETY: delegates to the system allocator with the caller-provided valid `layout`.
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ACTIVE.with(|active| {
                if active.get() {
                    FREES.fetch_add(1, Ordering::Relaxed);
                }
            });
            // SAFETY: `ptr`/`layout` come from a prior `alloc` on this allocator, as required by `GlobalAlloc`.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    pub fn start() {
        ALLOCS.store(0, Ordering::Relaxed);
        FREES.store(0, Ordering::Relaxed);
        ACTIVE.with(|active| active.set(true));
    }
    pub fn stop() -> (usize, usize) {
        ACTIVE.with(|active| active.set(false));
        (
            ALLOCS.load(Ordering::Relaxed),
            FREES.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
#[global_allocator]
static TEST_ALLOCATOR: allocation_probe::TrackingAllocator = allocation_probe::TrackingAllocator;
