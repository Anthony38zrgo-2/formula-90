use std::path::PathBuf;
use vehicle_physics_engine::{
    default_spawn_height, f1_94_physics_destroy,
    formula_one_physics_create_from_vehicle_profile_v1, VehicleSimulator,
};
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration;

fn repository_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap()
}

#[test]
fn native_profile_entry_loads_all_profiles_and_legacy_with_identical_configuration() {
    let repository = repository_directory();
    for relative_path in [
        "game/data/vehicles/f1_2030/profiles/geometric/vehicle.json",
        "game/data/vehicles/f1_2030/profiles/legacy/vehicle.json",
        "game/data/vehicles/f1_2026_2008/profiles/default/vehicle.json",
        "game/data/vehicles/f1_2030/f1_2030_v10_geometric.json",
    ] {
        let profile_path = repository.join(relative_path);
        let expected = load_vehicle_configuration(&profile_path).unwrap();
        let root_text = repository.to_str().unwrap().as_bytes();
        let profile_text = profile_path.to_str().unwrap().as_bytes();
        let mut error_buffer = [0u8; 512];
        let simulator_pointer = formula_one_physics_create_from_vehicle_profile_v1(
            root_text.as_ptr(), root_text.len() as u32,
            profile_text.as_ptr(), profile_text.len() as u32,
            error_buffer.as_mut_ptr(), error_buffer.len() as u32,
        );
        assert!(!simulator_pointer.is_null(), "{}", String::from_utf8_lossy(&error_buffer));
        let simulator = unsafe { &*(simulator_pointer as *const VehicleSimulator) };
        assert_eq!(simulator.config.to_json_value(), expected.to_json_value());
        assert_eq!(simulator.state.transform.origin.y, default_spawn_height(&expected));
        assert_eq!(error_buffer[0], 0);
        f1_94_physics_destroy(simulator_pointer);
    }
}

#[test]
fn native_profile_entry_rejects_paths_outside_authorized_root() {
    let repository = repository_directory();
    let unauthorized_root = repository.join("game/assets");
    let profile = repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json");
    let root_text = unauthorized_root.to_str().unwrap().as_bytes();
    let profile_text = profile.to_str().unwrap().as_bytes();
    let mut error_buffer = [0u8; 512];
    let simulator = formula_one_physics_create_from_vehicle_profile_v1(
        root_text.as_ptr(), root_text.len() as u32,
        profile_text.as_ptr(), profile_text.len() as u32,
        error_buffer.as_mut_ptr(), error_buffer.len() as u32,
    );
    assert!(simulator.is_null());
    assert!(String::from_utf8_lossy(&error_buffer).contains("outside the authorized root"));
}

#[test]
fn native_profile_entry_reports_invalid_input_and_terminates_short_error_buffers() {
    let mut error_buffer = [255u8; 2];
    let simulator = formula_one_physics_create_from_vehicle_profile_v1(
        std::ptr::null(), 0, std::ptr::null(), 0,
        error_buffer.as_mut_ptr(), error_buffer.len() as u32,
    );
    assert!(simulator.is_null());
    assert_eq!(error_buffer[1], 0);
    let invalid_text = [255u8];
    let simulator = formula_one_physics_create_from_vehicle_profile_v1(
        invalid_text.as_ptr(), 1, invalid_text.as_ptr(), 1,
        std::ptr::null_mut(), 0,
    );
    assert!(simulator.is_null());
}
