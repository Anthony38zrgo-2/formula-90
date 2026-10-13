use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::loader::{LoadedPhysicalConfiguration, VehicleConfigurationLoadError};

pub const RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION: u32 = 1;
pub const RESOLVED_VEHICLE_CONFIGURATION_DOCUMENT_KIND: &str = "resolved_vehicle_configuration";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleProfileDependencyDigest {
    pub domain: String,
    pub reference: String,
    pub digest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleProfileProvenance {
    pub contract_version: u32,
    pub manifest_path: String,
    pub manifest_digest_sha256: String,
    pub vehicle_identifier: String,
    pub profile_name: String,
    pub coordinate_contract: HashMap<String, String>,
    pub dependencies: Vec<VehicleProfileDependencyDigest>,
    pub global_digest_sha256: String,
}

impl VehicleProfileProvenance {
    pub fn compute_global_digest(
        contract_version: u32,
        manifest_digest_sha256: &str,
        dependencies: &[VehicleProfileDependencyDigest],
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("vehicle_profile_contract_v{contract_version}\n"));
        hasher.update(format!("manifest\t{manifest_digest_sha256}\n"));
        for dependency in dependencies {
            hasher.update(format!(
                "{}\t{}\t{}\n",
                dependency.domain, dependency.reference, dependency.digest_sha256
            ));
        }
        finalize_hex(hasher)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    finalize_hex(hasher)
}

fn finalize_hex(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedVehicleConfiguration {
    pub schema_version: u32,
    pub document_kind: String,
    pub configuration: serde_json::Value,
    pub provenance: VehicleProfileProvenance,
}

impl ResolvedVehicleConfiguration {
    pub fn from_loaded(loaded: LoadedPhysicalConfiguration) -> Self {
        let configuration = loaded.configuration.to_resolved_configuration_value();
        Self {
            schema_version: RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION,
            document_kind: RESOLVED_VEHICLE_CONFIGURATION_DOCUMENT_KIND.to_string(),
            configuration,
            provenance: loaded.provenance,
        }
    }

    pub fn to_json_string_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("resolved vehicle configuration could not serialize: {error}"))
    }

    pub fn from_json_str(json: &str) -> Result<Self, VehicleConfigurationLoadError> {
        let resolved: Self = serde_json::from_str(json)
            .map_err(|error| VehicleConfigurationLoadError::new(format!("invalid resolved vehicle configuration: {error}")))?;
        if resolved.schema_version != RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION {
            return Err(VehicleConfigurationLoadError::new(format!(
                "unsupported resolved vehicle configuration schema_version {} (supported: {})",
                resolved.schema_version, RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION
            )));
        }
        if resolved.document_kind != RESOLVED_VEHICLE_CONFIGURATION_DOCUMENT_KIND {
            return Err(VehicleConfigurationLoadError::new(format!(
                "unsupported resolved vehicle configuration document_kind {:?}",
                resolved.document_kind
            )));
        }
        crate::VehicleConfig::from_json_str(&resolved.configuration.to_string()).map_err(|error| {
            VehicleConfigurationLoadError::new(format!(
                "resolved configuration payload is not a valid physical configuration: {error}"
            ))
        })?;
        Ok(resolved)
    }

    pub fn configuration(&self) -> Result<crate::VehicleConfig, String> {
        crate::VehicleConfig::from_json_str(&self.configuration.to_string())
    }
}
