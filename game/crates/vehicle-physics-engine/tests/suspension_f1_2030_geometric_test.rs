//! SUS-GEO-09 acceptance: F1 2030 geometric profile migration + calibration.
use vehicle_physics_engine::*;

fn geo_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")
}

fn legacy_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_physics.json")
}

fn flat_hit(distance: f64) -> RaycastHit {
    RaycastHit {
        is_colliding: true,
        distance,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    }
}

#[test]
fn geometric_profile_loads_and_legacy_is_preserved() {
    let geo_text = std::fs::read_to_string(geo_path()).unwrap();
    let cfg = VehicleConfig::from_json_str(&geo_text).unwrap();
    assert_eq!(cfg.suspension_model, SuspensionModelKind::Geometric);
    let geo = cfg.geometric_suspension.as_ref().unwrap();
    assert_eq!(geo.version, 1);
    // Mirrors resolve to audited hardpoints.
    assert!((geo.corners.fl.hub_center.x + 0.753).abs() < 1e-9);
    assert!((geo.corners.rl.hub_center.x + 0.716).abs() < 1e-9);
    assert_eq!(geo.corners.fr_source, "mirror_of:FL");
    assert_eq!(geo.corners.rr_source, "mirror_of:RL");
    // Provenance labels: nothing reconstructed is presented as measured.
    assert_eq!(
        geo.corners.rl.provenance.origin,
        CornerProvenanceOrigin::Reconstructed
    );
    // Round-trip preserves the physical contract.
    let v = cfg.to_json_value();
    let cfg2 = VehicleConfig::from_json_str(&serde_json::to_string(&v).unwrap()).unwrap();
    assert_eq!(cfg2.geometric_suspension.unwrap(), *geo);

    // Legacy reference is byte-identical behaviour: still legacy, no model key.
    let legacy_text = std::fs::read_to_string(legacy_path()).unwrap();
    let legacy_json: serde_json::Value = serde_json::from_str(&legacy_text).unwrap();
    assert!(legacy_json["suspension"].get("model").is_none());
    assert!(legacy_json["suspension"].get("geometry_physical").is_none());
    let legacy = VehicleConfig::from_json_str(&legacy_text).unwrap();
    assert_eq!(legacy.suspension_model, SuspensionModelKind::Legacy1Dof);
    assert!((legacy.front_spring_length - 0.3).abs() < 1e-12);
    // Calibrated wheel rates reproduce the legacy static stance.
    assert!((cfg.front_spring_length - 0.3).abs() < 1e-12);
}

#[test]
fn f1_motion_ratio_healthy_across_envelope_and_steer() {
    let cfg =
        VehicleConfig::from_json_str(&std::fs::read_to_string(geo_path()).unwrap()).unwrap();
    let geo = cfg.geometric_suspension.as_ref().unwrap();
    for wheel in WheelIndex::ALL {
        let c = geo.corners.get(wheel);
        let axle = geo.axle(wheel);
        let (lo, hi) = travel_envelope(c, axle.wheel_droop_m, axle.wheel_bump_m, 0.0, wheel.is_front());
        // Element limits bind (linkage reaches further): full travel usable.
        assert!((lo + axle.wheel_droop_m).abs() < 1e-9, "{wheel:?} droop");
        assert!((hi - axle.wheel_bump_m).abs() < 1e-9, "{wheel:?} bump");
        for i in 0..=8 {
            let q = lo + (hi - lo) * (i as f64 / 8.0);
            for rack in if wheel.is_front() {
                vec![-0.02, 0.0, 0.02]
            } else {
                vec![0.0]
            } {
                let s = solve_corner(c, q, rack, wheel.is_front(), 0.0, 0.0, 1.0).unwrap();
                assert!(s.converged, "{wheel:?} q={q} rack={rack}");
                let j = jacobian(c, q, rack, wheel.is_front()).unwrap();
                assert!(j.motion_ratio > 0.15, "{wheel:?} r healthy, q={q}");
            }
        }
    }
}

#[test]
fn f1_geometric_settles_at_design() {
    let cfg =
        VehicleConfig::from_json_str(&std::fs::read_to_string(geo_path()).unwrap()).unwrap();
    let mut sus = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    let stat = |wheel: WheelIndex| {
        let m = cfg.mass_over_wheel(wheel) * 9.80665;
        let k = WheelMechanicalTuning::for_wheel(&cfg, wheel).tire_vertical_stiffness_n_m;
        m / k
    };
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius
        - stat(WheelIndex::FrontLeft);
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius
        - stat(WheelIndex::RearLeft);
    let mk = |d: f64| TriRaycastSample {
        inner: flat_hit(d),
        center: flat_hit(d),
        outer: flat_hit(d),
    };
    for _ in 0..150 {
        sus.step(&cfg, &[mk(rest_f), mk(rest_f), mk(rest_r), mk(rest_r)], dt);
    }
    for wheel in WheelIndex::ALL {
        let st = &sus.wheels[wheel as usize];
        let load = cfg.mass_over_wheel(wheel) * 9.80665;
        assert!(
            (st.total_normal_force - load).abs() < load * 0.05,
            "{wheel:?} Fz {}",
            st.total_normal_force
        );
        let rest = if wheel.is_front() {
            cfg.front_spring_length * cfg.front_resting_ratio
        } else {
            cfg.rear_spring_length * cfg.rear_resting_ratio
        };
        assert!(
            (st.suspension_compression_m - rest).abs() < 0.010,
            "{wheel:?} stance"
        );
        assert!(!st.geometric_clamped);
    }
}

#[test]
fn ab_legacy_vs_geometric_static_matches_dynamic_differs_sanely() {
    // Objective A/B (scripted, blind-capable): same static stance, sane
    // dynamic differences from geometry (progressivity), no NaN anywhere.
    let legacy =
        VehicleConfig::from_json_str(&std::fs::read_to_string(legacy_path()).unwrap()).unwrap();
    let geo =
        VehicleConfig::from_json_str(&std::fs::read_to_string(geo_path()).unwrap()).unwrap();
    let dt = 1.0 / 120.0;
    let mk = |d: f64| TriRaycastSample {
        inner: flat_hit(d),
        center: flat_hit(d),
        outer: flat_hit(d),
    };
    let stat = |cfg: &VehicleConfig, wheel: WheelIndex| {
        let m = cfg.mass_over_wheel(wheel) * 9.80665;
        let k = WheelMechanicalTuning::for_wheel(cfg, wheel).tire_vertical_stiffness_n_m;
        m / k
    };
    let road = |cfg: &VehicleConfig| {
        let rf = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius
            - stat(cfg, WheelIndex::FrontLeft);
        let rr = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius
            - stat(cfg, WheelIndex::RearLeft);
        [mk(rf), mk(rf), mk(rr), mk(rr)]
    };
    // Static stance: both must carry static loads (same car).
    let mut sl = SuspensionSystem::new(&legacy);
    let mut sg = SuspensionSystem::new(&geo);
    for _ in 0..80 {
        let rl = road(&legacy);
        let rg = road(&geo);
        sl.step(&legacy, &rl, dt);
        sg.step(&geo, &rg, dt);
    }
    for wheel in WheelIndex::ALL {
        let load = legacy.mass_over_wheel(wheel) * 9.80665;
        let fl = sl.wheels[wheel as usize].total_normal_force;
        let fg = sg.wheels[wheel as usize].total_normal_force;
        assert!((fl - load).abs() < load * 0.05, "legacy static {wheel:?}");
        assert!((fg - load).abs() < load * 0.05, "geo static {wheel:?}");
        assert!((fl - fg).abs() < load * 0.05, "static match {wheel:?}");
    }
    // 20 mm front step: peaks differ (geometry matters) but stay sane.
    let mut peak_l = 0.0f64;
    let mut peak_g = 0.0f64;
    for _ in 0..60 {
        let mut rl = road(&legacy);
        let mut rg = road(&geo);
        for i in 0..2 {
            rl[i] = mk(rl[i].center.distance - 0.020);
            rg[i] = mk(rg[i].center.distance - 0.020);
        }
        sl.step(&legacy, &rl, dt);
        sg.step(&geo, &rg, dt);
        peak_l = peak_l.max(sl.wheels[0].total_normal_force);
        peak_g = peak_g.max(sg.wheels[0].total_normal_force);
        for w in 0..4 {
            assert!(sl.wheels[w].total_normal_force.is_finite());
            assert!(sg.wheels[w].total_normal_force.is_finite());
        }
    }
    let rel = (peak_g - peak_l).abs() / peak_l;
    assert!(rel > 0.005, "geometry must matter, rel={rel}");
    assert!(rel < 0.50, "same car sanity, rel={rel}");
}

#[test]
fn godot_scene_still_points_at_legacy() {
    // Activation is opt-in: the shipped scene keeps the legacy profile until a
    // human validates the geometric one in-engine (SUS-GEO-10 A/B).
    let tscn = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn");
    let text = std::fs::read_to_string(tscn).unwrap();
    assert!(text.contains("f1_2030_v10_physics.json"));
    assert!(!text.contains("geometric"));
}

#[test]
fn f1_tables_generate_with_parity() {
    let cfg =
        VehicleConfig::from_json_str(&std::fs::read_to_string(geo_path()).unwrap()).unwrap();
    let geo = cfg.geometric_suspension.as_ref().unwrap();
    for wheel in WheelIndex::ALL {
        let (n_q, n_r) = if wheel.is_front() { (33, 9) } else { (33, 1) };
        let t = generate_table(geo, wheel, n_q, n_r, 0.02).expect("table");
        let (hub_err, ang_err) = parity_error(&t, geo);
        assert!(hub_err < TABLE_PARITY_HUB_M, "{wheel:?} {hub_err}");
        assert!(ang_err < TABLE_PARITY_ANGLE_RAD, "{wheel:?} {ang_err}");
    }
}
