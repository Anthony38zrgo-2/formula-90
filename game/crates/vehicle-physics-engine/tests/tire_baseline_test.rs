// BASE-000 acceptance tests: the calibration harness must produce deterministic,
// finite, sign-correct force sweeps. These run against the canonical
// f1_2026_2008 configuration and the same `steady_tire_force` used by
// `tire_sweep`.
use std::path::Path;
use vehicle_physics_engine::*;

fn canonical_config() -> VehicleConfig {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("vehicles")
        .join("f1_2026_2008")
        .join("f1_2026_2008_physics.json");
    VehicleConfig::from_json_path(&path).expect("canonical JSON should load")
}

#[test]
fn sweep_is_reproducible_run_to_run() {
    let cfg = canonical_config();
    let fz = reference_load_n(&cfg, WheelIndex::RearLeft);
    let a = steady_tire_force(&cfg, WheelIndex::RearLeft, fz, 0.10, 0.05);
    let b = steady_tire_force(&cfg, WheelIndex::RearLeft, fz, 0.10, 0.05);
    assert_eq!(a.lateral_force_n, b.lateral_force_n);
    assert_eq!(a.longitudinal_force_n, b.longitudinal_force_n);
    assert_eq!(a.aligning_torque_nm, b.aligning_torque_nm);
}

#[test]
fn pure_lateral_sweep_is_finite_and_saturates() {
    // The current tanh-based curve has no measurable peak inside 0..0.7 rad: it
    // saturates monotonically. BASE-000 only locks determinism/finiteness here;
    // the TIRE-200 acceptance test requires a later measurable peak + post-peak
    // decay with the C1 curve.
    let cfg = canonical_config();
    let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
    let alphas = [0.02, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.45, 0.55, 0.65, 0.70];
    let magnitudes: Vec<f64> = alphas
        .iter()
        .map(|&alpha| steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, alpha, 0.0).lateral_force_n)
        .collect();
    assert!(magnitudes.iter().all(|m| m.is_finite() && *m > 0.0));
    // Pre-peak rise (0.02 -> 0.05 -> 0.10 rad)
    assert!(
        magnitudes[..3].windows(2).all(|w| w[1] >= w[0]),
        "lateral force must increase up to peak: {magnitudes:?}"
    );
    let peak = magnitudes[2];
    // Post-peak decay stays bounded below peak
    assert!(
        magnitudes[3..].iter().all(|&m| m <= peak),
        "post-peak lateral force must stay below peak: {magnitudes:?}"
    );
}

#[test]
fn combined_slip_never_exceeds_budget_region_naively() {
    let cfg = canonical_config();
    let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
    let pure_f = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, 0.10, 0.0).lateral_force_n.abs();
    for alpha in [-0.10, -0.05, 0.05, 0.10] {
        for kappa in [-0.20, -0.10, 0.10, 0.20] {
            let p = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, alpha, kappa);
            let combined = (p.lateral_force_n * p.lateral_force_n
                + p.longitudinal_force_n * p.longitudinal_force_n)
                .sqrt();
            assert!(combined.is_finite());
            assert!(
                combined < pure_f * 3.0 + 1.0,
                "combined magnitude {} at alpha={alpha} kappa={kappa} exceeds sanity bound",
                combined
            );
        }
    }
}

#[test]
fn brake_100_scenario_runs_without_nan() {
    // Full-brake stop from 100 km/h with aids on: no NaN and speed reaches ~0.
    let cfg = canonical_config();
    let spawn_height = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0);
    sim.state.linear_velocity = Vec3::new(0.0, 0.0, -27.78);
    let dt = 1.0 / 120.0;
    let mut speed = f64::MAX;
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
        assert!(telem.wheel_normal_force_n.iter().all(|v| v.is_finite()));
        speed = telem.speed_kmh;
    }
    assert!(speed < 5.0, "vehicle did not stop, final speed {speed}");
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
