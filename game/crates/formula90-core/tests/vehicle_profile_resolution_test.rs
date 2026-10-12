use std::fs;
use std::path::PathBuf;

use formula90_core::vehicle_profile::{
    load_complete_vehicle_profile, VehicleProfileCoordinationError,
};

struct CompleteProfileFixture {
    root: PathBuf,
    profile_directory: PathBuf,
}

impl CompleteProfileFixture {
    fn create(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "complete_vehicle_profile_{label}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let profile_directory = root.join("profiles").join("test_profile");
        fs::create_dir_all(&profile_directory).unwrap();
        let fixture = Self {
            root,
            profile_directory,
        };
        fixture.write_physics_documents();
        fixture.write_audio_document(1.0);
        fixture.write_manifest();
        fixture
    }

    fn write_physics_documents(&self) {
        self.write(
            "chassis.json",
            r#"{"chassis":{"vehicle_mass":575.0},"geometry":{"wheelbase":2.5,"front_track":1.6,"rear_track":1.5}}"#,
        );
        self.write("fuel.json", r#"{"fuel":{}}"#);
        self.write("steering.json", r#"{"steering":{}}"#);
        self.write("powertrain.json", r#"{"powertrain":{"final_drive":3.1}}"#);
        self.write(
            "suspension.json",
            r#"{"suspension":{"front":{"spring_length":0.3},"rear":{"spring_length":0.3}}}"#,
        );
        self.write(
            "tires.json",
            r#"{"tires":{"front":{"radius":0.33},"rear":{"radius":0.34}}}"#,
        );
        self.write("brakes.json", r#"{"brakes":{}}"#);
        self.write("aerodynamics.json", r#"{"aero":{}}"#);
        self.write("driving_assistance.json", r#"{"aids":{}}"#);
    }

    fn write_audio_document(&self, engine_gain: f64) {
        self.write(
            "audio.json",
            &format!(
                r#"{{"continuous_source":"grand_prix_sampler","cpp_dsp":{{"enabled":false,"gain":0.12}},"grand_prix_sampler":{{"manifest":"sounds/banks/v10-v2-bank/manifest.json","schema_version":1,"engine_gain":{engine_gain}}}}}"#
            ),
        );
    }

    fn write_manifest(&self) {
        self.write(
            "vehicle.json",
            r#"{"schema_version":1,"vehicle_identifier":"test_vehicle","profile_name":"test profile","units":"SI","coordinate_contract":{"forward":"-Z","right":"+X","up":"+Y"},"domains":{"chassis":"chassis.json","fuel":"fuel.json","steering":"steering.json","powertrain":"powertrain.json","suspension":"suspension.json","tires":"tires.json","brakes":"brakes.json","aerodynamics":"aerodynamics.json","driving_assistance":"driving_assistance.json","audio":"audio.json"}}"#,
        );
    }

    fn write(&self, relative_path: &str, text: &str) {
        fs::write(self.profile_directory.join(relative_path), text).unwrap();
    }

    fn load(&self) -> Result<formula90_core::vehicle_profile::CompleteVehicleProfile, VehicleProfileCoordinationError> {
        load_complete_vehicle_profile(
            &self.profile_directory.join("vehicle.json"),
            &self.profile_directory,
        )
    }
}

impl Drop for CompleteProfileFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn loads_physical_and_audio_domains_from_one_manifest() {
    let fixture = CompleteProfileFixture::create("loads");
    let profile = fixture.load().expect("complete profile must load");
    assert_eq!(
        profile
            .physical_configuration
            .provenance
            .vehicle_identifier,
        "test_vehicle"
    );
    let audio_document = profile
        .audio_document
        .as_ref()
        .expect("audio document must be present");
    assert_eq!(
        audio_document.continuous_source.as_deref(),
        Some("grand_prix_sampler")
    );
    assert_eq!(
        audio_document
            .grand_prix_sampler
            .as_ref()
            .expect("grand prix sampler section must parse")
            .engine_gain,
        1.0
    );
    assert!(profile.provenance.audio.is_some());
    assert_eq!(profile.provenance.global_digest_sha256.len(), 64);
}

#[test]
fn audio_change_is_detectable_through_the_complete_digest() {
    let fixture = CompleteProfileFixture::create("digest");
    let first = fixture.load().expect("first load must succeed");
    fixture.write_audio_document(0.5);
    let second = fixture.load().expect("second load must succeed");
    assert_eq!(
        first.provenance.physical.global_digest_sha256,
        second.provenance.physical.global_digest_sha256
    );
    assert_ne!(
        first.provenance.global_digest_sha256,
        second.provenance.global_digest_sha256
    );
    assert_ne!(
        first.provenance.audio.as_ref().unwrap().digest_sha256,
        second.provenance.audio.as_ref().unwrap().digest_sha256
    );
}

#[test]
fn missing_audio_document_reports_its_path() {
    let fixture = CompleteProfileFixture::create("missing_audio");
    fs::remove_file(fixture.profile_directory.join("audio.json")).unwrap();
    let error = fixture.load().expect_err("missing audio must fail");
    match error {
        VehicleProfileCoordinationError::Audio(error) => {
            assert!(error.document_path.is_some(), "message: {error}");
            assert_eq!(error.field.as_deref(), Some("domains.audio"));
        }
        other => panic!("expected an audio error, got: {other}"),
    }
}

#[test]
fn unknown_audio_key_is_rejected() {
    let fixture = CompleteProfileFixture::create("audio_key");
    fixture.write(
        "audio.json",
        r#"{"continuous_source":"legacy","gadget":true}"#,
    );
    let error = fixture.load().expect_err("unknown audio key must fail");
    match error {
        VehicleProfileCoordinationError::Audio(error) => {
            assert!(error.message.contains("unknown field"), "message: {error}");
        }
        other => panic!("expected an audio error, got: {other}"),
    }
}

#[test]
fn missing_physical_document_maps_to_a_physical_error() {
    let fixture = CompleteProfileFixture::create("missing_physical");
    fs::remove_file(fixture.profile_directory.join("tires.json")).unwrap();
    let error = fixture.load().expect_err("missing physical domain must fail");
    match error {
        VehicleProfileCoordinationError::Physical(error) => {
            assert_eq!(error.field.as_deref(), Some("domains.tires"));
        }
        other => panic!("expected a physical error, got: {other}"),
    }
}

#[test]
fn profile_without_audio_domain_still_resolves() {
    let fixture = CompleteProfileFixture::create("no_audio");
    fixture.write(
        "vehicle.json",
        r#"{"schema_version":1,"vehicle_identifier":"test_vehicle","profile_name":"test profile","units":"SI","coordinate_contract":{"forward":"-Z","right":"+X","up":"+Y"},"domains":{"chassis":"chassis.json","fuel":"fuel.json","steering":"steering.json","powertrain":"powertrain.json","suspension":"suspension.json","tires":"tires.json","brakes":"brakes.json","aerodynamics":"aerodynamics.json","driving_assistance":"driving_assistance.json"}}"#,
    );
    let profile = fixture.load().expect("profile without audio must load");
    assert!(profile.audio_document.is_none());
    assert!(profile.provenance.audio.is_none());
}
