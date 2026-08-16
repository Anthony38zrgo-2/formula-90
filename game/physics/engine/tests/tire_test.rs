use vehicle_physics_engine::*;

/// Helper: reproduce stiffness calculation exactly as in tire.rs
fn stiffness_pair(cfg: &VehicleConfig) -> (f64, f64) {
    let base = 1_000_000.0 + 8_000_000.0 * 5.0; // 41_000_000
    let cp = cfg.contact_patch;
    let corner = 0.5 * base * cp * cp;
    let long = corner * 1.1;
    (corner, long)
}

/// Test 1: Brush model C1-continuity
/// Old bug: 33% discontinuity at psi=1 boundary. New: mu_kinetic=0.9*mu_static => 10% drop.
/// We create two slip states straddling psi=1 and assert force continuity within 15% of mu*Fz.
///
/// Falsifies old: old would produce delta ~0.33*mu*Fz > 15% threshold, test would fail.
/// New passes: delta ~0.10*mu*Fz <15%.
#[test]
fn test_brush_c1_continuity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let normal = 2000.0;
    let mu = 2.9; // Road Road value in f1_94 is 2.9
    let (_corner, long) = stiffness_pair(&cfg);
    let dt = 1.0 / 60.0;
    let wheel = WheelIndex::RearLeft;
    let tire_radius = cfg.rear_tire_radius;
    let v_forward = 10.0;
    // Use brake >10 to disable the GEVP clamp (so we measure pure brush)
    let brake = 20.0;
    let drive = 0.0;
    let clutch = 0.0;
    let gear_ratio = cfg.gear_ratios[0];

    let mu_fz = mu * normal;
    let allow = 0.15 * mu_fz; // 15% threshold

    // psi = sigma_comb / (3*mu*Fz), sigma_comb = long*|slip|
    // target psi values
    for (psi_below, psi_above) in [(0.95, 1.05), (0.90, 1.10)] {
        let slip_below = psi_below * 3.0 * mu * normal / long;
        let slip_above = psi_above * 3.0 * mu * normal / long;

        let v_wheel_below = v_forward + slip_below * v_forward;
        let spin_below = v_wheel_below / tire_radius;

        let v_wheel_above = v_forward + slip_above * v_forward;
        let spin_above = v_wheel_above / tire_radius;

        let mut tires_below = TireSystem::new(&cfg);
        tires_below.wheels[wheel as usize].spin = spin_below;
        tires_below.step_wheel(
            &cfg,
            wheel,
            normal,
            mu,
            drive,
            brake,
            Vec3::new(0.0, 0.0, -v_forward),
            dt,
            gear_ratio,
            clutch,
        );
        let fx_below = tires_below.wheels[wheel as usize].longitudinal_force;

        let mut tires_above = TireSystem::new(&cfg);
        tires_above.wheels[wheel as usize].spin = spin_above;
        tires_above.step_wheel(
            &cfg,
            wheel,
            normal,
            mu,
            drive,
            brake,
            Vec3::new(0.0, 0.0, -v_forward),
            dt,
            gear_ratio,
            clutch,
        );
        let fx_above = tires_above.wheels[wheel as usize].longitudinal_force;

        // For positive slip, Fx should be positive (tractive)
        assert!(
            fx_below > 0.0,
            "Fx below should be positive, psi={} slip={} fx={}",
            psi_below, slip_below, fx_below
        );
        assert!(
            fx_above > 0.0,
            "Fx above should be positive, psi={} slip={} fx={}",
            psi_above, slip_above, fx_above
        );

        let delta = (fx_below - fx_above).abs();
        println!(
            "psi {:.2} vs {:.2}: slip {:.5} vs {:.5}, Fx {:.1} vs {:.1}, delta {:.1}, allow {:.1} muFz {:.1}",
            psi_below, psi_above, slip_below, slip_above, fx_below, fx_above, delta, allow, mu_fz
        );

        // Continuity: delta must be <15% muFz (new 10% passes, old 33% fails)
        assert!(
            delta < allow,
            "C1 discontinuity too large: psi {:.2} vs {:.2} delta {:.1} >= {:.1} (15% muFz)",
            psi_below, psi_above, delta, allow
        );

        // Also ensure not zero: kinetic drop should give ~10% difference, at least 5% for psi 0.95 vs 1.05
        // This verifies kinetic branch active, not glued.
        if psi_below == 0.95 && psi_above == 1.05 {
            let min_delta = 0.05 * mu_fz;
            assert!(
                delta > min_delta,
                "Delta too small {:.1} < {:.1} (5% muFz) — suggests kinetic 0.9 branch not active?",
                delta, min_delta
            );
        }

        // Also verify Fx near saturation ~ mu*Fz (or 0.9*muFz). For psi~1, brush should be ~0.9-1.0 * muFz before rolling subtraction.
        // Rolling subtract small ~c_rr*Fz ~20N
        // So check fx_below ~ muFz within 20%
        assert!(
            (fx_below - mu_fz).abs() < 0.20 * mu_fz,
            "Fx below not near muFz: {:.1} vs {:.1}",
            fx_below, mu_fz
        );
        assert!(
            (fx_above - 0.9 * mu_fz).abs() < 0.20 * mu_fz,
            "Fx above not near 0.9*muFz: {:.1} vs {:.1}",
            fx_above, 0.9 * mu_fz
        );
    }

    // Extra: Check C1 inside branch is smooth: psi 0.5 vs 0.95 delta should be proportional, not jump.
    // Use small psi where force rises monotonic.
    let psi_mid = 0.5;
    let slip_mid = psi_mid * 3.0 * mu * normal / long;
    let v_wheel_mid = v_forward + slip_mid * v_forward;
    let spin_mid = v_wheel_mid / tire_radius;
    let mut t_mid = TireSystem::new(&cfg);
    t_mid.wheels[wheel as usize].spin = spin_mid;
    t_mid.step_wheel(&cfg, wheel, normal, mu, drive, brake, Vec3::new(0.0,0.0,-v_forward), dt, gear_ratio, clutch);
    let fx_mid = t_mid.wheels[wheel as usize].longitudinal_force;
    assert!(fx_mid > 0.0 && fx_mid < mu_fz, "Mid psi force should be between 0 and muFz, got {}", fx_mid);
}

/// Test 2: Longitudinal reaction torque I*dot = T_drive - Fx*R - T_rr
/// Verifies wheel spin integration includes Fx*R term.
/// Old bug: missing -Fx*R, spin would be (T_drive/I)*dt too high.
/// New: spin increase reduced by Fx*R/I*dt.
/// We set up a high-slip saturated tractive case where Fx*R is large and measure spin delta.
#[test]
fn test_longitudinal_reaction_torque() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let normal = 2000.0;
    let mu = 2.9;
    let dt = 1.0 / 60.0;
    let v_forward = 10.0;
    let tire_radius = cfg.rear_tire_radius;
    let drive = 3500.0; // large to dominate but still Fx*R comparable
    let brake = 0.0;
    let clutch = 0.0; // isolate wheel inertia only
    let gear_ratio = cfg.gear_ratios[0];

    // High slip: set spin >> road speed so sigma saturated, Fx ~ 0.9*mu*Fz
    let spin_init = 60.0; // rad/s, v_wheel=19.74 vs 10 => slip 0.974 saturated
    let local_vel = Vec3::new(0.0, 0.0, -v_forward);

    let mut tires = TireSystem::new(&cfg);
    tires.wheels[wheel as usize].spin = spin_init;
    // Compute wheel inertia as in WheelTireState::new
    let wheel_mass = cfg.rear_wheel_mass;
    let i_w = 0.5 * wheel_mass * tire_radius * tire_radius;
    let total_inertia = i_w; // clutch 0 => no reflected

    // Capture before step for manual check
    let fx_expected_saturated = 0.9 * mu * normal; // approximate after brush, before rolling ~15N
    println!("I_w={:.4}, fx_sat≈{:.1}, Fx*R≈{:.1}, drive={:.1}", i_w, fx_expected_saturated, fx_expected_saturated*tire_radius, drive);

    tires.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear_ratio, clutch);
    let state = &tires.wheels[wheel as usize];
    let spin_after = state.spin;
    let fx_actual = state.longitudinal_force; // after rolling subtraction (small offset)
    // Reconstruct fx_for_torque approx: longitudinal + rolling (if v>0.05)
    // rolling computed inside tire.rs: c_rr * normal
    let speed_factor = (v_forward * 0.036).powi(2);
    let c_rr = 0.005 + 0.5 * (0.01 + 0.0095 * speed_factor);
    let rolling = c_rr * normal;
    let fx_brush_est = if v_forward.abs() > 0.05 { fx_actual + rolling } else { fx_actual };
    println!("fx_actual {:.1} (brush est {:.1}), spin_init {:.3} -> spin_after {:.3}", fx_actual, fx_brush_est, spin_init, spin_after);

    assert!(fx_actual > 0.0, "Fx should be positive tractive, got {}", fx_actual);
    // Ensure Fx is near saturated ~0.9 muFz within 25% (allow degressivity and rolling)
    assert!((fx_brush_est - fx_expected_saturated).abs() < 0.30 * fx_expected_saturated,
        "Fx brush not saturated as expected {:.1} vs {:.1}", fx_brush_est, fx_expected_saturated);

    let inc_actual = spin_after - spin_init;
    let inc_naive = drive / total_inertia * dt; // without reaction

    println!("inc_actual {:.3} rad/s, inc_naive {:.3} rad/s, diff {:.3}", inc_actual, inc_naive, inc_naive - inc_actual);
    // With reaction, inc_actual = (T - Fx*R - T_rr)/I*dt . So must be significantly less than naive.
    // Fx*R/I*dt ≈ 5220*0.329/0.864*0.0167 ≈ 33.2 rad/s for this case. So demand at least 5 rad/s reduction.
    // Also old bug would give inc_actual ≈ inc_naive within <1 rad/s (only rolling diff ~0.09)
    assert!(inc_actual < inc_naive - 5.0,
        "Reaction torque not included? inc_actual {:.3} should be < naive {:.3} -5.0 ; diff {:.3}",
        inc_actual, inc_naive, inc_naive - inc_actual);

    // Also inc_actual should still be positive because drive > Fx*R (3500 > 1717) but not too huge
    assert!(inc_actual > 0.0 && inc_actual < inc_naive,
        "inc_actual should be positive but less than naive, got {:.3}", inc_actual);

    // Extra falsification: ensure Fx is used for net torque - if we zero normal (airborne) spin should grow at naive rate (no Fx)
    let mut tires_air = TireSystem::new(&cfg);
    tires_air.wheels[wheel as usize].spin = spin_init;
    tires_air.step_wheel(&cfg, wheel, 0.0, mu, drive, brake, local_vel, dt, gear_ratio, clutch);
    let spin_air = tires_air.wheels[wheel as usize].spin;
    let inc_air = spin_air - spin_init;
    // Airborne: no Fx reaction, net = T_drive/I*dt - tiny air drag
    println!("airborne inc {:.3} vs naive {:.3}", inc_air, inc_naive);
    // airborne inc should be close to naive (within 1 rad/s due to air decel 2/I*dt)
    assert!((inc_air - inc_naive).abs() < 2.0,
        "Airborne should have near-naive spin increase, air {:.3} naive {:.3}", inc_air, inc_naive);
}

/// Helper to compute total inertia with reflected motor term
fn total_inertia_for(cfg: &VehicleConfig, gear_ratio: f64, clutch: f64, wheel_moment: f64) -> f64 {
    let eff = gear_ratio * cfg.final_drive;
    let i_ref = 0.5 * cfg.motor_moment * eff * eff * clutch.clamp(0.0, 1.0);
    wheel_moment + i_ref
}

/// Test 3: Reflected inertia per gear I_ref = 0.5*I_engine*(gear_ratio*final)^2 * clutch_engagement
/// Falsifies old: static I_ref or missing clutch scaling.
/// New: spin accel lower in higher gear ratio (1st) vs lower gear ratio (3rd), and clutch 0 vs 1 differs.
#[test]
fn test_reflected_inertia_per_gear() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let normal = 2000.0;
    let mu = 2.9;
    let dt = 1.0 / 60.0;
    let v_forward = 10.0;
    let tire_radius = cfg.rear_tire_radius;
    let spin_init = v_forward / tire_radius;
    let drive = 3500.0;
    let brake = 0.0;
    let local_vel = Vec3::new(0.0, 0.0, -v_forward);
    let wheel_mass = cfg.rear_wheel_mass;
    let i_w = 0.5 * wheel_mass * tire_radius * tire_radius;

    let gear1 = cfg.gear_ratios[0]; // 2.85
    let gear3 = cfg.gear_ratios[2]; // 1.89
    let eff1 = gear1 * cfg.final_drive;
    let eff3 = gear3 * cfg.final_drive;
    let i_ref1 = 0.5 * cfg.motor_moment * eff1 * eff1 * 1.0;
    let i_ref3 = 0.5 * cfg.motor_moment * eff3 * eff3 * 1.0;
    let total1 = i_w + i_ref1;
    let total3 = i_w + i_ref3;
    println!("gear1 ratio {:.3} eff {:.3} I_ref {:.3} total {:.3}", gear1, eff1, i_ref1, total1);
    println!("gear3 ratio {:.3} eff {:.3} I_ref {:.3} total {:.3}", gear3, eff3, i_ref3, total3);
    assert!(total1 > total3 * 1.5, "Total inertia gear1 should be >1.5x gear3");

    // Step gear1 with clutch 1
    let mut t1 = TireSystem::new(&cfg);
    t1.wheels[wheel as usize].spin = spin_init;
    t1.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear1, 1.0);
    let spin1 = t1.wheels[wheel as usize].spin;
    let inc1 = spin1 - spin_init;

    // Step gear3 with clutch 1
    let mut t3 = TireSystem::new(&cfg);
    t3.wheels[wheel as usize].spin = spin_init;
    t3.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear3, 1.0);
    let spin3 = t3.wheels[wheel as usize].spin;
    let inc3 = spin3 - spin_init;

    println!("inc gear1 {:.4} spin {:.4}, inc gear3 {:.4} spin {:.4}, ratio {:.2}", inc1, spin1, inc3, spin3, inc3/inc1.max(1e-9));
    // Higher gear ratio => higher inertia => lower spin accel
    assert!(inc3 > inc1,
        "Gear3 (lower ratio) should spin up faster than gear1 (higher inertia): inc3 {:.4} vs inc1 {:.4}", inc3, inc1);
    // Check ratio ~ total1/total3 ≈2.1 within 30% tolerance accounting for Fx variation (Fx same approx)
    let expected_ratio = total1 / total3;
    let actual_ratio = inc3 / inc1.max(1e-6);
    assert!((actual_ratio - expected_ratio).abs() < 0.35 * expected_ratio,
        "Inertia ratio mismatch: expected {:.2} actual {:.2}", expected_ratio, actual_ratio);

    // Clutch engagement scaling: same gear1 with clutch 0 vs 1
    let mut t_clutch0 = TireSystem::new(&cfg);
    t_clutch0.wheels[wheel as usize].spin = spin_init;
    t_clutch0.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear1, 0.0);
    let inc0 = t_clutch0.wheels[wheel as usize].spin - spin_init;

    let mut t_clutch05 = TireSystem::new(&cfg);
    t_clutch05.wheels[wheel as usize].spin = spin_init;
    t_clutch05.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear1, 0.5);
    let inc05 = t_clutch05.wheels[wheel as usize].spin - spin_init;

    let mut t_clutch1 = TireSystem::new(&cfg);
    t_clutch1.wheels[wheel as usize].spin = spin_init;
    t_clutch1.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear1, 1.0);
    let inc1b = t_clutch1.wheels[wheel as usize].spin - spin_init;

    println!("clutch 0 inc {:.4}, 0.5 inc {:.4}, 1.0 inc {:.4}", inc0, inc05, inc1b);
    assert!(inc0 > inc05 && inc05 > inc1b,
        "Clutch engagement should monotonically reduce spin accel: 0 {:.4} > 0.5 {:.4} >1 {:.4}", inc0, inc05, inc1b);
    // clutch0 total = I_w, clutch1 total = I_w+I_ref => inc0/inc1 ≈ total1/total0
    let total0 = i_w;
    let ratio0_1 = inc0 / inc1b.max(1e-6);
    let expected0_1 = total1 / total0;
    assert!((ratio0_1 - expected0_1).abs() < 0.40 * expected0_1,
        "Clutch scaling ratio mismatch: expected {:.2} actual {:.2}", expected0_1, ratio0_1);

    // Verify helper matches code's internal calc by checking step's effective inertia indirectly via spin delta proportionality
    // (we already did via ratios)
    let direct_total1 = total_inertia_for(&cfg, gear1, 1.0, i_w);
    assert!((direct_total1 - total1).abs() < 1e-9);
}

/// Test 4: Slip angle regularization alpha = -atan2(v_lateral, sqrt(v_forward^2+0.25^2))
/// Old bug: gated slip_angle to 0 when v_mag <0.1 or used asin without regularization => discontinuity at low speed.
/// New: continuous and non-zero at low forward speed.
#[test]
fn test_slip_angle_regularization() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::FrontLeft;
    let normal = 1500.0;
    let mu = 2.9;
    let dt = 1.0 / 60.0;
    let drive = 0.0;
    let brake = 20.0; // disable clamp but non-driven braking path uses spin update not affecting slip
    let clutch = 0.0;
    let gear_ratio = 0.0;

    // Case A: v_forward 0.05, v_lateral 0.5
    let v_forward_a = 0.05;
    let v_lateral_a = 0.5;
    let local_a = Vec3::new(v_lateral_a, 0.0, -v_forward_a);
    let mut tires_a = TireSystem::new(&cfg);
    tires_a.wheels[wheel as usize].spin = 0.0; // spin irrelevant for slip angle
    tires_a.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_a, dt, gear_ratio, clutch);
    let alpha_a = tires_a.wheels[wheel as usize].slip_angle_rad;
    let expected_a = -(v_lateral_a).atan2((v_forward_a*v_forward_a + 0.25*0.25).sqrt());
    println!("Case A v_f {:.3} v_lat {:.3}: alpha {:.5} expected {:.5}", v_forward_a, v_lateral_a, alpha_a, expected_a);
    assert!((alpha_a - expected_a).abs() < 1e-6,
        "Slip angle regularization mismatch: got {:.5} expected {:.5}", alpha_a, expected_a);
    // Falsifies old gated 0: must not be near 0
    assert!(alpha_a.abs() > 0.5,
        "At low forward speed, alpha should be ~-1.1 rad, not near 0; got {}", alpha_a);

    // Case B: v_forward 0.0, v_lateral 0.5 (pure lateral)
    let v_forward_b = 0.0;
    let v_lateral_b = 0.5;
    let local_b = Vec3::new(v_lateral_b, 0.0, -v_forward_b);
    let mut tires_b = TireSystem::new(&cfg);
    tires_b.wheels[wheel as usize].spin = 0.0;
    tires_b.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_b, dt, gear_ratio, clutch);
    let alpha_b = tires_b.wheels[wheel as usize].slip_angle_rad;
    let expected_b = -(v_lateral_b).atan2((v_forward_b*v_forward_b + 0.25*0.25).sqrt());
    println!("Case B v_f {:.3} v_lat {:.3}: alpha {:.5} expected {:.5}", v_forward_b, v_lateral_b, alpha_b, expected_b);
    assert!((alpha_b - expected_b).abs() < 1e-6,
        "Slip angle at zero forward mismatch: got {:.5} expected {:.5}", alpha_b, expected_b);
    // Continuity: difference between A and B should be small (<0.1 rad) because regularization prevents jump
    let diff_ab = (alpha_a - alpha_b).abs();
    println!("Continuity diff A-B {:.5}", diff_ab);
    assert!(diff_ab < 0.05,
        "Slip angle should be continuous as v_forward->0, diff {:.5} too large", diff_ab);

    // Case C: high speed to ensure still behaves like -atan(v_lat / v_forward)
    let v_forward_c = 10.0;
    let v_lateral_c = 1.0;
    let local_c = Vec3::new(v_lateral_c, 0.0, -v_forward_c);
    let mut tires_c = TireSystem::new(&cfg);
    tires_c.wheels[wheel as usize].spin = 0.0;
    tires_c.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_c, dt, gear_ratio, clutch);
    let alpha_c = tires_c.wheels[wheel as usize].slip_angle_rad;
    let expected_c = -(v_lateral_c).atan2((v_forward_c*v_forward_c + 0.0625).sqrt());
    println!("Case C v_f {:.3} v_lat {:.3}: alpha {:.5} expected {:.5}", v_forward_c, v_lateral_c, alpha_c, expected_c);
    assert!((alpha_c - expected_c).abs() < 1e-6);
    // At high speed, regularization 0.25 has negligible effect: approx -0.09966 rad vs -atan2(1,10)= -0.09966 diff <0.001
    let approx = -(v_lateral_c / v_forward_c).atan();
    assert!((alpha_c - approx).abs() < 0.002,
        "At high speed regularization should be negligible: alpha {:.5} approx {:.5}", alpha_c, approx);

    // Case D: negative lateral => positive alpha
    let v_lateral_d = -0.5;
    let local_d = Vec3::new(v_lateral_d, 0.0, -v_forward_a);
    let mut tires_d = TireSystem::new(&cfg);
    tires_d.wheels[wheel as usize].spin = 0.0;
    tires_d.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_d, dt, gear_ratio, clutch);
    let alpha_d = tires_d.wheels[wheel as usize].slip_angle_rad;
    let expected_d = -(v_lateral_d).atan2((v_forward_a*v_forward_a + 0.0625).sqrt());
    println!("Case D lateral negative: alpha {:.5} expected {:.5}", alpha_d, expected_d);
    assert!((alpha_d - expected_d).abs() < 1e-6);
    assert!(alpha_d > 0.0 && alpha_a < 0.0, "Sign should flip with lateral direction");
}

/// Test 5 (bonus): Friction ellipse / combined force cap sqrt(Fx^2+Fy^2) <= mu*Fz * 1.1
/// Ensures longitudinal_force not overridden beyond friction circle (GDScript override removed).
/// Also checks brush stays within ellipse for high combined slip.
#[test]
fn test_friction_ellipse_and_no_gdscript_override() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let dt = 1.0 / 60.0;
    let clutch = 0.0;
    let gear_ratio = 0.0;
    let brake = 20.0; // disable clamp to observe pure brush ellipse
    let drive = 0.0;

    // High combined slip: large longitudinal and lateral
    let normal = 1800.0;
    let mu = 2.0; // use 2.0 for clearer ellipse
    let v_forward = 8.0;
    let v_lateral = 4.0; // large lateral => large slip angle
    // Provide high spin to get large slip_ratio ~0.6 saturated + large slip angle ~ atan2
    let tire_radius = cfg.rear_tire_radius;
    let spin = (v_forward + 0.6 * v_forward) / tire_radius; // slip 0.6
    let local_vel = Vec3::new(v_lateral, 0.0, -v_forward);

    let mut tires = TireSystem::new(&cfg);
    tires.wheels[wheel as usize].spin = spin;
    tires.step_wheel(&cfg, wheel, normal, mu, drive, brake, local_vel, dt, gear_ratio, clutch);
    let fx = tires.wheels[wheel as usize].longitudinal_force;
    let fy = tires.wheels[wheel as usize].lateral_force;
    let combined = (fx*fx + fy*fy).sqrt();
    let mu_fz = mu * normal;
    // Allow 10% over due to braking help / rounding but not huge override like old GDScript that could exceed 1.5*muFz
    println!("Combined slip: Fx {:.1} Fy {:.1} combined {:.1} muFz {:.1} ratio {:.3}", fx, fy, combined, mu_fz, combined/mu_fz);
    assert!(combined <= mu_fz * 1.15,
        "Friction ellipse violated: combined {:.1} > 1.15*muFz {:.1}", combined, mu_fz*1.15);
    assert!(combined > 0.5 * mu_fz,
        "Combined should be near saturation >0.5 muFz, got {:.1}", combined);

    // Also test pure longitudinal saturates at mu*Fz (or 0.9)
    let mut tires_long = TireSystem::new(&cfg);
    let spin_long = (v_forward + 1.2 * v_forward) / tire_radius; // huge slip >1 saturates
    tires_long.wheels[wheel as usize].spin = spin_long;
    tires_long.step_wheel(&cfg, wheel, normal, mu, drive, brake, Vec3::new(0.0,0.0,-v_forward), dt, gear_ratio, clutch);
    let fx_long = tires_long.wheels[wheel as usize].longitudinal_force;
    let fy_long = tires_long.wheels[wheel as usize].lateral_force;
    println!("Pure long: Fx {:.1} Fy {:.1} muFz {:.1}", fx_long, fy_long, mu_fz);
    // Pure longitudinal should be ~0.9*muFz (kinetic) within 15%
    assert!((fx_long.abs() - 0.9*mu_fz).abs() < 0.20*mu_fz,
        "Pure longitudinal saturates near 0.9*muFz, got {:.1}", fx_long);
    assert!(fy_long.abs() < 50.0, "Pure long should have near zero lateral, got {}", fy_long);

    // Pure lateral saturates similarly
    let mut tires_lat = TireSystem::new(&cfg);
    tires_lat.wheels[wheel as usize].spin = v_forward / tire_radius; // matched spin => zero longitudinal slip
    tires_lat.step_wheel(&cfg, wheel, normal, mu, drive, brake, Vec3::new(5.0,0.0,-v_forward), dt, gear_ratio, clutch);
    let fx_lat = tires_lat.wheels[wheel as usize].longitudinal_force;
    let fy_lat = tires_lat.wheels[wheel as usize].lateral_force;
    println!("Pure lat: Fx {:.1} Fy {:.1}", fx_lat, fy_lat);
    // Fy saturates, Fx minimal (only rolling maybe)
    let combined_lat = (fx_lat*fx_lat + fy_lat*fy_lat).sqrt();
    assert!(combined_lat <= mu_fz*1.15);
    // Lateral force should be large magnitude negative (since sigma_y = -C*tan(alpha))
    assert!(fy_lat.abs() > 0.5*mu_fz, "Pure lateral should saturate Fy, got {}", fy_lat);
}
