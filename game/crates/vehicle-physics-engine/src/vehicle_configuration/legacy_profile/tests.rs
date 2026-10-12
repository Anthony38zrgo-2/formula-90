use super::*;


#[test]
fn surface_type_name_covers_all_variants() {
    for st in [
        SurfaceType::Road,
        SurfaceType::Curb,
        SurfaceType::Dirt,
        SurfaceType::Grass,
        SurfaceType::Gravel,
        SurfaceType::Sand,
        SurfaceType::Wall,
        SurfaceType::Metal,
    ] {
        let name = surface_type_name(st);
        assert!(!name.is_empty());
        assert_eq!(name, format!("{st:?}"));
    }
}

#[test]
fn round_trip_preserves_all_fields() {
    let original = VehicleConfig::f1_94_canonical();
    let json_val = original.to_json_value();
    let json_str = serde_json::to_string_pretty(&json_val).unwrap();
    let loaded = VehicleConfig::from_json_str(&json_str).unwrap();
    assert_eq!(loaded.chassis.vehicle_name, original.chassis.vehicle_name);
    assert_eq!(loaded.chassis.vehicle_mass, original.chassis.vehicle_mass);
    assert_eq!(loaded.chassis.wheelbase, original.chassis.wheelbase);
    assert_eq!(loaded.powertrain.diff_preload, original.powertrain.diff_preload);
    assert_eq!(
        loaded.powertrain.diff_clutch_friction_coeff,
        original.powertrain.diff_clutch_friction_coeff
    );
    assert_eq!(loaded.powertrain.gear_ratios, original.powertrain.gear_ratios);
    assert_eq!(loaded.powertrain.torque_curve, original.powertrain.torque_curve);
    assert_eq!(
        loaded.aerodynamics.coefficient_of_drag,
        original.aerodynamics.coefficient_of_drag
    );
    assert!(
        (loaded.chassis.front_weight_distribution - original.chassis.front_weight_distribution).abs() < 1e-10
    );
    assert_eq!(
        loaded.tires.surface_friction.len(),
        original.tires.surface_friction.len()
    );
}

#[test]
fn golden_json_matches_canonical() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    let loaded = VehicleConfig::from_json_path(&path).unwrap();
    let canonical = VehicleConfig::f1_2030_geometric_canonical();
    assert_eq!(loaded.chassis.vehicle_mass, canonical.chassis.vehicle_mass);
    assert_eq!(loaded.chassis.wheelbase, canonical.chassis.wheelbase);
    assert_eq!(loaded.powertrain.diff_preload, canonical.powertrain.diff_preload);
    assert_eq!(loaded.powertrain.gear_ratios, canonical.powertrain.gear_ratios);
}

#[test]
fn rejects_invalid_schema_version() {
    let json = r#"{"schema_version": 99, "vehicle_id": "test"}"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}

#[test]
fn rejects_non_positive_mass() {
    let json = r#"{"schema_version": 1, "chassis": {"vehicle_mass": -1.0}}"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}

#[test]
fn minimal_json_uses_defaults() {
    let json = r#"{"schema_version": 1}"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert_eq!(cfg.chassis.vehicle_mass, 575.0);
    assert_eq!(cfg.powertrain.diff_preload, 170.0);
}

#[test]
fn schema_version_2_aids_round_trip() {
    let json = r#"{
        "schema_version": 2,
        "aids": {
            "traction_control_available": true,
            "traction_control_default_enabled": true,
            "traction_control_slip_threshold": 0.12,
            "abs_default_enabled": true,
            "stability_available": true,
            "stability_yaw_strength": 7.5,
            "brake_assist_force_multiplier": 1.4
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!(cfg.aids.traction_control_available);
    assert!(cfg.aids.traction_control_default_enabled);
    assert!((cfg.aids.traction_control_slip_threshold - 0.12).abs() < 1e-9);
    assert!(cfg.aids.abs_default_enabled);
    assert!(cfg.aids.stability_available);
    assert!((cfg.aids.stability_yaw_strength - 7.5).abs() < 1e-9);
    assert!((cfg.aids.brake_assist_force_multiplier - 1.4).abs() < 1e-9);
}

#[test]
fn deny_unknown_fields_rejects_typos() {
    let json_aids = r#"{"schema_version": 2, "aids": {"traction_controll": true}}"#;
    assert!(VehicleConfig::from_json_str(json_aids).is_err());

    let json_chassis = r#"{"schema_version": 2, "chassis": {"veh_mass": 575.0}}"#;
    assert!(VehicleConfig::from_json_str(json_chassis).is_err());

    let json_powertrain = r#"{"schema_version": 2, "powertrain": {"maximum_torque": 450.0}}"#;
    assert!(VehicleConfig::from_json_str(json_powertrain).is_err());

    let json_suspension =
        r#"{"schema_version": 2, "suspension": {"front": {"spring_len": 0.25}}}"#;
    assert!(VehicleConfig::from_json_str(json_suspension).is_err());
}

#[test]
fn suspension_geometry_parser_only_accepted_and_ignored() {
    let json = r#"{
        "schema_version": 3,
        "suspension": {
            "front": {"spring_length": 0.3, "resting_ratio": 0.15},
            "geometry": {
                "FL": {
                    "hub_center": [-0.753, 0.0, -1.475],
                    "lower_wishbone": {
                        "inner_front": [-0.50, 0.06, -1.35],
                        "inner_rear": [-0.52, 0.06, -1.70],
                        "outer": [-0.68, -0.04, -1.475]
                    }
                }
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    // Geometry is visual-only: it must not disturb any force-facing config value.
    assert!((cfg.suspension.front_spring_length - 0.3).abs() < 1e-9);
    assert!((cfg.suspension.front_resting_ratio - 0.15).abs() < 1e-9);
    // And it must not leak into the serialized force config round-trip.
    let re_emitted = cfg.to_json_value();
    assert!(re_emitted.get("suspension").is_some());
}

#[test]
fn golden_json_schema_2_aids_present() {
    let path = Path::new("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json");
    if !path.exists() {
        return;
    }
    let loaded = VehicleConfig::from_json_path(path).unwrap();
    assert!(loaded.aids.traction_control_available);
    assert!(loaded.aids.abs_available);
    assert_eq!(loaded.schema_version, 2);
}

#[test]
fn fidelity_fields_consumed_from_json() {
    let path = Path::new("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json");
    if !path.exists() {
        return;
    }
    let loaded = VehicleConfig::from_json_path(path).unwrap();
    assert!((loaded.powertrain.max_clutch_torque_ratio - 1.1).abs() < 1e-9);
    assert!((loaded.powertrain.clutch_out_rpm_offset - 1100.0).abs() < 1e-9);
    assert!((loaded.powertrain.handbrake_torque_fraction - 0.0).abs() < 1e-9);
    assert!((loaded.powertrain.diff_slip_transition_threshold_rad_s - 1.0).abs() < 1e-9);
    assert!((loaded.powertrain.variable_drag_ratio - 0.10).abs() < 1e-9);
}

#[test]
fn tire_pressure_thermal_json_round_trip() {
    let json = r#"{
        "schema_version": 2,
        "tires": {
            "pressure": {
                "units": "kPa_gauge",
                "cold_kpa_gauge": {"FL": 111.0, "FR": 112.0, "RL": 106.0, "RR": 107.0},
                "reference_hot_kpa_gauge": {"FL": 146.0, "FR": 146.0, "RL": 141.0, "RR": 141.0},
                "reference_temperature_c": 26.0,
                "atmospheric_pressure_kpa": 102.0
            },
            "thermal": {
                "optimal_tread_temperature_c": 96.0,
                "overheat_tread_temperature_c": 122.0,
                "pressure_stiffness_exponent": 0.75
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[0] - 111.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[1] - 112.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[2] - 106.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[3] - 107.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.reference_temperature_c - 26.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.atmospheric_pressure_kpa - 102.0).abs() < 1e-9);
    assert!((cfg.tires.tire_thermal.front.optimal_tread_temperature_c - 96.0).abs() < 1e-9);
    assert!((cfg.tires.tire_thermal.front.overheat_tread_temperature_c - 122.0).abs() < 1e-9);
    // THERM-700: the legacy pressure_stiffness_exponent key is accepted purely
    // for schema-v2 compatibility and is ignored; only the three
    // pressure_mechanics sensitivities apply (defaults here).
    assert_eq!(cfg.tires.pressure_mechanics, PressureMechanicsSensitivity::default());
    // Fields not present keep module defaults.
    assert!((cfg.tires.tire_thermal.front.initial_temperature_c - 25.0).abs() < 1e-9);
    // Round trip through JSON drops the deprecated exponent key (never emitted
    // for new profiles) and preserves the parsed values.
    let json_val = cfg.to_json_value();
    assert!(
        !json_val.to_string().contains("pressure_stiffness_exponent"),
        "v2 emission must not carry the deprecated exponent key"
    );
    let loaded =
        VehicleConfig::from_json_str(&serde_json::to_string(&json_val).unwrap()).unwrap();
    assert!((loaded.tires.tire_pressure.cold_kpa_gauge[0] - 111.0).abs() < 1e-9);
    assert_eq!(loaded.tires.pressure_mechanics, PressureMechanicsSensitivity::default());
}

#[test]
fn tire_thermal_front_and_rear_profiles_are_independent() {
    let json = r#"{
        "schema_version": 2,
        "tires": {
            "thermal": {
                "front": {
                    "tread_zone_heat_capacity_j_k": 3000.0,
                    "slip_heat_efficiency": 0.95
                },
                "rear": {
                    "tread_zone_heat_capacity_j_k": 6000.0,
                    "slip_heat_efficiency": 0.75
                }
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert_eq!(cfg.tires.tire_thermal.front.tread_zone_heat_capacity_j_k, 3000.0);
    assert_eq!(cfg.tires.tire_thermal.rear.tread_zone_heat_capacity_j_k, 6000.0);
    assert_eq!(cfg.tires.tire_thermal.front.slip_heat_efficiency, 0.95);
    assert_eq!(cfg.tires.tire_thermal.rear.slip_heat_efficiency, 0.75);

    let round_trip =
        VehicleConfig::from_json_str(&serde_json::to_string(&cfg.to_json_value()).unwrap())
            .unwrap();
    assert_eq!(
        round_trip.tires.tire_thermal.front.tread_zone_heat_capacity_j_k,
        3000.0
    );
    assert_eq!(
        round_trip.tires.tire_thermal.rear.tread_zone_heat_capacity_j_k,
        6000.0
    );
}

#[test]
fn fl_only_pressure_change_does_not_alter_other_wheels() {
    let json = r#"{
        "schema_version": 2,
        "tires": {
            "pressure": {
                "units": "kPa_gauge",
                "cold_kpa_gauge": {"FL": 120.0},
                "reference_hot_kpa_gauge": {"FL": 150.0}
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[0] - 120.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[1] - 110.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[2] - 105.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.cold_kpa_gauge[3] - 105.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.reference_hot_kpa_gauge[0] - 150.0).abs() < 1e-9);
    assert!((cfg.tires.tire_pressure.reference_hot_kpa_gauge[1] - 145.0).abs() < 1e-9);
}

#[test]
fn missing_thermal_section_uses_stable_defaults() {
    let json = r#"{"schema_version": 2}"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert_eq!(
        cfg.tires.tire_pressure.cold_kpa_gauge,
        [110.0, 110.0, 105.0, 105.0]
    );
    assert!((cfg.tires.tire_thermal.front.optimal_tread_temperature_c - 95.0).abs() < 1e-9);
    assert_eq!(cfg.tires.pressure_mechanics, PressureMechanicsSensitivity::default());
}

#[test]
fn deny_unknown_fields_rejects_tire_pressure_typo() {
    let json_typo = r#"{
        "schema_version": 2,
        "tires": {
            "pressure": {
                "cold_kpa_gaugee": {"FL": 110.0}
            }
        }
    }"#;
    assert!(VehicleConfig::from_json_str(json_typo).is_err());

    let json_thermal_typo = r#"{
        "schema_version": 2,
        "tires": {
            "thermal": {
                "optimal_tread_temperaturec": 95.0
            }
        }
    }"#;
    assert!(VehicleConfig::from_json_str(json_thermal_typo).is_err());
}

#[test]
fn tire_pressure_validation_rejects_invalid_values() {
    let json_bad = r#"{
        "schema_version": 2,
        "tires": {
            "pressure": {
                "cold_kpa_gauge": {"FL": -5.0}
            }
        }
    }"#;
    assert!(VehicleConfig::from_json_str(json_bad).is_err());
}

#[test]
fn canonical_uses_f1_94_pressure_and_thermal_defaults() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(
        cfg.tires.tire_pressure.cold_kpa_gauge,
        [110.0, 110.0, 105.0, 105.0]
    );
    assert_eq!(
        cfg.tires.tire_pressure.reference_hot_kpa_gauge,
        [145.0, 145.0, 140.0, 140.0]
    );
    assert!((cfg.tires.tire_pressure.reference_temperature_c - 25.0).abs() < 1e-9);
    assert!((cfg.tires.tire_thermal.front.initial_temperature_c - 25.0).abs() < 1e-9);
}

#[test]
fn brake_thermal_partial_json_keeps_omitted_defaults() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {
            "thermal": {
                "front_duct": {"opening": 0.72}
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!((cfg.brakes.brake_thermal.front_duct.opening - 0.72).abs() < 1e-9);
    assert!((cfg.brakes.brake_thermal.rear_duct.opening - 0.40).abs() < 1e-9);
    assert!((cfg.brakes.brake_thermal.front.rotor_to_rim_w_k - 26.0).abs() < 1e-9);
    assert!((cfg.brakes.brake_thermal.front.rotor_mass_kg - 1.35).abs() < 1e-9);
}

#[test]
fn brake_thermal_v2_lumping_maps_into_compact_v3() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {
            "front": {
                "disc_to_hub_w_k": 38.0,
                "hub_to_rim_w_k": 170.0,
                "disc_to_rim_radiation_w_k": 9.0
            },
            "front_duct": {
                "disc_cooling_weight": 0.6,
                "caliper_cooling_weight": 0.3
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let series = 1.0 / (1.0 / 38.0 + 1.0 / 170.0);
    assert!((cfg.brakes.brake_thermal.front.rotor_to_rim_w_k - (series + 9.0)).abs() < 1e-9);
    let total = 0.6 + 0.3 + 0.095 + 0.127;
    assert!(
        (cfg.brakes.brake_thermal.front_duct.rotor_cooling_fraction - (0.6 + 0.3 + 0.095) / total)
            .abs()
            < 1e-9
    );
    assert!((cfg.brakes.brake_thermal.front.rotor_mass_kg - 1.35).abs() < 1e-9);
}

#[test]
fn brake_thermal_v2_deprecated_fields_are_accepted_and_ignored() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {
            "direct_caliper_heat_fraction": 0.04,
            "front": {
                "surface_bulk_response_scale": 40.0,
                "disc_heat_capacity_j_k": 2400.0,
                "disc_surface_heat_capacity_j_k": 300.0,
                "disc_bulk_heat_capacity_j_k": 2100.0,
                "disc_surface_to_bulk_w_k": 450.0,
                "disc_surface_base_air_w_k": 4.0,
                "caliper_heat_capacity_j_k": 300.0,
                "hub_heat_capacity_j_k": 200.0,
                "disc_to_caliper_w_k": 150.0,
                "disc_base_air_w_k": 2.0,
                "caliper_base_air_w_k": 8.0,
                "hub_base_air_w_k": 6.0,
                "disc_flow_cooling_gain_w_k": 0.5,
                "caliper_flow_cooling_gain_w_k": 0.2,
                "hub_flow_cooling_gain_w_k": 0.1,
                "rim_flow_cooling_gain_w_k": 0.4
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!((cfg.brakes.brake_thermal.front.rotor_mass_kg - 1.35).abs() < 1e-9);
    assert!((cfg.brakes.brake_thermal.front.rotor_to_rim_w_k - 26.0).abs() < 1e-9);
}

#[test]
fn brake_thermal_nested_typos_are_rejected() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {"front_duct": {"openning": 0.5}}}
    }"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}

#[test]
fn brake_thermal_opening_is_validated() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {"front_duct": {"opening": 1.1}}}
    }"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}

#[test]
fn brake_thermal_round_trip_preserves_axle_openings() {
    let mut original = VehicleConfig::f1_94_canonical();
    original.brakes.brake_thermal.front_duct.opening = 0.63;
    original.brakes.brake_thermal.rear_duct.opening = 0.21;
    let json = serde_json::to_string(&original.to_json_value()).unwrap();
    let loaded = VehicleConfig::from_json_str(&json).unwrap();
    assert!((loaded.brakes.brake_thermal.front_duct.opening - 0.63).abs() < 1e-9);
    assert!((loaded.brakes.brake_thermal.rear_duct.opening - 0.21).abs() < 1e-9);
}

#[test]
fn tc_gear_curve_defaults_are_progressive_through_sixth() {
    let cfg = VehicleConfig::from_json_str(r#"{"schema_version":2}"#).unwrap();
    assert_eq!(cfg.aids.traction_control_gear_authority, vec![1.0, 0.90, 0.65, 0.45, 0.20, 0.10]);
    assert_eq!(cfg.aids.traction_control_gear_max_cut, vec![0.78, 0.65, 0.45, 0.30, 0.14, 0.07]);
    assert_eq!(cfg.aids.traction_control_gear_slip_target, vec![0.055, 0.065, 0.085, 0.105, 0.140, 0.180]);
}

#[test]
fn tc_gear_curve_accepts_mild_authority_in_fifth_and_sixth() {
    let json = r#"{
        "schema_version": 2,
        "aids": {
            "traction_control_gear_authority": [1.0, 0.9, 0.65, 0.45, 0.2, 0.1],
            "traction_control_gear_slip_target": [0.055, 0.065, 0.085, 0.105, 0.14, 0.18],
            "traction_control_gear_max_cut": [0.78, 0.65, 0.45, 0.30, 0.14, 0.07]
        }
    }"#;
    assert!(VehicleConfig::from_json_str(json).is_ok());
}

#[test]
fn tc_gear_curve_accepts_seven_gears_data_driven() {
    // A 7-speed car must be expressible purely in JSON (no code change / rebuild).
    let json = r#"{
        "schema_version": 2,
        "powertrain": {
            "gear_ratios": [3.15, 2.55, 2.05, 1.68, 1.38, 1.15, 0.95],
            "final_drive": 3.0
        },
        "aids": {
            "traction_control_gear_authority": [1.0, 0.9, 0.65, 0.45, 0.2, 0.1, 0.05],
            "traction_control_gear_slip_target": [0.055, 0.065, 0.085, 0.105, 0.14, 0.18, 0.21],
            "traction_control_gear_max_cut": [0.78, 0.65, 0.45, 0.30, 0.14, 0.07, 0.04]
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).expect("7-speed config must validate");
    assert_eq!(cfg.powertrain.gear_ratios.len(), 7);
    assert_eq!(cfg.aids.traction_control_gear_authority.len(), 7);
    assert_eq!(cfg.aids.traction_control_gear_max_cut.len(), 7);
}

#[test]
fn f1_2026_2008_physics_json_is_data_driven_seven_gears() {
    // Integration lock-in: the actual shipped 2026 content file must load via
    // the JSON path (no code change / rebuild needed for a 7-speed car).
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let cfg = VehicleConfig::from_json_str(&json)
        .unwrap_or_else(|e| panic!("2026 physics JSON must validate: {e}"));
    assert_eq!(cfg.powertrain.gear_ratios.len(), 7, "2026 car must have 7 gears");
    assert_eq!(
        cfg.aids.traction_control_gear_authority.len(),
        7,
        "TC authority array must match gear count"
    );
    assert_eq!(cfg.aids.traction_control_gear_max_cut.len(), 7);
    assert_eq!(
        cfg.powertrain.torque_attack_rate_nm_s, 1100.0,
        "2026 car must ship with the engine torque attack limiter enabled"
    );
    let audio = cfg
        .audio
        .as_ref()
        .expect("2026 profile must carry the audio section");
    assert!(
        audio.get("powertrain_synthesis").is_some(),
        "audio section must contain powertrain_synthesis contract"
    );
}

#[test]
fn schema_v3_full_profile_parses_and_round_trips() {
    let json = r#"{
        "schema_version": 3,
        "tires": {
            "force_model": {
                "front": {
                    "lateral_peak_slip_angle_rad": 0.18,
                    "longitudinal_peak_slip_ratio": 0.14,
                    "lateral_slide_mu_ratio": 0.92,
                    "longitudinal_slide_mu_ratio": 0.88,
                    "budget_blend_width": 0.12,
                    "falloff_sharpness": 1.5,
                    "slip_loss_tau_s": 0.04,
                    "slip_recovery_tau_s": 0.22
                },
                "rear": {
                    "lateral_peak_slip_angle_rad": 0.21,
                    "longitudinal_peak_slip_ratio": 0.16,
                    "lateral_slide_mu_ratio": 0.95,
                    "longitudinal_slide_mu_ratio": 0.90
                }
            },
            "pressure_mechanics": {
                "compliance_sensitivity": 0.72,
                "response_sensitivity": -0.18,
                "rolling_resistance_sensitivity": -0.35
            }
        },
        "suspension": {
            "front": {
                "spring_length": 0.25,
                "resting_ratio": 0.28,
                "damping_ratio": 0.80,
                "bump_damp_multiplier": 1.3,
                "rebound_damp_multiplier": 1.1,
                "arb_ratio": 0.20,
                "toe": 0.0017453,
                "camber": -0.0174533,
                "bump_stop_multiplier": 2.2,
                "camber_gain_rad_per_m": -0.021,
                "toe_gain_rad_per_m": 0.004
            },
            "rear": {
                "spring_length": 0.25,
                "resting_ratio": 0.28,
                "damping_ratio": 0.80,
                "bump_damp_multiplier": 1.3,
                "rebound_damp_multiplier": 1.1,
                "arb_ratio": 0.20,
                "toe": 0.0017453,
                "camber": -0.0174533,
                "bump_stop_multiplier": 2.2,
                "camber_gain_rad_per_m": 0.018,
                "toe_gain_rad_per_m": -0.002
            }
        }
    }"#;
    let cfg = VehicleConfig::from_json_str(json).expect("v3 profile must validate");
    assert_eq!(cfg.schema_version, 3);
    assert_eq!(cfg.tires.front_tire_force.lateral_peak_slip_angle_rad, 0.18);
    assert_eq!(cfg.tires.front_tire_force.longitudinal_peak_slip_ratio, 0.14);
    assert_eq!(cfg.tires.front_tire_force.lateral_slide_mu_ratio, 0.92);
    assert_eq!(cfg.tires.front_tire_force.longitudinal_slide_mu_ratio, 0.88);
    assert_eq!(cfg.tires.front_tire_force.budget_blend_width, 0.12);
    assert_eq!(cfg.tires.front_tire_force.falloff_sharpness, 1.5);
    assert_eq!(cfg.tires.front_tire_force.slip_loss_tau_s, 0.04);
    assert_eq!(cfg.tires.front_tire_force.slip_recovery_tau_s, 0.22);
    assert_eq!(cfg.tires.rear_tire_force.lateral_peak_slip_angle_rad, 0.21);
    assert_eq!(cfg.tires.rear_tire_force.longitudinal_peak_slip_ratio, 0.16);
    assert_eq!(cfg.tires.rear_tire_force.lateral_slide_mu_ratio, 0.95);
    assert_eq!(cfg.tires.rear_tire_force.longitudinal_slide_mu_ratio, 0.90);
    assert_eq!(cfg.tires.pressure_mechanics.compliance_sensitivity, 0.72);
    assert_eq!(cfg.tires.pressure_mechanics.response_sensitivity, -0.18);
    assert_eq!(
        cfg.tires.pressure_mechanics.rolling_resistance_sensitivity,
        -0.35
    );
    assert_eq!(cfg.suspension.front_camber_gain_rad_per_m, -0.021);
    assert_eq!(cfg.suspension.front_toe_gain_rad_per_m, 0.004);
    assert_eq!(cfg.suspension.rear_camber_gain_rad_per_m, 0.018);
    assert_eq!(cfg.suspension.rear_toe_gain_rad_per_m, -0.002);

    let round_trip = VehicleConfig::from_json_str(
        &serde_json::to_string(&cfg.to_json_value()).unwrap(),
    )
    .expect("v3 round trip must validate");
    assert_eq!(round_trip.schema_version, 3);
    assert_eq!(round_trip.tires.front_tire_force.lateral_peak_slip_angle_rad, 0.18);
    assert_eq!(round_trip.tires.front_tire_force.falloff_sharpness, 1.5);
    assert_eq!(round_trip.tires.front_tire_force.slip_recovery_tau_s, 0.22);
    assert_eq!(round_trip.tires.rear_tire_force.longitudinal_slide_mu_ratio, 0.90);
    assert_eq!(round_trip.tires.pressure_mechanics.compliance_sensitivity, 0.72);
    assert_eq!(round_trip.suspension.front_camber_gain_rad_per_m, -0.021);
    assert_eq!(round_trip.suspension.rear_toe_gain_rad_per_m, -0.002);
}

#[test]
fn schema_v3_absent_sections_use_parser_defaults() {
    let cfg = VehicleConfig::from_json_str(r#"{"schema_version": 3}"#).unwrap();
    assert_eq!(cfg.schema_version, 3);
    assert_eq!(
        cfg.tires.front_tire_force,
        TireForceProfile {
            lateral_peak_slip_angle_rad: 0.10,
            longitudinal_peak_slip_ratio: 0.12,
            lateral_slide_mu_ratio: 1.0,
            longitudinal_slide_mu_ratio: 1.0,
            budget_blend_width: 0.0,
            falloff_sharpness: 2.0,
            slip_loss_tau_s: 0.0,
            slip_recovery_tau_s: 0.0,
            combined_lateral_peak_migration: 0.0,
            combined_longitudinal_peak_migration: 0.0,
            min_relaxation_tau_s: 0.0,
            relaxation_recovery_scale: 1.0,
            contact_ramp_tau_s: 0.0,
        }
    );
    assert_eq!(cfg.tires.rear_tire_force, cfg.tires.front_tire_force);
    assert_eq!(
        cfg.tires.pressure_mechanics,
        PressureMechanicsSensitivity::default()
    );
    assert_eq!(cfg.suspension.front_camber_gain_rad_per_m, 0.0);
    assert_eq!(cfg.suspension.rear_toe_gain_rad_per_m, 0.0);
}

#[test]
fn schema_v3_invalid_force_model_values_rejected() {
    for bad in [
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"lateral_peak_slip_angle_rad": 0.61}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"lateral_peak_slip_angle_rad": 0.01}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"longitudinal_peak_slip_ratio": 1.6}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"lateral_slide_mu_ratio": 0.05}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"rear": {"longitudinal_slide_mu_ratio": 1.2}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"falloff_sharpness": 0.1}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"falloff_sharpness": 9.0}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"slip_loss_tau_s": -0.1}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"rear": {"slip_recovery_tau_s": 3.0}}}}"#,
    ] {
        assert!(
            VehicleConfig::from_json_str(bad).is_err(),
            "must reject: {bad}"
        );
    }
}

#[test]
fn wheel_lock_parameters_are_validated() {
    for bad in [
        r#"{"schema_version": 3, "brakes": {"wheel_lock_blend_spin_rad_s": -1.0}}"#,
        r#"{"schema_version": 3, "brakes": {"wheel_lock_blend_spin_rad_s": 500.0}}"#,
        r#"{"schema_version": 3, "brakes": {"wheel_lock_stick_tau_s": 2.0}}"#,
    ] {
        assert!(
            VehicleConfig::from_json_str(bad).is_err(),
            "must reject: {bad}"
        );
    }
    let ok = VehicleConfig::from_json_str(
        r#"{"schema_version": 3, "brakes": {"wheel_lock_blend_spin_rad_s": 8.0, "wheel_lock_stick_tau_s": 0.02}}"#,
    )
    .expect("valid wheel lock parameters must parse");
    assert_eq!(ok.brakes.wheel_lock_blend_spin_rad_s, 8.0);
    assert_eq!(ok.brakes.wheel_lock_stick_tau_s, 0.02);
}

#[test]
fn relaxation_parameters_are_validated() {
    for bad in [
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"min_relaxation_tau_s": 0.8}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"relaxation_recovery_scale": 0.01}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"relaxation_recovery_scale": 9.0}}}}"#,
    ] {
        assert!(
            VehicleConfig::from_json_str(bad).is_err(),
            "must reject: {bad}"
        );
    }
    let ok = VehicleConfig::from_json_str(
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"min_relaxation_tau_s": 0.035, "relaxation_recovery_scale": 1.6}}}}"#,
    )
    .expect("valid relaxation parameters must parse");
    assert_eq!(ok.tires.front_tire_force.min_relaxation_tau_s, 0.035);
    assert_eq!(ok.tires.front_tire_force.relaxation_recovery_scale, 1.6);
}

#[test]
fn contact_and_surface_ramp_parameters_are_validated() {
    for bad in [
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"contact_ramp_tau_s": 2.0}}}}"#,
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"contact_ramp_tau_s": -0.5}}}}"#,
        r#"{"schema_version": 3, "tires": {"surface_transition_tau_s": 2.0}}"#,
        r#"{"schema_version": 3, "tires": {"surface_transition_tau_s": -0.1}}"#,
    ] {
        assert!(
            VehicleConfig::from_json_str(bad).is_err(),
            "must reject: {bad}"
        );
    }
    let ok = VehicleConfig::from_json_str(
        r#"{"schema_version": 3, "tires": {"force_model": {"front": {"contact_ramp_tau_s": 0.06}}, "surface_transition_tau_s": 0.08}}"#,
    )
    .expect("valid ramp parameters must parse");
    assert_eq!(ok.tires.front_tire_force.contact_ramp_tau_s, 0.06);
    assert_eq!(ok.tires.surface_transition_tau_s, 0.08);
}

#[test]
fn schema_v3_invalid_pressure_mechanics_and_gains_rejected() {
    let bad_mechanics = r#"{
        "schema_version": 3,
        "tires": {"pressure_mechanics": {"compliance_sensitivity": 1e320}}
    }"#;
    assert!(VehicleConfig::from_json_str(bad_mechanics).is_err());
    let bad_gain = r#"{
        "schema_version": 3,
        "suspension": {
            "front": {
                "spring_length": 0.25,
                "resting_ratio": 0.28,
                "damping_ratio": 0.80,
                "camber_gain_rad_per_m": 1e320
            }
        }
    }"#;
    assert!(VehicleConfig::from_json_str(bad_gain).is_err());
}

#[test]
fn schema_v2_emission_does_not_include_v3_sections() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(cfg.schema_version, 2);
    let json = serde_json::to_string(&cfg.to_json_value()).unwrap();
    assert!(
        !json.contains("force_model"),
        "v2 emission must not carry force_model: {json}"
    );
    assert!(
        !json.contains("pressure_mechanics"),
        "v2 emission must not carry pressure_mechanics"
    );
    assert!(
        !json.contains("camber_gain_rad_per_m"),
        "v2 emission must not carry camber_gain_rad_per_m"
    );
}

#[test]
fn schema_v4_is_rejected() {
    let json = r#"{
        "schema_version": 4,
        "tires": {"force_model": {"front": {"lateral_peak_slip_angle_rad": 0.18}}}
    }"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}
