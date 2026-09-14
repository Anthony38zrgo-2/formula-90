// GRIP-01/GRIP-02 acceptance tests over the shipped f1_2030 runtime profile.
//
// The unit tests in `tire.rs` lock the generic machinery; this suite proves the
// actual calibrated profile declares and consumes the transition parameters,
// that the applied force respects the shared friction budget with the smoothing
// band enabled, that the sliding memory is per-axis and frequency consistent,
// and that zero taus reproduce the pure axis envelope under asymmetric slip.
use std::path::Path;
use vehicle_physics_engine::*;

const ROAD_FRICTION: f64 = 2.7;
const ROAD_STIFFNESS: f64 = 7.2;
const FORWARD_SPEED: f64 = 20.0;

fn f1_2030_config() -> VehicleConfig {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("vehicles")
        .join("f1_2030")
        .join("f1_2030_v10_physics.json");
    VehicleConfig::from_json_path(&path).expect("f1_2030 runtime profile must load")
}

struct WheelRun {
    fx: f64,
    fy: f64,
    rolling_resistance: f64,
    mu: f64,
    demand: f64,
    utilization: f64,
}

fn run_wheel(
    cfg: &VehicleConfig,
    wheel: WheelIndex,
    fz: f64,
    alpha: f64,
    kappa: f64,
    ticks: usize,
    dt: f64,
) -> WheelRun {
    let mut tires = TireSystem::new(cfg);
    tires.set_mechanical_state(cfg, wheel, fz, 0.0, 0.008, 1.0);
    let radius = tires.wheels[wheel as usize]
        .effective_rolling_radius
        .max(0.05);
    tires.wheels[wheel as usize].spin = FORWARD_SPEED * (1.0 + kappa) / radius;
    let vel = Vec3::new(-(alpha.tan()) * FORWARD_SPEED, 0.0, -FORWARD_SPEED);
    for _ in 0..ticks {
        tires.process_wheel_forces(
            cfg,
            wheel,
            fz,
            SurfaceType::Road,
            ROAD_FRICTION,
            ROAD_STIFFNESS,
            1.0,
            false,
            vel,
            dt,
        );
    }
    let s = &tires.wheels[wheel as usize];
    WheelRun {
        fx: s.longitudinal_force,
        fy: s.lateral_force,
        rolling_resistance: s.rolling_resistance,
        mu: ROAD_FRICTION
            * s.load_sensitivity_scale
            * s.mechanical_modifiers.grip_scale.clamp(0.05, 2.0),
        demand: s.combined_demand,
        utilization: s.combined_utilization,
    }
}

/// Brakes the wheel from `initial_speed_ms` rolling speed and returns the spin
/// trace. `blend = 0` uses the legacy hard-stick model.
fn run_wheel_lock(
    cfg: &VehicleConfig,
    blend: f64,
    stick_tau: f64,
    initial_speed_ms: f64,
    brake_torque_nm: f64,
    ticks: usize,
    dt: f64,
) -> Vec<f64> {
    let mut cfg = cfg.clone();
    cfg.wheel_lock_blend_spin_rad_s = blend;
    cfg.wheel_lock_stick_tau_s = stick_tau;
    let wheel = WheelIndex::FrontLeft;
    let fz = reference_load_n(&cfg, wheel);
    let mut tires = TireSystem::new(&cfg);
    tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
    let radius = tires.wheels[wheel as usize]
        .effective_rolling_radius
        .max(0.05);
    tires.wheels[wheel as usize].spin = initial_speed_ms / radius;
    let vel = Vec3::new(0.0, 0.0, -FORWARD_SPEED);
    let mut out = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        tires.process_wheel_torque(&cfg, wheel, 0.0, 0.0, brake_torque_nm, dt);
        tires.process_wheel_forces(
            &cfg,
            wheel,
            fz,
            SurfaceType::Road,
            ROAD_FRICTION,
            ROAD_STIFFNESS,
            1.0,
            false,
            vel,
            dt,
        );
        out.push(tires.wheels[wheel as usize].spin);
    }
    out
}

/// Runs `phase_a_ticks` of deep lateral slip (0.30 rad) followed by
/// `phase_b_ticks` of zero lateral slip and returns `(fy, mem_lat, mem_lon)`.
fn run_lateral_slide_recovery(
    cfg: &VehicleConfig,
    wheel: WheelIndex,
    fz: f64,
    phase_a_ticks: usize,
    phase_b_ticks: usize,
    dt: f64,
) -> Vec<(f64, f64, f64)> {
    let mut tires = TireSystem::new(cfg);
    tires.set_mechanical_state(cfg, wheel, fz, 0.0, 0.008, 1.0);
    let radius = tires.wheels[wheel as usize]
        .effective_rolling_radius
        .max(0.05);
    tires.wheels[wheel as usize].spin = FORWARD_SPEED / radius;
    let alpha = 0.30_f64;
    let deep = Vec3::new(-(alpha.tan()) * FORWARD_SPEED, 0.0, -FORWARD_SPEED);
    let grip = Vec3::new(0.0, 0.0, -FORWARD_SPEED);
    let mut out = Vec::with_capacity(phase_a_ticks + phase_b_ticks);
    for i in 0..(phase_a_ticks + phase_b_ticks) {
        let vel = if i < phase_a_ticks { deep } else { grip };
        tires.process_wheel_forces(
            cfg,
            wheel,
            fz,
            SurfaceType::Road,
            ROAD_FRICTION,
            ROAD_STIFFNESS,
            1.0,
            false,
            vel,
            dt,
        );
        let s = &tires.wheels[wheel as usize];
        out.push((
            s.lateral_force,
            s.post_peak_decay_lat,
            s.post_peak_decay_lon,
        ));
    }
    out
}

#[test]
fn f1_2030_profile_declares_grip_transition_parameters() {
    let cfg = f1_2030_config();
    let front = cfg.front_tire_force;
    assert!((front.budget_blend_width - 0.1).abs() < 1e-12);
    assert!((front.falloff_sharpness - 2.0).abs() < 1e-12);
    assert!((front.slip_loss_tau_s - 0.06).abs() < 1e-12);
    assert!((front.slip_recovery_tau_s - 0.25).abs() < 1e-12);
    assert!((front.lateral_slide_mu_ratio - 0.88).abs() < 1e-12);
    assert!((front.longitudinal_slide_mu_ratio - 0.88).abs() < 1e-12);
    assert_eq!(cfg.rear_tire_force, front);
    assert!((cfg.wheel_lock_blend_spin_rad_s - 8.0).abs() < 1e-12);
    assert!((cfg.wheel_lock_stick_tau_s - 0.02).abs() < 1e-12);
}

#[test]
fn f1_2030_wheel_lock_is_regularized_and_continuous() {
    let cfg = f1_2030_config();
    let dt = 1.0 / 120.0;
    // One wheel taking the full brake torque guarantees lock-up, so the test
    // exercises the regularization mechanics instead of the car's lock limit.
    let legacy = run_wheel_lock(&cfg, 0.0, 0.0, 20.0, 3000.0, 240, dt);
    let reg = run_wheel_lock(&cfg, 8.0, 0.02, 20.0, 3000.0, 240, dt);

    let first_stopped = |seq: &[f64]| seq.iter().position(|s| s.abs() < 0.5);
    let legacy_stop = first_stopped(&legacy).expect("legacy must lock the wheel");
    let reg_stop = first_stopped(&reg).expect("regularized must lock the wheel");
    assert!(
        legacy_stop <= 239 && reg_stop <= legacy_stop + 30,
        "regularized lock must not be much slower: legacy={legacy_stop} reg={reg_stop}"
    );
    let legacy_snap = legacy
        .windows(2)
        .any(|p| p[0].abs() > 0.5 && p[1] == 0.0);
    let reg_snap = reg.windows(2).any(|p| p[0].abs() > 0.5 && p[1] == 0.0);
    assert!(legacy_snap, "legacy is expected to teleport to lock");
    assert!(!reg_snap, "regularized lock must not teleport to zero");
    assert!(
        reg.iter().all(|s| s.is_finite() && *s >= -1e-9),
        "regularized spin must stay finite and non-negative"
    );

    // Frequency consistency: the lock should take the same physical time.
    let reg240 = run_wheel_lock(&cfg, 8.0, 0.02, 20.0, 3000.0, 480, 1.0 / 240.0);
    let stop240 = first_stopped(&reg240).expect("regularized must lock at 240 Hz") as f64 / 240.0;
    let stop120 = reg_stop as f64 / 120.0;
    println!(
        "[GRIP-03] f1_2030 lock time legacy={:.3}s regularized={:.3}s (120 Hz), {:.3}s (240 Hz)",
        legacy_stop as f64 / 120.0,
        stop120,
        stop240
    );
    assert!(
        (stop120 - stop240).abs() < 0.1,
        "lock time must be frequency consistent: {stop120} vs {stop240}"
    );
}

#[test]
fn f1_2030_applied_force_respects_budget_with_smoothing() {
    let cfg = f1_2030_config();
    let wheel = WheelIndex::FrontLeft;
    let reference = reference_load_n(&cfg, wheel);
    let long_ratio = cfg
        .surface_longitudinal_grip_ratio
        .get(&SurfaceType::Road)
        .copied()
        .unwrap_or(0.5);
    let dt = 1.0 / 120.0;
    let mut saw_smoothing = false;

    for fz_scale in [0.5, 1.0, 1.5] {
        let fz = reference * fz_scale;
        for alpha_steps in 0..=12 {
            let alpha = 0.02 * alpha_steps as f64;
            for kappa_steps in 0..=12 {
                let kappa = 0.02 * kappa_steps as f64;
                let run = run_wheel(&cfg, wheel, fz, alpha, kappa, 60, dt);
                let fx_max = (run.mu * long_ratio * fz).max(1e-6);
                let fy_max = (run.mu * fz).max(1e-6);
                let fx_traction = run.fx + run.rolling_resistance;
                let effective =
                    ((fx_traction / fx_max).powi(2) + (run.fy / fy_max).powi(2)).sqrt();
                assert!(
                    effective <= 1.0 + 1e-6,
                    "applied force must respect the budget (fz={fz_scale} alpha={alpha} kappa={kappa}: effective={effective})"
                );
                assert!(
                    (effective - run.utilization).abs() < 1e-6,
                    "telemetry utilization must match the applied traction norm: {effective} vs {}",
                    run.utilization
                );
                assert!(
                    run.demand + 1e-12 >= run.utilization,
                    "demand must never be below the delivered utilization"
                );
                if run.demand - run.utilization > 1e-4 {
                    saw_smoothing = true;
                }
            }
        }
    }
    assert!(
        saw_smoothing,
        "the sweep must include demand absorbed by the smoothing band"
    );
}

#[test]
fn f1_2030_memory_recovers_slower_than_loss_and_stays_per_axis() {
    let cfg = f1_2030_config();
    let wheel = WheelIndex::FrontLeft;
    let fz = reference_load_n(&cfg, wheel);
    let ticks = 300;
    let seq = run_lateral_slide_recovery(&cfg, wheel, fz, ticks, ticks, 1.0 / 120.0);

    let committed = seq[ticks - 1].1;
    assert!(committed > 0.5, "lateral memory must commit during the slide");
    let loss_t90 = (0..ticks)
        .find(|&i| seq[i].1 >= 0.9 * committed)
        .expect("loss t90 must exist");
    let recovery_t90 = (ticks..2 * ticks)
        .find(|&i| seq[i].1 <= 0.1 * committed)
        .expect("recovery t90 must exist")
        - ticks;
    println!(
        "[GRIP-02] f1_2030 t90 loss={:.3}s recovery={:.3}s (120 Hz)",
        loss_t90 as f64 / 120.0,
        recovery_t90 as f64 / 120.0
    );
    assert!(
        recovery_t90 > loss_t90 * 3,
        "recovery must be clearly slower than loss: loss={loss_t90} ticks recovery={recovery_t90} ticks"
    );

    // The lateral slide must not leak into the longitudinal memory.
    let max_lon = seq.iter().map(|r| r.2).fold(0.0_f64, f64::max);
    assert!(
        max_lon < 1e-12,
        "longitudinal memory must stay idle during a pure lateral slide: {max_lon}"
    );
}

#[test]
fn f1_2030_memory_timing_is_frequency_consistent() {
    let cfg = f1_2030_config();
    let wheel = WheelIndex::FrontLeft;
    let fz = reference_load_n(&cfg, wheel);
    let ticks = 300;
    let seq120 = run_lateral_slide_recovery(&cfg, wheel, fz, ticks, ticks, 1.0 / 120.0);
    let seq240 = run_lateral_slide_recovery(&cfg, wheel, fz, ticks, ticks, 1.0 / 240.0);

    let committed120 = seq120[ticks - 1].1;
    let committed240 = seq240[ticks - 1].1;
    let loss120 = (0..ticks)
        .find(|&i| seq120[i].1 >= 0.9 * committed120)
        .expect("120 Hz loss t90") as f64
        / 120.0;
    let loss240 = (0..ticks)
        .find(|&i| seq240[i].1 >= 0.9 * committed240)
        .expect("240 Hz loss t90") as f64
        / 240.0;
    let rec120 = ((ticks..2 * ticks)
        .find(|&i| seq120[i].1 <= 0.1 * committed120)
        .expect("120 Hz recovery t90")
        - ticks) as f64
        / 120.0;
    let rec240 = ((ticks..2 * ticks)
        .find(|&i| seq240[i].1 <= 0.1 * committed240)
        .expect("240 Hz recovery t90")
        - ticks) as f64
        / 240.0;

    assert!(
        (loss120 - loss240).abs() < 0.02,
        "loss t90 must be dt-independent: {loss120} vs {loss240}"
    );
    assert!(
        (rec120 - rec240).abs() < 0.03,
        "recovery t90 must be dt-independent: {rec120} vs {rec240}"
    );
}

fn flat_samples_for_test(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for (sample, &wheel) in samples.iter_mut().zip(WheelIndex::ALL.iter()) {
        let hub_local = sim.config.wheel_anchor_local(wheel);
        let hub_world = sim.state.transform.transform_point(hub_local);
        let tire_w = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        };
        let span = tire_w * sim.config.tri_ray_spacing_ratio;
        *sample = TriRaycastSample {
            inner: RaycastHit {
                is_colliding: true,
                distance: (hub_world.y - span).max(0.0),
                point: Vec3::new(hub_world.x - span, 0.0, hub_world.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            center: RaycastHit {
                is_colliding: true,
                distance: hub_world.y.max(0.0),
                point: Vec3::new(hub_world.x, 0.0, hub_world.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            outer: RaycastHit {
                is_colliding: true,
                distance: (hub_world.y + span).max(0.0),
                point: Vec3::new(hub_world.x + span, 0.0, hub_world.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
        };
    }
    samples
}

#[test]
fn f1_2030_full_vehicle_braking_stays_stable_with_regularized_lock() {
    let base = f1_2030_config();
    struct Outcome {
        final_speed_kmh: f64,
        max_near_zero_step: f64,
        snapped: bool,
    }
    let run = |blend: f64, stick: f64| -> Outcome {
        let mut cfg = base.clone();
        cfg.wheel_lock_blend_spin_rad_s = blend;
        cfg.wheel_lock_stick_tau_s = stick;
        let spawn = default_spawn_height(&cfg);
        let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
        sim.state.linear_velocity = Vec3::new(0.0, 0.0, -27.78);
        let dt = 1.0 / 120.0;
        let mut prev = [0.0_f64; 4];
        let mut max_near_zero_step = 0.0_f64;
        let mut snapped = false;
        for _ in 0..600 {
            let input = VehicleInput {
                throttle: 0.0,
                steering: 0.0,
                brake: 1.0,
                handbrake: 0.0,
                clutch: 0.0,
                gear_request: None,
            };
            let samples = flat_samples_for_test(&sim);
            let telem = sim.step(&input, &samples, dt);
            assert!(telem.speed_kmh.is_finite());
            for (i, spin) in sim
                .state
                .tires
                .wheels
                .iter()
                .map(|w| w.spin)
                .enumerate()
            {
                if prev[i].abs() < 8.0 {
                    max_near_zero_step = max_near_zero_step.max((spin - prev[i]).abs());
                }
                if prev[i].abs() > 0.5 && spin == 0.0 {
                    snapped = true;
                }
                prev[i] = spin;
            }
        }
        Outcome {
            final_speed_kmh: sim.state.linear_velocity.length() * 3.6,
            max_near_zero_step,
            snapped,
        }
    };

    let legacy = run(0.0, 0.0);
    let reg = run(8.0, 0.02);
    assert!(
        legacy.final_speed_kmh < 5.0,
        "legacy must stop: {} km/h",
        legacy.final_speed_kmh
    );
    assert!(
        reg.final_speed_kmh < 5.0,
        "regularized must stop: {} km/h",
        reg.final_speed_kmh
    );
    assert!(
        reg.max_near_zero_step < legacy.max_near_zero_step,
        "regularized lock must be smoother at vehicle level: legacy={} reg={}",
        legacy.max_near_zero_step,
        reg.max_near_zero_step
    );
    assert!(
        !reg.snapped,
        "regularized lock must not teleport a wheel to zero"
    );
    println!(
        "[GRIP-03] full brake 100 km/h->0: legacy near-zero step={:.3} rad/s snapped={} final={:.2} km/h | regularized step={:.3} snapped={} final={:.2} km/h",
        legacy.max_near_zero_step,
        legacy.snapped,
        legacy.final_speed_kmh,
        reg.max_near_zero_step,
        reg.snapped,
        reg.final_speed_kmh
    );
}

#[test]
fn f1_2030_zero_taus_reproduce_pure_axes_under_asymmetric_slip() {
    let mut cfg = f1_2030_config();
    for profile in [&mut cfg.front_tire_force, &mut cfg.rear_tire_force] {
        profile.slip_loss_tau_s = 0.0;
        profile.slip_recovery_tau_s = 0.0;
    }
    let wheel = WheelIndex::FrontLeft;
    let fz = reference_load_n(&cfg, wheel);
    let dt = 1.0 / 120.0;

    // Deep lateral slide with a small longitudinal demand: the longitudinal
    // delivery must equal the same run without lateral slip (both stay below
    // the budget knee, so the explicit combined projection is inactive).
    let asym_lat = run_wheel(&cfg, wheel, fz, 0.30, 0.005, 120, dt);
    let no_lat = run_wheel(&cfg, wheel, fz, 0.0, 0.005, 120, dt);
    assert!(
        (asym_lat.fx - no_lat.fx).abs() <= no_lat.fx.abs().max(1.0) * 1e-9,
        "zero-tau lateral slide must not alter longitudinal force: {} vs {}",
        asym_lat.fx,
        no_lat.fx
    );

    // Deep longitudinal slide with a small lateral demand: the lateral delivery
    // must equal the same run without longitudinal slip.
    let asym_lon = run_wheel(&cfg, wheel, fz, 0.005, 0.5, 120, dt);
    let no_lon = run_wheel(&cfg, wheel, fz, 0.005, 0.0, 120, dt);
    assert!(
        (asym_lon.fy - no_lon.fy).abs() <= no_lon.fy.abs().max(1.0) * 1e-9,
        "zero-tau longitudinal slide must not alter lateral force: {} vs {}",
        asym_lon.fy,
        no_lon.fy
    );
}
