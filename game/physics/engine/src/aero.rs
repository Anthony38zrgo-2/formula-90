use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Aerodynamic load distribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroForces {
    pub total_downforce: f64, // N (downward)
    pub front_downforce: f64, // N
    pub rear_downforce: f64,  // N
    pub drag_force: f64,      // N (opposing motion)
}

impl AeroForces {
    pub fn zero() -> Self {
        Self {
            total_downforce: 0.0,
            front_downforce: 0.0,
            rear_downforce: 0.0,
            drag_force: 0.0,
        }
    }

    /// Compute aerodynamic loads from vehicle velocity in chassis local frame.
    /// local_velocity: +X = right, +Y = up, -Z = forward (Godot convention).
    pub fn calculate(config: &VehicleConfig, local_velocity: Vec3) -> Self {
        let forward_speed = (-local_velocity.z).max(0.0);
        let q = 0.5 * config.air_density * forward_speed * forward_speed; // Dynamic pressure

        let drag_force = q * config.coefficient_of_drag * config.frontal_area;
        let total_downforce = q * config.coefficient_of_downforce * config.frontal_area;

        let front_downforce = total_downforce * config.aero_balance_front;
        let rear_downforce = total_downforce * (1.0 - config.aero_balance_front);

        Self {
            total_downforce,
            front_downforce,
            rear_downforce,
            drag_force,
        }
    }
}
