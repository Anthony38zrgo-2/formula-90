use std::path::Path;
use vehicle_physics_engine::suspension_kinematics::solve_corner;
use vehicle_physics_engine::suspension_kinematics_table::SuspensionKinematicsTable;
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::WheelIndex;

fn configuration() -> vehicle_physics_engine::VehicleConfig {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    load_vehicle_configuration_from_authorized_root(
        &repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
        &repository,
    )
    .unwrap()
}

#[test]
fn precomputed_kinematics_tracks_the_exact_solver() {
    let configuration = configuration();
    let table = SuspensionKinematicsTable::build(&configuration, 0.02).unwrap();
    let geometry = configuration.suspension.geometric_suspension.as_ref().unwrap();
    let mut state = 0x1234_5678_9abc_def0_u64;
    for wheel in WheelIndex::ALL {
        let (travel_min, travel_max) = table.travel_limits_metres(wheel);
        let corner = geometry.corners.get(wheel);
        for _ in 0..80 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let travel = travel_min + (travel_max - travel_min) * ((state >> 11) as f64 / (1_u64 << 53) as f64);
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let rack = if wheel.is_front() {
                -0.017 + 0.034 * ((state >> 11) as f64 / (1_u64 << 53) as f64)
            } else { 0.0 };
            let Some(sampled) = table.wheel_frame(wheel, travel, rack) else { continue; };
            let exact = solve_corner(corner, travel, rack, wheel.is_front(), 0.0, 0.0, 1.0)
                .expect("exact solution");
            let hub_error = (sampled.hub - exact.hub).length();
            let upright_error = (sampled.upright_basis.x - exact.upright_basis.x).length()
                + (sampled.upright_basis.y - exact.upright_basis.y).length()
                + (sampled.upright_basis.z - exact.upright_basis.z).length();
            assert!(hub_error <= 1e-6, "{wheel:?} hub error {hub_error} at {travel} {rack}");
            assert!(upright_error <= 1e-6, "{wheel:?} upright error {upright_error} at {travel} {rack}");
        }
    }
}
