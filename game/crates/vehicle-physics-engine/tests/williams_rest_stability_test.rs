use vehicle_physics_engine::simulation::default_spawn_height;
use vehicle_physics_engine::types::{RaycastHit, SurfaceType, TriRaycastSample};
use vehicle_physics_engine::{Vec3, VehicleConfig, VehicleInput, VehicleSimulator, WheelIndex};

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .expect("Williams profile")
}

fn flat_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        let anchor_local = sim.config.tires.wheel_anchor_local(
            &sim.config.chassis,
            &sim.config.suspension,
            wheel,
        );
        let anchor_world = sim.state.transform.transform_point(anchor_local);
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() {
            sim.config.tires.front_tire_width
        } else {
            sim.config.tires.rear_tire_width
        } * sim.config.suspension.tri_ray_spacing_ratio;
        let side = if wheel.is_left() { 1.0 } else { -1.0 };
        let hit = |offset_x: f64| RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::new(anchor_world.x + offset_x, 0.0, anchor_world.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        samples[index] = TriRaycastSample {
            inner: hit(side * span),
            center: hit(0.0),
            outer: hit(-side * span),
        };
    }
    samples
}

#[test]
fn williams_settles_at_rest_without_parasitic_yaw() {
    let configuration = williams_configuration();
    let spawn_height = default_spawn_height(&configuration);
    let mut simulator = VehicleSimulator::new(
        configuration,
        Vec3::new(0.0, spawn_height, 0.0),
        0.0,
    );
    let step_seconds = 1.0 / 120.0;
    let input = VehicleInput::default();
    let mut yaw_samples = Vec::new();
    let mut lateral_samples = Vec::new();
    let mut compression_ranges = [(f64::MAX, f64::MIN); 4];
    for tick in 0..2400 {
        let samples = flat_samples(&simulator);
        simulator.step(&input, &samples, step_seconds);
        if tick >= 600 {
            yaw_samples.push(simulator.state.angular_velocity.y);
            lateral_samples.push(simulator.state.linear_velocity.x);
            for wheel in WheelIndex::ALL {
                let compression = simulator.state.suspension.wheels[wheel as usize]
                    .suspension_compression_m;
                let range = &mut compression_ranges[wheel as usize];
                range.0 = range.0.min(compression);
                range.1 = range.1.max(compression);
            }
        }
    }
    let mean = yaw_samples.iter().sum::<f64>() / yaw_samples.len() as f64;
    let yaw_deviation = (yaw_samples
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / yaw_samples.len() as f64)
        .sqrt();
    let lateral_deviation = (lateral_samples
        .iter()
        .map(|value| value.powi(2))
        .sum::<f64>()
        / lateral_samples.len() as f64)
        .sqrt();
    let compression_travel = compression_ranges
        .iter()
        .map(|range| (range.1 - range.0) * 1000.0)
        .fold(0.0_f64, f64::max);
    println!(
        "WILLIAMS_REST {}",
        serde_json::json!({
            "yaw_rate_mean": mean,
            "yaw_rate_deviation": yaw_deviation,
            "lateral_velocity_deviation": lateral_deviation,
            "maximum_compression_travel_mm": compression_travel,
            "speed_kmh": simulator.state.linear_velocity.length() * 3.6,
            "position_z": simulator.state.transform.origin.z,
        })
    );
    assert!(yaw_deviation < 0.001, "rest yaw deviation {yaw_deviation}");
    assert!(lateral_deviation < 0.05, "rest lateral deviation {lateral_deviation}");
    assert!(compression_travel < 50.0, "rest compression travel {compression_travel} mm");
    let total_normal_force = simulator.state.suspension.wheels.iter().map(|wheel| wheel.total_normal_force).sum::<f64>();
    let total_weight = simulator.config.chassis.total_vehicle_mass(&simulator.config.tires, &simulator.config.fuel) * 9.80665;
    assert!((total_normal_force - total_weight).abs() < total_weight * 0.01,
        "normal load {total_normal_force} differs from vehicle weight {total_weight}");
}
