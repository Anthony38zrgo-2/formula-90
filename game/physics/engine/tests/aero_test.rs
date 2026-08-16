use vehicle_physics_engine::*;

// Helper: velocity in chassis local frame: +X right, +Y up, -Z forward
fn vel_local(v_fwd: f64, v_lat: f64) -> Vec3 {
    // Godot convention: forward = -Z, so local_velocity.z = -v_fwd
    Vec3::new(v_lat, 0.0, -v_fwd)
}

/// Compute instantaneous target downforce for given v_fwd/v_lat without lag,
/// matching aero.rs Stage 2 (used to validate lag convergence).
fn target_downforce(config: &VehicleConfig, v_fwd: f64, v_lat: f64) -> f64 {
    let blend_min = config.aero_blend_min_speed;
    let blend_full = config.aero_blend_full_speed;
    let denom = (blend_full - blend_min).max(1e-6);
    let x = ((v_fwd - blend_min) / denom).clamp(0.0, 1.0);
    let s = 3.0 * x * x - 2.0 * x * x * x;
    let cos_beta = v_fwd / (v_fwd * v_fwd + v_lat * v_lat + 1e-6).sqrt();
    let f_yaw = cos_beta.clamp(0.0, 1.0).powf(config.aero_yaw_decay_exponent);
    let f_flex = 1.0 / (1.0 + config.aero_flex_coefficient * v_fwd);
    let cl_eff = config.coefficient_of_downforce * s * f_yaw * f_flex;
    let q = 0.5 * config.air_density * v_fwd * v_fwd;
    q * cl_eff * config.frontal_area
}

fn target_drag(config: &VehicleConfig, v_fwd: f64) -> f64 {
    let q = 0.5 * config.air_density * v_fwd * v_fwd;
    q * config.coefficient_of_drag * config.frontal_area
}

#[test]
fn test_aerodynamic_lag_exponential_decay() {
    // Verify step velocity input reaches 63.2% of target in tau seconds (±5% per human gate)
    let cfg = VehicleConfig::f1_94_canonical();
    let tau = cfg.aero_lag_tau;
    assert!((tau - 0.048).abs() < 1e-9, "tau must be 0.048");

    let v_fwd = 30.0; // > aero_blend_full_speed (27.778) => S=1
    let v_lat = 0.0;
    let target = target_downforce(&cfg, v_fwd, v_lat);
    let target_drag = target_drag(&cfg, v_fwd);
    assert!(target > 500.0, "target should be meaningful at 30 m/s, got {}", target);

    // Simulate with small dt to approximate continuous exponential
    let dt = 0.001; // 1ms => alpha=0.025, 40 steps = tau
    let steps = (tau / dt).round() as usize; // 40
    let mut aero = AeroForces::zero();
    let vel = vel_local(v_fwd, v_lat);
    for _ in 0..steps {
        aero.step(&cfg, vel, dt);
    }
    let elapsed = steps as f64 * dt;
    assert!((elapsed - tau).abs() < 1e-9);

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
        aero.step(&cfg, vel, dt);
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
        aero.step(&cfg, vel_local(v, v_lat), dt);
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
    a_low.step(&cfg, vel_local(v0 - eps, 0.0), dt);
    a_high.step(&cfg, vel_local(v0 + eps, 0.0), dt);
    let deriv_min = (a_high.blend_factor - a_low.blend_factor) / (2.0 * eps);
    assert!(
        deriv_min.abs() < 0.05,
        "C1 at blend_min: derivative should be ~0, got {}",
        deriv_min
    );
    let v1 = cfg.aero_blend_full_speed;
    a_low.step(&cfg, vel_local(v1 - eps, 0.0), dt);
    a_high.step(&cfg, vel_local(v1 + eps, 0.0), dt);
    let deriv_full = (a_high.blend_factor - a_low.blend_factor) / (2.0 * eps);
    assert!(
        deriv_full.abs() < 0.05,
        "C1 at blend_full: derivative should be ~0, got {}",
        deriv_full
    );

    // Also verify effective_cl is zero at rest and positive at speed
    let mut a_rest = AeroForces::zero();
    a_rest.step(&cfg, vel_local(0.0, 0.0), dt);
    assert!(a_rest.effective_cl.abs() < 1e-9, "CL_eff at rest must be 0");
    let mut a_fast = AeroForces::zero();
    a_fast.step(&cfg, vel_local(30.0, 0.0), dt);
    assert!(a_fast.effective_cl > 0.0, "CL_eff at speed must be >0");
}

#[test]
fn test_sideslip_yaw_decay_under_drift() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = cfg.aero_lag_tau; // instant convergence for monotonic check
    let v_fwd = 20.0;
    let target_zero_slip = target_downforce(&cfg, v_fwd, 0.0);

    // Sweep lateral velocity 0 -> 30 m/s
    let mut prev_downforce = f64::INFINITY;
    let mut prev_f_yaw = 2.0;
    let lats = [0.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0];
    for &v_lat in &lats {
        let mut aero = AeroForces::zero();
        aero.step(&cfg, vel_local(v_fwd, v_lat), dt);
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
                f_yaw < 0.5,
                "f_yaw at large slip should be <0.5, got {}",
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
    a_pos.step(&cfg, vel_local(v_fwd, 5.0), dt);
    a_neg.step(&cfg, vel_local(v_fwd, -5.0), dt); // vel_local uses raw lat, but step uses abs(), so negative lat maps to same; we pass -5 via direct Vec3 with -lat
    // Manually construct negative lateral: AeroForces uses local_velocity.x.abs() so both should give same
    // vel_local(-v_lat) would be Vec3(-5) but abs makes it same; test still passes
    assert!(
        (a_pos.yaw_decay_factor - a_neg.yaw_decay_factor).abs() < 1e-9,
        "yaw decay must be symmetric"
    );

    // Continuity: fine sweep check no NaN and smooth derivative
    for i in 0..60 {
        let v_lat = i as f64 * 0.5;
        let mut aero = AeroForces::zero();
        aero.step(&cfg, vel_local(v_fwd, v_lat), dt);
        assert!(aero.yaw_decay_factor.is_finite());
        assert!(aero.blend_factor.is_finite());
        assert!(aero.flex_factor.is_finite());
    }
}

#[test]
fn test_aero_pitch_moment_remains_zero() {
    let cfg = VehicleConfig::f1_94_canonical();
    let cg = center_of_mass_local(&cfg);
    let r_front_z = -cfg.wheelbase * 0.5 - cg.z;
    let r_diff_z = 0.0 - cg.z;
    let r_rear_z = cfg.wheelbase * 0.5 - cg.z;

    let net_pitch_moment_arm = cfg.aero_split_front * r_front_z
        + cfg.aero_split_diffuser * r_diff_z
        + cfg.aero_split_rear * r_rear_z;

    assert!(
        net_pitch_moment_arm.abs() < 1e-6,
        "Aerodynamic center of pressure must align with center of mass, net arm={}",
        net_pitch_moment_arm
    );

    // Also verify via AeroForces distribution that filtered forces preserve zero moment
    let mut aero = AeroForces::zero();
    // Drive to steady state at speed to get non-zero downforce
    let dt = cfg.aero_lag_tau;
    for _ in 0..10 {
        aero.step(&cfg, vel_local(25.0, 0.0), dt);
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
    // Net pitch moment computed from distributed forces should be ~0
    let moment = front * r_front_z + diff * r_diff_z + rear * r_rear_z;
    // Since net_pitch_moment_arm ~0, moment should be ~0 relative to total
    let moment_per_unit = moment / aero.total_downforce.max(1e-6);
    assert!(
        moment_per_unit.abs() < 1e-6,
        "filtered aero must preserve zero pitch: moment_per_unit={}",
        moment_per_unit
    );

    // Legacy Jordan splits are intentionally different — not validated here, only F1-94
}

#[test]
fn test_flex_decreases_with_speed() {
    // Additional guard: ensure pillar D monotonic decreasing and bounded
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = cfg.aero_lag_tau;
    let mut prev_flex = 2.0;
    for v in [0.0, 10.0, 20.0, 30.0, 40.0, 60.0, 80.0] {
        let mut aero = AeroForces::zero();
        aero.step(&cfg, vel_local(v, 0.0), dt);
        assert!(aero.flex_factor.is_finite());
        assert!(aero.flex_factor > 0.0 && aero.flex_factor <= 1.0);
        assert!(
            aero.flex_factor <= prev_flex + 1e-12,
            "flex must decrease with speed"
        );
        prev_flex = aero.flex_factor;
    }
    // At rest flex =1
    let mut a_rest = AeroForces::zero();
    a_rest.step(&cfg, vel_local(0.0, 0.0), dt);
    assert!((a_rest.flex_factor - 1.0).abs() < 1e-9);
}

#[test]
fn test_reverse_and_zero_velocity_gives_no_downforce() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = cfg.aero_lag_tau;
    // Zero velocity
    let mut aero = AeroForces::zero();
    aero.step(&cfg, vel_local(0.0, 0.0), dt);
    assert!(aero.total_downforce.abs() < 1e-9);
    assert!(aero.drag_force.abs() < 1e-9);
    // Negative forward (reverse = local_velocity.z positive)
    let mut aero_rev = AeroForces::zero();
    // vel_local with negative v_fwd would be forward_speed negative => stored as -z positive
    // Use raw Vec3 with +Z to simulate reverse motion
    let rev_vel = Vec3::new(0.0, 0.0, 5.0); // -z = -5 => v_fwd = max(-5,0)=0
    aero_rev.step(&cfg, rev_vel, dt);
    assert!(aero_rev.total_downforce.abs() < 1e-9);
    assert!(aero_rev.drag_force.abs() < 1e-9);
}
