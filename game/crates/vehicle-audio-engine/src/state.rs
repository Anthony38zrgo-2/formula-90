//! Deterministic vehicle audio mix state — pure Rust, no Godot/GEVP deps.
//!
//! `VehicleAudioState` is the single source of mixing decisions. It mirrors the
//! offline oracle (`tools/audio/render_audio_scenario.py`) and the native DSP
//! semantics (`native/include/formula90s/audio/engine_dsp.hpp`): five engine
//! bands crossfaded by `normalized_rpm`, throttle-based engine gain, optional
//! surface bed (asphalt has none), and one-shot triggers. Determinism rule:
//! same inputs -> same `Mix`.

use serde::{Deserialize, Serialize};

/// Sample-specific playback metadata loaded from the audio-bank manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineBandProfile {
    pub key: String,
    pub native_rpm: f32,
    pub index: usize,
    pub center: f32,
    pub width: f32,
}

pub const ENGINE_BAND_COUNT: usize = 5;

/// Clamp for pitch_scale (rpm / native_rpm) to avoid extreme chipmunk.
/// PITCH_MAX 3.5: the high band (native 5580) reaches pitch ~3.05 at 17000 RPM,
/// so 2.0 was saturating it (weight-gated telemetry showed mean 1.9995).
pub const PITCH_MIN: f32 = 0.5;
pub const PITCH_MAX: f32 = 3.5;

/// Loop-wrap crossfade in samples (~11.6 ms @ 44.1 kHz). Mirrors offline mixer.
pub const LOOP_XFADE: usize = 512;

/// Surface bed key resolution. Asphalt intentionally has no tire bed (engine only).
pub const SURFACE_ASPHALT: &str = "asphalt";
pub const SURFACE_SAND: &str = "sand";
pub const SURFACE_GRASS: &str = "grass";
pub const SURFACE_RUMBLE: &str = "rumble";

/// One-shot trigger kinds mapped to bank keys (no wind, no ambient).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trigger {
    ShiftUp,
    ShiftDown,
    Backfire,
    Hit1,
    Hit2,
    Hit3,
    Hit4,
    Barrier,
    Cone,
    Fire,
    Scrape,
}

impl Trigger {
    pub fn bank_key(self) -> &'static str {
        match self {
            Trigger::ShiftUp => "shift_up",
            Trigger::ShiftDown => "shift_down",
            Trigger::Backfire => "engine_backfire",
            Trigger::Hit1 => "impact_hit_1",
            Trigger::Hit2 => "impact_hit_2",
            Trigger::Hit3 => "impact_hit_3",
            Trigger::Hit4 => "impact_hit_4",
            Trigger::Barrier => "impact_barrier",
            Trigger::Cone => "impact_cone",
            Trigger::Fire => "impact_fire",
            Trigger::Scrape => "impact_scrape",
        }
    }
}

/// Deterministic inputs to the mix core.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VehicleAudioState {
    /// Motor RPM (from GEVP `motor_rpm`).
    pub rpm: f64,
    /// RPM normalized to [0,1] over [idle_rpm, max_rpm]. Callers should use
    /// [`Self::with_rpm_range`] to compute it; a raw value is accepted too.
    pub normalized_rpm: f32,
    /// Throttle in [0,1] (from GEVP `throttle_amount`).
    pub throttle: f32,
    /// Longitudinal speed in km/h.
    pub speed_kph: f64,
    /// Current gear (>=1 forward, 0 neutral, -1 reverse).
    pub gear: i32,
    /// Aggregate wheel slip in [0,1] (max abs slip across drive wheels).
    pub slip: f32,
    /// Surface token.
    pub surface: &'static str,
}

/// Raw inputs used to build a `VehicleAudioState` (with RPM normalization).
#[derive(Debug, Clone, Copy)]
pub struct StateInput {
    pub rpm: f64,
    pub idle_rpm: f64,
    pub max_rpm: f64,
    pub throttle: f32,
    pub speed_kph: f64,
    pub gear: i32,
    pub slip: f32,
    pub surface: &'static str,
}

impl VehicleAudioState {
    /// Build a state from raw inputs, computing `normalized_rpm` over
    /// [idle_rpm, max_rpm].
    pub fn from_input(input: &StateInput) -> Self {
        let norm = if input.max_rpm > input.idle_rpm {
            ((input.rpm - input.idle_rpm) / (input.max_rpm - input.idle_rpm)) as f32
        } else {
            0.0
        };
        Self {
            rpm: input.rpm,
            normalized_rpm: norm.clamp(0.0, 1.0),
            throttle: input.throttle.clamp(0.0, 1.0),
            speed_kph: input.speed_kph,
            gear: input.gear,
            slip: input.slip.clamp(0.0, 1.0),
            surface: input.surface,
        }
    }

    /// The resulting mix for manifest-derived engine bands.
    pub fn mix(&self, engine_bands: &[EngineBandProfile]) -> Mix {
        mix(self, engine_bands)
    }
}

/// Result of mixing: per-band engine weights + gains + optional trigger.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mix {
    /// Crossfade weights over the five engine bands (normalized, sums to 1).
    pub engine_weights: [f32; 5],
    /// Per-band pitch_scale = rpm / native_rpm, clamped [PITCH_MIN, PITCH_MAX].
    pub engine_pitch_scales: [f32; 5],
    /// Engine gain from throttle (0.45 + 0.55*throttle), matching offline mixer.
    pub engine_gain: f32,
    /// Surface bed bank key, or None for asphalt (no bed).
    pub surface_key: Option<&'static str>,
    /// Surface bed gain.
    pub surface_gain: f32,
    /// Pending one-shot trigger (fire-and-forget), if any.
    pub trigger: Option<Trigger>,
}

/// Compute 5-band triangular crossfade weights from normalized RPM. Mirrors
/// `EngineLayerMixer::weights` in engine_dsp.hpp.
pub fn engine_weights(
    normalized_rpm: f32,
    engine_bands: &[EngineBandProfile],
) -> [f32; ENGINE_BAND_COUNT] {
    let x = normalized_rpm.clamp(0.0, 1.0);
    let mut weights = [0.0f32; ENGINE_BAND_COUNT];
    let mut sum = 0.0f32;
    for band in engine_bands.iter().take(ENGINE_BAND_COUNT) {
        let w = (1.0 - (x - band.center).abs() / band.width).max(0.0);
        weights[band.index] = w;
        sum += w;
    }
    if sum <= 0.0 {
        weights[0] = 1.0;
    } else {
        for weight in &mut weights {
            *weight /= sum;
        }
    }
    weights
}

/// Engine gain from throttle (mirrors offline: 0.45 + 0.55*throttle).
pub fn engine_gain(throttle: f32) -> f32 {
    0.45 + 0.55 * throttle.clamp(0.0, 1.0)
}

/// Per-band pitch scale (rpm / native_rpm), clamped.
pub fn engine_pitch_scale(rpm: f64, band: &EngineBandProfile) -> f32 {
    (rpm as f32 / band.native_rpm).clamp(PITCH_MIN, PITCH_MAX)
}

/// Surface bed gain from slip + speed (mirrors offline mixer).
pub fn surface_gain(slip: f32, speed_kph: f64) -> f32 {
    0.25 + 0.55 * slip.clamp(0.0, 1.0) + 0.12 * (speed_kph.min(120.0) / 120.0) as f32
}

/// Resolve a surface token to a bank bed key (asphalt -> None).
pub fn surface_key(surface: &str) -> Option<&'static str> {
    match surface {
        SURFACE_ASPHALT => None,
        SURFACE_SAND => Some("surf_sand"),
        SURFACE_GRASS => Some("surf_grass"),
        SURFACE_RUMBLE => Some("surf_rumble"),
        _ => None,
    }
}

/// Compute the deterministic mix for a state.
pub fn mix(state: &VehicleAudioState, engine_bands: &[EngineBandProfile]) -> Mix {
    let engine_weights = engine_weights(state.normalized_rpm, engine_bands);
    let egain = engine_gain(state.throttle);
    let skey = surface_key(state.surface);
    let sgain = if skey.is_some() {
        surface_gain(state.slip, state.speed_kph)
    } else {
        0.0
    };
    let mut pitch_scales = [1.0f32; 5];
    for (band, scale) in engine_bands.iter().zip(pitch_scales.iter_mut()) {
        *scale = engine_pitch_scale(state.rpm, band);
    }
    Mix {
        engine_weights,
        engine_pitch_scales: pitch_scales,
        engine_gain: egain,
        surface_key: skey,
        surface_gain: sgain,
        trigger: None,
    }
}

#[cfg(test)]
pub(crate) fn test_engine_bands() -> Vec<EngineBandProfile> {
    [
        ("engine_idle", 3941.0, 0.0),
        ("engine_low", 7429.0, 0.25),
        ("engine_mid", 9800.0, 0.5),
        ("engine_high", 16950.0, 0.75),
        ("engine_redline", 7687.0, 1.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (key, native_rpm, center))| EngineBandProfile {
        key: key.to_string(),
        native_rpm,
        index,
        center,
        width: 0.25,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(norm: f32, throttle: f32, surface: &'static str) -> VehicleAudioState {
        VehicleAudioState {
            rpm: 0.0,
            normalized_rpm: norm,
            throttle,
            speed_kph: 100.0,
            gear: 3,
            slip: 0.1,
            surface,
        }
    }

    #[test]
    fn weights_sum_to_one_and_follow_rpm() {
        for norm in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let w = engine_weights(norm, &test_engine_bands());
            let sum: f32 = w.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "norm={norm} sum={sum}");
        }
        // At idle the idle band dominates.
        assert!(engine_weights(0.0, &test_engine_bands())[0] > 0.9);
        // At redline the redline band dominates.
        assert!(engine_weights(1.0, &test_engine_bands())[4] > 0.9);
    }

    #[test]
    fn weights_are_deterministic() {
        assert_eq!(
            engine_weights(0.42, &test_engine_bands()),
            engine_weights(0.42, &test_engine_bands())
        );
    }

    #[test]
    fn surface_key_asphalt_is_none() {
        assert_eq!(surface_key("asphalt"), None);
        assert_eq!(surface_key("sand"), Some("surf_sand"));
        assert_eq!(surface_key("grass"), Some("surf_grass"));
        assert_eq!(surface_key("rumble"), Some("surf_rumble"));
        assert_eq!(surface_key("unknown"), None);
    }

    #[test]
    fn engine_gain_tracks_throttle() {
        assert!((engine_gain(1.0) - 1.0).abs() < 1e-6);
        assert!((engine_gain(0.0) - 0.45).abs() < 1e-6);
    }

    #[test]
    fn asphalt_mix_has_no_surface_bed() {
        let m = state(0.5, 0.7, "asphalt").mix(&test_engine_bands());
        assert!(m.surface_key.is_none());
        assert_eq!(m.surface_gain, 0.0);
        assert!((m.engine_gain - (0.45 + 0.55 * 0.7)).abs() < 1e-6);
    }

    #[test]
    fn sand_mix_has_bed() {
        let m = state(0.5, 0.7, "sand").mix(&test_engine_bands());
        assert_eq!(m.surface_key, Some("surf_sand"));
        assert!(m.surface_gain > 0.0);
    }

    #[test]
    fn with_rpm_range_normalizes_correctly() {
        let s = VehicleAudioState::from_input(&StateInput {
            rpm: 6000.0,
            idle_rpm: 1000.0,
            max_rpm: 7000.0,
            throttle: 1.0,
            speed_kph: 150.0,
            gear: 3,
            slip: 0.0,
            surface: "asphalt",
        });
        assert!((s.normalized_rpm - ((6000.0 - 1000.0) / 6000.0) as f32).abs() < 1e-5);
    }

    #[test]
    fn trigger_bank_keys_are_stable() {
        assert_eq!(Trigger::ShiftUp.bank_key(), "shift_up");
        assert_eq!(Trigger::Barrier.bank_key(), "impact_barrier");
        assert_eq!(Trigger::Cone.bank_key(), "impact_cone");
    }

    #[test]
    fn pitch_scales_track_rpm() {
        let bands = test_engine_bands();
        // At a band's native RPM its pitch is 1.0; above native it rises.
        assert!((engine_pitch_scale(3941.0, &bands[0]) - 1.0).abs() < 1e-5);
        assert!(engine_pitch_scale(8000.0, &bands[0]) > 1.0);
        assert!(engine_pitch_scale(2000.0, &bands[0]) < 1.0);
        // Clamped
        assert!(engine_pitch_scale(0.0, &bands[4]) >= PITCH_MIN);
        assert!(engine_pitch_scale(99999.0, &bands[0]) <= PITCH_MAX);
    }

    #[test]
    fn native_rpm_are_measured_not_laddered() {
        // Regression guard: high/redline were once cherry-picked to a monotonic
        // ladder (11208/15342) instead of the honest recorded revs (5580/7687).
        let bands = test_engine_bands();
        assert!((bands[2].native_rpm - 9800.0).abs() < 1.0);
        assert!((bands[3].native_rpm - 16950.0).abs() < 1.0);
        assert!((bands[4].native_rpm - 7687.0).abs() < 1.0);
        assert!((engine_pitch_scale(16950.0, &bands[3]) - 1.0).abs() < 1e-5);
        assert!((engine_pitch_scale(7687.0, &bands[4]) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn mix_pitch_scales_follow_rpm() {
        let s = VehicleAudioState::from_input(&StateInput {
            rpm: 7429.0,
            idle_rpm: 1000.0,
            max_rpm: 17000.0,
            throttle: 1.0,
            speed_kph: 0.0,
            gear: 1,
            slip: 0.0,
            surface: "asphalt",
        });
        let m = s.mix(&test_engine_bands());
        // At low band native RPM the low band pitch is ~1.0
        assert!((m.engine_pitch_scales[1] - 1.0).abs() < 1e-4);
    }
}
