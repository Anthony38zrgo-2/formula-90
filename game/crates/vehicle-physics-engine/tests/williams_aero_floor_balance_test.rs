use vehicle_physics_engine::aero::{AeroEnvironment, AeroForces, AeroKinematics, AeroProbeMode};
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::{
    center_of_mass_local_for_suspension_backend, SuspensionBackend, Vec3,
};

const REFERENCE_SPEED_METRES_PER_SECOND: f64 = 80.0;
const NOMINAL_FLOW_DENSITY_KILOGRAMS_PER_CUBIC_METRE: f64 = 1.20;
const DOCUMENTED_LIFT_AREA_SQUARE_METRES: f64 = 2.070;
const DOCUMENTED_FRONT_BALANCE: f64 = 0.47;
const INTEGRATION_STEP_SECONDS: f64 = 1.0 / 120.0;
/// Native underfloor probe stations: front centre at chassis z = -0.90 m and the
/// diffuser throat at chassis z = +0.85 m, with the axles at -1.146992 m and
/// +1.743008 m. The declared front and rear clearances are interpolated at those
/// stations exactly like the native probe placement does.
const FRONT_PROBE_STATION_Z_METRES: f64 = -0.90;
const THROAT_PROBE_STATION_Z_METRES: f64 = 0.85;
const FRONT_AXLE_Z_METRES: f64 = -1.146992;
const REAR_AXLE_Z_METRES: f64 = 1.743008;

fn station_clearance_m(
    declared_front_clearance_m: f64,
    declared_rear_clearance_m: f64,
    station_z_metres: f64,
) -> f64 {
    let axle_span_m = REAR_AXLE_Z_METRES - FRONT_AXLE_Z_METRES;
    let fraction =
        ((station_z_metres - FRONT_AXLE_Z_METRES) / axle_span_m).clamp(0.0, 1.0);
    declared_front_clearance_m
        + (declared_rear_clearance_m - declared_front_clearance_m) * fraction
}

fn repository_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn measured_environment(front_clearance_m: f64, throat_clearance_m: f64) -> AeroEnvironment {
    let mut environment = AeroEnvironment::unavailable();
    environment.valid_mask = 0b0000_1001;
    environment.clearance_m[0] = front_clearance_m;
    environment.clearance_m[3] = throat_clearance_m;
    environment
}

fn reference_kinematics(configuration: &vehicle_physics_engine::VehicleConfig) -> AeroKinematics {
    AeroKinematics {
        velocity_m_s: Vec3::new(0.0, 0.0, -REFERENCE_SPEED_METRES_PER_SECOND),
        angular_velocity_rad_s: Vec3::ZERO,
        wind_velocity_m_s: Vec3::ZERO,
        center_of_mass_m: center_of_mass_local_for_suspension_backend(
            configuration,
            SuspensionBackend::Simplified,
        ),
        probe_mode: AeroProbeMode::Measured,
    }
}

#[test]
fn williams_declared_floor_reference_meets_the_documented_aero_targets() {
    let root = repository_root();
    let configuration = load_vehicle_configuration_from_authorized_root(
        &root.join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &root,
    )
    .expect("Williams profile resolves");
    let floor = &configuration.aerodynamics.aero_model.underfloor;
    let front_clearance_m = floor
        .floor_clearance_front_m
        .expect("Williams declares the front floor clearance");
    let rear_clearance_m = floor
        .floor_clearance_rear_m
        .expect("Williams declares the rear floor clearance");
    let kinematics = reference_kinematics(&configuration);
    let mut forces = AeroForces::zero();
    forces.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &measured_environment(
            station_clearance_m(front_clearance_m, rear_clearance_m, FRONT_PROBE_STATION_Z_METRES),
            station_clearance_m(front_clearance_m, rear_clearance_m, THROAT_PROBE_STATION_Z_METRES),
        ),
        INTEGRATION_STEP_SECONDS,
    );
    assert!(!forces.invalid_input);
    let dynamic_pressure = 0.5
        * configuration.aerodynamics.air_density
        * REFERENCE_SPEED_METRES_PER_SECOND.powi(2);
    let lift_area_square_metres = forces.total_downforce / dynamic_pressure;
    println!(
        "williams floor balance: front_wing={:.1} rear_wing={:.1} floor={:.1} total={:.1} ClA={:.4} balance={:.4} front_cl={:.4} rear_cl={:.4} height_factor={:.4} rake_factor={:.4} front_axle={:.1} rear_axle={:.1} drag={:.1}",
        forces.front_downforce,
        forces.rear_downforce,
        forces.diffuser_downforce,
        forces.total_downforce,
        lift_area_square_metres,
        forces.balance_front,
        forces.front_wing_cl,
        forces.rear_wing_cl,
        forces.floor_height_factor,
        forces.floor_rake_factor,
        forces.front_axle_downforce,
        forces.rear_axle_downforce,
        forces.drag_force,
    );
    assert!(
        (lift_area_square_metres - DOCUMENTED_LIFT_AREA_SQUARE_METRES).abs() < 0.05,
        "documented ClA {DOCUMENTED_LIFT_AREA_SQUARE_METRES}, measured {lift_area_square_metres}"
    );
    assert!(
        (forces.balance_front - DOCUMENTED_FRONT_BALANCE).abs() < 0.01,
        "documented front balance {DOCUMENTED_FRONT_BALANCE}, measured {}",
        forces.balance_front
    );
    assert!(
        forces.diffuser_downforce > 0.0,
        "the underfloor must carry load at the declared clearances"
    );
    assert!(forces.front_downforce > 0.0 && forces.rear_downforce > 0.0);
    let documented_downforce_newtons = 0.5
        * NOMINAL_FLOW_DENSITY_KILOGRAMS_PER_CUBIC_METRE
        * DOCUMENTED_LIFT_AREA_SQUARE_METRES
        * REFERENCE_SPEED_METRES_PER_SECOND.powi(2);
    assert!(
        (forces.total_downforce - documented_downforce_newtons).abs()
            < 0.05 * documented_downforce_newtons,
        "documented downforce {documented_downforce_newtons} N, measured {} N",
        forces.total_downforce
    );
}

#[test]
fn declared_floor_clearances_keep_the_probes_inside_the_height_range() {
    let root = repository_root();
    for profile in [
        "game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json",
    ] {
        let configuration =
            load_vehicle_configuration_from_authorized_root(&root.join(profile), &root)
                .expect("profile resolves");
        let floor = &configuration.aerodynamics.aero_model.underfloor;
        let front = floor.floor_clearance_front_m.expect("declared front clearance");
        let rear = floor.floor_clearance_rear_m.expect("declared rear clearance");
        assert!(front > 0.0 && front <= floor.high_height_m, "{profile}: front {front}");
        assert!(rear > 0.0 && rear <= floor.high_height_m, "{profile}: rear {rear}");
    }
}

#[test]
fn f1_2030_measured_probes_keep_the_calibrated_underfloor_load() {
    let root = repository_root();
    let configuration = load_vehicle_configuration_from_authorized_root(
        &root.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
        &root,
    )
    .expect("F1 2030 profile resolves");
    let floor = &configuration.aerodynamics.aero_model.underfloor;
    assert!(
        floor.floor_clearance_front_m.is_none(),
        "the F1 2030 keeps the native probe layout"
    );
    let kinematics = reference_kinematics(&configuration);
    let mut forces = AeroForces::zero();
    forces.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &measured_environment(0.080, 0.115),
        INTEGRATION_STEP_SECONDS,
    );
    assert!(!forces.invalid_input);
    assert!(
        forces.diffuser_downforce > 4_000.0,
        "F1 2030 underfloor load regressed: {} N",
        forces.diffuser_downforce
    );
    assert!(
        (0.41..=0.46).contains(&forces.balance_front),
        "F1 2030 balance regressed: {}",
        forces.balance_front
    );
    assert!(
        forces.total_downforce > 10_000.0,
        "F1 2030 total downforce regressed: {} N",
        forces.total_downforce
    );
}
