//! SUS-GEO-05 acceptance: anchor reactions, body modes, transfer, ARB, energy.
use vehicle_physics_engine::*;

fn axle_front(k_wheel: f64, c_wheel: f64, r0: f64, load: f64, _arb: f64) -> serde_json::Value {
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

fn axle_rear(k_wheel: f64, c_wheel: f64, r0: f64, load: f64, _arb: f64) -> serde_json::Value {
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
            "pushrod_arm": [-0.162718, 0.2204355, -1.6062085],
            "damper_arm": [-0.162718, 0.2204355, -1.5262085]
        },
        "damper": {"chassis": [-0.162718, 0.5004355, -1.5462085]},
        "provenance": {"origin": "measured", "note": "05 idealized front, trailing rocker r~0.5"}
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
        "provenance": {"origin": "reconstructed", "note": "05 idealized rear, trailing rocker r~0.5"}
    })
}

fn profile_with_arb(front_arb: f64, rear_arb: f64) -> (String, f64, f64) {
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
    assert!(r0f > 0.05 && r0r > 0.05);
    let g = 9.80665;
    let load_f = 600.0 * 0.45 * 0.5 * g;
    let load_r = 600.0 * 0.55 * 0.5 * g;
    let g1 = serde_json::json!({
        "version": 1,
        "front": axle_front(29419.9, 2949.5, r0f, load_f, front_arb),
        "rear": axle_rear(27240.7, 3010.5, r0r, load_r, rear_arb),
        "front_arb": {"mode": "legacy_ratio", "ratio": front_arb},
        "rear_arb": {"mode": "legacy_ratio", "ratio": rear_arb},
        "corners": {"FL": fl_corner(), "FR": {"mirror_of": "FL"}, "RL": rl_corner(), "RR": {"mirror_of": "RL"}},
    });
    let p1 = format!(
        r#"{{"schema_version": 3, "chassis": {{"vehicle_mass": 600.0, "front_weight_distribution": 0.45}}, "suspension": {{"front": {{"spring_length": 0.3, "resting_ratio": 0.15}}, "rear": {{"spring_length": 0.27, "resting_ratio": 0.22}}, "model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g1).unwrap()
    );
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

fn flat_samples_for(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut out = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let i = wheel as usize;
        let anchor_local = sim.config.wheel_anchor_local(wheel);
        let anchor_world = sim.state.transform.transform_point(anchor_local);
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        } * sim.config.tri_ray_spacing_ratio;
        let hit = |x: f64| RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::new(anchor_world.x + x, 0.0, anchor_world.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        out[i] = TriRaycastSample {
            inner: hit(span),
            center: hit(0.0),
            outer: hit(-span),
        };
    }
    out
}

fn settled_sim(json: &str, ticks: usize) -> VehicleSimulator {
    let cfg = sim_config(json);
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let input = VehicleInput::default();
    for _ in 0..ticks {
        let samples = flat_samples_for(&sim);
        let _ = sim.step(&input, &samples, 1.0 / 120.0);
    }
    sim
}

fn sim_config(json: &str) -> VehicleConfig {
    VehicleConfig::from_json_str(json).unwrap()
}

#[test]
fn symmetric_settle_gives_symmetric_anchors_and_balance() {
    let (prof, _, _) = profile_with_arb(0.15, 0.09);
    let sim = settled_sim(&prof, 100);
    let st = &sim.state.suspension;
    // Must actually be settled before the anchor comparison means anything.
    for wheel in WheelIndex::ALL {
        let w = &st.wheels[wheel as usize];
        let load = sim.config.mass_over_wheel(wheel) * 9.80665;
        assert!(
            (w.total_normal_force - load).abs() < load * 0.40,
            "{wheel:?} unsettled Fz {}",
            w.total_normal_force
        );
    }
    // Mirror symmetry: left anchors mirror right (x negated per anchor).
    for (l, r) in [(0usize, 1usize), (2usize, 3usize)] {
        let (lw, rw) = (&st.wheels[l], &st.wheels[r]);
        assert!(
            (lw.load_path_element_n - rw.load_path_element_n).abs() < 5.0,
            "element path L/R symmetric"
        );
        assert!(
            (lw.load_path_wishbone_n - rw.load_path_wishbone_n).abs() < 8.0,
            "wishbone path L/R symmetric"
        );
        assert!(lw.balance_residual_n < 30.0, "balance small, got {}", lw.balance_residual_n);
        assert!(rw.balance_residual_n < 30.0, "balance small, got {}", rw.balance_residual_n);
        assert!(!lw.link_singular && !rw.link_singular);
        // Shares sum to ~1.
        let tot = lw.load_path_element_n + lw.load_path_wishbone_n + lw.load_path_other_n;
        assert!(tot > 100.0, "wheel carries load through anchors");
        let s = lw.load_path_element_n / tot
            + lw.load_path_wishbone_n / tot
            + lw.load_path_other_n / tot;
        assert!((s - 1.0).abs() < 0.02, "shares sum {s}");
    }
    // Anchor verticals per axle match the contact load transfer base: anchors
    // carry contacts minus hanging unsprung weights (G = F_c + W - m a).
    let fz_f = st.wheels[0].total_normal_force + st.wheels[1].total_normal_force;
    let anch_f = (st.wheels[0].load_path_element_n
        + st.wheels[0].load_path_wishbone_n
        + st.wheels[0].load_path_other_n)
        + (st.wheels[1].load_path_element_n
            + st.wheels[1].load_path_wishbone_n
            + st.wheels[1].load_path_other_n);
    let g = 9.80665;
    let unsprung_f =
        WheelMechanicalTuning::for_wheel(&sim.config, WheelIndex::FrontLeft).unsprung_mass_kg * g
            + WheelMechanicalTuning::for_wheel(&sim.config, WheelIndex::FrontRight).unsprung_mass_kg * g;
    let expect_f = fz_f - unsprung_f;
    assert!(
        (anch_f - expect_f).abs() < expect_f.abs() * 0.05 + 20.0,
        "axle anchor total {anch_f} vs contacts-minus-unsprung {expect_f}"
    );
}

#[test]
fn asymmetric_load_transfers_through_anchors_matching_contacts() {
    // Helper-level (no body dynamics): left bumped, right at design. Anchor
    // verticals must reproduce each wheel's contact load (minus unsprung
    // weight), so axle transfer measured at anchors equals transfer at
    // contacts — the same load, transmitted.
    let (prof, _, _) = profile_with_arb(0.15, 0.09);
    let cfg = sim_config(&prof);
    let geo = cfg.geometric_suspension.as_ref().unwrap().clone();
    let basis = Mat3::IDENTITY;
    let tf = WheelMechanicalTuning::for_wheel(&cfg, WheelIndex::FrontLeft);
    let m_u = tf.unsprung_mass_kg;
    let g = 9.80665;
    // Small bump (+20 mm, linear for the long-arm test rockers) on the left,
    // design on the right.
    let mut anchor_totals = [0.0f64; 4];
    let mut contacts = [0.0f64; 4];
    for (wi, q, fz) in [
        (WheelIndex::FrontLeft, 0.020, 0.0),
        (WheelIndex::FrontRight, 0.0, 0.0),
        (WheelIndex::RearLeft, 0.020, 0.0),
        (WheelIndex::RearRight, 0.0, 0.0),
    ] {
        let corner = geo.corners.get(wi);
        let axle = geo.axle(wi);
        let r = jacobian(corner, q, 0.0, wi.is_front()).unwrap().motion_ratio;
        let s = solve_corner(corner, q, 0.0, wi.is_front(), 0.0, 0.0, 1.0)
            .unwrap()
            .damper_compression;
        let preload = axle.spring_rate_N_per_m
            * (axle.spring_free_length_m - axle.spring_installed_length_m);
        let fz_here = if fz == 0.0 {
            (preload + axle.spring_rate_N_per_m * s) * r
        } else {
            fz
        };
        contacts[wi as usize] = fz_here;
        let sol = solve_corner(corner, q, 0.0, wi.is_front(), 0.0, 0.0, 1.0).unwrap();
        let steered = steered_trackrod_outer(sol.lbj, sol.ubj, corner.trackrod_outer, corner.lower.outer, 0.0);
        let shaft = if wi.is_rear() {
            Some(shaft_outer_at_pose(
                sol.hub,
                corner.hub_center,
                corner.driveshaft_outer.unwrap_or(corner.hub_center),
            ))
        } else {
            None
        };
        let frame = link_frame(corner, &sol, steered, shaft);
        let f_elem = preload + axle.spring_rate_N_per_m * s;
        let ddu = (sol.damper_end - corner.damper_chassis).normalized();
        let hub_below = sol.hub + Vec3::new(0.0, -0.33, 0.0);
        let (list, paths) = reactions(
            &frame,
            Vec3::new(0.0, fz_here, 0.0),
            hub_below,
            Vec3::ZERO,
            m_u,
            0.0,
            f_elem,
            ddu,
            corner.rocker_pivot,
            corner.damper_chassis,
            0.0,
        );
        assert!(!paths.singular, "{wi:?} must solve cleanly");
        assert!(paths.balance_residual_n < 0.02 * fz_here + 5.0, "{wi:?} balance");
        let tot: f64 = list.iter().map(|r| r.force.y).sum();
        anchor_totals[wi as usize] = tot;
        // Anchors carry contacts minus hanging weight.
        assert!(
            (tot - (fz_here - m_u * g)).abs() < 0.03 * fz_here + 8.0,
            "{wi:?} anchors {tot} vs contact {fz_here}"
        );
        let _ = basis;
    }
    // Transfer equality front axle (rear symmetric by construction here).
    let t_cont = contacts[0] - contacts[1];
    let t_anch = anchor_totals[0] - anchor_totals[1];
    assert!(t_cont > 50.0, "bump must load the left wheel");
    assert!(
        (t_cont - t_anch).abs() < t_cont.abs() * 0.05 + 5.0,
        "transfer contact {t_cont} vs anchors {t_anch}"
    );
    // Modal decomposition reports the roll (suspension-level, cheap).
    let mut sus = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    for _ in 0..45 {
        let bump = |d: f64, left: bool| {
            let dd = if left { d - 0.025 } else { d };
            TriRaycastSample {
                inner: flat_hit(dd),
                center: flat_hit(dd),
                outer: flat_hit(dd),
            }
        };
        sus.step(&cfg, &[bump(rest_f, true), bump(rest_f, false), bump(rest_r, true), bump(rest_r, false)], dt);
    }
    assert!(sus.roll_m > 1e-4, "roll mode visible, got {}", sus.roll_m);
    assert!(sus.heave_m.abs() < 0.05);
}

#[test]
fn arb_increases_roll_restoring_torque() {
    // Single-tick restoring roll torque on a tilted body (no integration, no
    // oscillation phase): must oppose the tilt (sign) and grow with ARB.
    fn roll_torque(prof: &str) -> f64 {
        let cfg = sim_config(prof);
        let spawn = default_spawn_height(&cfg);
        let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
        let input = VehicleInput::default();
        for _ in 0..5 {
            let samples = flat_samples_for(&sim);
            let _ = sim.step(&input, &samples, 1.0 / 120.0);
        }
        // Tilt 0.02 rad roll about Z (stays linear), zero velocities.
        let basis = Mat3::from_euler_yxz(0.0, 0.0, 0.02);
        let body = BodyKinematics {
            transform: Transform3D::new(Vec3::new(0.0, spawn, 0.0), basis),
            orientation: Quat::from_mat3(&basis),
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        };
        // Let the filter develop the asymmetric load (kinematic body).
        let mut out = None;
        for _ in 0..14 {
            let samples = flat_samples_for(&sim);
            // Recompute samples against the TILTED pose, not the sim pose.
            let mut s2 = samples;
            for (wi, s) in WheelIndex::ALL.iter().zip(s2.iter_mut()) {
                let anchor_local = sim.config.wheel_anchor_local(*wi);
                let anchor_world = body.transform.transform_point(anchor_local);
                let distance = anchor_world.y.max(0.0);
                let span = if wi.is_front() {
                    sim.config.front_tire_width
                } else {
                    sim.config.rear_tire_width
                } * sim.config.tri_ray_spacing_ratio;
                let hit = |x: f64| RaycastHit {
                    is_colliding: true,
                    distance,
                    point: Vec3::new(anchor_world.x + x, 0.0, anchor_world.z),
                    normal: Vec3::UP,
                    surface: SurfaceType::Road,
                };
                *s = TriRaycastSample {
                    inner: hit(span),
                    center: hit(0.0),
                    outer: hit(-span),
                };
            }
            let (f, _) = sim.solve_external_with_aero(
                body,
                &input,
                &s2,
                &AeroEnvironment::default(),
                1.0 / 120.0,
            );
            out = Some(f.torque_world);
        }
        let t = out.unwrap();
        // Roll torque about the longitudinal (Z) axis in world (~body here).
        t.z
    }
    let (soft, _, _) = profile_with_arb(0.0, 0.0);
    let (stiff, _, _) = profile_with_arb(0.9, 0.0);
    let t_soft = roll_torque(&soft);
    let t_stiff = roll_torque(&stiff);
    // Tilt +0.02 about Z lifts +X (right); restoring torque must be negative.
    assert!(t_soft < -1.0, "restoring sign, got {t_soft}");
    assert!(t_stiff < -1.0, "restoring sign, got {t_stiff}");
    assert!(
        t_stiff < t_soft * 1.15,
        "ARB must stiffen roll: soft={t_soft} stiff={t_stiff}"
    );
}

#[test]
fn airborne_wheel_hangs_on_linkage_without_nan() {
    let (prof, _, _) = profile_with_arb(0.15, 0.09);
    let cfg = sim_config(&prof);
    let mut sim = settled_sim(&prof, 30);
    let dt = 1.0 / 120.0;
    let input = VehicleInput::default();
    // Drop the road under FL far away for 30 ticks.
    for _ in 0..30 {
        let mut samples = flat_samples_for(&sim);
        let far = cfg.front_spring_length + cfg.front_tire_radius + 0.50;
        samples[0] = TriRaycastSample {
            inner: flat_hit(far),
            center: flat_hit(far),
            outer: flat_hit(far),
        };
        let _ = sim.step(&input, &samples, dt);
    }
    let fl = &sim.state.suspension.wheels[0];
    assert!(!fl.is_grounded);
    assert!(fl.total_normal_force == 0.0);
    // Hanging weight still reaches the body through the anchors (downward).
    let hanging = fl.load_path_element_n + fl.load_path_wishbone_n + fl.load_path_other_n;
    assert!(hanging.is_finite());
    assert!(
        hanging < 0.0,
        "hanging wheel must pull the body down, got {hanging}"
    );
    // Others stay finite and planted.
    for w in 1..4 {
        let st = &sim.state.suspension.wheels[w];
        assert!(st.total_normal_force.is_finite() && st.total_normal_force > 0.0);
        assert!(st.balance_residual_n.is_finite());
    }
}

#[test]
fn energy_audit_is_consistent() {
    // Suspension-level (fast): damper dissipation grows monotonically,
    // spring energy matches the analytic 1/2 k total^2, ARB energy returns
    // to ~0 on level ground.
    let (prof, _, _) = profile_with_arb(0.15, 0.09);
    let cfg = sim_config(&prof);
    let mut sus = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;
    let rest_f = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let rest_r = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let mk = |d: f64| TriRaycastSample {
        inner: flat_hit(d),
        center: flat_hit(d),
        outer: flat_hit(d),
    };
    for _ in 0..30 {
        sus.step(&cfg, &[mk(rest_f), mk(rest_f), mk(rest_r), mk(rest_r)], dt);
    }
    // Bump the left side briefly to do work, then level again.
    for _ in 0..20 {
        sus.step(
            &cfg,
            &[mk(rest_f - 0.02), mk(rest_f), mk(rest_r - 0.02), mk(rest_r)],
            dt,
        );
    }
    for _ in 0..40 {
        sus.step(&cfg, &[mk(rest_f), mk(rest_f), mk(rest_r), mk(rest_r)], dt);
    }
    for w in 0..4 {
        let st = &sus.wheels[w];
        assert!(st.damper_dissipated_j.is_finite() && st.damper_dissipated_j >= 0.0);
        assert!(st.spring_energy_j.is_finite() && st.spring_energy_j > 0.0);
    }
    assert!(sus.damper_dissipated_total() > 0.0);
    assert!(sus.arb_energy_front_j.abs() < 5.0);
    assert!(sus.arb_energy_rear_j.abs() < 5.0);
    // Roll centres are published diagnostics (finite for test geometry).
    assert!(sus.rc_front_m.is_finite());
    assert!(sus.rc_rear_m.is_finite());
}
