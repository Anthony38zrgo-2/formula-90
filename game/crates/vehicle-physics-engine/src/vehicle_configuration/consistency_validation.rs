use super::manifest::{VehicleProfileManifest, COUPLED_WORLD_DOMAIN_NAME};

pub(crate) fn validate_cross_domain(
    manifest: &VehicleProfileManifest,
    chassis_declares_mass_inventory: bool,
) -> Result<(), String> {
    if manifest.domains.contains_key(COUPLED_WORLD_DOMAIN_NAME)
        && !chassis_declares_mass_inventory
    {
        return Err(format!(
            "domains.{COUPLED_WORLD_DOMAIN_NAME} requires physical_mass_inventory in the chassis document"
        ));
    }
    Ok(())
}
