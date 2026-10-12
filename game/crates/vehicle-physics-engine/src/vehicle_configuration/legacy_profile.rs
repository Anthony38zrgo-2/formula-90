use super::*;
use super::brakes::document::{JsonBrakeThermal, JsonBrakes};
use super::brakes::validation::validate_brake_thermal;
use super::chassis::document::{JsonChassis, JsonGeometry, JsonVec3};
use super::coupled_world::CoupledVehicleWorldConfiguration;
use super::driving_assistance::document::JsonAids;
use super::fuel::document::JsonFuel;
use super::powertrain::document::{JsonDifferential, JsonPowertrain};
use super::steering::document::JsonSteering;
use super::suspension::document::{JsonSuspension, JsonSuspensionAxle};
use super::tires::conversion::{
    build_surface_assist_map, parse_surface_type, surface_type_name, tire_pressure_from_json,
    tire_thermal_from_json, tire_wear_from_json, wheel_name_map,
};
use super::aerodynamics::document::JsonAero;
use super::steering::document::default_counter_speed;
use super::tires::document::{
    JsonPressureMechanics, JsonSurfaceEntry, JsonTireAxle, JsonTireForceModel, JsonTireForceProfile,
    JsonTirePressure, JsonTireThermal, JsonTireWear, JsonTires,
};
use super::underfloor_contact::UnderfloorContactConfig;
use super::vehicle_configuration::VehicleConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JsonVehicleSpec {
    #[serde(default = "default_schema_version")]
    pub(crate) schema_version: u32,
    #[serde(default)]
    pub(crate) vehicle_id: String,
    #[serde(default)]
    pub(crate) profile_name: String,
    #[serde(default)]
    pub(crate) units: String,
    #[serde(default)]
    pub(crate) chassis: JsonChassis,
    #[serde(default)]
    pub(crate) fuel: JsonFuel,
    #[serde(default)]
    pub(crate) geometry: JsonGeometry,
    #[serde(default)]
    pub(crate) steering: JsonSteering,
    #[serde(default)]
    pub(crate) powertrain: JsonPowertrain,
    #[serde(default)]
    pub(crate) suspension: JsonSuspension,
    #[serde(default)]
    pub(crate) tires: JsonTires,
    #[serde(default)]
    pub(crate) brakes: JsonBrakes,
    #[serde(default)]
    pub(crate) aero: JsonAero,
    #[serde(default)]
    pub(crate) aids: JsonAids,
    #[serde(default)]
    pub(crate) coordinate_contract: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) audio: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) physical_mass_inventory: Option<crate::vehicle_mass_inventory::VehicleMassInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) coupled_world: Option<CoupledVehicleWorldConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) underfloor_contact: Option<UnderfloorContactConfig>,
}

fn default_schema_version() -> u32 {
    1
}

impl JsonVehicleSpec {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 && self.schema_version != 2 && self.schema_version != 3 {
            return Err(format!(
                "Unsupported schema_version: {}",
                self.schema_version
            ));
        }
        super::chassis::validation::validate_chassis(&self.chassis)?;
        super::chassis::validation::validate_geometry(&self.geometry)?;
        super::powertrain::validation::validate_powertrain(&self.powertrain)?;
        super::tires::validation::validate_axle_radii(&self.tires)?;
        super::fuel::validation::validate_fuel(&self.fuel)?;
        if let Some(configuration) = self.underfloor_contact.as_ref() {
            configuration.validate()?;
        }
        self.tires.validate()?;
        self.suspension.validate_geometry()?;
        self.suspension.validate_spring_lengths()?;
        let brake_thermal = self
            .brakes
            .thermal
            .as_ref()
            .map(JsonBrakeThermal::to_config)
            .unwrap_or_default();
        validate_brake_thermal(&brake_thermal)?;
        self.powertrain
            .thermal
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .validate()?;
        self.brakes.validate_wheel_lock()?;
        self.aids.validate(&self.powertrain)?;
        Ok(())
    }

    pub(crate) fn to_config(self) -> VehicleConfig {
        let mut surface_friction = HashMap::new();
        let mut surface_stiffness = HashMap::new();
        let mut surface_rolling_resistance = HashMap::new();

        for (name, entry) in &self.tires.surfaces {
            if let Ok(st) = parse_surface_type(name) {
                surface_friction.insert(st, entry.friction);
                surface_stiffness.insert(st, entry.stiffness);
                surface_rolling_resistance.insert(st, entry.rolling_resistance);
            }
        }

        let mut fuel = FuelConfig {
            capacity_kg: self.fuel.capacity_kg.max(0.0),
            initial_kg: self.fuel.initial_kg,
            tank_position_local_m: Vec3::new(
                self.fuel.tank_position_local_m.x,
                self.fuel.tank_position_local_m.y,
                self.fuel.tank_position_local_m.z,
            ),
            brake_specific_consumption_kg_per_kwh: self.fuel.brake_specific_consumption_kg_per_kwh,
            idle_consumption_kg_per_hour: self.fuel.idle_consumption_kg_per_hour,
            estimated_lap_consumption_kg: self.fuel.estimated_lap_consumption_kg,
            reference_lap_time_s: self.fuel.reference_lap_time_s,
            current_kg: 0.0,
            consumed_kg: 0.0,
        };
        fuel.refill_to_initial();

        VehicleConfig {
            schema_version: self.schema_version,
            chassis: ChassisConfiguration {
                vehicle_name: self.chassis.vehicle_name,
                vehicle_mass: self.chassis.vehicle_mass,
                vehicle_mass_excludes_wheel_assemblies: self.chassis.vehicle_mass_excludes_wheel_assemblies,
                front_weight_distribution: self.chassis.front_weight_distribution,
                center_of_gravity_height_offset: self.chassis.center_of_gravity_height_offset,
                inertia_multipliers: Vec3::new(
                    self.chassis.inertia_multipliers.x,
                    self.chassis.inertia_multipliers.y,
                    self.chassis.inertia_multipliers.z,
                ),
                wheelbase: self.geometry.wheelbase,
                front_track: self.geometry.front_track,
                rear_track: self.geometry.rear_track,
                physical_mass_inventory: self.physical_mass_inventory.clone(),
            },
            fuel,
            steering: SteeringConfiguration {
                max_steering_angle: self.steering.max_steering_angle,
                front_steering_ratio: self.steering.front_steering_ratio,
                rear_steering_ratio: self.steering.rear_steering_ratio,
                steering_speed: self.steering.steering_speed,
                countersteer_speed: default_counter_speed(),
                steering_speed_decay: self.steering.steering_speed_decay,
                steering_slip_assist: self.steering.steering_slip_assist,
                countersteer_assist: self.steering.countersteer_assist,
                steering_exponent: self.steering.steering_exponent,
                ackermann: self.steering.ackermann,
            },
            powertrain: PowertrainConfiguration {
                max_torque: self.powertrain.max_torque,
                max_rpm: self.powertrain.max_rpm,
                idle_rpm: self.powertrain.idle_rpm,
                motor_moment: self.powertrain.motor_moment,
                torque_curve: self.powertrain.torque_curve,
                gear_ratios: self.powertrain.gear_ratios,
                final_drive: self.powertrain.final_drive,
                reverse_ratio: self.powertrain.reverse_ratio,
                shift_time: self.powertrain.shift_time,
                automatic_transmission: self.powertrain.automatic_transmission,
                downshift_rev_matching_enabled: self.powertrain.downshift_rev_matching_enabled,
                downshift_maximum_target_revolutions_per_minute_fraction: self.powertrain.downshift_maximum_target_revolutions_per_minute_fraction,
                downshift_blip_torque_fraction: self.powertrain.downshift_blip_torque_fraction,
                downshift_minimum_interval_seconds: self.powertrain.downshift_minimum_interval_seconds,
                downshift_clutch_reengagement_seconds: self.powertrain.downshift_clutch_reengagement_seconds,
                front_torque_split: self.powertrain.front_torque_split,
                diff_preload: self.powertrain.differential.preload_nm,
                diff_power_ramp_angle_deg: self.powertrain.differential.power_ramp_angle_deg,
                diff_coast_ramp_angle_deg: self.powertrain.differential.coast_ramp_angle_deg,
                diff_clutches: self.powertrain.differential.clutches,
                diff_clutch_friction_coeff: self.powertrain.differential.clutch_friction_coefficient,
                max_clutch_torque_ratio: self.powertrain.max_clutch_torque_ratio,
                clutch_out_rpm_offset: self.powertrain.clutch_out_rpm_offset,
                idle_disengagement_hysteresis_rpm: self.powertrain.idle_disengagement_hysteresis_rpm,
                variable_drag_ratio: self.powertrain.variable_drag_ratio,
                constant_brake_ratio: self.powertrain.constant_brake_ratio,
                handbrake_torque_fraction: self.powertrain.handbrake_torque_fraction,
                torque_attack_rate_nm_s: self.powertrain.torque_attack_rate_nm_s,
                diff_slip_transition_threshold_rad_s: self
                    .powertrain
                    .differential
                    .slip_transition_threshold_rad_s,
                automatic_shift: self
                    .powertrain
                    .automatic_shift
                    .as_ref()
                    .map(AutomaticShift::from_json)
                    .unwrap_or_default(),
                gear_inertia: self.powertrain.gear_inertia,
                powertrain_thermal: self.powertrain.thermal.unwrap_or_default(),
            },
            suspension: SuspensionConfiguration {
                front_spring_length: self.suspension.front.spring_length,
                front_resting_ratio: self.suspension.front.resting_ratio,
                front_damping_ratio: self.suspension.front.damping_ratio,
                front_bump_damp_multiplier: self.suspension.front.bump_damp_multiplier,
                front_rebound_damp_multiplier: self.suspension.front.rebound_damp_multiplier,
                front_arb_ratio: self.suspension.front.arb_ratio,
                front_toe: self.suspension.front.toe,
                front_camber: self.suspension.front.camber,
                front_bump_stop_multiplier: self.suspension.front.bump_stop_multiplier,
                rear_spring_length: self.suspension.rear.spring_length,
                rear_resting_ratio: self.suspension.rear.resting_ratio,
                rear_damping_ratio: self.suspension.rear.damping_ratio,
                rear_bump_damp_multiplier: self.suspension.rear.bump_damp_multiplier,
                rear_rebound_damp_multiplier: self.suspension.rear.rebound_damp_multiplier,
                rear_arb_ratio: self.suspension.rear.arb_ratio,
                rear_toe: self.suspension.rear.toe,
                rear_camber: self.suspension.rear.camber,
                rear_bump_stop_multiplier: self.suspension.rear.bump_stop_multiplier,
                tri_ray_spacing_ratio: self.suspension.tri_ray_spacing_ratio,
                front_camber_gain_rad_per_m: self.suspension.front.camber_gain_rad_per_m.unwrap_or(0.0),
                rear_camber_gain_rad_per_m: self.suspension.rear.camber_gain_rad_per_m.unwrap_or(0.0),
                front_toe_gain_rad_per_m: self.suspension.front.toe_gain_rad_per_m.unwrap_or(0.0),
                rear_toe_gain_rad_per_m: self.suspension.rear.toe_gain_rad_per_m.unwrap_or(0.0),
                suspension_model: SuspensionModelKind::from_str_name(self.suspension.model.as_str())
                    .unwrap_or(SuspensionModelKind::Legacy1Dof),
                suspension_model_version: self.suspension.model_version,
                geometric_suspension: self
                    .suspension
                    .geometry_physical
                    .as_ref()
                    .map(|v| geometric_from_json_value(v).expect("validated geometry_physical must parse")),
            },
            tires: TiresConfiguration {
                front_tire_radius: self.tires.front.radius,
                rear_tire_radius: self.tires.rear.radius,
                front_tire_width: self.tires.front.width,
                rear_tire_width: self.tires.rear.width,
                front_wheel_mass: self.tires.front.wheel_mass,
                rear_wheel_mass: self.tires.rear.wheel_mass,
                contact_patch: self.tires.contact_patch,
                front_contact_patch: self
                    .tires
                    .front
                    .contact_patch
                    .unwrap_or(self.tires.contact_patch),
                rear_contact_patch: self
                    .tires
                    .rear
                    .contact_patch
                    .unwrap_or(self.tires.contact_patch),
                front_airborne_decay: self
                    .tires
                    .front
                    .airborne_spin_decay_torque
                    .unwrap_or(self.tires.airborne_spin_decay_torque),
                rear_airborne_decay: self
                    .tires
                    .rear
                    .airborne_spin_decay_torque
                    .unwrap_or(self.tires.airborne_spin_decay_torque),
                tire_pressure: tire_pressure_from_json(&self.tires.pressure),
                tire_thermal: tire_thermal_from_json(&self.tires.thermal),
                tire_wear: tire_wear_from_json(&self.tires.wear),
                front_tire_force: self
                    .tires
                    .force_model
                    .as_ref()
                    .map(|fm| TireForceProfile {
                        lateral_peak_slip_angle_rad: fm.front.lateral_peak_slip_angle_rad,
                        longitudinal_peak_slip_ratio: fm.front.longitudinal_peak_slip_ratio,
                        lateral_slide_mu_ratio: fm.front.lateral_slide_mu_ratio,
                        longitudinal_slide_mu_ratio: fm.front.longitudinal_slide_mu_ratio,
                        budget_blend_width: fm.front.budget_blend_width,
                        falloff_sharpness: fm.front.falloff_sharpness,
                        slip_loss_tau_s: fm.front.slip_loss_tau_s,
                        slip_recovery_tau_s: fm.front.slip_recovery_tau_s,
                        combined_lateral_peak_migration: fm.front.combined_lateral_peak_migration,
                        combined_longitudinal_peak_migration: fm.front
                            .combined_longitudinal_peak_migration,
                        min_relaxation_tau_s: fm.front.min_relaxation_tau_s,
                        relaxation_recovery_scale: fm.front.relaxation_recovery_scale,
                        contact_ramp_tau_s: fm.front.contact_ramp_tau_s,
                    })
                    .unwrap_or_default(),
                rear_tire_force: self
                    .tires
                    .force_model
                    .as_ref()
                    .map(|fm| TireForceProfile {
                        lateral_peak_slip_angle_rad: fm.rear.lateral_peak_slip_angle_rad,
                        longitudinal_peak_slip_ratio: fm.rear.longitudinal_peak_slip_ratio,
                        lateral_slide_mu_ratio: fm.rear.lateral_slide_mu_ratio,
                        longitudinal_slide_mu_ratio: fm.rear.longitudinal_slide_mu_ratio,
                        budget_blend_width: fm.rear.budget_blend_width,
                        falloff_sharpness: fm.rear.falloff_sharpness,
                        slip_loss_tau_s: fm.rear.slip_loss_tau_s,
                        slip_recovery_tau_s: fm.rear.slip_recovery_tau_s,
                        combined_lateral_peak_migration: fm.rear.combined_lateral_peak_migration,
                        combined_longitudinal_peak_migration: fm.rear
                            .combined_longitudinal_peak_migration,
                        min_relaxation_tau_s: fm.rear.min_relaxation_tau_s,
                        relaxation_recovery_scale: fm.rear.relaxation_recovery_scale,
                        contact_ramp_tau_s: fm.rear.contact_ramp_tau_s,
                    })
                    .unwrap_or_default(),
                pressure_mechanics: self
                    .tires
                    .pressure_mechanics
                    .as_ref()
                    .map(|pm| PressureMechanicsSensitivity {
                        compliance_sensitivity: pm.compliance_sensitivity,
                        response_sensitivity: pm.response_sensitivity,
                        rolling_resistance_sensitivity: pm.rolling_resistance_sensitivity,
                    })
                    .unwrap_or_default(),
                surface_friction,
                surface_stiffness,
                surface_rolling_resistance,
                // Surface longitudinal grip ratio (lateral_grip_assist is removed in v3 and
                // never reaches runtime config).
                surface_longitudinal_grip_ratio: build_surface_assist_map(&self.tires.surfaces, |e| {
                    e.longitudinal_grip_ratio
                }),
                surface_transition_tau_s: self.tires.surface_transition_tau_s.unwrap_or(0.0),
            },
            brakes: BrakesConfiguration {
                front_brake_bias: self.brakes.front_brake_bias,
                max_brake_torque: self.brakes.max_brake_torque,
                brake_thermal: self
                    .brakes
                    .thermal
                    .as_ref()
                    .map(JsonBrakeThermal::to_config)
                    .unwrap_or_default(),
                enable_abs: self.brakes.enable_abs,
                abs_pulse_time: self.brakes.abs_pulse_time,
                abs_spin_diff_threshold: self.brakes.abs_spin_diff_threshold,
                wheel_lock_blend_spin_rad_s: self.brakes.wheel_lock_blend_spin_rad_s.unwrap_or(0.0),
                wheel_lock_stick_tau_s: self.brakes.wheel_lock_stick_tau_s.unwrap_or(0.0),
            },
            aerodynamics: AerodynamicsConfiguration {
                coefficient_of_drag: self.aero.drag_coefficient,
                frontal_area: self.aero.frontal_area,
                aero_model: self.aero.model,
                air_density: self.aero.air_density,
                aero_lag_tau: self.aero.lag_tau_s,
                aero_blend_min_speed: self.aero.blend_min_speed_mps,
                aero_blend_full_speed: self.aero.blend_full_speed_mps,
            },

            // Driving aids
            aids: AidsConfig {
                traction_control_available: self.aids.traction_control_available,
                traction_control_default_enabled: self.aids.traction_control_default_enabled,
                traction_control_player_selectable: self.aids.traction_control_player_selectable,
                traction_control_slip_threshold: self.aids.traction_control_slip_threshold,
                traction_control_cut_gain: self.aids.traction_control_cut_gain,
                traction_control_actuator: self.aids.traction_control_actuator.clone(),
                traction_control_gear_authority: self.aids.traction_control_gear_authority,
                traction_control_gear_slip_target: self.aids.traction_control_gear_slip_target,
                traction_control_gear_max_cut: self.aids.traction_control_gear_max_cut,
                traction_control_attack_rate: self.aids.traction_control_attack_rate,
                traction_control_release_rate: self.aids.traction_control_release_rate,
                abs_available: self.aids.abs_available,
                abs_default_enabled: self.aids.abs_default_enabled,
                abs_player_selectable: self.aids.abs_player_selectable,
                stability_available: self.aids.stability_available,
                stability_default_enabled: self.aids.stability_default_enabled,
                stability_player_selectable: self.aids.stability_player_selectable,
                stability_yaw_engage_angle_rad: self.aids.stability_yaw_engage_angle_rad,
                stability_yaw_strength: self.aids.stability_yaw_strength,
                stability_reference_lateral_accel_m_s2: self
                    .aids
                    .stability_reference_lateral_accel_m_s2,
                stability_grounded_multiplier: self.aids.stability_grounded_multiplier,
                stability_upright_spring: self.aids.stability_upright_spring,
                stability_upright_damping: self.aids.stability_upright_damping,
                steering_slip_assist_default_enabled: self
                    .aids
                    .steering_slip_assist_default_enabled,
                countersteer_default_enabled: self.aids.countersteer_default_enabled,
                steering_assist_player_selectable: self.aids.steering_assist_player_selectable,
                auto_clutch_default_enabled: self.aids.auto_clutch_default_enabled,
                auto_clutch_player_selectable: self.aids.auto_clutch_player_selectable,
                launch_control_available: self.aids.launch_control_available,
                launch_control_default_enabled: self.aids.launch_control_default_enabled,
                launch_control_player_selectable: self.aids.launch_control_player_selectable,
                launch_control_target_rpm: self.aids.launch_control_target_rpm,
                launch_control_throttle_limit: self.aids.launch_control_throttle_limit,
                launch_control_clutch_release_rate_per_s: self
                    .aids
                    .launch_control_clutch_release_rate_per_s,
                brake_assist_available: self.aids.brake_assist_available,
                brake_assist_default_enabled: self.aids.brake_assist_default_enabled,
                brake_assist_player_selectable: self.aids.brake_assist_player_selectable,
                brake_assist_force_multiplier: self.aids.brake_assist_force_multiplier,
                handbrake_rear_only: self.aids.handbrake_rear_only,
                handbrake_abs_interlock: self.aids.handbrake_abs_interlock,
                handbrake_clutch_coupling: self.aids.handbrake_clutch_coupling,
                input_smoothing_enabled: self.aids.input_smoothing_enabled,
                input_smoothing_steering_rate: self.aids.input_smoothing_steering_rate,
                input_smoothing_throttle_rate: self.aids.input_smoothing_throttle_rate,
                input_smoothing_brake_rate: self.aids.input_smoothing_brake_rate,
            },
            audio: self.audio.clone(),
            coupled_world: self.coupled_world.clone(),
            underfloor_contact: self.underfloor_contact,
        }
    }

    pub(crate) fn from_config(cfg: &VehicleConfig) -> Self {
        let mut surfaces = HashMap::new();
        for (st, friction) in &cfg.tires.surface_friction {
            let name = surface_type_name(*st);
            let stiffness = cfg.tires.surface_stiffness.get(st).copied().unwrap_or(1.0);
            let rolling = cfg
                .tires
                .surface_rolling_resistance
                .get(st)
                .copied()
                .unwrap_or(1.0);
            let long = cfg
                .tires
                .surface_longitudinal_grip_ratio
                .get(st)
                .copied()
                .unwrap_or(0.5);
            surfaces.insert(
                name,
                JsonSurfaceEntry {
                    friction: *friction,
                    stiffness,
                    rolling_resistance: rolling,
                    // DEPRECATED (TIRE-500): emitted as a neutral value for legacy v2
                    // compatibility only; v3 runtime never reads it.
                    lateral_grip_assist: 0.0,
                    longitudinal_grip_ratio: long,
                },
            );
        }
        Self {
            physical_mass_inventory: cfg.chassis.physical_mass_inventory.clone(),
            coupled_world: cfg.coupled_world.clone(),
            underfloor_contact: cfg.underfloor_contact,
            schema_version: cfg.schema_version,
            vehicle_id: "f1_94".to_string(),
            profile_name: cfg.chassis.vehicle_name.clone(),
            units: "SI".to_string(),
            audio: cfg.audio.clone(),
            chassis: JsonChassis {
                vehicle_name: cfg.chassis.vehicle_name.clone(),
                vehicle_mass: cfg.chassis.vehicle_mass,
                vehicle_mass_excludes_wheel_assemblies: cfg.chassis.vehicle_mass_excludes_wheel_assemblies,
                front_weight_distribution: cfg.chassis.front_weight_distribution,
                center_of_gravity_height_offset: cfg.chassis.center_of_gravity_height_offset,
                inertia_multipliers: JsonVec3 {
                    x: cfg.chassis.inertia_multipliers.x,
                    y: cfg.chassis.inertia_multipliers.y,
                    z: cfg.chassis.inertia_multipliers.z,
                },
            },
            fuel: JsonFuel {
                capacity_kg: cfg.fuel.capacity_kg,
                initial_kg: cfg.fuel.initial_kg,
                tank_position_local_m: JsonVec3 {
                    x: cfg.fuel.tank_position_local_m.x,
                    y: cfg.fuel.tank_position_local_m.y,
                    z: cfg.fuel.tank_position_local_m.z,
                },
                brake_specific_consumption_kg_per_kwh: cfg.fuel.brake_specific_consumption_kg_per_kwh,
                idle_consumption_kg_per_hour: cfg.fuel.idle_consumption_kg_per_hour,
                estimated_lap_consumption_kg: cfg.fuel.estimated_lap_consumption_kg,
                reference_lap_time_s: cfg.fuel.reference_lap_time_s,
            },
            geometry: JsonGeometry {
                wheelbase: cfg.chassis.wheelbase,
                front_track: cfg.chassis.front_track,
                rear_track: cfg.chassis.rear_track,
                wheel_hubs: None,
            },
            steering: JsonSteering {
                max_steering_angle: cfg.steering.max_steering_angle,
                front_steering_ratio: cfg.steering.front_steering_ratio,
                rear_steering_ratio: cfg.steering.rear_steering_ratio,
                steering_speed: cfg.steering.steering_speed,
                steering_speed_decay: cfg.steering.steering_speed_decay,
                steering_slip_assist: cfg.steering.steering_slip_assist,
                countersteer_assist: cfg.steering.countersteer_assist,
                steering_exponent: cfg.steering.steering_exponent,
                ackermann: cfg.steering.ackermann,
            },
            powertrain: JsonPowertrain {
                max_torque: cfg.powertrain.max_torque,
                max_rpm: cfg.powertrain.max_rpm,
                idle_rpm: cfg.powertrain.idle_rpm,
                motor_moment: cfg.powertrain.motor_moment,
                torque_curve: cfg.powertrain.torque_curve.clone(),
                gear_ratios: cfg.powertrain.gear_ratios.clone(),
                final_drive: cfg.powertrain.final_drive,
                reverse_ratio: cfg.powertrain.reverse_ratio,
                shift_time: cfg.powertrain.shift_time,
                automatic_transmission: cfg.powertrain.automatic_transmission,
                downshift_rev_matching_enabled: cfg.powertrain.downshift_rev_matching_enabled,
                downshift_maximum_target_revolutions_per_minute_fraction: cfg.powertrain.downshift_maximum_target_revolutions_per_minute_fraction,
                downshift_blip_torque_fraction: cfg.powertrain.downshift_blip_torque_fraction,
                downshift_minimum_interval_seconds: cfg.powertrain.downshift_minimum_interval_seconds,
                downshift_clutch_reengagement_seconds: cfg.powertrain.downshift_clutch_reengagement_seconds,
                thermal: Some(cfg.powertrain.powertrain_thermal.clone()),
                front_torque_split: cfg.powertrain.front_torque_split,
                gear_inertia: 0.02,
                max_clutch_torque_ratio: cfg.powertrain.max_clutch_torque_ratio,
                clutch_out_rpm_offset: cfg.powertrain.clutch_out_rpm_offset,
                torque_attack_rate_nm_s: cfg.powertrain.torque_attack_rate_nm_s,
                idle_disengagement_hysteresis_rpm: cfg.powertrain.idle_disengagement_hysteresis_rpm,
                variable_drag_ratio: cfg.powertrain.variable_drag_ratio,
                constant_brake_ratio: cfg.powertrain.constant_brake_ratio,
                handbrake_torque_fraction: cfg.powertrain.handbrake_torque_fraction,
                automatic_shift: None,
                differential: JsonDifferential {
                    preload_nm: cfg.powertrain.diff_preload,
                    power_ramp_angle_deg: cfg.powertrain.diff_power_ramp_angle_deg,
                    coast_ramp_angle_deg: cfg.powertrain.diff_coast_ramp_angle_deg,
                    clutches: cfg.powertrain.diff_clutches,
                    clutch_friction_coefficient: cfg.powertrain.diff_clutch_friction_coeff,
                    slip_transition_threshold_rad_s: cfg.powertrain.diff_slip_transition_threshold_rad_s,
                },
            },
            suspension: JsonSuspension {
                front: JsonSuspensionAxle {
                    spring_length: cfg.suspension.front_spring_length,
                    resting_ratio: cfg.suspension.front_resting_ratio,
                    damping_ratio: cfg.suspension.front_damping_ratio,
                    bump_damp_multiplier: cfg.suspension.front_bump_damp_multiplier,
                    rebound_damp_multiplier: cfg.suspension.front_rebound_damp_multiplier,
                    arb_ratio: cfg.suspension.front_arb_ratio,
                    toe: cfg.suspension.front_toe,
                    camber: cfg.suspension.front_camber,
                    bump_stop_multiplier: cfg.suspension.front_bump_stop_multiplier,
                    camber_gain_rad_per_m: (cfg.schema_version >= 3)
                        .then(|| cfg.suspension.front_camber_gain_rad_per_m),
                    toe_gain_rad_per_m: (cfg.schema_version >= 3).then(|| cfg.suspension.front_toe_gain_rad_per_m),
                },
                rear: JsonSuspensionAxle {
                    spring_length: cfg.suspension.rear_spring_length,
                    resting_ratio: cfg.suspension.rear_resting_ratio,
                    damping_ratio: cfg.suspension.rear_damping_ratio,
                    bump_damp_multiplier: cfg.suspension.rear_bump_damp_multiplier,
                    rebound_damp_multiplier: cfg.suspension.rear_rebound_damp_multiplier,
                    arb_ratio: cfg.suspension.rear_arb_ratio,
                    toe: cfg.suspension.rear_toe,
                    camber: cfg.suspension.rear_camber,
                    bump_stop_multiplier: cfg.suspension.rear_bump_stop_multiplier,
                    camber_gain_rad_per_m: (cfg.schema_version >= 3)
                        .then(|| cfg.suspension.rear_camber_gain_rad_per_m),
                    toe_gain_rad_per_m: (cfg.schema_version >= 3).then(|| cfg.suspension.rear_toe_gain_rad_per_m),
                },
                tri_ray_spacing_ratio: cfg.suspension.tri_ray_spacing_ratio,
                // Visual-only geometry is not part of the force config; it is
                // never re-emitted by to_json_value.
                geometry: None,
                model: cfg.suspension.suspension_model.as_str_name().to_string(),
                model_version: cfg.suspension.suspension_model_version,
                geometry_physical: cfg
                    .suspension.geometric_suspension
                    .as_ref()
                    .map(geometric_to_json_value),
            },
            tires: JsonTires {
                front: JsonTireAxle {
                    radius: cfg.tires.front_tire_radius,
                    width: cfg.tires.front_tire_width,
                    wheel_mass: cfg.tires.front_wheel_mass,
                    contact_patch: Some(cfg.tires.front_contact_patch),
                    // DEPRECATED (TIRE-600): legacy key parsed for compatibility only;
                    // runtime config no longer carries a braking grip multiplier.
                    braking_grip_multiplier: None,
                    airborne_spin_decay_torque: Some(cfg.tires.front_airborne_decay),
                },
                rear: JsonTireAxle {
                    radius: cfg.tires.rear_tire_radius,
                    width: cfg.tires.rear_tire_width,
                    wheel_mass: cfg.tires.rear_wheel_mass,
                    contact_patch: Some(cfg.tires.rear_contact_patch),
                    // DEPRECATED (TIRE-600): see front axle note.
                    braking_grip_multiplier: None,
                    airborne_spin_decay_torque: Some(cfg.tires.rear_airborne_decay),
                },
                contact_patch: cfg.tires.contact_patch,
                // DEPRECATED (TIRE-600): emitted neutral (1.0) for legacy v2 compatibility
                // only; schema-v3 payloads must not carry deprecated keys (CAL-1300).
                braking_grip_multiplier: (cfg.schema_version < 3).then_some(1.0),
                airborne_spin_decay_torque: cfg.tires.front_airborne_decay,
                surfaces,
                surface_transition_tau_s: (cfg.schema_version >= 3)
                    .then_some(cfg.tires.surface_transition_tau_s),
                pressure: Some(JsonTirePressure {
                    units: Some("kPa_gauge".to_string()),
                    cold_kpa_gauge: Some(wheel_name_map(cfg.tires.tire_pressure.cold_kpa_gauge)),
                    reference_hot_kpa_gauge: Some(wheel_name_map(
                        cfg.tires.tire_pressure.reference_hot_kpa_gauge,
                    )),
                    reference_temperature_c: Some(cfg.tires.tire_pressure.reference_temperature_c),
                    atmospheric_pressure_kpa: Some(cfg.tires.tire_pressure.atmospheric_pressure_kpa),
                }),
                thermal: Some(JsonTireThermal::from_config(&cfg.tires.tire_thermal)),
                wear: (cfg.schema_version >= 3).then(|| JsonTireWear::from_config(&cfg.tires.tire_wear)),
                force_model: (cfg.schema_version >= 3).then(|| JsonTireForceModel {
                    front: JsonTireForceProfile {
                        lateral_peak_slip_angle_rad: cfg.tires.front_tire_force.lateral_peak_slip_angle_rad,
                        longitudinal_peak_slip_ratio: cfg.tires.front_tire_force.longitudinal_peak_slip_ratio,
                        lateral_slide_mu_ratio: cfg.tires.front_tire_force.lateral_slide_mu_ratio,
                        longitudinal_slide_mu_ratio: cfg.tires.front_tire_force.longitudinal_slide_mu_ratio,
                        budget_blend_width: cfg.tires.front_tire_force.budget_blend_width,
                        falloff_sharpness: cfg.tires.front_tire_force.falloff_sharpness,
                        slip_loss_tau_s: cfg.tires.front_tire_force.slip_loss_tau_s,
                        slip_recovery_tau_s: cfg.tires.front_tire_force.slip_recovery_tau_s,
                        combined_lateral_peak_migration: cfg
                            .tires
                            .front_tire_force
                            .combined_lateral_peak_migration,
                        combined_longitudinal_peak_migration: cfg
                            .tires
                            .front_tire_force
                            .combined_longitudinal_peak_migration,
                        min_relaxation_tau_s: cfg.tires.front_tire_force.min_relaxation_tau_s,
                        relaxation_recovery_scale: cfg.tires.front_tire_force.relaxation_recovery_scale,
                        contact_ramp_tau_s: cfg.tires.front_tire_force.contact_ramp_tau_s,
                    },
                    rear: JsonTireForceProfile {
                        lateral_peak_slip_angle_rad: cfg.tires.rear_tire_force.lateral_peak_slip_angle_rad,
                        longitudinal_peak_slip_ratio: cfg.tires.rear_tire_force.longitudinal_peak_slip_ratio,
                        lateral_slide_mu_ratio: cfg.tires.rear_tire_force.lateral_slide_mu_ratio,
                        longitudinal_slide_mu_ratio: cfg.tires.rear_tire_force.longitudinal_slide_mu_ratio,
                        budget_blend_width: cfg.tires.rear_tire_force.budget_blend_width,
                        falloff_sharpness: cfg.tires.rear_tire_force.falloff_sharpness,
                        slip_loss_tau_s: cfg.tires.rear_tire_force.slip_loss_tau_s,
                        slip_recovery_tau_s: cfg.tires.rear_tire_force.slip_recovery_tau_s,
                        combined_lateral_peak_migration: cfg
                            .tires
                            .rear_tire_force
                            .combined_lateral_peak_migration,
                        combined_longitudinal_peak_migration: cfg
                            .tires
                            .rear_tire_force
                            .combined_longitudinal_peak_migration,
                        min_relaxation_tau_s: cfg.tires.rear_tire_force.min_relaxation_tau_s,
                        relaxation_recovery_scale: cfg.tires.rear_tire_force.relaxation_recovery_scale,
                        contact_ramp_tau_s: cfg.tires.rear_tire_force.contact_ramp_tau_s,
                    },
                }),
                pressure_mechanics: (cfg.schema_version >= 3).then(|| JsonPressureMechanics {
                    compliance_sensitivity: cfg.tires.pressure_mechanics.compliance_sensitivity,
                    response_sensitivity: cfg.tires.pressure_mechanics.response_sensitivity,
                    rolling_resistance_sensitivity: cfg.tires.pressure_mechanics.rolling_resistance_sensitivity,
                }),
            },
            brakes: JsonBrakes {
                front_brake_bias: cfg.brakes.front_brake_bias,
                max_brake_torque: cfg.brakes.max_brake_torque,
                enable_abs: cfg.brakes.enable_abs,
                abs_pulse_time: cfg.brakes.abs_pulse_time,
                abs_spin_diff_threshold: cfg.brakes.abs_spin_diff_threshold,
                wheel_lock_blend_spin_rad_s: (cfg.schema_version >= 3)
                    .then_some(cfg.brakes.wheel_lock_blend_spin_rad_s),
                wheel_lock_stick_tau_s: (cfg.schema_version >= 3)
                    .then_some(cfg.brakes.wheel_lock_stick_tau_s),
                thermal: Some(JsonBrakeThermal::from_config(&cfg.brakes.brake_thermal)),
            },
            aero: JsonAero {
                model: cfg.aerodynamics.aero_model.clone(),
                drag_coefficient: cfg.aerodynamics.coefficient_of_drag,
                frontal_area: cfg.aerodynamics.frontal_area,
                air_density: cfg.aerodynamics.air_density,
                lag_tau_s: cfg.aerodynamics.aero_lag_tau,
                blend_min_speed_mps: cfg.aerodynamics.aero_blend_min_speed,
                blend_full_speed_mps: cfg.aerodynamics.aero_blend_full_speed,
            },
            coordinate_contract: Some({
                let mut m = HashMap::new();
                m.insert("forward".to_string(), "-Z".to_string());
                m.insert("right".to_string(), "+X".to_string());
                m.insert("up".to_string(), "+Y".to_string());
                m
            }),
            aids: JsonAids {
                traction_control_available: cfg.aids.traction_control_available,
                traction_control_default_enabled: cfg.aids.traction_control_default_enabled,
                traction_control_player_selectable: cfg.aids.traction_control_player_selectable,
                traction_control_slip_threshold: cfg.aids.traction_control_slip_threshold,
                traction_control_cut_gain: cfg.aids.traction_control_cut_gain,
                traction_control_actuator: cfg.aids.traction_control_actuator.clone(),
                traction_control_gear_authority: cfg.aids.traction_control_gear_authority.clone(),
                traction_control_gear_slip_target: cfg.aids.traction_control_gear_slip_target.clone(),
                traction_control_gear_max_cut: cfg.aids.traction_control_gear_max_cut.clone(),
                traction_control_attack_rate: cfg.aids.traction_control_attack_rate,
                traction_control_release_rate: cfg.aids.traction_control_release_rate,
                abs_available: cfg.aids.abs_available,
                abs_default_enabled: cfg.aids.abs_default_enabled,
                abs_player_selectable: cfg.aids.abs_player_selectable,
                stability_available: cfg.aids.stability_available,
                stability_default_enabled: cfg.aids.stability_default_enabled,
                stability_player_selectable: cfg.aids.stability_player_selectable,
                stability_yaw_engage_angle_rad: cfg.aids.stability_yaw_engage_angle_rad,
                stability_yaw_strength: cfg.aids.stability_yaw_strength,
                stability_reference_lateral_accel_m_s2: cfg
                    .aids
                    .stability_reference_lateral_accel_m_s2,
                stability_grounded_multiplier: cfg.aids.stability_grounded_multiplier,
                stability_upright_spring: cfg.aids.stability_upright_spring,
                stability_upright_damping: cfg.aids.stability_upright_damping,
                steering_slip_assist_default_enabled: cfg.aids.steering_slip_assist_default_enabled,
                countersteer_default_enabled: cfg.aids.countersteer_default_enabled,
                steering_assist_player_selectable: cfg.aids.steering_assist_player_selectable,
                auto_clutch_default_enabled: cfg.aids.auto_clutch_default_enabled,
                auto_clutch_player_selectable: cfg.aids.auto_clutch_player_selectable,
                launch_control_available: cfg.aids.launch_control_available,
                launch_control_default_enabled: cfg.aids.launch_control_default_enabled,
                launch_control_player_selectable: cfg.aids.launch_control_player_selectable,
                launch_control_target_rpm: cfg.aids.launch_control_target_rpm,
                launch_control_throttle_limit: cfg.aids.launch_control_throttle_limit,
                launch_control_clutch_release_rate_per_s: cfg
                    .aids
                    .launch_control_clutch_release_rate_per_s,
                brake_assist_available: cfg.aids.brake_assist_available,
                brake_assist_default_enabled: cfg.aids.brake_assist_default_enabled,
                brake_assist_player_selectable: cfg.aids.brake_assist_player_selectable,
                brake_assist_force_multiplier: cfg.aids.brake_assist_force_multiplier,
                handbrake_rear_only: cfg.aids.handbrake_rear_only,
                handbrake_abs_interlock: cfg.aids.handbrake_abs_interlock,
                handbrake_clutch_coupling: cfg.aids.handbrake_clutch_coupling,
                input_smoothing_enabled: cfg.aids.input_smoothing_enabled,
                input_smoothing_steering_rate: cfg.aids.input_smoothing_steering_rate,
                input_smoothing_throttle_rate: cfg.aids.input_smoothing_throttle_rate,
                input_smoothing_brake_rate: cfg.aids.input_smoothing_brake_rate,
            },
        }
    }
}

#[cfg(test)]
mod tests;
