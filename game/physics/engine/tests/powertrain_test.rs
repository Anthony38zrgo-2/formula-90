use vehicle_physics_engine::*;

/// Helper to simulate a flat ground (plane at Y = 0.0) with accurate wheel hub sampling.
fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for i in 0..4 {
        let wheel = WheelIndex::ALL[i];
        let hub_local = sim.config.wheel_anchor_local(wheel);
        let hub_world = sim.state.transform.transform_point(hub_local);

        let tire_w = if wheel.is_front() { sim.config.front_tire_width } else { sim.config.rear_tire_width };
        let span = tire_w * sim.config.tri_ray_spacing_ratio;

        let left_offset = sim.state.transform.basis.transform_vector(Vec3::new(-span, 0.0, 0.0));
        let right_offset = sim.state.transform.basis.transform_vector(Vec3::new(span, 0.0, 0.0));

        let p_in = hub_world + left_offset;
        let p_mid = hub_world;
        let p_out = hub_world + right_offset;

        samples[i] = TriRaycastSample {
            inner: RaycastHit {
                is_colliding: true,
                distance: p_in.y.max(0.0),
                point: Vec3::new(p_in.x, 0.0, p_in.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            center: RaycastHit {
                is_colliding: true,
                distance: p_mid.y.max(0.0),
                point: Vec3::new(p_mid.x, 0.0, p_mid.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            outer: RaycastHit {
                is_colliding: true,
                distance: p_out.y.max(0.0),
                point: Vec3::new(p_out.x, 0.0, p_out.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
        };
    }
    samples
}

fn full_throttle_input() -> VehicleInput {
    VehicleInput {
        throttle: 1.0,
        steering: 0.0,
        brake: 0.0,
        handbrake: 0.0,
        clutch: 0.0,
        gear_request: None,
    }
}

fn spawn_sim() -> VehicleSimulator {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_height = cfg.front_tire_radius + cfg.front_spring_length * (1.0 - cfg.front_resting_ratio);
    VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0)
}

/// Counts consecutive-frame RPM delta sign flips (an oscillation signature)
/// and reports the maximum per-step delta magnitude over non-skipped frames.
/// Small deltas (|d| <= min_delta) are ignored; frames flagged by `skip` are excluded.
fn count_rpm_flips(rpm_log: &[f64], skip: &[bool], min_delta: f64) -> (usize, f64) {
    let mut flips = 0usize;
    let mut max_delta = 0.0f64;
    let mut prev_d: Option<f64> = None;
    for i in 1..rpm_log.len() {
        if skip[i] || skip[i - 1] {
            prev_d = None;
            continue;
        }
        let d = rpm_log[i] - rpm_log[i - 1];
        if d.abs() > min_delta {
            max_delta = max_delta.max(d.abs());
            if let Some(p) = prev_d {
                if p.abs() > min_delta && p * d < 0.0 {
                    flips += 1;
                }
            }
            prev_d = Some(d);
        }
    }
    (flips, max_delta)
}

/// Launch from standstill must not exhibit the old 12 Hz limit-cycle oscillation.
/// Checks the spec's 0-45 km/h launch window and the final second of the run
/// (excluding legitimate gearshift transients). Both the flip count and the
/// per-step RPM-delta amplitude are bounded.
#[test]
fn test_launch_no_limit_cycle() {
    let mut sim = spawn_sim();
    let dt = 1.0 / 60.0;
    let steps = 180; // 3.0 s

    let mut rpm_log: Vec<f64> = Vec::with_capacity(steps);
    let mut speed_log: Vec<f64> = Vec::with_capacity(steps);
    let mut gear_log: Vec<i8> = Vec::with_capacity(steps);
    let mut shift_timer_log: Vec<f64> = Vec::with_capacity(steps);

    for _ in 0..steps {
        let input = full_throttle_input();
        let samples = flat_ground_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        rpm_log.push(telem.rpm);
        speed_log.push(telem.speed_kmh);
        gear_log.push(telem.gear);
        shift_timer_log.push(sim.state.powertrain.shift_timer);
    }

    // Frames inside a gearshift transient: shift in progress, or within 8 frames
    // (0.133 s) after a completed gear change (clutch resync period).
    let mut skip = vec![false; steps];
    let mut gear_change_frame: Option<usize> = None;
    for i in 0..steps {
        if shift_timer_log[i] > 0.0 {
            skip[i] = true;
        }
        if i > 0 && gear_log[i] != gear_log[i - 1] {
            gear_change_frame = Some(i);
        }
        if let Some(g) = gear_change_frame {
            if i >= g && i <= g + 8 {
                skip[i] = true;
            }
        }
    }

    // Spec launch window: 0 -> 45 km/h
    let mut launch_end = 0usize;
    for i in 0..steps {
        if speed_log[i] <= 45.0 {
            launch_end = i;
        }
    }
    let launch_slice = &rpm_log[..=launch_end];
    let launch_skip = &skip[..=launch_end];
    let (launch_flips, launch_max_delta) = count_rpm_flips(launch_slice, launch_skip, 50.0);

    // Final 1.0 s (2.0 s - 3.0 s)
    let final_slice = &rpm_log[steps - 61..];
    let final_skip = &skip[steps - 61..];
    let (final_flips, final_max_delta) = count_rpm_flips(final_slice, final_skip, 50.0);

    let final_speed = *speed_log.last().unwrap();
    let speed_at_2s = speed_log[120];
    println!(
        "Launch test: final_speed={:.1} km/h, flips(0-45)={} max_d={:.0}, flips(final 1s)={} max_d={:.0}, speed_growth_last_1s={:.1} km/h",
        final_speed, launch_flips, launch_max_delta, final_flips, final_max_delta, final_speed - speed_at_2s
    );

    assert!(launch_flips <= 3, "RPM oscillated in 0-45 km/h launch window: {} flips", launch_flips);
    assert!(final_flips <= 3, "RPM oscillated in final second outside shifts: {} flips", final_flips);
    // Amplitude guard: legitimate gear-pull steps stay below ~350 rpm/step at 60 Hz;
    // the old 12 Hz limit cycle swung hundreds-to-thousands of rpm per step.
    assert!(launch_max_delta <= 600.0, "RPM delta amplitude {:.0} too large in launch window", launch_max_delta);
    assert!(final_max_delta <= 600.0, "RPM delta amplitude {:.0} too large in final second", final_max_delta);
    assert!(final_speed > 30.0, "Car should be above ~30 km/h after 3 s launch, got {:.1}", final_speed);
    assert!(final_speed - speed_at_2s > 5.0, "Speed must still be growing in the final second ({} -> {})", speed_at_2s, final_speed);
}

/// Automatic upshifts must respect per-gear true ground speed gates.
#[test]
fn test_upshift_speed_gates() {
    const THRESHOLDS_KMH: [f64; 5] = [75.0, 105.0, 130.0, 160.0, 190.0];

    let mut sim = spawn_sim();
    let dt = 1.0 / 60.0;
    let steps = 360; // 6.0 s

    let mut upshifts: Vec<(i8, f64)> = Vec::new();
    let mut last_gear = sim.state.powertrain.current_gear;

    for _ in 0..steps {
        let input = full_throttle_input();
        let samples = flat_ground_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        // Only record true upshifts (ignore any downshifts / neutral events).
        if telem.gear > last_gear {
            upshifts.push((telem.gear, telem.speed_kmh));
            println!("Upshift to gear {} at {:.1} km/h", telem.gear, telem.speed_kmh);
        }
        last_gear = telem.gear;
    }

    assert!(
        upshifts.len() >= 3,
        "Expected at least 3 upshifts in 6 s of full throttle, got {}",
        upshifts.len()
    );

    for (gear, speed) in &upshifts {
        // gear >= 2 for a real upshift; threshold index = gear - 2
        let idx = (gear - 1) as usize;
        assert!(idx >= 1 && idx - 1 < THRESHOLDS_KMH.len(), "Unexpected upshift to gear {}", gear);
        let threshold = THRESHOLDS_KMH[idx - 1];
        assert!(
            *speed >= threshold - 0.5,
            "Upshift to gear {} at {:.1} km/h, expected >= {:.1} km/h",
            gear,
            speed,
            threshold - 0.5
        );
    }

    // Explicit 1->2 assertion per spec (>= 74.5 km/h)
    if let Some((gear, speed)) = upshifts.first() {
        assert_eq!(*gear, 2, "First recorded upshift should be 1->2, got gear {}", gear);
        assert!(*speed >= 74.5, "1->2 upshift at {:.1} km/h, expected >= 74.5 km/h", speed);
    }
}

/// Ignition cut (item 1.2) must throttle the engine down to 5% during upshifts,
/// preventing the 17,500 RPM rev-limiter bounce signature.
#[test]
fn test_upshift_ignition_cut_prevents_limiter_bounce() {
    let mut sim = spawn_sim();
    let dt = 1.0 / 60.0;
    let steps = 360; // 6.0 s

    let mut upshift_count = 0usize;
    let mut in_upshift = false;
    let mut shift_start_rpm: f64 = 0.0;
    let mut prev_rpm: f64 = 0.0;
    let mut cut_violations = 0usize;
    let mut rpm_rise_violations = 0usize;
    let mut bad_shift_ends = 0usize;

    for _ in 0..steps {
        let input = full_throttle_input();
        let samples = flat_ground_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        let st = &sim.state.powertrain;

        let upshifting = st.shift_timer > 0.0 && st.target_gear > st.current_gear;
        if upshifting && !in_upshift {
            in_upshift = true;
            shift_start_rpm = telem.rpm;
        }
        if upshifting {
            if st.engine_torque > 17.5 {
                cut_violations += 1;
            }
            // Limiter bounce signature: RPM climbing into the rev-limiter band during the cut.
            if telem.rpm - prev_rpm > 5.0 && telem.rpm >= 16900.0 {
                rpm_rise_violations += 1;
            }
        } else if in_upshift {
            upshift_count += 1;
            if telem.rpm >= shift_start_rpm - 100.0 {
                bad_shift_ends += 1;
            }
            in_upshift = false;
        }
        prev_rpm = telem.rpm;
    }

    println!(
        "Ignition cut: upshifts={}, cut_violations={}, rpm_rise_violations={}, bad_shift_ends={}",
        upshift_count, cut_violations, rpm_rise_violations, bad_shift_ends
    );
    assert!(upshift_count >= 2, "Expected at least 2 upshifts in 6 s, got {}", upshift_count);
    assert_eq!(cut_violations, 0, "Engine torque not cut to <=5% during upshifts");
    assert_eq!(rpm_rise_violations, 0, "RPM rose during ignition-cut upshifts (limiter bounce)");
    assert_eq!(bad_shift_ends, 0, "RPM did not fall during upshift");
}
