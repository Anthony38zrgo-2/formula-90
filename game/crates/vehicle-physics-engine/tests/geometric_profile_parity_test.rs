use std::path::PathBuf;

use vehicle_physics_engine::vehicle_configuration::{
    load_physical_configuration, ResolvedVehicleConfiguration, VehicleProfileLoadRequest,
};
use vehicle_physics_engine::VehicleConfig;

fn repository_data_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn legacy_monolith_path() -> PathBuf {
    repository_data_directory().join("vehicles/f1_2030/f1_2030_v10_geometric.json")
}

fn manifest_path() -> PathBuf {
    repository_data_directory().join("vehicles/f1_2030/profiles/geometric/vehicle.json")
}

fn authorized_root() -> PathBuf {
    repository_data_directory().join("vehicles")
}

fn load_geometric() -> vehicle_physics_engine::vehicle_configuration::LoadedPhysicalConfiguration {
    load_physical_configuration(&VehicleProfileLoadRequest::new(
        &manifest_path(),
        &authorized_root(),
    ))
    .expect("geometric manifest must load")
}

#[test]
fn manifest_loading_matches_the_legacy_monolith_effective_configuration() {
    let legacy = VehicleConfig::from_json_path(&legacy_monolith_path())
        .expect("legacy geometric monolith must load");
    let loaded = load_geometric();
    let mut legacy_value = legacy.to_json_value();
    let mut loaded_value = loaded.configuration.to_json_value();
    if let Some(object) = legacy_value.as_object_mut() {
        object.remove("audio");
    }
    if let Some(object) = loaded_value.as_object_mut() {
        object.remove("audio");
    }
    assert_eq!(
        loaded_value, legacy_value,
        "resolved physical configuration must equal the frozen monolith configuration"
    );
    assert!(
        loaded.configuration.audio.is_none(),
        "audio is no longer owned by the physical configuration; formula90-core composes it"
    );
    assert!(
        legacy.audio.is_some(),
        "the frozen monolith still carries the legacy audio passthrough"
    );
}

#[test]
fn embedded_resolved_document_matches_the_loader() {
    let loaded = load_geometric();
    let embedded = VehicleConfig::from_json_str(
        VehicleConfig::f1_2030_v10_geometric_embedded_document(),
    )
    .expect("embedded resolved document must parse");
    assert_eq!(
        embedded.to_json_value(),
        loaded.configuration.to_json_value(),
        "build-time embedded document must match the runtime loader"
    );
}

#[test]
fn resolved_transport_round_trips_for_the_geometric_profile() {
    let resolved = ResolvedVehicleConfiguration::from_loaded(load_geometric());
    let text = resolved
        .to_json_string_pretty()
        .expect("resolved document must serialize");
    let restored =
        ResolvedVehicleConfiguration::from_json_str(&text).expect("resolved document must parse");
    assert_eq!(
        restored.configuration().unwrap().to_json_value(),
        resolved.configuration().unwrap().to_json_value()
    );
    assert_eq!(
        restored.provenance.global_digest_sha256,
        resolved.provenance.global_digest_sha256
    );
}

#[test]
fn provenance_covers_physical_documents_and_excludes_external_owners() {
    let loaded = load_geometric();
    let domains: Vec<String> = loaded
        .provenance
        .dependencies
        .iter()
        .map(|dependency| dependency.domain.clone())
        .collect();
    assert_eq!(
        domains,
        vec![
            "aerodynamics",
            "brakes",
            "chassis",
            "coupled_world",
            "driving_assistance",
            "fuel",
            "powertrain",
            "steering",
            "suspension",
            "tires",
            "underfloor_contact",
        ]
    );
    assert!(!domains.contains(&"audio".to_string()));
    assert!(!domains.contains(&"suspension_presentation".to_string()));
}

#[test]
fn moved_documents_preserve_every_source_value() {
    let profile_directory = repository_data_directory().join("vehicles/f1_2030/profiles/geometric");
    let legacy: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(legacy_monolith_path()).expect("legacy monolith must be readable"),
    )
    .expect("legacy monolith must parse");

    let audio: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(profile_directory.join("audio.json")).expect("audio must be readable"),
    )
    .expect("audio must parse");
    assert_eq!(audio, legacy["audio"]);

    let suspension: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(profile_directory.join("suspension.json"))
            .expect("suspension must be readable"),
    )
    .expect("suspension must parse");
    let mut expected_suspension = legacy["suspension"].clone();
    expected_suspension
        .as_object_mut()
        .unwrap()
        .remove("geometry");
    assert_eq!(suspension["suspension"], expected_suspension);

    let presentation: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(profile_directory.join("suspension_presentation.json"))
            .expect("presentation must be readable"),
    )
    .expect("presentation must parse");
    assert_eq!(
        presentation["suspension"]["geometry"],
        legacy["suspension"]["geometry"]
    );

    let chassis: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(profile_directory.join("chassis.json"))
            .expect("chassis must be readable"),
    )
    .expect("chassis must parse");
    assert_eq!(chassis["chassis"], legacy["chassis"]);
    assert_eq!(chassis["geometry"], legacy["geometry"]);
    assert_eq!(
        chassis["physical_mass_inventory"],
        legacy["physical_mass_inventory"]
    );
}

#[test]
fn embedded_document_is_deterministic_across_parses() {
    let first = VehicleConfig::from_json_str(
        VehicleConfig::f1_2030_v10_geometric_embedded_document(),
    )
    .unwrap();
    let second = VehicleConfig::from_json_str(
        VehicleConfig::f1_2030_v10_geometric_embedded_document(),
    )
    .unwrap();
    assert_eq!(first.to_json_value(), second.to_json_value());
    assert_eq!(first.vehicle_mass, second.vehicle_mass);
    assert_eq!(first.gear_ratios, second.gear_ratios);
}
