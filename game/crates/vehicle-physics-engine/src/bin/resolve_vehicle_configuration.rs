use std::path::PathBuf;
use vehicle_physics_engine::vehicle_configuration::{
    load_physical_configuration, ResolvedVehicleConfiguration, VehicleProfileLoadRequest,
};

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 2 {
        return Err("usage: resolve_vehicle_configuration AUTHORIZED_ROOT MANIFEST_PATH".to_string());
    }
    let authorized_root = PathBuf::from(&arguments[0]);
    let manifest_path = PathBuf::from(&arguments[1]);
    let canonical_root = std::fs::canonicalize(&authorized_root).map_err(|error| error.to_string())?;
    let canonical_manifest = std::fs::canonicalize(&manifest_path).map_err(|error| error.to_string())?;
    if !canonical_manifest.starts_with(&canonical_root) {
        return Err(format!("vehicle profile {} is outside authorized root {}", manifest_path.display(), authorized_root.display()));
    }
    let loaded = load_physical_configuration(&VehicleProfileLoadRequest::new(
        &manifest_path,
        &authorized_root,
    ))
    .map_err(|error| error.to_string())?;
    println!("{}", ResolvedVehicleConfiguration::from_loaded(loaded).to_json_string_pretty()?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
