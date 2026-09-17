//! SUS-GEO-06 acceptance: linkage camber/toe + hub trajectory into contact.
use vehicle_physics_engine::*;

fn axle_front() -> serde_json::Value {
    serde_json::json!({
        "spring_rate_N_per_m": 117000.0,
        "spring_free_length_m": 0.2826,
        "spring_installed_length_m": 0.26,
        "damper_bump_Ns_per_m": 11800.0,
        "damper_rebound_Ns_per_m": 15930.0,
        "damper_knee_m_per_s": 0.127,
        "damper_fast_factor": 0.5,
        "wheel_droop_m": 0.05,
        "wheel_bump_m": 0.10,
        "damper_min_m": 0.15,
        "damper_max_m": 0.40,
        "bump_stop_mult": 2.6,
    })
}

fn axle_rear() -> serde_json::Value {
    serde_json::json!({
        "spring_rate_N_per_m": 109000.0,
        "spring_free_length_m": 0.2797,
        "spring_installed_length_m": 0.25,
        "damper_bump_Ns_per_m": 12000.0,
        "damper_rebound_Ns_per_m": 16200.0,
        "damper_knee_m_per_s": 0.127,
        "damper_fast_factor": 0.5,
        "wheel_droop_m": 0.05,
        "wheel_bump_m": 0.10,
        "damper_min_m": 0.15,
        "damper_max_m": 0.40,
        "bump_stop_mult": 2.8,
    })
}

fn fl_corner() -> serde_json::Value {
    serde_json::json!({
        "hub_center": [-0.753, 0.0, -1.475],
        "lower_wishbone": {
            "inner_front": [-0.165237, -0.032255, -1.520731],
            "inner_rear": [-0.1912, -0.031258, -1.063059],
            "outer": [-0.585831, -0.0317, -1.478898]
        },
        "upper_wishbone": {
            "inner_front": [-0.165237, 0.136096, -1.520731],
            "inner_rear": [-0.1912, 0.137093, -1.196632],
            "outer": [-0.585831, 0.13665, -1.478898]
        },
        "trackrod": {
            "inner": [-0.163643, -0.0329515, -1.6063275],
            "outer": [-0.581991, -0.0334055, -1.586078]
        },
        "rod": {"outer": [-0.549739, -0.019274, -1.4389295], "attachment": "lower", "type": "pushrod"},
        "rocker": {
            "pivot": [-0.162718, 0.2204355, -1.4462085],
            "axis": [1.0, 0.0, 0.0],
            "pushrod_arm": [-0.162718, 0.2204355, -1.6062085],
            "damper_arm": [-0.162718, 0.2204355, -1.5262085]
        },
        "damper": {"chassis": [-0.162718, 0.5004355, -1.5462085]},
        "provenance": {"origin": "measured", "note": "06 trailing rocker"}
    })
}

fn rl_corner() -> serde_json::Value {
    serde_json::json!({
        "hub_center": [-0.716, 0.0, 1.475],
        "lower_wishbone": {
            "inner_front": [-0.2916855, -0.1203565, 1.1482685],
            "inner_rear": [-0.073654, -0.1203565, 1.624112],
            "outer": [-0.505193, -0.0596035, 1.474073]
        },
        "upper_wishbone": {
            "inner_front": [-0.1457255, 0.0552935, 1.1460325],
            "inner_rear": [-0.073654, 0.0512445, 1.624112],
            "outer": [-0.505193, 0.1160465, 1.474073]
        },
        "trackrod": {"inner": [-0.085, -0.015, 1.61], "outer": [-0.505193, -0.015, 1.55]},
        "rod": {"outer": [-0.48, 0.11, 1.43], "attachment": "upper", "type": "pullrod"},
        "rocker": {
            "pivot": [-0.2, -0.145, 1.39],
            "axis": [1.0, 0.0, 0.0],
            "pushrod_arm": [-0.2, -0.145, 1.25],
            "damper_arm": [-0.2, -0.145, 1.32]
        },
        "damper": {"chassis": [-0.2, 0.115, 1.34]},
        "driveshaft": {"inner": [-0.088833, -0.0033785, 1.487802], "outer": [-0.716, 0.0, 1.475]},
        "provenance": {"origin": "reconstructed", "note": "06 trailing rocker"}
    })
}

fn axle_front_m(k_wheel: f64, c_wheel: f64, r0: f64, load: f64) -> serde_json::Value {
    let k_s = k_wheel / (r0 * r0);
    let c = c_wheel / (r0 * r0);
    let installed = 0.26;
    let free = installed + (load / r0) / k_s;
    serde_json::json!({
        "spring_rate_N_per_m": k_s,
        "spring_free_length_m": free,
        "spring_installed_length_m": installed,
        "damper_bump_Ns_per_m": c,
        "damper_rebound_Ns_per_m": c * 1.35,
        "damper_knee_m_per_s": 0.127,
        "damper_fast_factor": 0.5,
        "wheel_droop_m": 0.05,
        "wheel_bump_m": 0.10,
        "damper_min_m": 0.15,
        "damper_max_m": 0.40,
        "bump_stop_mult": 2.6,
    })
}

fn axle_rear_m(k_wheel: f64, c_wheel: f64, r0: f64, load: f64) -> serde_json::Value {
    let k_s = k_wheel / (r0 * r0);
    let c = c_wheel / (r0 * r0);
    let installed = 0.25;
    let free = installed + (load / r0) / k_s;
    serde_json::json!({
        "spring_rate_N_per_m": k_s,
        "spring_free_length_m": free,
        "spring_installed_length_m": installed,
        "damper_bump_Ns_per_m": c,
        "damper_rebound_Ns_per_m": c * 1.35,
        "damper_knee_m_per_s": 0.127,
        "damper_fast_factor": 0.5,
        "wheel_droop_m": 0.05,
        "wheel_bump_m": 0.10,
        "damper_min_m": 0.15,
        "damper_max_m": 0.40,
        "bump_stop_mult": 2.8,
    })
}

fn geometric_profile() -> String {
    let g0 = serde_json::json!({
        "version": 1,
        "front": axle_front(),
        "rear": axle_rear(),
        "front_arb": {"mode": "legacy_ratio", "ratio": 0.15},
        "rear_arb": {"mode": "legacy_ratio", "ratio": 0.09},
        "corners": {"FL": fl_corner(), "FR": {"mirror_of": "FL"}, "RL": rl_corner(), "RR": {"mirror_of": "RL"}},
    });
    let p0 = format!(
        r#"{{"schema_version": 3, "suspension": {{"front": {{"spring_length": 0.3, "resting_ratio": 0.15}}, "rear": {{"spring_length": 0.27, "resting_ratio": 0.22}}, "model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g0).unwrap()
    );
    let cfg0 = VehicleConfig::from_json_str(&p0).unwrap();
    let geo0 = cfg0.geometric_suspension.unwrap();
    let r0f = jacobian(&geo0.corners.fl, 0.0, 0.0, true).unwrap().motion_ratio;
    let r0r = jacobian(&geo0.corners.rl, 0.0, 0.0, false).unwrap().motion_ratio;
    assert!(r0f > 0.05 && r0r > 0.05);
    let g = 9.80665;
    let load_f = 600.0 * 0.45 * 0.5 * g;
    let load_r = 600.0 * 0.55 * 0.5 * g;
    let g = serde_json::json!({
        "version": 1,
        "front": axle_front_m(29419.9, 2949.5, r0f, load_f),
        "rear": axle_rear_m(27240.7, 3010.5, r0r, load_r),
        "front_arb": {"mode": "legacy_ratio", "ratio": 0.15},
        "rear_arb": {"mode": "legacy_ratio", "ratio": 0.09},
        "corners": {"FL": fl_corner(), "FR": {"mirror_of": "FL"}, "RL": rl_corner(), "RR": {"mirror_of": "RL"}},
    });
    format!(
        r#"{{"schema_version": 3, "chassis": {{"vehicle_mass": 600.0, "front_weight_distribution": 0.45}}, "suspension": {{"front": {{"spring_length": 0.3, "resting_ratio": 0.15, "camber": -0.0484, "camber_gain_rad_per_m": -0.02, "toe": -0.0018, "toe_gain_rad_per_m": 0.0}}, "rear": {{"spring_length": 0.27, "resting_ratio": 0.22, "camber": -0.029, "camber_gain_rad_per_m": -0.02, "toe": 0.0042, "toe_gain_rad_per_m": 0.025}}, "model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g).unwrap()
    )
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
fn linkage_pose_matches_statics_at_rest() {
    let cfg = VehicleConfig::from_json_str(&geometric_profile()).unwrap();
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
    for _ in 0..30 {
        sus.step(&cfg, &[mk(rest_f), mk(rest_f), mk(rest_r), mk(rest_r)], dt);
    }
    // At design rest the linkage contributes ~0: totals equal statics.
    assert!((sus.wheels[0].geometric_camber_rad - -0.0484).abs() < 5e-3);
    assert!((sus.wheels[2].geometric_camber_rad - -0.029).abs() < 5e-3);
    assert!((sus.wheels[0].geometric_toe_rad - -0.0018).abs() < 5e-3);
    assert!((sus.wheels[2].geometric_toe_rad - 0.0042).abs() < 5e-3);
    assert!(sus.wheels[0].hub_lateral_m.abs() < 5e-3);
    assert!(sus.wheels[0].hub_long_m.abs() < 5e-3);
}

#[test]
fn pose_finite_continuous_and_mirrored() {
    let cfg = VehicleConfig::from_json_str(&geometric_profile()).unwrap();
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
    let mut prev = [[0.0f64; 4]; 4];
    for step in 0..25 {
        // Progressive left bump (15 mm max), right holds.
        let b = if step < 10 { 0.0 } else { -0.001 * ((step - 10) as f64) };
        sus.step(
            &cfg,
            &[mk(rest_f + b), mk(rest_f), mk(rest_r + b), mk(rest_r)],
            dt,
        );
        for w in 0..4 {
            let st = &sus.wheels[w];
            for v in [st.geometric_camber_rad, st.geometric_toe_rad, st.hub_lateral_m, st.hub_long_m] {
                assert!(v.is_finite(), "wheel {w} non-finite pose");
            }
            assert!(st.geometric_camber_rad.abs() < 0.15, "camber sane");
            assert!(st.geometric_toe_rad.abs() < 0.05, "toe sane");
            assert!(st.hub_lateral_m.abs() < 0.02 && st.hub_long_m.abs() < 0.02);
            let cur = [st.geometric_camber_rad, st.geometric_toe_rad, st.hub_lateral_m, st.hub_long_m];
            if step > 0 {
                for k in 0..4 {
                    assert!((cur[k] - prev[w][k]).abs() < 0.02, "continuous");
                }
            }
            prev[w] = cur;
        }
    }
    // Axle-frame convention: left and right totals match at symmetric pose
    // (mirroring happens at consumption, like legacy gains).
    let (fl, fr) = (&sus.wheels[0], &sus.wheels[1]);
    assert!((fl.geometric_camber_rad - fr.geometric_camber_rad).abs() < 0.02);
    // Hub lateral offsets mirror (x negated).
    assert!((fl.hub_lateral_m + fr.hub_lateral_m).abs() < 2e-3);
    assert!((fl.hub_long_m - fr.hub_long_m).abs() < 2e-3);
}

#[test]
fn steering_consumes_linkage_toe_one_to_one() {
    let cfg = VehicleConfig::from_json_str(&geometric_profile()).unwrap();
    // dSteer/dToe == +1 right / -1 left (same mirror as legacy).
    let a = steering_angle_for_wheel_geo(&cfg, WheelIndex::FrontRight, 0.0, 0.0042);
    let b = steering_angle_for_wheel_geo(&cfg, WheelIndex::FrontRight, 0.0, 0.0052);
    assert!((b - a - 0.001).abs() < 1e-9);
    let c = steering_angle_for_wheel_geo(&cfg, WheelIndex::FrontLeft, 0.0, 0.0042);
    let d = steering_angle_for_wheel_geo(&cfg, WheelIndex::FrontLeft, 0.0, 0.0052);
    assert!((d - c + 0.001).abs() < 1e-9);
    // Zero steer input + zero toe gain profile: legacy would give statics;
    // geometric matches statics at rest by construction (previous test).
    for v in [a, b, c, d] {
        assert!(v.is_finite());
    }
}

#[test]
fn legacy_gain_path_untouched() {
    // Legacy profile: dynamic camber is exactly static + gain*travel.
    let cfg = VehicleConfig::from_json_str(r#"{"schema_version": 3}"#).unwrap();
    let mut sus = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    let rest = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let mk = |d: f64| TriRaycastSample {
        inner: flat_hit(d),
        center: flat_hit(d),
        outer: flat_hit(d),
    };
    for _ in 0..10 {
        sus.step(&cfg, &[mk(rest), mk(rest), mk(rest), mk(rest)], dt);
    }
    for w in 0..4 {
        let st = &sus.wheels[w];
        assert_eq!(st.geometric_camber_rad, 0.0);
        assert_eq!(st.geometric_toe_rad, 0.0);
        assert_eq!(st.hub_lateral_m, 0.0);
    }
}

/// The checked-in F1 2030 V10 geometric profile (real hardpoints).
fn f1_2030_geometric_config() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .expect("checked-in f1_2030_v10_geometric.json must be valid")
}

#[test]
fn f1_2030_linkage_pose_sweep_is_sane_and_mirrored() {
    // The telemetry shows the rear axle operating between design rest (q = 0)
    // and ~+55 mm bump under aero load and power squat, with the bump stop in
    // play from ~+50 mm. The synthetic sweep above only covers 15 mm, so this
    // test sweeps the REAL profile hardpoints across the full travel envelope
    // to rule out large bump-steer or camber swings exactly where the car lives.
    let cfg = f1_2030_geometric_config();
    let geo = cfg
        .geometric_suspension
        .as_ref()
        .expect("geometric suspension present");

    for (axle, is_front, corners) in [
        ("front", true, [WheelIndex::FrontLeft, WheelIndex::FrontRight]),
        ("rear", false, [WheelIndex::RearLeft, WheelIndex::RearRight]),
    ] {
        let mut max_toe_change: f64 = 0.0;
        let mut max_camber_change: f64 = 0.0;
        let mut max_hub_lat: f64 = 0.0;
        let mut max_hub_long: f64 = 0.0;
        let mut max_lr_toe_delta: f64 = 0.0;
        let mut max_lr_camber_delta: f64 = 0.0;
        let mut max_lr_hub_lat_sum: f64 = 0.0;

        for step in 0..=19 {
            let q = -0.040 + step as f64 * 0.005; // -40 mm droop .. +55 mm bump
            let mut poses = [0.0f64; 8];
            for (slot, wheel) in corners.into_iter().enumerate() {
                let corner = geo.corners.get(wheel);
                let sol = solve_corner(corner, q, 0.0, is_front, 0.0, 0.0, 1.0)
                    .expect("linkage must solve inside the travel envelope");
                let side = if wheel.is_left() { 1.0 } else { -1.0 };
                let link_camber = side * sol.upright_basis.x.y.clamp(-1.0, 1.0).asin();
                let link_toe = side * sol.upright_basis.z.x.clamp(-1.0, 1.0).asin();
                let hub_lat = sol.hub.x - corner.hub_center.x;
                let hub_long = sol.hub.z - corner.hub_center.z;
                assert!(
                    link_camber.is_finite() && link_toe.is_finite(),
                    "non-finite {axle} pose at q={q:.3}"
                );
                let base = slot * 4;
                poses[base] = link_toe;
                poses[base + 1] = link_camber;
                poses[base + 2] = hub_lat;
                poses[base + 3] = hub_long;
                max_toe_change = max_toe_change.max(link_toe.abs());
                max_camber_change = max_camber_change.max(link_camber.abs());
                max_hub_lat = max_hub_lat.max(hub_lat.abs());
                max_hub_long = max_hub_long.max(hub_long.abs());
            }
            // Mirrored corners must produce mirrored poses at the same travel.
            max_lr_toe_delta = max_lr_toe_delta.max((poses[0] - poses[4]).abs());
            max_lr_camber_delta = max_lr_camber_delta.max((poses[1] - poses[5]).abs());
            max_lr_hub_lat_sum = max_lr_hub_lat_sum.max((poses[2] + poses[6]).abs());
        }

        println!(
            "f1_2030 {axle} linkage sweep: |toe|max={:.4} rad ({:.2} deg) |camber|max={:.4} rad ({:.2} deg) |hubLat|max={:.1} mm |hubLong|max={:.1} mm L/R dToe={:.4} dCam={:.4}",
            max_toe_change,
            max_toe_change.to_degrees(),
            max_camber_change,
            max_camber_change.to_degrees(),
            max_hub_lat * 1000.0,
            max_hub_long * 1000.0,
            max_lr_toe_delta,
            max_lr_camber_delta,
        );

        assert!(max_hub_lat < 0.02 && max_hub_long < 0.02);
        assert!(max_lr_toe_delta < 1e-9, "{axle} toe must mirror L/R");
        assert!(max_lr_camber_delta < 1e-9, "{axle} camber must mirror L/R");
        assert!(max_lr_hub_lat_sum < 1e-9, "{axle} hub lateral must mirror L/R");

        // Regression bounds for the working range. The rear profile shipped with
        // a trackrod inner 70 mm too high, producing up to 7.1 deg of bump steer
        // (clamped to 2.9 deg at the wheel) that steered the rear axle under
        // squat/aero load. Keep the linkage near zero over the full envelope.
        let toe_bound = if is_front { 0.002 } else { 0.010 };
        assert!(
            max_toe_change < toe_bound,
            "{axle} bump-steer excessive over real travel: {:.3} deg (bound {:.3})",
            max_toe_change.to_degrees(),
            toe_bound.to_degrees()
        );
        assert!(
            max_camber_change < 0.02,
            "{axle} camber swing excessive over real travel: {:.3} deg",
            max_camber_change.to_degrees()
        );
    }
}
