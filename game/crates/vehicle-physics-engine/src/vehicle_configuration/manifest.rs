use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const VEHICLE_PROFILE_MANIFEST_SCHEMA_VERSION: u32 = 1;

pub const CHASSIS_DOMAIN_NAME: &str = "chassis";
pub const FUEL_DOMAIN_NAME: &str = "fuel";
pub const STEERING_DOMAIN_NAME: &str = "steering";
pub const POWERTRAIN_DOMAIN_NAME: &str = "powertrain";
pub const SUSPENSION_DOMAIN_NAME: &str = "suspension";
pub const TIRES_DOMAIN_NAME: &str = "tires";
pub const BRAKES_DOMAIN_NAME: &str = "brakes";
pub const AERODYNAMICS_DOMAIN_NAME: &str = "aerodynamics";
pub const DRIVING_ASSISTANCE_DOMAIN_NAME: &str = "driving_assistance";
pub const UNDERFLOOR_CONTACT_DOMAIN_NAME: &str = "underfloor_contact";
pub const COUPLED_WORLD_DOMAIN_NAME: &str = "coupled_world";
pub const AUDIO_DOMAIN_NAME: &str = "audio";
pub const SUSPENSION_PRESENTATION_DOMAIN_NAME: &str = "suspension_presentation";

pub const REQUIRED_PHYSICAL_DOMAIN_NAMES: [&str; 9] = [
    CHASSIS_DOMAIN_NAME,
    FUEL_DOMAIN_NAME,
    STEERING_DOMAIN_NAME,
    POWERTRAIN_DOMAIN_NAME,
    SUSPENSION_DOMAIN_NAME,
    TIRES_DOMAIN_NAME,
    BRAKES_DOMAIN_NAME,
    AERODYNAMICS_DOMAIN_NAME,
    DRIVING_ASSISTANCE_DOMAIN_NAME,
];

pub const OPTIONAL_PHYSICAL_DOMAIN_NAMES: [&str; 2] =
    [UNDERFLOOR_CONTACT_DOMAIN_NAME, COUPLED_WORLD_DOMAIN_NAME];

pub const EXTERNALLY_OWNED_DOMAIN_NAMES: [&str; 2] =
    [AUDIO_DOMAIN_NAME, SUSPENSION_PRESENTATION_DOMAIN_NAME];

pub const SUPPORTED_UNITS: &str = "SI";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleProfileManifest {
    pub schema_version: u32,
    pub vehicle_identifier: String,
    pub profile_name: String,
    pub units: String,
    pub coordinate_contract: HashMap<String, String>,
    pub domains: BTreeMap<String, String>,
}

impl VehicleProfileManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != VEHICLE_PROFILE_MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "unsupported vehicle profile manifest schema_version {} (supported: {})",
                self.schema_version, VEHICLE_PROFILE_MANIFEST_SCHEMA_VERSION
            ));
        }
        if self.vehicle_identifier.trim().is_empty() {
            return Err("vehicle_identifier must not be empty".to_string());
        }
        if self.profile_name.trim().is_empty() {
            return Err("profile_name must not be empty".to_string());
        }
        if self.units != SUPPORTED_UNITS {
            return Err(format!(
                "unsupported units {:?} (supported: {:?})",
                self.units, SUPPORTED_UNITS
            ));
        }
        for coordinate_name in ["forward", "right", "up"] {
            match self.coordinate_contract.get(coordinate_name) {
                Some(value) if !value.trim().is_empty() => {}
                Some(_) => {
                    return Err(format!(
                        "coordinate_contract.{coordinate_name} must not be empty"
                    ))
                }
                None => {
                    return Err(format!(
                        "coordinate_contract must declare the {coordinate_name} axis"
                    ))
                }
            }
        }
        for domain_name in self.domains.keys() {
            if !is_known_domain_name(domain_name) {
                return Err(format!("domains.{domain_name} is not a known domain"));
            }
        }
        for reference in self.domains.values() {
            if reference.trim().is_empty() {
                return Err("domain references must not be empty".to_string());
            }
            if reference.contains("://") {
                return Err(format!(
                    "domain reference {reference:?} must be a relative filesystem path"
                ));
            }
        }
        for required in REQUIRED_PHYSICAL_DOMAIN_NAMES {
            if !self.domains.contains_key(required) {
                return Err(format!("domains.{required} is required"));
            }
        }
        Ok(())
    }
}

pub fn is_known_domain_name(domain_name: &str) -> bool {
    REQUIRED_PHYSICAL_DOMAIN_NAMES.contains(&domain_name)
        || OPTIONAL_PHYSICAL_DOMAIN_NAMES.contains(&domain_name)
        || EXTERNALLY_OWNED_DOMAIN_NAMES.contains(&domain_name)
}

pub fn is_physical_domain_name(domain_name: &str) -> bool {
    REQUIRED_PHYSICAL_DOMAIN_NAMES.contains(&domain_name)
        || OPTIONAL_PHYSICAL_DOMAIN_NAMES.contains(&domain_name)
}
