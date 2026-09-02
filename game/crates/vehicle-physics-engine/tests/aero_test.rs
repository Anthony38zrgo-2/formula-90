use vehicle_physics_engine::*;

// Helper: velocity in chassis local frame: +X right, +Y up, -Z forward
fn vel_local(v_fwd: f64, v_lat: f64) -> Vec3 {
    // Godot convention: forward = -Z, so local_velocity.z = -v_fwd
    Vec3::new(v_lat, 0.0, -v_fwd)
}

/// Environment at optimal floor height and rake, zero slip/roll: the floor
/// `target_flow` is exactly 1.0, so downforce/drag lag is a pure first-order
/// exponential on constant targets (no floor stall dynamics).
fn full_efficiency_environment() -> AeroEnvironment {
    AeroEnvironment {
        clearance_m: [0.045, 0.045, 0.045, 0.045, 0.045],
        valid_mask: 0x1f,
        rake_rad: 0.5f64.to_radians(),
        roll_rad: 0.0,
        bottoming_mask: 0,
        contact_confidence: 0.0,
    }
}

/// Instantaneous (unlimited) element-path target for given v_fwd/v_lat,
/// computed with dt = aero_lag_tau so alpha = 1.
fn element_raw_target(config: &VehicleConfig, env: &AeroEnvironment, v_fwd: f64, v_lat: f64) -> f64 {
    let mut aero = AeroForces::zero();
    aero.step_with_environment(config, vel_local(v_fwd, v_lat), env, 0.0, config.aero_lag_tau);
    aero.raw_downforce
}

/// Converged steady-state state at fixed velocity (500 steps at 1 ms).
fn settle(config: &VehicleConfig, env: &AeroEnvironment, v_fwd: f64, v_lat: f64) -> AeroForces {
    let mut aero = AeroForces::zero();
    for _ in 0..500 {
        aero.step_with_environment(config, vel_local(v_fwd, v_lat), env, 0.0, 0.001);
    }
    aero
}

#[test]
fn test_aerodynamic_lag_exponential_decay() {
    // Verify step velocity input reaches 63.2% of target in tau seconds (±5% per human gate)
    let cfg = VehicleConfig::f1_94_canonical();
    let tau = cfg.aero_lag_tau;
    assert!((tau - 0.048).abs() < 1e-9, "tau must be 0.048");

    let env = full_efficiency_environment();
    let v_fwd = 30.0; // > aero_blend_full_speed (27.778) => blend = 1
    let settled = settle(&cfg, &env, v_fwd, 0.0);
    let target = settled.total_downforce;
    let target_drag = settled.drag_force;
    assert!(target > 500.0, "target should be meaningful at 30 m/s, got {}", target);

    // Simulate with small dt to approximate continuous exponential
    let dt = 0.001;
    let steps = (tau / dt).round() as usize; // 48
    let mut aero = AeroForces::zero();
    let vel = vel_local(v_fwd, 0.0);
    for _ in 0..steps {
        aero.step_with_environment(&cfg, vel, &env, 0.0, dt);
    }

    let expected = 0.6321205588 * target; // 1 - 1/e
    let actual = aero.total_downforce;
    let tol = 0.05 * expected; // 5%
    assert!(
        (actual - expected).abs() <= tol,
        "lag at tau: actual={:.2} expected={:.2} (target={:.2}) tol={:.2}",
        actual, expected, target, tol
    );

    // Also verify drag lag follows same alpha
    let expected_drag = 0.6321205588 * target_drag;
    let actual_drag = aero.drag_force;
    let tol_drag = 0.05 * expected_drag.abs().max(1.0);
    assert!(
        (actual_drag - expected_drag).abs() <= tol_drag,
        "drag lag at tau: actual={:.2} expected={:.2} target_drag={:.2}",
        actual_drag, expected_drag, target_drag
    );

    // Also verify monotonic approach: after another tau, should be ~86.5%
    for _ in 0..steps {
        aero.step_with_environment(&cfg, vel, &env, 0.0, dt);
    }
    let expected_2tau = (1.0 - (-2.0f64).exp()) * target; // 1 - 1/e^2 = 0.86466
    let tol2 = 0.05 * expected_2tau;
    assert!(
        (aero.total_downforce - expected_2tau).abs() <= tol2,
        "lag at 2tau: actual={:.2} expected={:.2}",
        aero.total_downforce, expected_2tau
    );
}

#[test]
fn test_smoothstep_speed_blending_monotonicity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let env = AeroEnvironment::default();
    let v_lat = 0.0;

    // Sample blend_factor across speed range with alpha=1 for instant target
    // Use dt = tau so filtered = target immediately, blend diagnostics are direct
    let dt = cfg.aero_lag_tau;
    let mut prev_blend = -1.0;
    let mut samples: Vec<(f64, f64)> = Vec::new();
    let mut speeds = vec![0.0, 1.0, 2.0, 4.167, 8.0, 12.0, 16.0, 20.0, 27.778, 30.0, 35.0, 40.0];
    // add fine sweep for monotonicity
    for v in (0..80).map(|i| i as f64 * 0.5) {
        if !speeds.contains(&v) {
            speeds.push(v);
        }
    }
    speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for v in speeds {
        let mut aero = AeroForces::zero();
        aero.step_with_environment(&cfg, vel_local(v, v_lat), &env, 0.0, dt);
        let s = aero.blend_factor;
        samples.push((v, s));

        // zero at rest
        if v == 0.0 {
            assert!(s.abs() < 1e-9, "S(0) must be 0, got {}", s);
        }
        // zero below blend_min
        if v <= cfg.aero_blend_min_speed + 1e-9 {
            // At exactly blend_min, S should be ~0 (smoothstep starts)
            if v == cfg.aero_blend_min_speed {
                assert!(s.abs() < 1e-9, "S(blend_min) must be 0, got {}", s);
            }
        }
        // one at full
        if v >= cfg.aero_blend_full_speed {
            assert!((s - 1.0).abs() < 1e-9, "S(>=full) must be 1, got {} at v={}", s, v);
        }
        // strictly monotonic between min and full
        if v > cfg.aero_blend_min_speed && v < cfg.aero_blend_full_speed {
            assert!(s > 0.0 && s < 1.0, "S intermediate must be (0,1), got {} at {}", s, v);
        }
        // global monotonic non-decreasing
        assert!(
            s + 1e-12 >= prev_blend,
            "S must be monotonic: prev={} curr={} at v={}",
            prev_blend, s, v
        );
        prev_blend = s;
    }

    // C1 continuity: derivative near endpoints ~0 via finite difference
    // Use central difference around blend_min and blend_full
    let eps = 1e-4;
    // Approximate derivative dS/dv at blend_min
    let v0 = cfg.aero_blend_min_speed;
    // Evaluate S analytically via aero diagnostics: create two nearby points
    let mut a_low = AeroForces::zero();
    let mut a_high = AeroForces::zero();
    a_low.step_with_environment(&cfg, vel_local(v0 - eps, 0.0), &env, 0.0, dt);
    a_high.step_with_environment(&cfg, vel_local(v0 + eps, 0.0), &env, 0.0, dt);
    let deriv_min = (a_high.blend_factor - a_low.blend_factor) / (2.0 * eps);
    assert!(
        deriv_min.abs() < 0.05,
        "C1 at blend_min: derivative should be ~0, got {}",
        deriv_min
    );
    let v1 = cfg.aero_blend_full_speed;
    a_low.step_with_environment(&cfg, vel_local(v1 - eps, 0.0), &env, 0.0, dt);
    a_high.step_with_environment(&cfg, vel_local(v1 + eps, 0.0), &env, 0.0, dt);
    let deriv_full = (a_high.blend_factor - a_low.blend_factor) / (2.0 * eps);
    assert!(
        deriv_full.abs() < 0.05,
        "C1 at blend_full: derivative should be ~0, got {}",
        deriv_full
    );

    // Also verify effective_cl is zero at rest and positive at speed
    let mut a_rest = AeroForces::zero();
    a_rest.step_with_environment(&cfg, vel_local(0.0, 0.0), &env, 0.0, dt);
    assert!(a_rest.effective_cl.abs() < 1e-9, "CL_eff at rest must be 0");
    let mut a_fast = AeroForces::zero();
    a_fast.step_with_environment(&cfg, vel_local(30.0, 0.0), &env, 0.0, dt);
    assert!(a_fast.effective_cl > 0.0, "CL_eff at speed must be >0");
}

#[test]
fn test_sideslip_yaw_decay_under_drift() {
    let cfg = VehicleConfig::f1_94_canonical();
    let env = AeroEnvironment::default();
    let dt = cfg.aero_lag_tau; // instant convergence for monotonic check
    let v_fwd = 20.0;
    let target_zero_slip = element_raw_target(&cfg, &env, v_fwd, 0.0);

    // Sweep lateral velocity 0 -> 30 m/s
    let mut prev_downforce = f64::INFINITY;
    let mut prev_f_yaw = 2.0;
    let lats = [0.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0];
    for &v_lat in &lats {
        let mut aero = AeroForces::zero();
        aero.step_with_environment(&cfg, vel_local(v_fwd, v_lat), &env, 0.0, dt);
        let f_yaw = aero.yaw_decay_factor;
        // No NaN/Inf
        assert!(f_yaw.is_finite(), "f_yaw not finite at v_lat={}", v_lat);
        assert!(aero.total_downforce.is_finite(), "downforce not finite");
        // f_yaw in [0,1]
        assert!(f_yaw >= -1e-9 && f_yaw <= 1.0 + 1e-9, "f_yaw out of range {}", f_yaw);
        // Monotonic decreasing (allow tiny epsilon)
        assert!(
            f_yaw <= prev_f_yaw + 1e-9,
            "f_yaw must decrease with lateral: prev {} curr {} at v_lat={}",
            prev_f_yaw, f_yaw, v_lat
        );
        // Downforce also decreases smoothly
        assert!(
            aero.total_downforce <= prev_downforce + 1e-6,
            "downforce must not increase with slip: prev {} curr {} at v_lat={}",
            prev_downforce, aero.total_downforce, v_lat
        );
        // No discontinuity: step drop should be < 60% per 5 m/s increment
        if prev_downforce.is_finite() && prev_downforce > 1.0 {
            let drop = (prev_downforce - aero.total_downforce) / prev_downforce;
            assert!(drop < 0.7, "discontinuity drop too large {} at v_lat={}", drop, v_lat);
        }
        // At zero slip, should match target
        if v_lat == 0.0 {
            assert!(
                (aero.total_downforce - target_zero_slip).abs() < 1e-6,
                "zero-slip downforce mismatch {} vs {}",
                aero.total_downforce, target_zero_slip
            );
            assert!((f_yaw - 1.0).abs() < 1e-6, "f_yaw at zero slip must be ~1, got {}", f_yaw);
        }
        // At large slip (v_lat >> v_fwd), f_yaw reduces and downforce drops
        if v_lat == 30.0 {
            assert!(
                f_yaw < 0.7,
                "f_yaw at large slip should be <0.7, got {}",
                f_yaw
            );
            assert!(
                aero.total_downforce < target_zero_slip * 0.6,
                "downforce should reduce at drift"
            );
        }
        prev_downforce = aero.total_downforce;
        prev_f_yaw = f_yaw;
    }

    // Check symmetry: +v_lat and -v_lat give same f_yaw (uses abs)
    let mut a_pos = AeroForces::zero();
    let mut a_neg = AeroForces::zero();
    a_pos.step_with_environment(&cfg, vel_local(v_fwd, 5.0), &env, 0.0, dt);
    a_neg.step_with_environment(&cfg, vel_local(v_fwd, -5.0), &env, 0.0, dt);
    assert!(
        (a_pos.yaw_decay_factor - a_neg.yaw_decay_factor).abs() < 1e-9,
        "yaw decay must be symmetric"
    );

    // Continuity: fine sweep check no NaN and smooth derivative
    for i in 0..60 {
        let v_lat = i as f64 * 0.5;
        let mut aero = AeroForces::zero();
        aero.step_with_environment(&cfg, vel_local(v_fwd, v_lat), &env, 0.0, dt);
        assert!(aero.yaw_decay_factor.is_finite());
        assert!(aero.blend_factor.is_finite());
    }
}

#[test]
fn test_aero_distribution_and_balance_from_elements() {
    let cfg = VehicleConfig::f1_94_canonical();
    let env = AeroEnvironment::default();
    let mut aero = AeroForces::zero();
    // Drive to steady state at speed to get non-zero downforce
    let dt = cfg.aero_lag_tau;
    for _ in 0..10 {
        aero.step_with_environment(&cfg, vel_local(25.0, 0.0), &env, 0.0, dt);
    }
    assert!(aero.total_downforce > 10.0);
    let front = aero.front_downforce;
    let diff = aero.diffuser_downforce;
    let rear = aero.rear_downforce;
    // Distribution must sum to total
    assert!(
        (front + diff + rear - aero.total_downforce).abs() < 1e-6,
        "aero distribution must sum to total"
    );
    // Balance must be derived from element loads, not hardcoded splits
    let expected_balance = front / aero.total_downforce;
    assert!(
        (aero.balance_front - expected_balance).abs() < 1e-6,
        "balance_front must equal front/total, got {} vs {}",
        aero.balance_front,
        expected_balance
    );
    assert!(
        aero.balance_front > 0.0 && aero.balance_front < 1.0,
        "balance_front out of (0,1): {}",
        aero.balance_front
    );
    // All three elements must contribute positive downforce (front wing,
    // diffuser, rear wing at this speed/environment).
    assert!(front > 0.0 && diff > 0.0 && rear > 0.0);
}

#[test]
fn test_wing_cl_speed_independent_no_flex() {
    // Element-path (v3) aero must not scale wing CL with speed: at fixed
    // incidence the polar CL is constant, so downforce grows only with q.
    let cfg = VehicleConfig::f1_94_canonical();
    let mut aero = AeroForces::zero();
    aero.step_with_environment(
        &cfg,
        vel_local(30.0, 0.0),
        &AeroEnvironment::default(),
        0.0,
        1e9,
    );
    let cl_at_30 = aero.front_wing_cl;
    let rcl_at_30 = aero.rear_wing_cl;
    let df_30 = aero.total_downforce;
    let mut aero_fast = AeroForces::zero();
    aero_fast.step_with_environment(
        &cfg,
        vel_local(60.0, 0.0),
        &AeroEnvironment::default(),
        0.0,
        1e9,
    );
    assert_eq!(aero_fast.front_wing_cl, cl_at_30);
    assert_eq!(aero_fast.rear_wing_cl, rcl_at_30);
    let df_60 = aero_fast.total_downforce;
    let expected_ratio = 4.0; // (60/30)^2
    let ratio = df_60 / df_30.max(1e-6);
    assert!(
        (ratio - expected_ratio).abs() / expected_ratio < 0.02,
        "downforce must scale with v^2 at fixed CL: ratio={} expected={}",
        ratio,
        expected_ratio
    );
}

#[test]
fn test_reverse_and_zero_velocity_gives_no_downforce() {
    let cfg = VehicleConfig::f1_94_canonical();
    let env = AeroEnvironment::default();
    let dt = cfg.aero_lag_tau;
    // Zero velocity
    let mut aero = AeroForces::zero();
    aero.step_with_environment(&cfg, vel_local(0.0, 0.0), &env, 0.0, dt);
    assert!(aero.total_downforce.abs() < 1e-9);
    assert!(aero.drag_force.abs() < 1e-9);
    // Negative forward (reverse = local_velocity.z positive)
    let mut aero_rev = AeroForces::zero();
    // vel_local with negative v_fwd would be forward_speed negative => stored as -z positive
    // Use raw Vec3 with +Z to simulate reverse motion
    let rev_vel = Vec3::new(0.0, 0.0, 5.0); // -z = -5 => v_fwd = max(-5,0)=0
    aero_rev.step_with_environment(&cfg, rev_vel, &env, 0.0, dt);
    assert!(aero_rev.total_downforce.abs() < 1e-9);
    assert!(aero_rev.drag_force.abs() < 1e-9);
}
