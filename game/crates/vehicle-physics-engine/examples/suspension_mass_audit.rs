use std::path::Path;
use vehicle_physics_engine::suspension_mass_audit::SuspensionMassAudit;
use vehicle_physics_engine::VehicleConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile_path = std::env::args()
        .nth(1)
        .ok_or("Expected vehicle profile path")?;
    let configuration = VehicleConfig::from_json_path(Path::new(&profile_path))?;
    let audit = SuspensionMassAudit::from_vehicle_configuration(&configuration)?;
    println!("{}", serde_json::to_string_pretty(&audit)?);
    Ok(())
}
