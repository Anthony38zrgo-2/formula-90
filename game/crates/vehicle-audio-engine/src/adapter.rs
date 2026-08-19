//! GEVP telemetry adapter — maps values read from `vehicle.gd` /
//! `vehicle_controllergd.gd` into [`crate::state::VehicleAudioState`].
//!
//! Pure data mapping (no audio rules). The GDScript side only reads GEVP fields
//! and calls [`GevpTelemetry::to_state`]; all mixing logic stays in Rust.

use crate::state::{
    StateInput, VehicleAudioState, SURFACE_ASPHALT, SURFACE_GRASS, SURFACE_RUMBLE, SURFACE_SAND,
};

/// Telemetry as exposed by GEVP `vehicle.gd` (`motor_rpm`, `throttle_amount`,
/// `current_gear`) plus runtime speed/slip/surface read by the controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GevpTelemetry {
    pub motor_rpm: f64,
    pub idle_rpm: f64,
    pub max_rpm: f64,
    pub throttle_amount: f32,
    pub speed_kph: f64,
    pub current_gear: i32,
    /// Max abs wheel slip across drive wheels (0..1 approximation).
    pub wheel_slip: f32,
    /// Surface token: asphalt | sand | grass | rumble.
    pub surface: &'static str,
}

impl GevpTelemetry {
    /// Map GEVP telemetry into a `VehicleAudioState` for the mix core.
    pub fn to_state(&self) -> VehicleAudioState {
        VehicleAudioState::from_input(&StateInput {
            rpm: self.motor_rpm,
            idle_rpm: self.idle_rpm,
            max_rpm: self.max_rpm,
            throttle: self.throttle_amount,
            speed_kph: self.speed_kph,
            gear: self.current_gear,
            slip: self.wheel_slip,
            surface: self.surface,
        })
    }
}

/// Map a surface string (from Godot group/area detection) to a stable token.
/// Unknown surfaces fall back to asphalt (no tire bed) to stay safe.
pub fn surface_token(surface: &str) -> &'static str {
    match surface.trim().to_ascii_lowercase().as_str() {
        SURFACE_SAND => SURFACE_SAND,
        SURFACE_GRASS => SURFACE_GRASS,
        SURFACE_RUMBLE => SURFACE_RUMBLE,
        _ => SURFACE_ASPHALT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_maps_to_state() {
        let t = GevpTelemetry {
            motor_rpm: 6000.0,
            idle_rpm: 1000.0,
            max_rpm: 7000.0,
            throttle_amount: 0.8,
            speed_kph: 160.0,
            current_gear: 4,
            wheel_slip: 0.12,
            surface: SURFACE_SAND,
        };
        let s = t.to_state();
        assert!((s.normalized_rpm - ((6000.0 - 1000.0) / 6000.0) as f32).abs() < 1e-5);
        assert_eq!(s.gear, 4);
        assert_eq!(s.surface, SURFACE_SAND);
        assert_eq!(s.mix().surface_key, Some("surf_sand"));
    }

    #[test]
    fn surface_token_roundtrip() {
        assert_eq!(surface_token("Asphalt"), SURFACE_ASPHALT);
        assert_eq!(surface_token("Sand"), SURFACE_SAND);
        assert_eq!(surface_token("grass"), SURFACE_GRASS);
        assert_eq!(surface_token("rumble"), SURFACE_RUMBLE);
        assert_eq!(surface_token("unknown_thing"), SURFACE_ASPHALT);
    }
}
