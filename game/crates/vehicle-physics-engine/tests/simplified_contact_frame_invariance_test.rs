use std::f64::consts::{FRAC_PI_2, PI};
use vehicle_physics_engine::slow_states::{SlowStateBackend, SlowStateSystem};
use vehicle_physics_engine::*;

const TICK_SECONDS: f64 = 1.0 / 120.0;
const WARM_UP_TICKS: usize = 480;
const REAR_LEFT_ROAD_BUMP_METRES: f64 = 0.015;
const CONTACT_FRAME_FORCE_TOLERANCE_NEWTONS: f64 = 1.0e-6;
const CONTACT_FRAME_TORQUE_TOLERANCE_NEWTON_METRES: f64 = 1.0e-6;
const REFERENCE_TOLERANCE_RELATIVE: f64 = 1.0e-9;

const STANDARD_REFERENCE_FORCE_WORLD: Vec3 =
    Vec3::new(-1.9381879271183777, 4564.495341967745, 13906.652476086985);
const STANDARD_REFERENCE_TORQUE_WORLD: Vec3 =
    Vec3::new(-4220.050209489664, -1664.9879003031183, 8.337791924479916);
const STANDARD_REFERENCE_YAW_QUARTER_TURN_FORCE_WORLD: Vec3 =
    Vec3::new(13906.61975421577, 4564.495341961043, 2.4488016249520967);
const STANDARD_REFERENCE_YAW_QUARTER_TURN_TORQUE_WORLD: Vec3 =
    Vec3::new(13.627663176032456, -1681.2881887470216, 4224.493321098267);
const SIMPLIFIED_REFERENCE_FORCE_WORLD: Vec3 =
    Vec3::new(38.59780997859917, 4556.4130420540705, 13860.901379212957);
const SIMPLIFIED_REFERENCE_TORQUE_WORLD: Vec3 =
    Vec3::new(-6204.157054424261, -2564.2648684534115, 459.3152818524188);

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .unwrap()
}

fn planar_samples(simulator: &VehicleSimulator) -> [TriRaycastSample; 4] {
    std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        let anchor_local = simulator.config.tires.wheel_anchor_local(
            &simulator.config.chassis,
            &simulator.config.suspension,
            wheel,
        );
        let anchor_world = simulator.state.transform.transform_point(anchor_local);
        let suspension_direction_world = simulator.state.transform.basis.y;
        let span_metres = if wheel.is_front() {
            simulator.config.tires.front_tire_width
        } else {
            simulator.config.tires.rear_tire_width
        } * simulator.config.suspension.tri_ray_spacing_ratio;
        let hit = |offset_metres| {
            let origin_world = anchor_world + simulator.state.transform.basis.x * offset_metres;
            let distance_metres = if suspension_direction_world.y > 0.1 {
                origin_world.y / suspension_direction_world.y
            } else {
                100.0
            };
            RaycastHit {
                is_colliding: distance_metres >= 0.0 && distance_metres < 1.0,
                distance: distance_metres.max(0.0),
                point: origin_world - suspension_direction_world * distance_metres,
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            }
        };
        TriRaycastSample {
            inner: hit(span_metres),
            center: hit(0.0),
            outer: hit(-span_metres),
        }
    })
}

fn asymmetric_rear_samples(simulator: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = planar_samples(simulator);
    let rear_left = &mut samples[WheelIndex::RearLeft as usize];
    for hit in [
        &mut rear_left.inner,
        &mut rear_left.center,
        &mut rear_left.outer,
    ] {
        hit.distance -= REAR_LEFT_ROAD_BUMP_METRES;
        hit.point = hit.point + Vec3::UP * REAR_LEFT_ROAD_BUMP_METRES;
    }
    samples
}

fn rotated_basis(rotation: Mat3, basis: Mat3) -> Mat3 {
    Mat3::from_cols(
        rotation.transform_vector(basis.x),
        rotation.transform_vector(basis.y),
        rotation.transform_vector(basis.z),
    )
}

fn rotated_samples(rotation: Mat3, samples: &[TriRaycastSample; 4]) -> [TriRaycastSample; 4] {
    let rotated_hit = |hit: &RaycastHit| RaycastHit {
        is_colliding: hit.is_colliding,
        distance: hit.distance,
        point: rotation.transform_vector(hit.point),
        normal: rotation.transform_vector(hit.normal),
        surface: hit.surface,
    };
    std::array::from_fn(|index| {
        let sample = samples[index];
        TriRaycastSample {
            inner: rotated_hit(&sample.inner),
            center: rotated_hit(&sample.center),
            outer: rotated_hit(&sample.outer),
        }
    })
}

struct ContactFrameScenario {
    warm_simulator: VehicleSimulator,
    base_body: BodyKinematics,
    base_contact_samples: [TriRaycastSample; 4],
}

fn contact_frame_scenario(suspension_backend: SuspensionBackend) -> ContactFrameScenario {
    let mut configuration = williams_configuration();
    configuration.suspension.front_arb_ratio = 0.15;
    configuration.suspension.rear_arb_ratio = 0.09;
    let spawn_position_metres = Vec3::new(0.0, default_spawn_height(&configuration), 0.0);
    let mut warm_simulator = VehicleSimulator::new(configuration, spawn_position_metres, 0.0);
    warm_simulator.aerodynamics_backend = AerodynamicsBackend::Simplified;
    warm_simulator
        .state
        .suspension
        .select_backend(&warm_simulator.config, suspension_backend);
    warm_simulator.state.tires.backend = TireBackend::SimplifiedWithRelaxation;
    warm_simulator.state.slow_states = SlowStateSystem::new(SlowStateBackend::Simplified);
    for _tick in 0..WARM_UP_TICKS {
        let contact_samples = asymmetric_rear_samples(&warm_simulator);
        warm_simulator.step(&VehicleInput::default(), &contact_samples, TICK_SECONDS);
    }
    let mut base_body = warm_simulator.state.body_kinematics();
    base_body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    base_body.angular_velocity = Vec3::new(0.12, 0.35, -0.08);
    let base_contact_samples = asymmetric_rear_samples(&warm_simulator);
    ContactFrameScenario {
        warm_simulator,
        base_body,
        base_contact_samples,
    }
}

fn solve_with_rotated_scene(
    scenario: &ContactFrameScenario,
    yaw_radians: f64,
) -> ForceTorqueOutput {
    let rotation = Mat3::from_axis_angle(Vec3::UP, yaw_radians);
    let rotated_body_basis = rotated_basis(rotation, scenario.base_body.transform.basis);
    let rotated_body = BodyKinematics {
        transform: Transform3D::new(
            rotation.transform_vector(scenario.base_body.transform.origin),
            rotated_body_basis,
        ),
        orientation: Quat::from_mat3(&rotated_body_basis),
        linear_velocity: rotation.transform_vector(scenario.base_body.linear_velocity),
        angular_velocity: rotation.transform_vector(scenario.base_body.angular_velocity),
    };
    let rotated_contact_samples = rotated_samples(rotation, &scenario.base_contact_samples);
    let mut simulator = scenario.warm_simulator.clone();
    let (output, _) = simulator.solve_external(
        rotated_body,
        &VehicleInput::default(),
        &rotated_contact_samples,
        TICK_SECONDS,
    );
    output
}

fn assert_matches_reference(actual: Vec3, expected: Vec3, label: &str) {
    let difference = (actual - expected).length();
    let tolerance = REFERENCE_TOLERANCE_RELATIVE * expected.length().max(1.0);
    assert!(
        difference <= tolerance,
        "{label}: difference {difference} exceeds {tolerance}; expected {expected:?} got {actual:?}"
    );
}

#[test]
fn simplified_contact_rotates_with_the_body_frame() {
    let scenario = contact_frame_scenario(SuspensionBackend::Simplified);
    for wheel in scenario.warm_simulator.state.suspension.wheels {
        assert_eq!(wheel.hub_lateral_m, 0.0);
        assert_eq!(wheel.hub_long_m, 0.0);
    }
    let mut outputs = Vec::new();
    for yaw_radians in [0.0, FRAC_PI_2, PI] {
        let rotation = Mat3::from_axis_angle(Vec3::UP, yaw_radians);
        let output = solve_with_rotated_scene(&scenario, yaw_radians);
        println!(
            "SIMPLIFIED_CONTACT_FRAME yaw_degrees={} force={:?} torque={:?}",
            yaw_radians.to_degrees(),
            output.force_world,
            output.torque_world
        );
        outputs.push((rotation, output));
    }
    assert_matches_reference(
        outputs[0].1.force_world,
        SIMPLIFIED_REFERENCE_FORCE_WORLD,
        "simplified reference force",
    );
    assert_matches_reference(
        outputs[0].1.torque_world,
        SIMPLIFIED_REFERENCE_TORQUE_WORLD,
        "simplified reference torque",
    );
    let reference = outputs[0].1;
    for (rotation, output) in &outputs[1..] {
        let expected_force_world = rotation.transform_vector(reference.force_world);
        let expected_torque_world = rotation.transform_vector(reference.torque_world);
        let force_difference = (output.force_world - expected_force_world).length();
        let torque_difference = (output.torque_world - expected_torque_world).length();
        println!(
            "SIMPLIFIED_CONTACT_FRAME_ROTATION_DIFFERENCE force_newtons={force_difference} torque_newton_metres={torque_difference}"
        );
        assert!(
            force_difference <= CONTACT_FRAME_FORCE_TOLERANCE_NEWTONS,
            "simplified force does not rotate with the body: expected {expected_force_world:?} got {:?}",
            output.force_world
        );
        assert!(
            torque_difference <= CONTACT_FRAME_TORQUE_TOLERANCE_NEWTON_METRES,
            "simplified torque does not rotate with the body: expected {expected_torque_world:?} got {:?}",
            output.torque_world
        );
    }
}

#[test]
fn standard_contact_reference_matches_recorded_goldens() {
    let scenario = contact_frame_scenario(SuspensionBackend::Standard);
    let aligned_output = solve_with_rotated_scene(&scenario, 0.0);
    let quarter_turn_output = solve_with_rotated_scene(&scenario, FRAC_PI_2);
    println!(
        "STANDARD_CONTACT_REFERENCE force={:?} torque={:?}",
        aligned_output.force_world, aligned_output.torque_world
    );
    println!(
        "STANDARD_CONTACT_REFERENCE_YAW_QUARTER_TURN force={:?} torque={:?}",
        quarter_turn_output.force_world, quarter_turn_output.torque_world
    );
    assert_matches_reference(
        aligned_output.force_world,
        STANDARD_REFERENCE_FORCE_WORLD,
        "standard reference force",
    );
    assert_matches_reference(
        aligned_output.torque_world,
        STANDARD_REFERENCE_TORQUE_WORLD,
        "standard reference torque",
    );
    assert_matches_reference(
        quarter_turn_output.force_world,
        STANDARD_REFERENCE_YAW_QUARTER_TURN_FORCE_WORLD,
        "standard quarter turn reference force",
    );
    assert_matches_reference(
        quarter_turn_output.torque_world,
        STANDARD_REFERENCE_YAW_QUARTER_TURN_TORQUE_WORLD,
        "standard quarter turn reference torque",
    );
}
