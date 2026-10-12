use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Map, Value};

fn main() {
    let source_output = Command::new("git").args(["rev-parse", "HEAD"]).output().expect("Physical world requires Git source provenance");
    assert!(source_output.status.success(), "Cannot resolve physical world source");
    let source_identifier = String::from_utf8(source_output.stdout).expect("Physical source identifier must be UTF-8");
    let source_identifier = source_identifier.trim();
    assert_eq!(source_identifier.len(), 40, "Physical world source must be a full commit identifier");
    println!("cargo:rustc-env=COUPLED_WORLD_SOURCE_IDENTIFIER={source_identifier}");
    if let Ok(reference_output) = Command::new("git").args(["symbolic-ref", "-q", "HEAD"]).output() {
        if reference_output.status.success() {
            let reference = String::from_utf8(reference_output.stdout).unwrap();
            let reference_path_output = Command::new("git").args(["rev-parse", "--git-path", reference.trim()]).output().unwrap();
            let reference_path = String::from_utf8(reference_path_output.stdout).unwrap();
            println!("cargo:rerun-if-changed={}", reference_path.trim());
        }
    }
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    // Re-run the build script whenever crate sources change so the embedded
    // GIT_HASH reflects the commit the DLL was actually built from.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    // HEAD changes on every commit (even with unchanged sources); watch it so
    // the stale build-script output is invalidated and GIT_HASH is refreshed
    // (CAL-1300 parity fix, mirrors formula90-core/vehicle-audio-engine).
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=FORMULA90_FORCE_BUILD_SHA_REFRESH");
    generate_embedded_geometric_profile();
}

const EMBEDDED_DOCUMENT_FILE_NAME: &str = "resolved_f1_2030_v10_geometric.json";
const LEGACY_ASSEMBLY_SCHEMA_VERSION: u32 = 3;

fn generate_embedded_geometric_profile() {
    let crate_directory =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let profile_directory = crate_directory.join("../../data/vehicles/f1_2030/profiles/geometric");
    let manifest_path = profile_directory.join("vehicle.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest_text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest_path.display()));
    let manifest: Value = serde_json::from_str(&manifest_text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", manifest_path.display()));
    let schema_version = manifest["schema_version"]
        .as_u64()
        .expect("manifest schema_version must be an integer");
    assert_eq!(
        schema_version, 1,
        "unsupported vehicle profile manifest schema version"
    );

    let domain_references = manifest["domains"]
        .as_object()
        .expect("manifest domains must be an object");

    let mut merged_document = Map::new();
    merged_document.insert(
        "schema_version".to_string(),
        json!(LEGACY_ASSEMBLY_SCHEMA_VERSION),
    );
    merged_document.insert("vehicle_id".to_string(), manifest["vehicle_identifier"].clone());
    merged_document.insert("profile_name".to_string(), manifest["profile_name"].clone());
    merged_document.insert("units".to_string(), manifest["units"].clone());
    merged_document.insert(
        "coordinate_contract".to_string(),
        manifest["coordinate_contract"].clone(),
    );

    let section_map: [(&str, &[&str]); 11] = [
        ("chassis", &["chassis", "geometry", "physical_mass_inventory"]),
        ("fuel", &["fuel"]),
        ("steering", &["steering"]),
        ("powertrain", &["powertrain"]),
        ("suspension", &["suspension"]),
        ("tires", &["tires"]),
        ("brakes", &["brakes"]),
        ("aerodynamics", &["aero"]),
        ("driving_assistance", &["aids"]),
        ("underfloor_contact", &["underfloor_contact"]),
        ("coupled_world", &["coupled_world"]),
    ];

    for (domain_name, section_names) in section_map {
        let reference = domain_references
            .get(domain_name)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("manifest domains.{domain_name} is required"));
        let document_path = profile_directory.join(reference);
        println!("cargo:rerun-if-changed={}", document_path.display());
        let document_text = fs::read_to_string(&document_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", document_path.display()));
        let document: Value = serde_json::from_str(&document_text)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", document_path.display()));
        for section_name in section_names {
            if let Some(section_value) = document.get(*section_name) {
                if !section_value.is_null() {
                    merged_document.insert(section_name.to_string(), section_value.clone());
                }
            }
        }
    }

    let output_directory = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let output_path = output_directory.join(EMBEDDED_DOCUMENT_FILE_NAME);
    let output_text = serde_json::to_string(&Value::Object(merged_document))
        .expect("resolved vehicle configuration must serialize");
    fs::write(&output_path, output_text).expect("resolved vehicle configuration must be written");
}
