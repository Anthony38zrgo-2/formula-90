//! Element aerodynamics in chassis axes (+X right, +Y up, -Z forward).
//! Configuration, equilibrium coefficients and transient flow states are separate.
//! Forces use instantaneous air-relative velocity; only coefficients/flow lag.
use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingPolarPoint {
    pub angle_deg: f64,
    pub cl: f64,
    pub cd: f64,
}
impl Default for WingPolarPoint {
    fn default() -> Self {
        Self {
            angle_deg: 0.0,
            cl: 0.0,
            cd: 0.05,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingAeroConfig {
    pub area_m2: f64,
    pub incidence_deg: f64,
    pub min_angle_deg: f64,
    pub max_angle_deg: f64,
    pub yaw_decay_exponent: f64,
    pub polar: Vec<WingPolarPoint>,
    /// Chassis-local application point; None uses the corresponding axle.
    pub position_m: Option<Vec3>,
    /// None inherits VehicleConfig::aero_lag_tau.
    pub response_tau_s: Option<f64>,
    /// Smoothly lose lift beyond the tabulated polar instead of freezing CL.
    pub post_stall_fade_deg: f64,
    pub stalled_drag_coefficient: f64,
}
impl WingAeroConfig {
    fn front_default() -> Self {
        Self {
            area_m2: 0.72,
            incidence_deg: 7.0,
            min_angle_deg: 4.0,
            max_angle_deg: 20.0,
            yaw_decay_exponent: 1.15,
            position_m: None,
            response_tau_s: None,
            post_stall_fade_deg: 35.0,
            stalled_drag_coefficient: 1.2,
            polar: vec![
                WingPolarPoint {
                    angle_deg: 4.0,
                    cl: 0.45,
                    cd: 0.075,
                },
                WingPolarPoint {
                    angle_deg: 8.0,
                    cl: 0.82,
                    cd: 0.105,
                },
                WingPolarPoint {
                    angle_deg: 12.0,
                    cl: 1.12,
                    cd: 0.155,
                },
                WingPolarPoint {
                    angle_deg: 16.0,
                    cl: 1.30,
                    cd: 0.230,
                },
                WingPolarPoint {
                    angle_deg: 20.0,
                    cl: 1.20,
                    cd: 0.340,
                },
            ],
        }
    }
    fn rear_default() -> Self {
        Self {
            area_m2: 0.84,
            incidence_deg: 11.0,
            min_angle_deg: 6.0,
            max_angle_deg: 26.0,
            yaw_decay_exponent: 1.05,
            position_m: None,
            response_tau_s: None,
            post_stall_fade_deg: 35.0,
            stalled_drag_coefficient: 1.2,
            polar: vec![
                WingPolarPoint {
                    angle_deg: 6.0,
                    cl: 0.55,
                    cd: 0.090,
                },
                WingPolarPoint {
                    angle_deg: 10.0,
                    cl: 0.90,
                    cd: 0.130,
                },
                WingPolarPoint {
                    angle_deg: 16.0,
                    cl: 1.35,
                    cd: 0.220,
                },
                WingPolarPoint {
                    angle_deg: 21.0,
                    cl: 1.52,
                    cd: 0.320,
                },
                WingPolarPoint {
                    angle_deg: 26.0,
                    cl: 1.38,
                    cd: 0.470,
                },
            ],
        }
    }
}
impl Default for WingAeroConfig {
    fn default() -> Self {
        Self::front_default()
    }
}

fn default_seal_asymmetry_m() -> f64 {
    0.080
}
fn default_seal_roll_rad() -> f64 {
    0.12
}
fn default_bottoming_factor() -> f64 {
    0.20
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnderfloorAeroConfig {
    pub lift_area_m2: f64,
    pub optimal_height_m: f64,
    pub choke_height_m: f64,
    pub high_height_m: f64,
    pub optimal_rake_deg: f64,
    pub rake_window_deg: f64,
    pub yaw_decay_exponent: f64,
    pub stall_attack_tau_s: f64,
    pub stall_recovery_tau_s: f64,
    pub induced_drag_ratio: f64,
    #[serde(default = "default_seal_asymmetry_m")]
    pub seal_asymmetry_m: f64,
    #[serde(default = "default_seal_roll_rad")]
    pub seal_roll_rad: f64,
    #[serde(default = "default_bottoming_factor")]
    pub bottoming_factor: f64,
    /// CL*A away from the ground, independent of the ground-effect increment.
    pub free_air_lift_area_m2: f64,
    /// Optional measured front-height x rear-height map (row-major by front).
    pub map: Option<GroundEffectMap>,
    pub position_m: Option<Vec3>,
    /// Separation between the two floor load patches, centered on position_m.
    pub pressure_span_m: f64,
    pub max_pressure_shift_m: f64,
    pub pressure_response_tau_s: f64,
    pub probe_loss_tau_s: f64,
    pub probe_recovery_tau_s: f64,
    /// Reattachment occurs above choke_height_m * this multiplier.
    pub choke_recovery_ratio: f64,
    pub separated_flow_factor: f64,
}
impl Default for UnderfloorAeroConfig {
    fn default() -> Self {
        Self {
            lift_area_m2: 0.90,
            optimal_height_m: 0.045,
            choke_height_m: 0.012,
            high_height_m: 0.160,
            optimal_rake_deg: 0.5,
            rake_window_deg: 2.5,
            yaw_decay_exponent: 1.8,
            stall_attack_tau_s: 0.030,
            stall_recovery_tau_s: 0.160,
            induced_drag_ratio: 0.16,
            seal_asymmetry_m: default_seal_asymmetry_m(),
            seal_roll_rad: default_seal_roll_rad(),
            bottoming_factor: default_bottoming_factor(),
            free_air_lift_area_m2: 0.0,
            map: None,
            position_m: None,
            pressure_span_m: 1.30,
            max_pressure_shift_m: 0.25,
            pressure_response_tau_s: 0.050,
            probe_loss_tau_s: 0.030,
            probe_recovery_tau_s: 0.100,
            choke_recovery_ratio: 1.5,
            separated_flow_factor: 0.20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EraAeroLimits {
    pub profile: String,
    pub soft_max_load_ratio: f64,
    pub hard_max_load_ratio: f64,
    pub hard_max_downforce_n: f64,
}
impl Default for EraAeroLimits {
    fn default() -> Self {
        Self {
            profile: "f1_1994_post_safety".into(),
            soft_max_load_ratio: 1.65,
            hard_max_load_ratio: 1.85,
            hard_max_downforce_n: 10_800.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AeroModelConfig {
    pub front_wing: WingAeroConfig,
    pub rear_wing: WingAeroConfig,
    pub underfloor: UnderfloorAeroConfig,
    pub limits: EraAeroLimits,
    pub body: BodyAeroConfig,
}
impl Default for AeroModelConfig {
    fn default() -> Self {
        Self {
            front_wing: WingAeroConfig::front_default(),
            rear_wing: WingAeroConfig::rear_default(),
            underfloor: UnderfloorAeroConfig::default(),
            limits: EraAeroLimits::default(),
            body: BodyAeroConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DragCoefficientScope {
    /// VehicleConfig::coefficient_of_drag describes the body without wings/floor.
    #[default]
    BodyOnly,
    /// Subtract the nominal element drag areas from the supplied whole-car Cd*A.
    WholeVehicleReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BodyAeroConfig {
    pub drag_coefficient_scope: DragCoefficientScope,
    /// Cd*A, not geometric area. All three body axes dissipate relative motion.
    pub side_drag_area_m2: f64,
    pub vertical_drag_area_m2: f64,
    /// None applies body drag at the center of mass.
    pub position_m: Option<Vec3>,
}
impl Default for BodyAeroConfig {
    fn default() -> Self {
        Self {
            drag_coefficient_scope: DragCoefficientScope::BodyOnly,
            side_drag_area_m2: 1.8,
            vertical_drag_area_m2: 1.0,
            position_m: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundEffectCell {
    /// Ground-effect increment relative to UnderfloorAeroConfig::lift_area_m2.
    pub load_factor: f64,
    /// Longitudinal offset from the configured floor application point.
    pub pressure_shift_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundEffectMap {
    pub front_heights_m: Vec<f64>,
    pub rear_heights_m: Vec<f64>,
    /// cells[front_index * rear_heights_m.len() + rear_index]
    pub cells: Vec<GroundEffectCell>,
}

/// Input availability is explicit: a missed ray is never an optimal ride height.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AeroProbeMode {
    #[default]
    Measured,
    Disabled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AeroGroundStatus {
    Complete,
    Partial,
    #[default]
    OutOfRange,
    Disabled,
}

/// All vectors are in chassis axes; linear velocity is the velocity at the CG.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AeroKinematics {
    pub velocity_m_s: Vec3,
    pub angular_velocity_rad_s: Vec3,
    pub wind_velocity_m_s: Vec3,
    pub center_of_mass_m: Vec3,
    pub probe_mode: AeroProbeMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WingFlowState {
    pub cl: f64,
    pub cd: f64,
}

/// Serialized with the force facade so a restored simulation retains flow memory.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AeroState {
    pub front_wing: WingFlowState,
    pub rear_wing: WingFlowState,
    pub floor_attachment: [f64; 2],
    pub floor_separated: [bool; 2],
    pub ground_confidence: f64,
    pub pressure_shift_m: f64,
    pub last_ground: GroundGeometry,
    pub has_ground_sample: bool,
}
impl Default for AeroState {
    fn default() -> Self {
        Self {
            front_wing: WingFlowState::default(),
            rear_wing: WingFlowState::default(),
            floor_attachment: [1.0; 2],
            floor_separated: [false; 2],
            ground_confidence: 0.0,
            pressure_shift_m: 0.0,
            last_ground: GroundGeometry::default(),
            has_ground_sample: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GroundGeometry {
    pub front_height_m: f64,
    pub rear_height_m: f64,
    pub rake_rad: f64,
    pub roll_rad: f64,
    pub asymmetry_m: f64,
    pub confidence: f64,
    pub valid_probe_mask: u32,
    pub status: AeroGroundStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroEnvironment {
    pub clearance_m: [f64; 5],
    pub valid_mask: u32,
    pub rake_rad: f64,
    pub roll_rad: f64,
    pub bottoming_mask: u32,
    /// Confidence in a mechanical ground contact, NOT aerodynamic sample quality.
    pub contact_confidence: f64,
}
impl Default for AeroEnvironment {
    fn default() -> Self {
        Self {
            clearance_m: [0.09, 0.09, 0.095, 0.130, 0.130],
            valid_mask: 0x1f,
            rake_rad: 0.0,
            roll_rad: 0.0,
            bottoming_mask: 0,
            contact_confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AeroForces {
    pub total_downforce: f64,
    pub front_downforce: f64,
    pub diffuser_downforce: f64,
    pub rear_downforce: f64,
    pub drag_force: f64,
    pub effective_cl: f64,
    pub yaw_decay_factor: f64,
    pub blend_factor: f64,
    pub front_wing_angle_deg: f64,
    pub rear_wing_angle_deg: f64,
    pub front_wing_cl: f64,
    pub rear_wing_cl: f64,
    pub floor_height_factor: f64,
    pub floor_rake_factor: f64,
    pub floor_seal_factor: f64,
    pub diffuser_stall_factor: f64,
    pub raw_downforce: f64,
    pub global_limit_factor: f64,
    pub load_ratio: f64,
    /// Equivalent front axle share, including the pitch moment of aerodynamic drag.
    /// May leave [0, 1] when one axle is aerodynamically unloaded.
    pub balance_front: f64,
    pub front_axle_downforce: f64,
    pub rear_axle_downforce: f64,
    /// Apply this force and this moment about the CG exactly once in the host.
    pub force_local: Vec3,
    pub torque_local: Vec3,
    pub drag_force_local: Vec3,
    pub center_of_pressure_local: Vec3,
    pub ground_status: AeroGroundStatus,
    pub ground_confidence: f64,
    pub invalid_input: bool,
    pub state: AeroState,
}
impl Default for AeroForces {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy)]
struct ElementLoad {
    position: Vec3,
    lift: Vec3,
    drag: Vec3,
}
impl ElementLoad {
    fn downforce(self) -> f64 {
        (-self.lift.y).max(0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct AirFlow {
    direction: Vec3,
    q: f64,
    angle_deg: f64,
    cos_yaw: f64,
    forward_projection: f64,
    blend: f64,
}
impl AirFlow {
    fn at(config: &VehicleConfig, k: &AeroKinematics, point: Vec3) -> Self {
        let velocity = k.velocity_m_s - k.wind_velocity_m_s
            + k.angular_velocity_rad_s.cross(point - k.center_of_mass_m);
        let speed = velocity.length();
        let forward = (-velocity.z).max(0.0);
        let horizontal = velocity.x.hypot(velocity.z);
        let active = speed > 1e-8;
        Self {
            direction: if active { velocity / speed } else { Vec3::ZERO },
            q: 0.5 * config.air_density * speed * speed,
            angle_deg: if active {
                (-velocity.y).atan2(-velocity.z).to_degrees()
            } else {
                0.0
            },
            cos_yaw: if horizontal > 1e-8 {
                forward / horizontal
            } else {
                0.0
            },
            // Projected forward exposure is separate from the empirical yaw loss.
            forward_projection: if active {
                (forward / speed).powi(2)
            } else {
                0.0
            },
            blend: smoothstep(
                (forward - config.aero_blend_min_speed)
                    / (config.aero_blend_full_speed - config.aero_blend_min_speed),
            ),
        }
    }
    fn lift_direction(self) -> Vec3 {
        // Lift is perpendicular to the local air-relative velocity, including heave.
        (Vec3::DOWN - self.direction * Vec3::DOWN.dot(self.direction)).normalized()
    }
    fn exposure(self, yaw_exponent: f64) -> f64 {
        self.forward_projection * self.cos_yaw.powf(yaw_exponent) * self.blend
    }
}

impl AeroEnvironment {
    /// No ray measurement; this never implies a favorable or optimal stance.
    pub fn unavailable() -> Self {
        Self {
            valid_mask: 0,
            clearance_m: [0.35; 5],
            ..Self::default()
        }
    }

    /// Explicit nominal geometry for offline tools that deliberately omit raycasts.
    /// The live host must use unavailable() for rays that miss the ground.
    pub fn nominal(config: &UnderfloorAeroConfig) -> Self {
        let h = config.optimal_height_m;
        let slope = config.optimal_rake_deg.to_radians().tan();
        Self {
            clearance_m: [
                (h + 0.90 * slope).max(0.0),
                (h + 0.90 * slope).max(0.0),
                h,
                (h - 0.85 * slope + 0.040).max(0.0),
                (h - 1.65 * slope + 0.040).max(0.0),
            ],
            valid_mask: 0x1f,
            rake_rad: config.optimal_rake_deg.to_radians(),
            roll_rad: 0.0,
            bottoming_mask: 0,
            contact_confidence: 0.0,
        }
    }

    fn geometry(&self, mode: AeroProbeMode) -> GroundGeometry {
        if mode == AeroProbeMode::Disabled {
            return GroundGeometry {
                status: AeroGroundStatus::Disabled,
                ..GroundGeometry::default()
            };
        }
        let valid = |i: usize| {
            self.valid_mask & (1 << i) != 0
                && self.clearance_m[i].is_finite()
                && self.clearance_m[i] >= 0.0
        };
        let mut mask = 0u32;
        for i in 0..5 {
            if valid(i) {
                mask |= 1 << i;
            }
        }
        if mask == 0 {
            return GroundGeometry::default();
        }

        // Probe origins match the native five-ray layout. Remove the diffuser's
        // fixed 40 mm mounting offset before comparing floor heights.
        let center = if valid(2) {
            Some(self.clearance_m[2])
        } else {
            None
        };
        let front_count = u32::from(valid(0)) + u32::from(valid(1));
        let front_measured = if front_count > 0 {
            Some(
                ((if valid(0) { self.clearance_m[0] } else { 0.0 })
                    + (if valid(1) { self.clearance_m[1] } else { 0.0 }))
                    / front_count as f64,
            )
        } else {
            None
        };
        let (rear_measured, rear_z) = if valid(3) {
            (Some((self.clearance_m[3] - 0.040).max(0.0)), 0.85)
        } else if valid(4) {
            (Some((self.clearance_m[4] - 0.040).max(0.0)), 1.65)
        } else {
            (None, 0.85)
        };
        // Reconstruct a common floor plane. The map always sees heights at
        // z=-0.90 and z=+0.85, even when the exit substitutes for the throat.
        let (center_height, slope) = match (front_measured, center, rear_measured) {
            (Some(a), _, Some(b)) => {
                let slope = (b - a) / (0.90 + rear_z);
                (a + 0.90 * slope, slope)
            }
            (Some(a), Some(b), None) => (b, (b - a) / 0.90),
            (None, Some(a), Some(b)) => (a, (b - a) / rear_z),
            _ => (
                center.or(front_measured).or(rear_measured).unwrap_or(0.35),
                0.0,
            ),
        };
        let both_front = valid(0) && valid(1);
        let delta = if both_front {
            self.clearance_m[0] - self.clearance_m[1]
        } else {
            0.0
        };
        let confidence = 0.20 * front_count as f64
            + if valid(2) { 0.30 } else { 0.0 }
            + if rear_measured.is_some() { 0.30 } else { 0.0 };
        GroundGeometry {
            front_height_m: (center_height - 0.90 * slope).max(0.0),
            rear_height_m: (center_height + 0.85 * slope).max(0.0),
            rake_rad: (-slope).atan(),
            roll_rad: (delta / 0.90).atan(),
            asymmetry_m: delta.abs(),
            confidence,
            valid_probe_mask: mask,
            status: if valid(0) && valid(1) && valid(2) && rear_measured.is_some() {
                AeroGroundStatus::Complete
            } else {
                AeroGroundStatus::Partial
            },
        }
    }
}

impl AeroForces {
    pub fn zero() -> Self {
        Self {
            total_downforce: 0.0,
            front_downforce: 0.0,
            diffuser_downforce: 0.0,
            rear_downforce: 0.0,
            drag_force: 0.0,
            effective_cl: 0.0,
            yaw_decay_factor: 1.0,
            blend_factor: 0.0,
            front_wing_angle_deg: 0.0,
            rear_wing_angle_deg: 0.0,
            front_wing_cl: 0.0,
            rear_wing_cl: 0.0,
            floor_height_factor: 0.0,
            floor_rake_factor: 0.0,
            floor_seal_factor: 0.0,
            diffuser_stall_factor: 1.0,
            raw_downforce: 0.0,
            global_limit_factor: 1.0,
            load_ratio: 0.0,
            balance_front: 0.5,
            front_axle_downforce: 0.0,
            rear_axle_downforce: 0.0,
            force_local: Vec3::ZERO,
            torque_local: Vec3::ZERO,
            drag_force_local: Vec3::ZERO,
            center_of_pressure_local: Vec3::ZERO,
            ground_status: AeroGroundStatus::OutOfRange,
            ground_confidence: 0.0,
            invalid_input: false,
            state: AeroState::default(),
        }
    }

    /// Compatibility entry point. World pitch is intentionally ignored: the angle
    /// of attack comes from local air velocity. New hosts should pass full kinematics.
    pub fn step_with_environment(
        &mut self,
        config: &VehicleConfig,
        local_velocity: Vec3,
        env: &AeroEnvironment,
        _pitch_rad: f64,
        dt: f64,
    ) {
        let cg = Vec3::new(
            0.0,
            config.center_of_gravity_height_offset,
            (0.5 - config.front_weight_distribution) * config.wheelbase,
        );
        self.step_with_kinematics(
            config,
            &AeroKinematics {
                velocity_m_s: local_velocity,
                center_of_mass_m: cg,
                ..AeroKinematics::default()
            },
            env,
            dt,
        );
    }

    pub fn step_with_kinematics(
        &mut self,
        config: &VehicleConfig,
        k: &AeroKinematics,
        env: &AeroEnvironment,
        dt: f64,
    ) {
        // Configuration is also validated on JSON load. Recheck here because the
        // public Rust config is mutable (live setup changes); valid checks allocate
        // nothing and maps have bounded dimensions.
        if !dt.is_finite()
            || dt <= 0.0
            || !finite_vec(k.velocity_m_s)
            || !finite_vec(k.angular_velocity_rad_s)
            || !finite_vec(k.wind_velocity_m_s)
            || !finite_vec(k.center_of_mass_m)
            || validate_aero_config(config).is_err()
        {
            *self = Self::zero();
            self.invalid_input = true;
            return;
        }
        let m = &config.aero_model;
        let floor = &m.underfloor;
        let front_point =
            m.front_wing
                .position_m
                .unwrap_or(Vec3::new(0.0, 0.0, -0.5 * config.wheelbase));
        let rear_point =
            m.rear_wing
                .position_m
                .unwrap_or(Vec3::new(0.0, 0.0, 0.5 * config.wheelbase));
        let floor_point =
            floor
                .position_m
                .unwrap_or(Vec3::new(0.0, config.center_of_gravity_height_offset, 0.0));
        let body_point = m.body.position_m.unwrap_or(k.center_of_mass_m);
        let fw_air = AirFlow::at(config, k, front_point);
        let rw_air = AirFlow::at(config, k, rear_point);
        let body_air = AirFlow::at(config, k, body_point);
        let fw_angle = m.front_wing.incidence_deg + fw_air.angle_deg;
        let rw_angle = m.rear_wing.incidence_deg + rw_air.angle_deg;
        let fw = wing_load(
            &m.front_wing,
            config.aero_lag_tau,
            &mut self.state.front_wing,
            fw_air,
            front_point,
            fw_angle,
            dt,
        );
        let rw = wing_load(
            &m.rear_wing,
            config.aero_lag_tau,
            &mut self.state.rear_wing,
            rw_air,
            rear_point,
            rw_angle,
            dt,
        );

        let mut measured = env.geometry(k.probe_mode);
        if self.state.has_ground_sample && measured.valid_probe_mask & 3 != 3 {
            // Missing one side must not instantly restore a previously lost seal.
            measured.roll_rad = self.state.last_ground.roll_rad;
            measured.asymmetry_m = self.state.last_ground.asymmetry_m;
        }
        self.ground_status = measured.status;
        let target_confidence = measured.confidence;
        let confidence_tau = if target_confidence < self.state.ground_confidence {
            floor.probe_loss_tau_s
        } else {
            floor.probe_recovery_tau_s
        };
        relax(
            &mut self.state.ground_confidence,
            target_confidence,
            dt,
            confidence_tau,
        );
        // A missing sample decays the last measured ground contribution. It never
        // fabricates nominal geometry or reads the payload of an invalid probe.
        if measured.confidence > 0.0 {
            self.state.last_ground = measured;
            self.state.has_ground_sample = true;
        }
        let ground = self.state.last_ground;
        let height_factor;
        let rake_factor;
        let pressure_target;
        if let Some(map) = &floor.map {
            let cell = map.sample(ground.front_height_m, ground.rear_height_m);
            height_factor = cell.load_factor;
            rake_factor = 1.0; // The two-height map already contains rake coupling.
            pressure_target = cell.pressure_shift_m;
        } else {
            let front_eff = height_efficiency(ground.front_height_m, floor);
            let rear_eff = height_efficiency(ground.rear_height_m, floor);
            height_factor = (front_eff * rear_eff).sqrt();
            rake_factor = 1.0
                - smoothstep(
                    (ground.rake_rad.to_degrees() - floor.optimal_rake_deg).abs()
                        / floor.rake_window_deg,
                );
            pressure_target = floor.max_pressure_shift_m * (rear_eff - front_eff)
                / (front_eff + rear_eff).max(1e-8);
        }
        // Roll and left/right asymmetry describe the same loss when obtained from
        // the same probes. Use the stronger estimate instead of counting it twice.
        let seal_loss = (ground.asymmetry_m / floor.seal_asymmetry_m)
            .max(ground.roll_rad.abs() / floor.seal_roll_rad);
        let seal_factor = 1.0 - smoothstep(seal_loss);
        relax(
            &mut self.state.pressure_shift_m,
            pressure_target.clamp(-floor.max_pressure_shift_m, floor.max_pressure_shift_m),
            dt,
            floor.pressure_response_tau_s,
        );

        let bottom = env.bottoming_mask & 0x1f;
        let bottom_severity = [
            0.25 * f64::from((bottom & 1 != 0) as u8)
                + 0.25 * f64::from((bottom & 2 != 0) as u8)
                + 0.50 * f64::from((bottom & 4 != 0) as u8),
            0.75 * f64::from((bottom & 8 != 0) as u8) + 0.25 * f64::from((bottom & 16 != 0) as u8),
        ];
        let heights = [ground.front_height_m, ground.rear_height_m];
        let mut floor_loads = [ElementLoad {
            position: floor_point,
            lift: Vec3::ZERO,
            drag: Vec3::ZERO,
        }; 2];
        let confidence = if self.state.has_ground_sample {
            self.state.ground_confidence.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ground_factor = height_factor * rake_factor * seal_factor * confidence;
        for i in 0..2 {
            // Separate entry/recovery thresholds give explicit, bounded hysteresis.
            if heights[i] <= floor.choke_height_m || bottom_severity[i] > 0.0 {
                self.state.floor_separated[i] = true;
            } else if heights[i] >= floor.choke_height_m * floor.choke_recovery_ratio {
                self.state.floor_separated[i] = false;
            }
            let target = if self.state.floor_separated[i] {
                floor.separated_flow_factor
            } else {
                1.0
            };
            let tau = if target < self.state.floor_attachment[i] {
                floor.stall_attack_tau_s
            } else {
                floor.stall_recovery_tau_s
            };
            relax(&mut self.state.floor_attachment[i], target, dt, tau);
            let local_contact = 1.0 - bottom_severity[i] * (1.0 - floor.bottoming_factor);
            let area = 0.5
                * (floor.free_air_lift_area_m2
                    + floor.lift_area_m2
                        * ground_factor
                        * self.state.floor_attachment[i]
                        * local_contact);
            let z_offset = if i == 0 { -0.5 } else { 0.5 };
            let point = floor_point
                + Vec3::new(
                    0.0,
                    0.0,
                    self.state.pressure_shift_m + z_offset * floor.pressure_span_m,
                );
            let air = AirFlow::at(config, k, point);
            let lift =
                air.lift_direction() * (air.q * area * air.exposure(floor.yaw_decay_exponent));
            floor_loads[i] = ElementLoad {
                position: point,
                lift,
                drag: -air.direction * (lift.length() * floor.induced_drag_ratio),
            };
        }

        let raw = fw.downforce()
            + rw.downforce()
            + floor_loads[0].downforce()
            + floor_loads[1].downforce();
        let weight = config.vehicle_mass * 9.81;
        let hard = (weight * m.limits.hard_max_load_ratio).min(m.limits.hard_max_downforce_n);
        let soft = (weight * m.limits.soft_max_load_ratio).min(hard);
        let limited = soft_limit(raw, soft, hard);
        let limit_factor = if raw > 1e-8 { limited / raw } else { 1.0 };

        let mut body_drag_area = config.coefficient_of_drag * config.frontal_area;
        if m.body.drag_coefficient_scope == DragCoefficientScope::WholeVehicleReference {
            let fw_cd = polar_at(&m.front_wing.polar, m.front_wing.incidence_deg).1;
            let rw_cd = polar_at(&m.rear_wing.polar, m.rear_wing.incidence_deg).1;
            let reference_elements = m.front_wing.area_m2 * fw_cd
                + m.rear_wing.area_m2 * rw_cd
                + (floor.free_air_lift_area_m2 + floor.lift_area_m2) * floor.induced_drag_ratio;
            body_drag_area = (body_drag_area - reference_elements).max(0.0);
        }
        let body_drag = Vec3::new(
            -body_air.q * m.body.side_drag_area_m2 * body_air.direction.x,
            -body_air.q * m.body.vertical_drag_area_m2 * body_air.direction.y,
            -body_air.q * body_drag_area * body_air.direction.z,
        );
        let mut force = body_drag;
        let mut torque = (body_point - k.center_of_mass_m).cross(body_drag);
        let mut drag_vector = body_drag;
        let mut drag_magnitude = body_drag.length();
        let mut pressure_sum = Vec3::ZERO;
        for (index, element) in [fw, floor_loads[0], floor_loads[1], rw]
            .into_iter()
            .enumerate()
        {
            // Floor drag is induced by its actual limited load. Wing polar CD
            // remains a measured coefficient; the gameplay cap is not a new polar.
            let drag = element.drag
                * if index == 1 || index == 2 {
                    limit_factor
                } else {
                    1.0
                };
            let applied = element.lift * limit_factor + drag;
            force += applied;
            torque += (element.position - k.center_of_mass_m).cross(applied);
            drag_vector += drag;
            drag_magnitude += drag.length();
            pressure_sum += element.position * (element.downforce() * limit_factor);
        }
        self.front_downforce = fw.downforce() * limit_factor;
        self.diffuser_downforce =
            (floor_loads[0].downforce() + floor_loads[1].downforce()) * limit_factor;
        self.rear_downforce = rw.downforce() * limit_factor;
        // Keep the published hard bound exact even if summing rounded element
        // products differs from the limited total by a floating-point ulp.
        self.total_downforce = limited;
        self.force_local = force;
        self.torque_local = torque;
        self.drag_force_local = drag_vector;
        self.drag_force = drag_magnitude;
        self.center_of_pressure_local = if self.total_downforce > 1e-8 {
            pressure_sum / self.total_downforce
        } else {
            floor_point
        };
        let axle_load = -force.y;
        let moment_at_origin = torque + k.center_of_mass_m.cross(force);
        self.front_axle_downforce =
            (0.5 * config.wheelbase * axle_load - moment_at_origin.x) / config.wheelbase;
        self.rear_axle_downforce = axle_load - self.front_axle_downforce;
        self.balance_front = if axle_load > 1e-8 {
            self.front_axle_downforce / axle_load
        } else {
            0.5
        };
        self.effective_cl = if body_air.q * config.frontal_area > 1e-8 {
            self.total_downforce / (body_air.q * config.frontal_area)
        } else {
            0.0
        };
        self.yaw_decay_factor = body_air.cos_yaw;
        self.blend_factor = body_air.blend;
        self.front_wing_angle_deg = fw_angle;
        self.rear_wing_angle_deg = rw_angle;
        self.front_wing_cl = self.state.front_wing.cl;
        self.rear_wing_cl = self.state.rear_wing.cl;
        self.floor_height_factor = height_factor;
        self.floor_rake_factor = rake_factor;
        self.floor_seal_factor = seal_factor;
        self.diffuser_stall_factor =
            0.5 * (self.state.floor_attachment[0] + self.state.floor_attachment[1]);
        self.raw_downforce = raw;
        self.global_limit_factor = limit_factor;
        self.load_ratio = self.total_downforce / weight;
        self.ground_confidence = confidence;
        self.invalid_input = false;
        if !finite_vec(force)
            || !finite_vec(torque)
            || !self.total_downforce.is_finite()
            || !self.drag_force.is_finite()
            || !self.balance_front.is_finite()
        {
            *self = Self::zero();
            self.invalid_input = true;
        }
    }
}

fn wing_load(
    wing: &WingAeroConfig,
    inherited_tau: f64,
    state: &mut WingFlowState,
    air: AirFlow,
    point: Vec3,
    angle_deg: f64,
    dt: f64,
) -> ElementLoad {
    let (cl, cd) = wing_polar_at(wing, angle_deg);
    let tau = wing.response_tau_s.unwrap_or(inherited_tau);
    relax(&mut state.cl, cl, dt, tau);
    relax(&mut state.cd, cd, dt, tau);
    let lift = air.lift_direction()
        * (air.q * wing.area_m2 * state.cl * air.exposure(wing.yaw_decay_exponent));
    let crossflow = 1.0 - air.forward_projection;
    let effective_cd = state.cd * (1.0 - crossflow) + wing.stalled_drag_coefficient * crossflow;
    ElementLoad {
        position: point,
        lift,
        drag: -air.direction * (air.q * wing.area_m2 * effective_cd),
    }
}

fn finite_vec(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}
fn smoothstep(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn relax(value: &mut f64, target: f64, dt: f64, tau: f64) {
    let alpha = -(-dt / tau).exp_m1();
    *value += alpha * (target - *value);
}
fn polar_at(p: &[WingPolarPoint], angle: f64) -> (f64, f64) {
    let Some(first) = p.first() else {
        return (0.0, 0.0);
    };
    if angle <= first.angle_deg {
        return (first.cl, first.cd);
    }
    for pair in p.windows(2) {
        if angle <= pair[1].angle_deg {
            let t = ((angle - pair[0].angle_deg) / (pair[1].angle_deg - pair[0].angle_deg))
                .clamp(0.0, 1.0);
            return (
                pair[0].cl + t * (pair[1].cl - pair[0].cl),
                pair[0].cd + t * (pair[1].cd - pair[0].cd),
            );
        }
    }
    let last = p[p.len() - 1];
    (last.cl, last.cd)
}
fn wing_polar_at(c: &WingAeroConfig, angle: f64) -> (f64, f64) {
    let first = c.polar[0];
    let last = c.polar[c.polar.len() - 1];
    if angle >= first.angle_deg && angle <= last.angle_deg {
        return polar_at(&c.polar, angle);
    }
    let (endpoint, distance) = if angle < first.angle_deg {
        (first, first.angle_deg - angle)
    } else {
        (last, angle - last.angle_deg)
    };
    let fade = smoothstep(distance / c.post_stall_fade_deg);
    let cl = if angle < first.angle_deg {
        let next = c.polar[1];
        let slope = ((next.cl - first.cl) / (next.angle_deg - first.angle_deg)).max(0.0);
        (first.cl - slope * distance).max(0.0) * (1.0 - fade)
    } else {
        endpoint.cl * (1.0 - fade)
    };
    (
        cl,
        endpoint.cd + fade * (c.stalled_drag_coefficient - endpoint.cd),
    )
}
fn height_efficiency(h: f64, c: &UnderfloorAeroConfig) -> f64 {
    if h <= 0.0 {
        return 0.0;
    }
    if h < c.choke_height_m {
        return 0.20 * smoothstep(h / c.choke_height_m);
    }
    if h < c.optimal_height_m {
        return 0.20
            + 0.80 * smoothstep((h - c.choke_height_m) / (c.optimal_height_m - c.choke_height_m));
    }
    // Only the ground-effect increment decays to zero; free-air load is separate.
    1.0 - smoothstep((h - c.optimal_height_m) / (c.high_height_m - c.optimal_height_m))
}
fn soft_limit(raw: f64, soft: f64, hard: f64) -> f64 {
    let hard = hard.max(0.0);
    let soft = soft.clamp(0.0, hard);
    let raw = raw.max(0.0);
    if raw <= soft {
        return raw;
    }
    let span = hard - soft;
    if span <= f64::EPSILON {
        return raw.min(hard);
    }
    (soft + span * -(-(raw - soft) / span).exp_m1()).min(hard)
}

impl GroundEffectMap {
    fn sample(&self, front: f64, rear: f64) -> GroundEffectCell {
        let bracket = |axis: &[f64], value: f64| {
            let i = axis
                .partition_point(|h| *h <= value)
                .saturating_sub(1)
                .min(axis.len() - 2);
            let t = ((value - axis[i]) / (axis[i + 1] - axis[i])).clamp(0.0, 1.0);
            (i, t)
        };
        let (fi, ft) = bracket(&self.front_heights_m, front);
        let (ri, rt) = bracket(&self.rear_heights_m, rear);
        let nr = self.rear_heights_m.len();
        let weights = [
            (fi * nr + ri, (1.0 - ft) * (1.0 - rt)),
            (fi * nr + ri + 1, (1.0 - ft) * rt),
            ((fi + 1) * nr + ri, ft * (1.0 - rt)),
            ((fi + 1) * nr + ri + 1, ft * rt),
        ];
        let mut result = GroundEffectCell {
            load_factor: 0.0,
            pressure_shift_m: 0.0,
        };
        for (index, weight) in weights {
            result.load_factor += self.cells[index].load_factor * weight;
            result.pressure_shift_m += self.cells[index].pressure_shift_m * weight;
        }
        // Bounded extrapolation: a clamped edge must not preserve ground effect
        // at zero clearance or indefinitely above the measured map.
        let envelope = |h: f64, axis: &[f64]| {
            let first = axis[0];
            let last = axis[axis.len() - 1];
            if h < first {
                smoothstep(h / first)
            } else if h > last {
                1.0 - smoothstep((h - last) / last)
            } else {
                1.0
            }
        };
        result.load_factor *=
            envelope(front, &self.front_heights_m) * envelope(rear, &self.rear_heights_m);
        result
    }
}

/// Shared JSON/programmatic validation. Success does not allocate.
pub fn validate_aero_config(c: &VehicleConfig) -> Result<(), String> {
    c.aero_model.validate()?;
    for value in [c.air_density, c.coefficient_of_drag] {
        if !value.is_finite() || value < 0.0 {
            return Err("aero density/Cd must be finite and nonnegative".into());
        }
    }
    for value in [c.frontal_area, c.vehicle_mass, c.wheelbase, c.aero_lag_tau] {
        if !value.is_finite() || value <= 0.0 {
            return Err(
                "aero reference area, mass, wheelbase and lag must be positive and finite".into(),
            );
        }
    }
    if !c.aero_blend_min_speed.is_finite()
        || c.aero_blend_min_speed < 0.0
        || !c.aero_blend_full_speed.is_finite()
        || c.aero_blend_full_speed <= c.aero_blend_min_speed
    {
        return Err("aero blend speeds must satisfy 0 <= min < full".into());
    }
    if !c.center_of_gravity_height_offset.is_finite() || !c.front_weight_distribution.is_finite() {
        return Err("aero center of mass must be finite".into());
    }
    let floor = &c.aero_model.underfloor;
    let floor_z = floor.position_m.map_or(0.0, |p| p.z);
    if floor_z.abs() + 0.5 * floor.pressure_span_m + floor.max_pressure_shift_m > 0.5 * c.wheelbase
    {
        return Err(
            "aero floor pressure patches and allowed shift must stay between the axles".into(),
        );
    }
    let m = &c.aero_model;
    if m.body.drag_coefficient_scope == DragCoefficientScope::WholeVehicleReference {
        let elements = m.front_wing.area_m2
            * polar_at(&m.front_wing.polar, m.front_wing.incidence_deg).1
            + m.rear_wing.area_m2 * polar_at(&m.rear_wing.polar, m.rear_wing.incidence_deg).1
            + (floor.free_air_lift_area_m2 + floor.lift_area_m2) * floor.induced_drag_ratio;
        if c.coefficient_of_drag * c.frontal_area < elements {
            return Err(
                "aero whole-vehicle reference drag cannot be smaller than its nominal elements"
                    .into(),
            );
        }
    }
    Ok(())
}

impl AeroModelConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_wings()?;
        self.validate_underfloor()?;
        self.validate_limits_and_body()
    }

    fn validate_wings(&self) -> Result<(), String> {
        for (name, wing) in [
            ("front_wing", &self.front_wing),
            ("rear_wing", &self.rear_wing),
        ] {
            if !wing.area_m2.is_finite()
                || wing.area_m2 < 0.0
                || !wing.yaw_decay_exponent.is_finite()
                || wing.yaw_decay_exponent < 0.0
                || !wing.min_angle_deg.is_finite()
                || !wing.max_angle_deg.is_finite()
                || wing.min_angle_deg > wing.max_angle_deg
                || !wing.incidence_deg.is_finite()
                || wing.incidence_deg < wing.min_angle_deg
                || wing.incidence_deg > wing.max_angle_deg
                || !wing.post_stall_fade_deg.is_finite()
                || wing.post_stall_fade_deg <= 0.0
                || !wing.stalled_drag_coefficient.is_finite()
                || wing.stalled_drag_coefficient < 0.0
                || wing
                    .response_tau_s
                    .is_some_and(|t| !t.is_finite() || t <= 0.0)
                || wing.position_m.is_some_and(|p| !finite_vec(p))
            {
                return Err(format!(
                    "aero.{name}: invalid area, setup angles, response or position"
                ));
            }
            if wing.polar.len() < 2
                || wing.polar.len() > 128
                || wing.polar.iter().any(|p| {
                    !p.angle_deg.is_finite()
                        || !p.cl.is_finite()
                        || p.cl < 0.0
                        || !p.cd.is_finite()
                        || p.cd < 0.0
                })
                || wing
                    .polar
                    .windows(2)
                    .any(|p| p[1].angle_deg <= p[0].angle_deg)
            {
                return Err(format!(
                    "aero.{name}: polar needs 2..128 ordered finite points with nonnegative CL/CD"
                ));
            }
            if wing.min_angle_deg < wing.polar[0].angle_deg
                || wing.max_angle_deg > wing.polar[wing.polar.len() - 1].angle_deg
            {
                return Err(format!(
                    "aero.{name}: polar must cover the mechanical setup range"
                ));
            }
        }
        Ok(())
    }

    fn validate_underfloor(&self) -> Result<(), String> {
        self.validate_underfloor_scalars()?;
        self.validate_underfloor_geometry()?;
        self.validate_underfloor_map()
    }

    fn validate_underfloor_scalars(&self) -> Result<(), String> {
        let f = &self.underfloor;
        for value in [
            f.lift_area_m2,
            f.free_air_lift_area_m2,
            f.yaw_decay_exponent,
            f.induced_drag_ratio,
            f.max_pressure_shift_m,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err("aero.underfloor: areas, yaw, drag and pressure shift must be finite and nonnegative".into());
            }
        }
        for value in [
            f.choke_height_m,
            f.optimal_height_m,
            f.high_height_m,
            f.rake_window_deg,
            f.stall_attack_tau_s,
            f.stall_recovery_tau_s,
            f.seal_asymmetry_m,
            f.seal_roll_rad,
            f.pressure_span_m,
            f.pressure_response_tau_s,
            f.probe_loss_tau_s,
            f.probe_recovery_tau_s,
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(
                    "aero.underfloor: dimensions and time constants must be positive and finite"
                        .into(),
                );
            }
        }
        Ok(())
    }

    fn validate_underfloor_geometry(&self) -> Result<(), String> {
        let f = &self.underfloor;
        if f.choke_height_m >= f.optimal_height_m
            || f.optimal_height_m >= f.high_height_m
            || !f.optimal_rake_deg.is_finite()
            || !f.choke_recovery_ratio.is_finite()
            || f.choke_recovery_ratio <= 1.0
            || f.choke_height_m * f.choke_recovery_ratio >= f.optimal_height_m
            || f.position_m.is_some_and(|p| !finite_vec(p))
        {
            return Err(
                "aero.underfloor: inconsistent heights, recovery threshold, rake or position"
                    .into(),
            );
        }
        for value in [f.bottoming_factor, f.separated_flow_factor] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(
                    "aero.underfloor: bottoming/separation factors must be in [0,1]".into(),
                );
            }
        }
        Ok(())
    }

    fn validate_underfloor_map(&self) -> Result<(), String> {
        let f = &self.underfloor;
        if let Some(map) = &f.map {
            Self::validate_underfloor_map_axes(map)?;
            Self::validate_underfloor_map_cells(map, f.max_pressure_shift_m)?;
        }
        Ok(())
    }

    fn validate_underfloor_map_axes(map: &GroundEffectMap) -> Result<(), String> {
        for axis in [&map.front_heights_m, &map.rear_heights_m] {
            if axis.len() < 2
                || axis.len() > 16
                || axis.iter().any(|h| !h.is_finite() || *h <= 0.0)
                || axis.windows(2).any(|h| h[1] <= h[0])
            {
                return Err(
                    "aero.underfloor.map: each axis needs 2..16 increasing positive heights".into(),
                );
            }
        }
        Ok(())
    }

    fn validate_underfloor_map_cells(
        map: &GroundEffectMap,
        max_pressure_shift_m: f64,
    ) -> Result<(), String> {
        if map.cells.len() != map.front_heights_m.len() * map.rear_heights_m.len()
            || map.cells.iter().any(|cell| {
                !cell.load_factor.is_finite()
                    || !(0.0..=4.0).contains(&cell.load_factor)
                    || !cell.pressure_shift_m.is_finite()
                    || cell.pressure_shift_m.abs() > max_pressure_shift_m
            })
        {
            return Err(
                "aero.underfloor.map: wrong cell count or load/pressure outside bounds".into(),
            );
        }
        Ok(())
    }

    fn validate_limits_and_body(&self) -> Result<(), String> {
        for value in [
            self.limits.soft_max_load_ratio,
            self.limits.hard_max_load_ratio,
            self.limits.hard_max_downforce_n,
            self.body.side_drag_area_m2,
            self.body.vertical_drag_area_m2,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(
                    "aero limits and body drag areas must be finite and nonnegative".into(),
                );
            }
        }
        if self.limits.soft_max_load_ratio > self.limits.hard_max_load_ratio
            || self.body.position_m.is_some_and(|p| !finite_vec(p))
        {
            return Err(
                "aero soft limit must not exceed hard limit; body position must be finite".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polar_interpolates_and_clamps() {
        let p = WingAeroConfig::front_default().polar;
        assert_eq!(polar_at(&p, -10.0), (0.45, 0.075));
        assert!((polar_at(&p, 10.0).0 - 0.97).abs() < 1e-9);
        assert_eq!(polar_at(&p, 40.0), (1.20, 0.340));
    }

    #[test]
    fn era_limiter_never_exceeds_hard_cap() {
        assert!(soft_limit(100_000.0, 9_600.0, 10_800.0) <= 10_800.0);
        assert_eq!(soft_limit(5_000.0, 9_600.0, 10_800.0), 5_000.0);
        let cfg = VehicleConfig::f1_94_canonical();
        let mut aero = AeroForces::zero();
        for _ in 0..100 {
            aero.step_with_environment(
                &cfg,
                Vec3::new(0.0, 0.0, -100.0),
                &AeroEnvironment::default(),
                0.0,
                1.0 / 120.0,
            );
        }
        let hard = (cfg.vehicle_mass * 9.81 * cfg.aero_model.limits.hard_max_load_ratio)
            .min(cfg.aero_model.limits.hard_max_downforce_n);
        assert!(aero.total_downforce <= hard + 1e-6);
    }

    #[test]
    fn floor_chokes_near_ground() {
        let c = UnderfloorAeroConfig::default();
        assert!(height_efficiency(0.004, &c) < height_efficiency(c.optimal_height_m, &c));
    }

    #[test]
    fn localized_bottoming_collapses_floor_before_wings() {
        let cfg = VehicleConfig::f1_94_canonical();
        let velocity = Vec3::new(0.0, 0.0, -55.0);
        let mut clear = AeroForces::zero();
        let mut bottoming = AeroForces::zero();
        let mut hit = AeroEnvironment::default();
        hit.bottoming_mask = 1;
        for _ in 0..80 {
            clear.step_with_environment(
                &cfg,
                velocity,
                &AeroEnvironment::default(),
                0.0,
                1.0 / 120.0,
            );
            bottoming.step_with_environment(&cfg, velocity, &hit, 0.0, 1.0 / 120.0);
        }
        // The floor is two elements (front/rear). One bottoming probe collapses the
        // front element, so the combined diffuser load drops clearly (to ~58% of the
        // clear case) while the wings are untouched.
        assert!(clear.diffuser_downforce > 0.0);
        assert!(
            bottoming.diffuser_downforce < clear.diffuser_downforce * 0.65,
            "localized bottoming must collapse the floor: {} vs {}",
            bottoming.diffuser_downforce,
            clear.diffuser_downforce
        );
        assert!((bottoming.front_wing_cl - clear.front_wing_cl).abs() < 1e-9);
        assert!((bottoming.rear_wing_cl - clear.rear_wing_cl).abs() < 1e-9);
    }

    #[test]
    fn floor_ignores_probe_5_and_responds_to_rake() {
        let cfg = VehicleConfig::f1_94_canonical();
        let velocity = Vec3::new(0.0, 0.0, -55.0);
        let step_floor = |env: &AeroEnvironment, n: usize| {
            let mut aero = AeroForces::zero();
            for _ in 0..n {
                aero.step_with_environment(&cfg, velocity, env, 0.0, 1.0 / 120.0);
            }
            aero
        };
        let base = AeroEnvironment {
            clearance_m: [0.05, 0.05, 0.06, 0.075, 0.075],
            valid_mask: 0x1f,
            rake_rad: 0.5f64.to_radians(),
            roll_rad: 0.0,
            bottoming_mask: 0,
            contact_confidence: 1.0,
        };
        let mut expanded = base;
        expanded.clearance_m[4] = 0.30;
        let a = step_floor(&base, 200);
        let b = step_floor(&expanded, 200);
        assert_eq!(a.diffuser_downforce, b.diffuser_downforce);
        assert_eq!(a.diffuser_stall_factor, b.diffuser_stall_factor);
        // Rake is reconstructed from the probe plane, so tilt the front/rear heights
        // (mean height preserved) to raise the rake and check the floor responds.
        let mut raked = base;
        raked.clearance_m = [0.06, 0.06, 0.06, 0.065, 0.065];
        let r = step_floor(&raked, 200);
        assert!((r.diffuser_downforce - a.diffuser_downforce).abs() > 1.0);
    }
}
