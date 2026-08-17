use serde::{Deserialize, Serialize};
use vehicle_physics_engine::{AidsMask, VehicleInput};

/// Raw, already-mapped driver input for one simulation tick. This mirrors the role
/// of `f1_94_rust_input_controller.gd`: it turns device/action state into a
/// `VehicleInput` and carries edge-triggered commands (gear shifts, aid toggles,
/// transmission mode) that the core applies to the authoritative state.
///
/// In the Godot bridge, the `InputMap` is captured in-engine and mapped to this
/// struct; the core never reads Godot input directly. Headless tests/scripted
/// runners construct it directly.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DriverInput {
    pub throttle: f64,    // 0.0 .. 1.0
    pub brake: f64,      // 0.0 .. 1.0
    pub steer: f64,      // -1.0 (full left) .. +1.0 (full right)
    pub handbrake: f64,  // 0.0 .. 1.0
    pub clutch: f64,     // 0.0 (engaged) .. 1.0 (disengaged)
    pub gear_request: Option<i8>, // None = no change
    // Edge pulses: the consumer applies and clears these once per tick.
    pub shift_up: bool,
    pub shift_down: bool,
    pub toggle_traction_control: bool,
    pub toggle_transmission: bool,
}

impl DriverInput {
    pub fn to_vehicle_input(&self) -> VehicleInput {
        VehicleInput {
            throttle: self.throttle.clamp(0.0, 1.0),
            brake: self.brake.clamp(0.0, 1.0),
            steering: self.steer.clamp(-1.0, 1.0),
            handbrake: self.handbrake.clamp(0.0, 1.0),
            clutch: self.clutch.clamp(0.0, 1.0),
            gear_request: self.gear_request,
        }
    }
}

/// Authoritative aids state for a vehicle. This is the Rust replacement for
/// `driving_aids.gd`: a single source of truth for which assists are active and
/// their contract values. The booleans feed the physics solver's `AidsMask`; the
/// grip multipliers are the `VehicleTunableContract` values from the GDScript
/// (coefficient_of_friction *1.3 floor 1.5, lateral_grip_assist +0.15) kept here
/// until the physics crate consumes them.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AidsState {
    pub traction_control: bool,
    pub steering_slip_assist: bool,
    pub abs: bool,
    pub stability: bool,
    pub grip_aid: f64,            // coefficient_of_friction multiplier
    pub lateral_grip_assist: f64, // additive lateral grip
}

impl AidsState {
    pub fn from_config_mask(mask: &AidsMask) -> Self {
        Self {
            traction_control: mask.traction_control,
            steering_slip_assist: mask.steering_slip_assist,
            abs: mask.abs,
            stability: mask.stability,
            grip_aid: 1.0,
            lateral_grip_assist: 0.0,
        }
    }
}
