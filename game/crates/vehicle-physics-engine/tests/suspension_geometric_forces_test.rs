//! SUS-GEO-04 acceptance: explicit spring/damper via kinematics + virtual work.
use vehicle_physics_engine::*;

fn axle_front(k_wheel: f64, c_wheel: f64, r0: f64, load: f64) -> serde_json::Value {
    let k_s = k_wheel / (r0 * r0);
    let c = c_wheel / (r0 * r0);
    let installed = 0.26;
    let preload = load / r0;
    let free = installed + preload / k_s;
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

fn axle_rear(k_wheel: f64, c_wheel: f64, r0: f64, load: f64) -> serde_json::Value {
    let k_s = k_wheel / (r0 * r0);
    let c = c_wheel / (r0 * r0);
    let installed = 0.25;
    let preload = load / r0;
    let free = installed + preload / k_s;
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
            "pushrod_arm": [-0.162718, 0.2004355, -1.5362085],
            "damper_arm": [-0.162718, 0.2504355, -1.3962085]
        },
        "damper": {"chassis": [-0.162718, 0.3004355, -1.07]},
        "provenance": {"origin": "measured", "note": "04 idealized front"}
    })
}

fn rl_corner() -> serde_json::Value {
    // Corrected rear: chassis to the rear of the damper arm so bump shortens
    // the damper (r > 0). Shipped visual uses forward chassis (r < 0, blocked).
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
            "pushrod_arm": [-0.2, -0.095, 1.3],
            "damper_arm": [-0.2, -0.11, 1.425]
        },
        "damper": {"chassis": [-0.2, -0.11, 1.68]},
        "driveshaft": {"inner": [-0.088833, -0.0033785, 1.487802], "outer": [-0.716, 0.0, 1.475]},
        "provenance": {"origin": "reconstructed", "note": "04 idealized rear, chassis rear for r>0"}
    })
}

/// Build a matched-preload geometric profile: preload*r0 == static load so
/// equilibrium sits at design (q = 0). Returns (json_string, r0_front, r0_rear).
fn matched_profile() -> (String, f64, f64) {
    // First pass with nominal rates to measure r0, then scale springs/dampers
    // to legacy-like wheel rates with matched preload.
    let g0 = serde_json::json!({
        "version": 1,
        "front": {
            "spring_rate_N_per_m": 30000.0, "spring_free_length_m": 0.30,
            "spring_installed_length_m": 0.26,
            "damper_bump_Ns_per_m": 3000.0, "damper_rebound_Ns_per_m": 4000.0,
            "damper_knee_m_per_s": 0.127, "damper_fast_factor": 0.5,
            "wheel_droop_m": 0.05, "wheel_bump_m": 0.10,
            "damper_min_m": 0.15, "damper_max_m": 0.40, "bump_stop_mult": 2.6,
        },
        "rear": {
            "spring_rate_N_per_m": 28000.0, "spring_free_length_m": 0.30,
            "spring_installed_length_m": 0.25,
            "damper_bump_Ns_per_m": 3000.0, "damper_rebound_Ns_per_m": 4000.0,
            "damper_knee_m_per_s": 0.127, "damper_fast_factor": 0.5,
            "wheel_droop_m": 0.05, "wheel_bump_m": 0.10,
            "damper_min_m": 0.15, "damper_max_m": 0.40, "bump_stop_mult": 2.8,
        },
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
    assert!(r0f > 0.05 && r0r > 0.05, "fixtures must give r>0 (f={r0f}, r={r0r})");
    // Legacy-like wheel rates with matched preload.
    let g = 9.80665;
    let load_f = 600.0 * 0.45 * 0.5 * g;
    let load_r = 600.0 * 0.55 * 0.5 * g;
    let g1 = serde_json::json!({
        "version": 1,
        "front": axle_front(29419.9, 2949.5, r0f, load_f),
        "rear": axle_rear(27240.7, 3010.5, r0r, load_r),
        "front_arb": {"mode": "legacy_ratio", "ratio": 0.15},
        "rear_arb": {"mode": "legacy_ratio", "ratio": 0.09},
        "corners": {"FL": fl_corner(), "FR": {"mirror_of": "FL"}, "RL": rl_corner(), "RR": {"mirror_of": "RL"}},
    });
    let p1 = format!(
        r#"{{"schema_version": 3, "chassis": {{"vehicle_mass": 600.0, "front_weight_distribution": 0.45}}, "suspension": {{"front": {{"spring_length": 0.3, "resting_ratio": 0.15}}, "rear": {{"spring_length": 0.27, "resting_ratio": 0.22}}, "model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g1).unwrap()
    );
    // Validate before returning.
    let cfg1 = VehicleConfig::from_json_str(&p1).unwrap();
    assert_eq!(cfg1.suspension_model, SuspensionModelKind::Geometric);
    (p1, r0f, r0r)
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
fn constant_ratio_reproduces_analytic() {
    let (prof, r0f, _) = matched_profile();
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let geo = cfg.geometric_suspension.unwrap();
    // Small window where r ≈ const: F_wheel ≈ (preload + k*r0*q) * r0.
    let axle = &geo.front;
    let preload = axle.spring_rate_N_per_m * (axle.spring_free_length_m - axle.spring_installed_length_m);
    for q in [-0.001, 0.0, 0.001] {
        let sol = solve_corner(&geo.corners.fl, q, 0.0, true, 0.0, 0.0, 1.0).unwrap();
        let jac = jacobian(&geo.corners.fl, q, 0.0, true).unwrap();
        assert!((jac.motion_ratio - r0f).abs() < 0.02, "near-rest r stable");
        // Direct element law vs solver state after one quiet step is covered
        // by the integration tests; here verify the analytic wheel map.
        let s_lin = r0f * q;
        assert!((sol.damper_compression - s_lin).abs() < 2e-4, "s≈r*q, q={q}");
        let f_elem = preload + axle.spring_rate_N_per_m * sol.damper_compression;
        let f_wheel = f_elem * jac.motion_ratio;
        let f_lin = (preload + axle.spring_rate_N_per_m * s_lin) * r0f;
        assert!(
            (f_wheel - f_lin).abs() < f_lin.abs() * 0.05 + 5.0,
            "constant-ratio analytic, q={q}"
        );
    }
}

#[test]
fn variable_ratio_tangent_includes_geometric_term() {
    let (prof, _, _) = matched_profile();
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let geo = cfg.geometric_suspension.unwrap();
    // Tangent k*r^2 + F*dr/dq must match numeric dF/dq; and the geometric term
    // must matter away from rest (i.e. tangent != k*r^2 alone).
    for (wheel, q) in [(WheelIndex::FrontLeft, 0.015), (WheelIndex::RearLeft, 0.015)] {
        let corner = geo.corners.get(wheel);
        let axle = geo.axle(wheel);
        let e = 1e-4;
        let f_of = |qq: f64| {
            let s = solve_corner(corner, qq, 0.0, wheel.is_front(), 0.0, 0.0, 1.0)
                .unwrap()
                .damper_compression;
            let total = (axle.spring_free_length_m - axle.spring_installed_length_m) + s;
            let f_s = (axle.spring_rate_N_per_m * total).max(0.0);
            let r = jacobian(corner, qq, 0.0, wheel.is_front()).unwrap().motion_ratio;
            f_s * r
        };
        let f0 = f_of(q);
        let num = (f_of(q + e) - f_of(q - e)) / (2.0 * e);
        let r0 = jacobian(corner, q, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let s0 = solve_corner(corner, q, 0.0, wheel.is_front(), 0.0, 0.0, 1.0)
            .unwrap()
            .damper_compression;
        let total0 = (axle.spring_free_length_m - axle.spring_installed_length_m) + s0;
        let fs0 = (axle.spring_rate_N_per_m * total0).max(0.0);
        let rp = jacobian(corner, q + e, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let rm = jacobian(corner, q - e, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let drdq = (rp - rm) / (2.0 * e);
        let tangent = axle.spring_rate_N_per_m * r0 * r0 + fs0 * drdq;
        assert!(
            (tangent - num).abs() < num.abs() * 0.05 + 50.0,
            "{wheel:?} tangent {tangent} vs numeric {num}"
        );
        let bare = axle.spring_rate_N_per_m * r0 * r0;
        assert!(
            (tangent - bare).abs() > 1.0,
            "{wheel:?} geometric term must matter (F*dr/dq), tangent={tangent} bare={bare} F={fs0} drdq={drdq}"
        );
        let _ = f0;
    }
}

#[test]
fn damper_dissipates_energy() {
    let (prof, _, _) = matched_profile();
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let geo = cfg.geometric_suspension.as_ref().unwrap();
    // Element damper opposes shaft velocity; wheel power F*v >= 0 dissipates.
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let corner = geo.corners.get(wheel);
        for v in [0.1, -0.1, 0.5, -0.5] {
            let q = 0.01;
            let r = jacobian(corner, q, 0.0, wheel.is_front()).unwrap().motion_ratio;
            assert!(r > 0.0);
            // Reproduce the element law through the public solver state: run
            // one substep-free evaluation via a quiet SuspensionSystem tick is
            // covered below; here check sign via kinematics + axle law.
            let axle = geo.axle(wheel);
            let vs = r * v;
            let fd = if vs >= 0.0 {
                let c = axle.damper_bump_Ns_per_m;
                if vs > axle.damper_knee_m_per_s {
                    (vs - axle.damper_knee_m_per_s) * c * axle.damper_fast_factor
                        + axle.damper_knee_m_per_s * c
                } else {
                    vs * c
                }
            } else if vs < -axle.damper_knee_m_per_s {
                let c = axle.damper_rebound_Ns_per_m;
                (vs + axle.damper_knee_m_per_s) * c * axle.damper_fast_factor
                    - axle.damper_knee_m_per_s * c
            } else {
                vs * axle.damper_rebound_Ns_per_m
            };
            assert!(fd * vs >= 0.0, "{wheel:?} element must oppose shaft motion");
            assert!((fd * r) * v >= 0.0, "{wheel:?} wheel damper must dissipate");
        }
    }
    // Cycle energy through the integrated solver is strictly dissipative.
    let mut sim = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    // Pin the road far away (no contact → no tire force) is not useful; drive
    // the wheel DOF directly through alternating road targets and accumulate
    // damper power from solver telemetry.
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let mk = |d: f64| TriRaycastSample {
        inner: flat_hit(d),
        center: flat_hit(d),
        outer: flat_hit(d),
    };
    let mut energy = 0.0;
    for i in 0..240 {
        let bump = if i < 120 { -0.02 } else { 0.02 };
        let samples = [mk(rest_f + bump), mk(rest_f + bump), mk(rest_r + bump), mk(rest_r + bump)];
        sim.step(&cfg, &samples, dt);
        for w in 0..4 {
            let st = &sim.wheels[w];
            // Damper power at the wheel (dissipative when >= 0 with our sign
            // convention: force opposes velocity).
            energy += st.damping_force * st.unsprung_velocity_m_s * dt;
        }
    }
    assert!(energy.is_finite() && energy > 0.0, "cycle must dissipate, got {energy}");
}

#[test]
fn static_equilibrium_correct() {
    let (prof, _, _) = matched_profile();
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let mut sim = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    // Flat road at design minus static carcass squash (same construction as
    // the legacy equilibrium test).
    let stat = |wheel: WheelIndex| {
        let m = cfg.mass_over_wheel(wheel) * 9.80665;
        let k = WheelMechanicalTuning::for_wheel(&cfg, wheel).tire_vertical_stiffness_n_m;
        m / k
    };
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius
        - stat(WheelIndex::FrontLeft);
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius
        - stat(WheelIndex::RearLeft);
    let samples = [
        TriRaycastSample {
            inner: flat_hit(rest_f),
            center: flat_hit(rest_f),
            outer: flat_hit(rest_f),
        },
        TriRaycastSample {
            inner: flat_hit(rest_f),
            center: flat_hit(rest_f),
            outer: flat_hit(rest_f),
        },
        TriRaycastSample {
            inner: flat_hit(rest_r),
            center: flat_hit(rest_r),
            outer: flat_hit(rest_r),
        },
        TriRaycastSample {
            inner: flat_hit(rest_r),
            center: flat_hit(rest_r),
            outer: flat_hit(rest_r),
        },
    ];
    for _ in 0..300 {
        sim.step(&cfg, &samples, dt);
    }
    for wheel in WheelIndex::ALL {
        let st = &sim.wheels[wheel as usize];
        let load = cfg.mass_over_wheel(wheel) * 9.80665;
        assert!(
            (st.total_normal_force - load).abs() < load * 0.05,
            "{wheel:?} Fz {} vs static {load}",
            st.total_normal_force
        );
        // Matched preload puts equilibrium at design (q ≈ 0): legacy rest.
        let rest = if wheel.is_front() {
            cfg.front_spring_length * cfg.front_resting_ratio
        } else {
            cfg.rear_spring_length * cfg.rear_resting_ratio
        };
        assert!(
            (st.suspension_compression_m - rest).abs() < 0.010,
            "{wheel:?} must rest near design, got {} vs {rest}",
            st.suspension_compression_m
        );
        assert!(!st.geometric_clamped, "{wheel:?} must not sit on a stop");
        assert!(st.motion_ratio > 0.0);
        // Pushrod in compression (-) on bump-loaded front lower mount;
        // pullrod in tension (+) on the rear upper mount. Signs differ by
        // geometry (not by label): what matters is both are finite,
        // non-zero under load, and stable across ticks.
        assert!(st.rod_axial_force_n.is_finite());
    }
    // Front lower-mount pushrod carries compression, rear upper-mount pullrod
    // carries tension under static load.
    assert!(sim.wheels[0].rod_axial_force_n < 0.0, "FL pushrod must compress");
    assert!(sim.wheels[2].rod_axial_force_n > 0.0, "RL pullrod must pull");
}

#[test]
fn push_pull_labels_identical_response() {
    let (prof, _, _) = matched_profile();
    let v: serde_json::Value = serde_json::from_str(&prof).unwrap();
    // Same hardpoints, labels swapped: FL pull (was push), RL push (was pull).
    let mut v2 = v.clone();
    v2["suspension"]["geometry_physical"]["corners"]["FL"]["rod"]["type"] = serde_json::json!("pullrod");
    v2["suspension"]["geometry_physical"]["corners"]["RL"]["rod"]["type"] = serde_json::json!("pushrod");
    let a = VehicleConfig::from_json_str(&serde_json::to_string(&v).unwrap()).unwrap();
    let b = VehicleConfig::from_json_str(&serde_json::to_string(&v2).unwrap()).unwrap();
    let dt = 1.0 / 120.0;
    let mut sa = SuspensionSystem::new(&a);
    let mut sb = SuspensionSystem::new(&b);
    let rest_f = a.front_spring_length * (1.0 - a.front_resting_ratio) + a.front_tire_radius;
    let rest_r = a.rear_spring_length * (1.0 - a.rear_resting_ratio) + a.rear_tire_radius;
    for i in 0..120 {
        let bump = if i < 60 { -0.015 } else { 0.010 };
        let mk = |d: f64| TriRaycastSample {
            inner: flat_hit(d + bump),
            center: flat_hit(d + bump),
            outer: flat_hit(d + bump),
        };
        let samples = [mk(rest_f), mk(rest_f), mk(rest_r), mk(rest_r)];
        sa.step(&a, &samples, dt);
        sb.step(&b, &samples, dt);
    }
    for w in 0..4 {
        let (xa, xb) = (sa.wheels[w].suspension_compression_m, sb.wheels[w].suspension_compression_m);
        let (fa, fb) = (sa.wheels[w].chassis_suspension_force, sb.wheels[w].chassis_suspension_force);
        assert!((xa - xb).abs() < 1e-9, "wheel {w} travel label-independent");
        assert!((fa - fb).abs() < 1e-6, "wheel {w} force label-independent");
    }
}

#[test]
fn travel_stops_engage_on_overtravel() {
    let (prof, _, _) = matched_profile();
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let mut sim = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    // Slam all wheels 120 mm into bump (beyond the 100 mm physical bump):
    // solver must engage progressive + hard stops, clamp travel to the
    // physical envelope + measurable overtravel, and flag it — without NaN.
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let slam = [
        TriRaycastSample {
            inner: flat_hit(rest_f - 0.120),
            center: flat_hit(rest_f - 0.120),
            outer: flat_hit(rest_f - 0.120),
        },
        TriRaycastSample {
            inner: flat_hit(rest_f - 0.120),
            center: flat_hit(rest_f - 0.120),
            outer: flat_hit(rest_f - 0.120),
        },
        TriRaycastSample {
            inner: flat_hit(rest_r - 0.120),
            center: flat_hit(rest_r - 0.120),
            outer: flat_hit(rest_r - 0.120),
        },
        TriRaycastSample {
            inner: flat_hit(rest_r - 0.120),
            center: flat_hit(rest_r - 0.120),
            outer: flat_hit(rest_r - 0.120),
        },
    ];
    for _ in 0..120 {
        sim.step(&cfg, &slam, dt);
    }
    for wheel in WheelIndex::ALL {
        let st = &sim.wheels[wheel as usize];
        assert!(st.bottom_out_force > 0.0, "{wheel:?} stops must engage");
        assert!(st.bottom_out_force.is_finite());
        assert!(st.total_normal_force.is_finite() && st.total_normal_force >= 0.0);
        // Physical envelope + overtravel (0.03), not the legacy spring_len.
        let rest = if wheel.is_front() {
            cfg.front_spring_length * cfg.front_resting_ratio
        } else {
            cfg.rear_spring_length * cfg.rear_resting_ratio
        };
        let geo = cfg.geometric_suspension.as_ref().unwrap();
        let axle = geo.axle(wheel);
        let x_max = rest + axle.wheel_bump_m + 0.030 + 1e-9;
        assert!(
            st.suspension_compression_m <= x_max,
            "{wheel:?} must respect physical travel, got {}",
            st.suspension_compression_m
        );
    }
}
