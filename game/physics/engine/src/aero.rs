use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Aerodynamic load distribution across 3 points (front wing, floor/diffuser, rear wing).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroForces {
    pub total_downforce: f64,    // N (downward)
    pub front_downforce: f64,    // N (front wing, 20%)
    pub diffuser_downforce: f64, // N (floor/diffuser, 60%)
    pub rear_downforce: f64,     // N (rear wing, 20%)
    pub drag_force: f64,         // N (opposing motion)
}

impl AeroForces {
    pub fn zero() -> Self {
        Self {
            total_downforce: 0.0,
            front_downforce: 0.0,
            diffuser_downforce: 0.0,
            rear_downforce: 0.0,
            drag_force: 0.0,
        }
    }

    /// Compute aerodynamic loads from vehicle velocity in chassis local frame.
    /// local_velocity: +X = right, +Y = up, -Z = forward (Godot convention).
    pub fn calculate(config: &VehicleConfig, local_velocity: Vec3) -> Self {
        let forward_speed = -local_velocity.z;
        let speed_sq = forward_speed * forward_speed;
        let q = 0.5 * config.air_density * speed_sq; // Dynamic pressure

        let drag_force = q * config.coefficient_of_drag * config.frontal_area * forward_speed.signum();
        let total_downforce = if forward_speed > 0.0 {
            q * config.coefficient_of_downforce * config.frontal_area
        } else {
            0.0
        };

        let front_downforce = total_downforce * config.aero_split_front;
        let diffuser_downforce = total_downforce * config.aero_split_diffuser;
        let rear_downforce = total_downforce * config.aero_split_rear;

        Self {
            total_downforce,
            front_downforce,
            diffuser_downforce,
            rear_downforce,
            drag_force,
        }
    }
}
