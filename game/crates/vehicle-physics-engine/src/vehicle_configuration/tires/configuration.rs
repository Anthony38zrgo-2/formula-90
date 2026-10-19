use serde::{Deserialize, Serialize};

use super::super::*;

/// Schema v3 steady tire force curve (parser-only until TIRE-100/TIRE-200).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TireForceProfile {
    pub lateral_peak_slip_angle_rad: f64,
    pub longitudinal_peak_slip_ratio: f64,
    pub lateral_slide_mu_ratio: f64,
    pub longitudinal_slide_mu_ratio: f64,
    /// Half-width of the C1 blend band around the combined-slip budget knee
    /// (GRIP-01). `0.0` reproduces the legacy hard clip exactly; a positive value
    /// spreads the transition over `utilization in [1 - w, 1 + w]` and keeps the
    /// applied force inside the shared budget at all times.
    pub budget_blend_width: f64,
    /// Sharpness of the post-peak decay `d = sharpness * (u - 1)` (GRIP-02).
    /// `2.0` matches the legacy internal constant.
    pub falloff_sharpness: f64,
    /// Time constant (s) for the sliding-memory state to follow a rising
    /// post-peak demand. `0.0` disables the loss-side lag (legacy instant decay).
    pub slip_loss_tau_s: f64,
    /// Time constant (s) for the sliding-memory state to release once the tire
    /// is back inside the envelope. Larger than `slip_loss_tau_s` models the
    /// slower, progressive grip recovery (GRIP-02). `0.0` disables recovery lag.
    pub slip_recovery_tau_s: f64,
    /// Fraction of the lateral peak slip angle consumed by full longitudinal
    /// demand (combined-slip peak migration, GRIP-04). `0.0` keeps independent
    /// axes.
    pub combined_lateral_peak_migration: f64,
    /// Fraction of the longitudinal peak slip ratio consumed by full lateral
    /// demand. `0.0` keeps independent axes.
    pub combined_longitudinal_peak_migration: f64,
    /// Lower bound (s) for the carcass relaxation time constant (GRIP-05), so
    /// high-speed transients stay readable. `0.0` keeps pure distance-based tau.
    pub min_relaxation_tau_s: f64,
    /// Multiplier applied to the relaxation tau while an axis releases slip
    /// (effective slip above target), modelling slower grip recovery. `1.0`
    /// keeps the legacy symmetric relaxation.
    pub relaxation_recovery_scale: f64,
    /// Time constant (s) for the contact blend that ramps tire forces in and out
    /// when a wheel loses or regains load (GRIP-06). `0.0` keeps the legacy hard
    /// contact switch.
    pub contact_ramp_tau_s: f64,
}
impl Default for TireForceProfile {
    fn default() -> Self {
        Self {
            lateral_peak_slip_angle_rad: 0.10,
            longitudinal_peak_slip_ratio: 0.12,
            // 1.0 = no post-peak drop (matches current saturating tanh baseline).
            // TIRE-200 introduces an explicit post-peak decay via profile values.
            lateral_slide_mu_ratio: 1.0,
            longitudinal_slide_mu_ratio: 1.0,
            // Legacy parity: profiles that never opt in keep the exact hard clip.
            budget_blend_width: 0.0,
            falloff_sharpness: 2.0,
            // Legacy parity: zero taus make the sliding memory instant.
            slip_loss_tau_s: 0.0,
            slip_recovery_tau_s: 0.0,
            // Legacy parity: no combined-slip peak migration by default.
            combined_lateral_peak_migration: 0.0,
            combined_longitudinal_peak_migration: 0.0,
            // Legacy parity: pure distance-based, symmetric relaxation.
            min_relaxation_tau_s: 0.0,
            relaxation_recovery_scale: 1.0,
            // Legacy parity: hard contact switch.
            contact_ramp_tau_s: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiresConfiguration {
    pub front_tire_radius: f64,
    pub rear_tire_radius: f64,
    pub front_tire_width: f64, // meters
    pub rear_tire_width: f64,  // meters
    pub front_wheel_mass: f64,
    pub rear_wheel_mass: f64,
    // Global fallback (used when a per-axle value is omitted in JSON).
    pub contact_patch: f64,
    // Per-axle contact patch (front/rear).
    pub front_contact_patch: f64,
    pub rear_contact_patch: f64,
    pub front_airborne_decay: f64,
    pub rear_airborne_decay: f64,

    // Tire pressure + thermal (pressure-aware carcass + 5-node thermal model)
    pub tire_pressure: TirePressureConfig,
    pub tire_thermal: TireThermalAxleConfig,

    #[serde(default)]
    pub tire_wear: TireWearAxleConfig,

    // Schema v3: per-axle steady tire force envelope (consumed by tire.rs).
    #[serde(default)]
    pub front_tire_force: TireForceProfile,
    #[serde(default)]
    pub rear_tire_force: TireForceProfile,
    #[serde(default)]
    pub pressure_mechanics: PressureMechanicsSensitivity,

    // Surface properties
    pub surface_friction: HashMap<SurfaceType, f64>,
    pub surface_stiffness: HashMap<SurfaceType, f64>,
    pub surface_rolling_resistance: HashMap<SurfaceType, f64>,
    pub surface_longitudinal_grip_ratio: HashMap<SurfaceType, f64>,
    /// GRIP-06: first-order filter tau (s) for the per-wheel effective surface
    /// friction/stiffness/rolling values, so kerb and grass crossings are
    /// gradual. `0.0` keeps instantaneous surface steps (legacy).
    #[serde(default)]
    pub surface_transition_tau_s: f64,
    /// Radial carcass damping ratio used to derive the tire vertical damping.
    /// Absent keeps the legacy 0.28 calibration.
    #[serde(default = "default_tire_radial_damping_ratio")]
    pub radial_damping_ratio: f64,
}

fn default_tire_radial_damping_ratio() -> f64 {
    0.28
}

impl TiresConfiguration {
    /// Nominal suspension-ray origin in vehicle-local geometric space (-Z forward, +Y up, +X right),
    /// where origin (0, 0, 0) is centered between axles at mean wheel-center height.
    pub fn wheel_anchor_local(
        &self,
        chassis: &ChassisConfiguration,
        suspension: &SuspensionConfiguration,
        wheel: WheelIndex,
    ) -> Vec3 {
        if suspension.use_authored_wheel_centers {
            if let Some(geometry) = &suspension.geometric_suspension {
                let spring_extension = if wheel.is_front() {
                    suspension.front_spring_length * (1.0 - suspension.front_resting_ratio)
                } else {
                    suspension.rear_spring_length * (1.0 - suspension.rear_resting_ratio)
                };
                return geometry.corners.get(wheel).hub_center
                    + Vec3::new(0.0, spring_extension, 0.0);
            }
        }
        let z = if wheel.is_front() {
            -chassis.wheelbase * 0.5
        } else {
            chassis.wheelbase * 0.5
        };
        let x = if wheel.is_front() {
            if wheel.is_right() {
                chassis.front_track * 0.5
            } else {
                -chassis.front_track * 0.5
            }
        } else if wheel.is_right() {
            chassis.rear_track * 0.5
        } else {
            -chassis.rear_track * 0.5
        };

        let mean_radius = (self.front_tire_radius + self.rear_tire_radius) * 0.5;
        let hub_y = if wheel.is_front() {
            self.front_tire_radius - mean_radius
        } else {
            self.rear_tire_radius - mean_radius
        };
        let spring_length = if wheel.is_front() {
            suspension.front_spring_length
        } else {
            suspension.rear_spring_length
        };
        let resting_ratio = if wheel.is_front() {
            suspension.front_resting_ratio
        } else {
            suspension.rear_resting_ratio
        };
        let anchor_y = hub_y + spring_length * (1.0 - resting_ratio);

        Vec3::new(x, anchor_y, z)
    }
}

#[cfg(test)]
mod authored_wheel_center_tests {
    use super::*;

    #[test]
    fn williams_solver_anchors_preserve_authored_wheel_centers() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let configuration =
            crate::vehicle_configuration::load_vehicle_configuration_from_authorized_root(
                &repository_root
                    .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
                &repository_root,
            )
            .expect("Williams profile");
        let geometry = configuration
            .suspension
            .geometric_suspension
            .as_ref()
            .expect("physical geometry");
        for wheel in [
            WheelIndex::FrontLeft,
            WheelIndex::FrontRight,
            WheelIndex::RearLeft,
            WheelIndex::RearRight,
        ] {
            let anchor = configuration.tires.wheel_anchor_local(
                &configuration.chassis,
                &configuration.suspension,
                wheel,
            );
            let extension = if wheel.is_front() {
                configuration.suspension.front_spring_length
                    * (1.0 - configuration.suspension.front_resting_ratio)
            } else {
                configuration.suspension.rear_spring_length
                    * (1.0 - configuration.suspension.rear_resting_ratio)
            };
            assert!(
                (anchor - Vec3::new(0.0, extension, 0.0) - geometry.corners.get(wheel).hub_center)
                    .length()
                    < 1e-9
            );
            let expected_height = if wheel.is_front() {
                -0.226701
            } else {
                -0.216634
            };
            let expected_longitudinal_position = if wheel.is_front() {
                -1.146992
            } else {
                1.743008
            };
            assert!((anchor.y - extension - expected_height).abs() < 1e-9);
            assert!((anchor.z - expected_longitudinal_position).abs() < 1e-9);
        }
        let encoded = configuration.to_resolved_configuration_value();
        assert_eq!(encoded["suspension"]["use_authored_wheel_centers"], true);
    }
}
