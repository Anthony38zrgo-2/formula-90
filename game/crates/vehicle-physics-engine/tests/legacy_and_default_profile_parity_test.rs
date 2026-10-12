use std::path::PathBuf;

use vehicle_physics_engine::vehicle_configuration::{
    load_physical_configuration, load_vehicle_configuration, VehicleProfileLoadRequest,
};
use vehicle_physics_engine::VehicleConfig;

fn repository_data_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn assert_manifest_matches_monolith(monolith_relative_path: &str, manifest_relative_path: &str) {
    let data_directory = repository_data_directory();
    let monolith = VehicleConfig::from_json_path(&data_directory.join(monolith_relative_path))
        .expect("frozen monolith must load");
    let loaded = load_physical_configuration(&VehicleProfileLoadRequest::new(
        &data_directory.join(manifest_relative_path),
        &data_directory.join("vehicles"),
    ))
    .expect("profile manifest must load");
    let mut monolith_value = monolith.to_json_value();
    let mut loaded_value = loaded.configuration.to_json_value();
    if let Some(object) = monolith_value.as_object_mut() {
        object.remove("audio");
    }
    if let Some(object) = loaded_value.as_object_mut() {
        object.remove("audio");
    }
    assert_eq!(
        loaded_value, monolith_value,
        "resolved physical configuration must equal the frozen monolith configuration"
    );
    assert!(
        loaded.configuration.audio.is_none(),
        "audio is no longer owned by the physical configuration; formula90-core composes it"
    );
    assert!(
        monolith.audio.is_some(),
        "the frozen monolith still carries the legacy audio passthrough"
    );
}

#[test]
fn legacy_manifest_loading_matches_the_frozen_monolith_effective_configuration() {
    assert_manifest_matches_monolith(
        "vehicles/f1_2030/f1_2030_v10_physics.json",
        "vehicles/f1_2030/profiles/legacy/vehicle.json",
    );
}

#[test]
fn default_manifest_loading_matches_the_frozen_monolith_effective_configuration() {
    assert_manifest_matches_monolith(
        "vehicles/f1_2026_2008/f1_2026_2008_physics.json",
        "vehicles/f1_2026_2008/profiles/default/vehicle.json",
    );
}

#[test]
fn load_vehicle_configuration_accepts_manifests_and_flat_documents() {
    let data_directory = repository_data_directory();
    let manifest = data_directory.join("vehicles/f1_2030/profiles/geometric/vehicle.json");
    let from_manifest =
        load_vehicle_configuration(&manifest).expect("manifest must resolve through the helper");
    let loaded = load_physical_configuration(&VehicleProfileLoadRequest::new(
        &manifest,
        &data_directory.join("vehicles"),
    ))
    .expect("geometric manifest must load");
    assert_eq!(
        from_manifest.to_json_value(),
        loaded.configuration.to_json_value()
    );

    let monolith = data_directory.join("vehicles/f1_2030/f1_2030_v10_geometric.json");
    let from_flat =
        load_vehicle_configuration(&monolith).expect("flat document must parse through the helper");
    let direct = VehicleConfig::from_json_path(&monolith).expect("flat document must load");
    assert_eq!(from_flat.to_json_value(), direct.to_json_value());
}
