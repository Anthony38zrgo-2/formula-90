use super::*;
use super::driving_assistance::configuration::default_aids;
use super::powertrain::document::{default_gear_inertia, default_torque_attack_rate};
use super::steering::document::default_counter_speed;
use super::aerodynamics::configuration::{default_aero_blend_full_speed, default_aero_blend_min_speed, default_aero_lag_tau};

impl VehicleConfiguration {
    /// Resolved F1 2030 geometric profile embedded at build time from its domain manifest.
    pub fn f1_2030_v10_geometric_embedded_document() -> &'static str {
        include_str!(concat!(
            env!("OUT_DIR"),
            "/resolved_f1_2030_v10_geometric.json"
        ))
    }

    /// Canonical F1-94 specification matching data/vehicles/f1_94/ resources.
    pub fn f1_94_canonical() -> Self {
        let mut surface_friction = HashMap::new();
        surface_friction.insert(SurfaceType::Road, 2.9);
        surface_friction.insert(SurfaceType::Curb, 2.2);
        surface_friction.insert(SurfaceType::Dirt, 1.4);
        surface_friction.insert(SurfaceType::Grass, 0.9);
        surface_friction.insert(SurfaceType::Gravel, 1.1);

        let mut surface_stiffness = HashMap::new();
        surface_stiffness.insert(SurfaceType::Road, 8.75);
        surface_stiffness.insert(SurfaceType::Curb, 7.0);
        surface_stiffness.insert(SurfaceType::Dirt, 0.5);
        surface_stiffness.insert(SurfaceType::Grass, 0.5);
        surface_stiffness.insert(SurfaceType::Gravel, 0.5);

        let mut surface_rolling_resistance = HashMap::new();
        surface_rolling_resistance.insert(SurfaceType::Road, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Curb, 1.5);
        surface_rolling_resistance.insert(SurfaceType::Dirt, 2.0);
        surface_rolling_resistance.insert(SurfaceType::Grass, 4.0);
        surface_rolling_resistance.insert(SurfaceType::Gravel, 2.0);

        let mut surface_longitudinal_grip_ratio = HashMap::new();
        for st in [
            SurfaceType::Road,
            SurfaceType::Curb,
            SurfaceType::Dirt,
            SurfaceType::Grass,
            SurfaceType::Gravel,
        ] {
            surface_longitudinal_grip_ratio.insert(st, 0.5);
        }

        Self {
            schema_version: 2,
            chassis: ChassisConfiguration {
                vehicle_name: "F1 1994 (V10)".to_string(),
                vehicle_mass: 575.0,
                vehicle_mass_excludes_wheel_assemblies: false,
                front_weight_distribution: 0.45,
                center_of_gravity_height_offset: -0.12,
                inertia_multipliers: Vec3::new(1.10, 1.10, 1.10),
                wheelbase: 2.92065,
                front_track: 1.5925,
                rear_track: 1.5246,
                physical_mass_inventory: None,
            },
            fuel: FuelConfig::default(),

            // Steering
            steering: SteeringConfiguration {
                max_steering_angle: 0.436332,
                front_steering_ratio: 1.0,
                rear_steering_ratio: 0.0,
                steering_speed: 4.25,
                countersteer_speed: default_counter_speed(),
                steering_speed_decay: 0.20,
                steering_slip_assist: 0.54,
                countersteer_assist: 0.89,
                steering_exponent: 1.50,
                ackermann: 0.15,
            },

            // Powertrain & Gearing
            powertrain: PowertrainConfiguration {
                max_torque: 455.0,
                max_rpm: 15000.0,
                idle_rpm: 4500.0,
                motor_moment: 0.12,
                torque_curve: vec![
                    (0.00, 0.20),
                    (0.265, 0.28),
                    (0.44, 0.58),
                    (0.62, 0.85),
                    (0.79, 0.98),
                    (0.88, 1.00),
                    (0.97, 0.95),
                    (1.00, 0.88),
                ],
                gear_ratios: vec![2.65, 2.10, 1.75, 1.50, 1.32, 1.18],
                final_drive: 6.30,
                reverse_ratio: 3.00,
                shift_time: 0.12,
                automatic_transmission: false,
                downshift_rev_matching_enabled: false,
                downshift_maximum_target_revolutions_per_minute_fraction: 1.0,
                downshift_blip_torque_fraction: 0.0,
                downshift_minimum_interval_seconds: 0.0,
                downshift_clutch_reengagement_seconds: 0.0,
                front_torque_split: 0.0,

                // Differential (Salisbury Clutch-Pack LSD, AMS2/Reiza aligned) — CORR-03 candidate B: 170/65/75/mu0.0
                diff_preload: 170.0,
                diff_power_ramp_angle_deg: 65.0,
                diff_coast_ramp_angle_deg: 75.0,
                diff_clutches: 4.0,
                diff_clutch_friction_coeff: 0.0,

                max_clutch_torque_ratio: 1.6,
                clutch_out_rpm_offset: 1000.0,
                idle_disengagement_hysteresis_rpm: 50.0,
                variable_drag_ratio: 0.10,
                constant_brake_ratio: 0.02,
                handbrake_torque_fraction: 0.4,
                torque_attack_rate_nm_s: default_torque_attack_rate(),
                diff_slip_transition_threshold_rad_s: 0.5,
                automatic_shift: AutomaticShift::default(),
                gear_inertia: default_gear_inertia(),
                powertrain_thermal: PowertrainThermalConfig::default(),
            },
            audio: None,
            coupled_world: None,
            underfloor_contact: None,

            suspension: SuspensionConfiguration {
                front_spring_length: 0.250,
                front_resting_ratio: 0.280,
                front_damping_ratio: 0.80,
                front_bump_damp_multiplier: 1.3,
                front_rebound_damp_multiplier: 1.1,
                front_arb_ratio: 0.20,
                front_toe: 0.0017453,
                front_camber: -0.0174533,
                front_bump_stop_multiplier: 2.2,
                rear_spring_length: 0.180,
                rear_resting_ratio: 0.350,
                rear_damping_ratio: 0.80,
                rear_bump_damp_multiplier: 1.3,
                rear_rebound_damp_multiplier: 1.1,
                rear_arb_ratio: 0.05,
                rear_toe: 0.0017453,
                rear_camber: -0.0174533,
                rear_bump_stop_multiplier: 3.5,
                tri_ray_spacing_ratio: 0.40,
                front_camber_gain_rad_per_m: 0.0,
                rear_camber_gain_rad_per_m: 0.0,
                front_toe_gain_rad_per_m: 0.0,
                rear_toe_gain_rad_per_m: 0.0,
                suspension_model: SuspensionModelKind::Legacy1Dof,
                suspension_model_version: default_suspension_model_version(),
                geometric_suspension: None,
            },

            // Tires & Contact
            tires: TiresConfiguration {
                front_tire_radius: 0.31695,
                rear_tire_radius: 0.32901,
                front_tire_width: 0.30030,
                rear_tire_width: 0.36832,
                front_wheel_mass: 12.0,
                rear_wheel_mass: 16.0,
                contact_patch: 0.21,
                front_contact_patch: 0.21,
                rear_contact_patch: 0.21,
                front_airborne_decay: 2.0,
                rear_airborne_decay: 2.0,

                // Tire pressure + thermal (F1-94 cold setup + nominal hot reference)
                tire_pressure: TirePressureConfig::default(),
                tire_thermal: TireThermalAxleConfig::default(),
                tire_wear: TireWearAxleConfig::default(),
                front_tire_force: TireForceProfile::default(),
                rear_tire_force: TireForceProfile::default(),
                pressure_mechanics: PressureMechanicsSensitivity::default(),

                // Surface Maps
                surface_friction: surface_friction.clone(),
                surface_stiffness: surface_stiffness.clone(),
                surface_rolling_resistance: surface_rolling_resistance.clone(),
                surface_longitudinal_grip_ratio: surface_longitudinal_grip_ratio.clone(),
                surface_transition_tau_s: 0.0,
            },
            brakes: BrakesConfiguration {
                front_brake_bias: 0.57,
                max_brake_torque: 2800.0,
                brake_thermal: BrakeThermalConfig::default(),
                enable_abs: false,
                abs_pulse_time: 0.03,
                abs_spin_diff_threshold: 12.0,
                wheel_lock_blend_spin_rad_s: 0.0,
                wheel_lock_stick_tau_s: 0.0,
            },

            // Driving aids
            aids: default_aids(),

            // Aerodynamics (element-based model)
            aerodynamics: AerodynamicsConfiguration {
                coefficient_of_drag: 0.78,
                frontal_area: 1.25,
                aero_model: AeroModelConfig::default(),
                air_density: 1.225,
                aero_lag_tau: default_aero_lag_tau(),
                aero_blend_min_speed: default_aero_blend_min_speed(),
                aero_blend_full_speed: default_aero_blend_full_speed(),
            },
        }
    }

    /// Canonical active vehicle profile loaded from the checked-in runtime JSON.
    pub fn f1_2026_2008_canonical() -> Self {
        Self::from_json_str(include_str!("../../../../data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"))
            .expect("checked-in f1_2026_2008 physics profile must be valid")
    }

    /// Canonical Jordan 197 specification (legacy — aero extensions use same defaults, not validated).
    pub fn jordan_197_canonical() -> Self {
        let mut surface_friction = HashMap::new();
        surface_friction.insert(SurfaceType::Road, 2.0);
        surface_friction.insert(SurfaceType::Curb, 1.85);
        surface_friction.insert(SurfaceType::Dirt, 1.4);
        surface_friction.insert(SurfaceType::Grass, 1.0);

        let mut surface_stiffness = HashMap::new();
        surface_stiffness.insert(SurfaceType::Road, 5.0);
        surface_stiffness.insert(SurfaceType::Curb, 4.5);
        surface_stiffness.insert(SurfaceType::Dirt, 0.5);
        surface_stiffness.insert(SurfaceType::Grass, 0.5);

        let mut surface_rolling_resistance = HashMap::new();
        surface_rolling_resistance.insert(SurfaceType::Road, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Curb, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Dirt, 1.5);
        surface_rolling_resistance.insert(SurfaceType::Grass, 2.0);

        Self {
            schema_version: 2,
            chassis: ChassisConfiguration {
                vehicle_name: "Jordan 197 (V10)".to_string(),
                vehicle_mass: 505.0,
                vehicle_mass_excludes_wheel_assemblies: false,
                front_weight_distribution: 0.45,
                center_of_gravity_height_offset: -0.12,
                inertia_multipliers: Vec3::new(1.10, 1.10, 1.10),
                wheelbase: 2.950,
                front_track: 1.762,
                rear_track: 1.748,
                physical_mass_inventory: None,
            },
            fuel: FuelConfig::default(),

            // Steering
            steering: SteeringConfiguration {
                max_steering_angle: 0.436332, // ~25.0 degrees
                front_steering_ratio: 1.0,
                rear_steering_ratio: 0.0,
                steering_speed: 3.7,
                countersteer_speed: default_counter_speed(),
                steering_speed_decay: 0.26,
                steering_slip_assist: 0.11,
                countersteer_assist: 0.70,
                steering_exponent: 1.70,
                ackermann: 0.15,
            },

            // Powertrain & Gearing
            powertrain: PowertrainConfiguration {
                max_torque: 430.0,
                max_rpm: 13000.0,
                idle_rpm: 3500.0,
                motor_moment: 0.22,
                torque_curve: vec![
                    (0.00, 0.38),
                    (0.30, 0.65),
                    (0.55, 0.88),
                    (0.72, 1.00),
                    (0.90, 0.92),
                    (1.00, 0.76),
                ],
                gear_ratios: vec![3.00, 2.25, 1.75, 1.42, 1.20, 1.03],
                final_drive: 3.10,
                reverse_ratio: 3.00,
                shift_time: 0.16,
                automatic_transmission: true,
                downshift_rev_matching_enabled: false,
                downshift_maximum_target_revolutions_per_minute_fraction: 1.0,
                downshift_blip_torque_fraction: 0.0,
                downshift_minimum_interval_seconds: 0.0,
                downshift_clutch_reengagement_seconds: 0.0,
                front_torque_split: 0.0, // RWD

                // Differential (Salisbury Clutch-Pack LSD)
                diff_preload: 90.0,
                diff_power_ramp_angle_deg: 45.0,
                diff_coast_ramp_angle_deg: 60.0,
                diff_clutches: 4.0,
                diff_clutch_friction_coeff: 0.25,

                max_clutch_torque_ratio: 1.6,
                clutch_out_rpm_offset: 1000.0,
                idle_disengagement_hysteresis_rpm: 50.0,
                variable_drag_ratio: 0.10,
                constant_brake_ratio: 0.02,
                handbrake_torque_fraction: 0.4,
                torque_attack_rate_nm_s: default_torque_attack_rate(),
                diff_slip_transition_threshold_rad_s: 0.5,
                automatic_shift: AutomaticShift::default(),
                gear_inertia: default_gear_inertia(),
                powertrain_thermal: PowertrainThermalConfig::default(),
            },
            audio: None,
            coupled_world: None,
            underfloor_contact: None,

            suspension: SuspensionConfiguration {
                front_spring_length: 0.100,
                front_resting_ratio: 0.500,
                front_damping_ratio: 0.80,
                front_bump_damp_multiplier: 1.3,
                front_rebound_damp_multiplier: 1.1,
                front_arb_ratio: 0.25,
                front_toe: 0.002,
                front_camber: -0.052,
                front_bump_stop_multiplier: 2.2,
                rear_spring_length: 0.100,
                rear_resting_ratio: 0.500,
                rear_damping_ratio: 0.85,
                rear_bump_damp_multiplier: 1.3,
                rear_rebound_damp_multiplier: 1.1,
                rear_arb_ratio: 0.10,
                rear_toe: 0.001,
                rear_camber: -0.026,
                rear_bump_stop_multiplier: 2.2,
                tri_ray_spacing_ratio: 0.40,
                front_camber_gain_rad_per_m: 0.0,
                rear_camber_gain_rad_per_m: 0.0,
                front_toe_gain_rad_per_m: 0.0,
                rear_toe_gain_rad_per_m: 0.0,
                suspension_model: SuspensionModelKind::Legacy1Dof,
                suspension_model_version: default_suspension_model_version(),
                geometric_suspension: None,
            },

            // Tires & Brakes
            tires: TiresConfiguration {
                front_tire_radius: 0.324,
                rear_tire_radius: 0.324,
                front_tire_width: 0.308, // 308 mm
                rear_tire_width: 0.358,  // 358 mm
                front_wheel_mass: 12.0,
                rear_wheel_mass: 16.0,
                contact_patch: 0.20,
                front_contact_patch: 0.20,
                rear_contact_patch: 0.20,
                front_airborne_decay: 2.0,
                rear_airborne_decay: 2.0,

                // Tire pressure + thermal (legacy Jordan uses canonical defaults)
                tire_pressure: TirePressureConfig::default(),
                tire_thermal: TireThermalAxleConfig::default(),
                tire_wear: TireWearAxleConfig::default(),
                front_tire_force: TireForceProfile::default(),
                rear_tire_force: TireForceProfile::default(),
                pressure_mechanics: PressureMechanicsSensitivity::default(),

                // Surface Maps
                surface_friction,
                surface_stiffness,
                surface_rolling_resistance,
                surface_longitudinal_grip_ratio: HashMap::new(),
                surface_transition_tau_s: 0.0,
            },
            brakes: BrakesConfiguration {
                front_brake_bias: 0.58,
                max_brake_torque: 2800.0,
                brake_thermal: BrakeThermalConfig::default(),
                enable_abs: false,
                abs_pulse_time: 0.03,
                abs_spin_diff_threshold: 12.0,
                wheel_lock_blend_spin_rad_s: 0.0,
                wheel_lock_stick_tau_s: 0.0,
            },

            // Driving aids
            aids: default_aids(),

            // Aerodynamics (element-based model)
            aerodynamics: AerodynamicsConfiguration {
                coefficient_of_drag: 0.75,
                frontal_area: 1.20,
                aero_model: AeroModelConfig::default(),
                air_density: 1.225,
                aero_lag_tau: default_aero_lag_tau(),
                aero_blend_min_speed: default_aero_blend_min_speed(),
                aero_blend_full_speed: default_aero_blend_full_speed(),
            },
        }
    }
}
