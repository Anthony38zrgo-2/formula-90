use std::fs;
use std::path::{Path, PathBuf};

use vehicle_physics_engine::vehicle_configuration::{
    load_physical_configuration, LoadedPhysicalConfiguration, ResolvedVehicleConfiguration,
    VehicleConfigurationLoadError, VehicleProfileLoadRequest,
};

struct ProfileFixture {
    root: PathBuf,
    profile_directory: PathBuf,
}

impl ProfileFixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "vehicle_profile_manifest_{label}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let profile_directory = root.join("profiles").join("test_profile");
        fs::create_dir_all(&profile_directory).unwrap();
        let fixture = Self {
            root,
            profile_directory,
        };
        fixture.write_domain_documents(0.33, 0.34);
        fixture.write_manifest(&fixture.default_domains());
        fixture
    }

    fn write_domain_documents(&self, front_radius: f64, rear_radius: f64) {
        self.write(
            "chassis.json",
            &format!(
                r#"{{"chassis":{{"vehicle_mass":575.0}},"geometry":{{"wheelbase":2.5,"front_track":1.6,"rear_track":1.5}}}}"#
            ),
        );
        self.write("fuel.json", r#"{"fuel":{}}"#);
        self.write("steering.json", r#"{"steering":{}}"#);
        self.write(
            "powertrain.json",
            r#"{"powertrain":{"final_drive":3.1}}"#,
        );
        self.write(
            "suspension.json",
            r#"{"suspension":{"front":{"spring_length":0.3},"rear":{"spring_length":0.3}}}"#,
        );
        self.write(
            "tires.json",
            &format!(
                r#"{{"tires":{{"front":{{"radius":{front_radius}}},"rear":{{"radius":{rear_radius}}}}}}}"#
            ),
        );
        self.write("brakes.json", r#"{"brakes":{}}"#);
        self.write("aerodynamics.json", r#"{"aero":{}}"#);
        self.write("driving_assistance.json", r#"{"aids":{}}"#);
    }

    fn default_domains(&self) -> String {
        r#""chassis":"chassis.json","fuel":"fuel.json","steering":"steering.json","powertrain":"powertrain.json","suspension":"suspension.json","tires":"tires.json","brakes":"brakes.json","aerodynamics":"aerodynamics.json","driving_assistance":"driving_assistance.json""#.to_string()
    }

    fn write_manifest(&self, domains: &str) {
        self.write(
            "vehicle.json",
            &format!(
                r#"{{"schema_version":1,"vehicle_identifier":"test_vehicle","profile_name":"test profile","units":"SI","coordinate_contract":{{"forward":"-Z","right":"+X","up":"+Y"}},"domains":{{{domains}}}}}"#
            ),
        );
    }

    fn write(&self, relative_path: &str, text: &str) {
        fs::write(self.profile_directory.join(relative_path), text).unwrap();
    }

    fn manifest_path(&self) -> PathBuf {
        self.profile_directory.join("vehicle.json")
    }

    fn load(&self) -> Result<LoadedPhysicalConfiguration, VehicleConfigurationLoadError> {
        load_physical_configuration(&VehicleProfileLoadRequest::new(
            &self.manifest_path(),
            &self.profile_directory,
        ))
    }
}

impl Drop for ProfileFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn loads_minimal_manifest_with_complete_provenance() {
    let fixture = ProfileFixture::create("loads");
    let loaded = fixture.load().expect("fixture manifest must load");
    assert_eq!(loaded.provenance.vehicle_identifier, "test_vehicle");
    assert_eq!(loaded.provenance.profile_name, "test profile");
    assert_eq!(loaded.provenance.dependencies.len(), 9);
    assert_eq!(loaded.provenance.global_digest_sha256.len(), 64);
    assert_eq!(loaded.configuration.chassis.vehicle_mass, 575.0);
    assert_eq!(loaded.configuration.tires.front_tire_radius, 0.33);
    assert_eq!(loaded.configuration.tires.rear_tire_radius, 0.34);
    assert!(loaded
        .provenance
        .dependencies
        .iter()
        .any(|dependency| dependency.domain == "tires"));
}

#[test]
fn external_audio_reference_is_ignored_by_physics_and_not_hashed() {
    let fixture = ProfileFixture::create("audio");
    let domains = format!(
        "{},\"audio\":\"missing_audio.json\"",
        fixture.default_domains()
    );
    fixture.write_manifest(&domains);
    let loaded = fixture
        .load()
        .expect("audio is owned by the audio engine, not the physics loader");
    assert!(!loaded
        .provenance
        .dependencies
        .iter()
        .any(|dependency| dependency.domain == "audio"));
}

#[test]
fn resolution_is_independent_from_the_process_working_directory() {
    let fixture = ProfileFixture::create("cwd");
    let absolute_manifest = fixture.manifest_path();
    let loaded = load_physical_configuration(&VehicleProfileLoadRequest::new(
        &absolute_manifest,
        &fixture.profile_directory,
    ))
    .expect("absolute manifest path with relative references must load");
    assert_eq!(loaded.configuration.chassis.vehicle_mass, 575.0);
}

#[test]
fn missing_referenced_document_reports_domain_and_path() {
    let fixture = ProfileFixture::create("missing");
    fs::remove_file(fixture.profile_directory.join("tires.json")).unwrap();
    let error = fixture.load().expect_err("missing tires document must fail");
    assert!(error.message.contains("tires"), "message: {error}");
    assert!(
        error.document_path.is_some(),
        "missing document must report its path: {error}"
    );
    assert_eq!(error.field.as_deref(), Some("domains.tires"));
}

#[test]
fn missing_required_domain_is_rejected() {
    let fixture = ProfileFixture::create("required");
    let domains = fixture.default_domains().replace(",\"brakes\":\"brakes.json\"", "");
    fixture.write_manifest(&domains);
    let error = fixture.load().expect_err("missing brakes domain must fail");
    assert!(error.message.contains("brakes"), "message: {error}");
}

#[test]
fn unsupported_manifest_schema_version_is_rejected() {
    let fixture = ProfileFixture::create("version");
    fixture.write(
        "vehicle.json",
        &format!(
            r#"{{"schema_version":2,"vehicle_identifier":"test_vehicle","profile_name":"test profile","units":"SI","coordinate_contract":{{"forward":"-Z","right":"+X","up":"+Y"}},"domains":{{{}}}}}"#,
            fixture.default_domains()
        ),
    );
    let error = fixture.load().expect_err("unsupported version must fail");
    assert!(error.message.contains("schema_version"), "message: {error}");
}

#[test]
fn unknown_manifest_field_is_rejected() {
    let fixture = ProfileFixture::create("unknown_field");
    fixture.write(
        "vehicle.json",
        &format!(
            r#"{{"schema_version":1,"vehicle_identifier":"test_vehicle","profile_name":"test profile","units":"SI","coordinate_contract":{{"forward":"-Z","right":"+X","up":"+Y"}},"domains":{{{}}},"extra":true}}"#,
            fixture.default_domains()
        ),
    );
    let error = fixture.load().expect_err("unknown manifest field must fail");
    assert!(error.message.contains("unknown field"), "message: {error}");
}

#[test]
fn unknown_domain_name_is_rejected() {
    let fixture = ProfileFixture::create("unknown_domain");
    let domains = format!("{},\"gadgets\":\"gadgets.json\"", fixture.default_domains());
    fixture.write_manifest(&domains);
    let error = fixture.load().expect_err("unknown domain must fail");
    assert!(error.message.contains("gadgets"), "message: {error}");
}

#[test]
fn unknown_domain_document_field_reports_document_path() {
    let fixture = ProfileFixture::create("document_field");
    fixture.write(
        "tires.json",
        r#"{"tires":{"front":{"radius":0.33},"rear":{"radius":0.34}},"wheels":{}}"#,
    );
    let error = fixture.load().expect_err("unknown document field must fail");
    assert!(error.message.contains("tires"), "message: {error}");
    assert!(error.document_path.is_some(), "message: {error}");
    assert!(error.message.contains("unknown field"), "message: {error}");
}

#[test]
fn reference_outside_authorized_root_is_rejected() {
    let fixture = ProfileFixture::create("escape");
    let outside_directory = fixture.root.join("outside");
    fs::create_dir_all(&outside_directory).unwrap();
    fs::write(outside_directory.join("tires.json"), r#"{"tires":{}}"#).unwrap();
    let domains = fixture
        .default_domains()
        .replace("\"tires\":\"tires.json\"", "\"tires\":\"../../outside/tires.json\"");
    fixture.write_manifest(&domains);
    let error = fixture.load().expect_err("escaping reference must fail");
    assert!(error.message.contains("authorized root"), "message: {error}");
}

#[test]
fn dependency_change_is_detectable_through_the_global_digest() {
    let fixture = ProfileFixture::create("digest");
    let first = fixture.load().expect("first load must succeed");
    fixture.write_domain_documents(0.36, 0.37);
    let second = fixture.load().expect("second load must succeed");
    assert_ne!(
        first.provenance.global_digest_sha256,
        second.provenance.global_digest_sha256
    );
    let first_tires = first
        .provenance
        .dependencies
        .iter()
        .find(|dependency| dependency.domain == "tires")
        .unwrap();
    let second_tires = second
        .provenance
        .dependencies
        .iter()
        .find(|dependency| dependency.domain == "tires")
        .unwrap();
    assert_ne!(first_tires.digest_sha256, second_tires.digest_sha256);
}

#[test]
fn out_of_range_value_is_rejected_with_field_context() {
    let fixture = ProfileFixture::create("range");
    fixture.write("chassis.json", r#"{"chassis":{"vehicle_mass":-1.0},"geometry":{"wheelbase":2.5}}"#);
    let error = fixture.load().expect_err("negative mass must fail");
    assert!(error.message.contains("vehicle_mass"), "message: {error}");
}

#[test]
fn resolved_document_round_trips_and_tracks_provenance() {
    let fixture = ProfileFixture::create("resolved");
    let loaded = fixture.load().expect("fixture manifest must load");
    let resolved = ResolvedVehicleConfiguration::from_loaded(loaded);
    let text = resolved.to_json_string_pretty().unwrap();
    let restored = ResolvedVehicleConfiguration::from_json_str(&text)
        .expect("resolved document must round trip");
    assert_eq!(
        restored.provenance.global_digest_sha256,
        resolved.provenance.global_digest_sha256
    );
    let restored_configuration = restored.configuration().unwrap();
    assert_eq!(restored_configuration.chassis.vehicle_mass, 575.0);
    assert_eq!(restored_configuration.tires.front_tire_radius, 0.33);
}

#[test]
fn resolved_document_rejects_a_tampered_configuration_payload() {
    let fixture = ProfileFixture::create("tampered");
    let loaded = fixture.load().expect("fixture manifest must load");
    let resolved = ResolvedVehicleConfiguration::from_loaded(loaded);
    let mut document: serde_json::Value =
        serde_json::from_str(&resolved.to_json_string_pretty().unwrap()).unwrap();
    document["configuration"]["chassis"]["vehicle_mass"] = serde_json::json!(-5.0);
    let error = ResolvedVehicleConfiguration::from_json_str(&document.to_string())
        .expect_err("tampered payload must fail validation");
    assert!(error.message.contains("vehicle_mass"), "message: {error}");
}

#[test]
fn provenance_helpers_are_deterministic() {
    let fixture = ProfileFixture::create("deterministic");
    let first = fixture.load().expect("first load must succeed");
    let second = fixture.load().expect("second load must succeed");
    assert_eq!(
        first.provenance.manifest_digest_sha256,
        second.provenance.manifest_digest_sha256
    );
    assert_eq!(
        first.provenance.global_digest_sha256,
        second.provenance.global_digest_sha256
    );
    assert_eq!(
        first
            .provenance
            .dependencies
            .iter()
            .map(|dependency| dependency.domain.clone())
            .collect::<Vec<_>>(),
        vec![
            "aerodynamics",
            "brakes",
            "chassis",
            "driving_assistance",
            "fuel",
            "powertrain",
            "steering",
            "suspension",
            "tires",
        ]
    );
}

#[test]
fn manifest_document_paths_stay_inside_the_profile_directory() {
    let fixture = ProfileFixture::create("paths");
    let loaded = fixture.load().expect("fixture manifest must load");
    assert!(Path::new(&loaded.provenance.manifest_path).is_absolute());
    assert!(fixture
        .profile_directory
        .join("tires.json")
        .canonicalize()
        .unwrap()
        .is_file());
}
