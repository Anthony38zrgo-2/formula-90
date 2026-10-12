use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

use super::manifest::{
    is_physical_domain_name, VehicleProfileManifest, AERODYNAMICS_DOMAIN_NAME, BRAKES_DOMAIN_NAME,
    CHASSIS_DOMAIN_NAME, COUPLED_WORLD_DOMAIN_NAME, DRIVING_ASSISTANCE_DOMAIN_NAME,
    FUEL_DOMAIN_NAME, POWERTRAIN_DOMAIN_NAME, STEERING_DOMAIN_NAME, SUSPENSION_DOMAIN_NAME,
    TIRES_DOMAIN_NAME, UNDERFLOOR_CONTACT_DOMAIN_NAME,
};
use super::resolved_profile::{
    sha256_hex, VehicleProfileDependencyDigest, VehicleProfileProvenance,
    RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION,
};
use super::vehicle_configuration::VehicleConfig;

/// Loads a physical configuration from a profile manifest or a legacy flat document.
/// Manifest resolution uses the manifest directory as the authorized root.
pub fn load_vehicle_configuration(path: &Path) -> Result<VehicleConfig, String> {
    let authorized_root = path.parent().filter(|directory| !directory.as_os_str().is_empty()).unwrap_or(Path::new("."));
    load_vehicle_configuration_from_authorized_root(path, authorized_root)
}

pub fn load_vehicle_configuration_from_authorized_root(
    path: &Path,
    authorized_root: &Path,
) -> Result<VehicleConfig, String> {
    let canonical_root = std::fs::canonicalize(authorized_root)
        .map_err(|error| format!("authorized root {} is not accessible: {error}", authorized_root.display()))?;
    let canonical_path = std::fs::canonicalize(path)
        .map_err(|error| format!("vehicle configuration {} is not accessible: {error}", path.display()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!("vehicle configuration {} resolves outside the authorized root {}", path.display(), authorized_root.display()));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let document: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse config: {error}"))?;
    if document.get("domains").is_some() {
        load_physical_configuration(&VehicleProfileLoadRequest::new(path, authorized_root))
            .map(|loaded| loaded.configuration)
            .map_err(|error| error.to_string())
    } else {
        VehicleConfig::from_json_str(&text)
    }
}

pub const LEGACY_ASSEMBLY_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone)]
pub struct VehicleConfigurationLoadError {
    pub document_path: Option<PathBuf>,
    pub field: Option<String>,
    pub message: String,
}

impl VehicleConfigurationLoadError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            document_path: None,
            field: None,
            message: message.into(),
        }
    }

    pub fn at_document(mut self, document_path: impl Into<PathBuf>) -> Self {
        self.document_path = Some(document_path.into());
        self
    }

    pub fn at_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl std::fmt::Display for VehicleConfigurationLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(document_path) = &self.document_path {
            write!(formatter, " (document: {})", document_path.display())?;
        }
        if let Some(field) = &self.field {
            write!(formatter, " (field: {field})")?;
        }
        Ok(())
    }
}

impl std::error::Error for VehicleConfigurationLoadError {}

pub struct VehicleProfileLoadRequest<'a> {
    pub manifest_path: &'a Path,
    pub authorized_root: &'a Path,
}

impl<'a> VehicleProfileLoadRequest<'a> {
    pub fn new(manifest_path: &'a Path, authorized_root: &'a Path) -> Self {
        Self {
            manifest_path,
            authorized_root,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedPhysicalConfiguration {
    pub configuration: crate::VehicleConfig,
    pub provenance: VehicleProfileProvenance,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChassisDomainDocument {
    chassis: super::chassis::document::JsonChassis,
    geometry: super::chassis::document::JsonGeometry,
    #[serde(default)]
    physical_mass_inventory: Option<crate::vehicle_mass_inventory::VehicleMassInventory>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FuelDomainDocument {
    fuel: super::fuel::document::JsonFuel,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SteeringDomainDocument {
    steering: super::steering::document::JsonSteering,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PowertrainDomainDocument {
    powertrain: super::powertrain::document::JsonPowertrain,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuspensionDomainDocument {
    suspension: super::suspension::document::JsonSuspension,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TiresDomainDocument {
    tires: super::tires::document::JsonTires,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BrakesDomainDocument {
    brakes: super::brakes::document::JsonBrakes,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AerodynamicsDomainDocument {
    aero: super::aerodynamics::document::JsonAero,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DrivingAssistanceDomainDocument {
    aids: super::driving_assistance::document::JsonAids,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnderfloorContactDomainDocument {
    underfloor_contact: crate::underfloor_contact_configuration::UnderfloorContactConfig,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoupledWorldDomainDocument {
    coupled_world: crate::coupled_vehicle_world::CoupledVehicleWorldConfiguration,
}

pub fn load_physical_configuration(
    request: &VehicleProfileLoadRequest<'_>,
) -> Result<LoadedPhysicalConfiguration, VehicleConfigurationLoadError> {
    let manifest_bytes = std::fs::read(request.manifest_path).map_err(|error| {
        VehicleConfigurationLoadError::new(format!(
            "vehicle profile manifest could not be read: {error}"
        ))
        .at_document(request.manifest_path)
    })?;
    let manifest_digest_sha256 = sha256_hex(&manifest_bytes);
    let manifest_text = String::from_utf8(manifest_bytes).map_err(|error| {
        VehicleConfigurationLoadError::new(format!(
            "vehicle profile manifest is not valid UTF-8: {error}"
        ))
        .at_document(request.manifest_path)
    })?;
    let manifest: VehicleProfileManifest =
        serde_json::from_str(&manifest_text).map_err(|error| {
            VehicleConfigurationLoadError::new(format!(
                "vehicle profile manifest is invalid JSON: {error}"
            ))
            .at_document(request.manifest_path)
        })?;
    manifest.validate().map_err(|message| {
        VehicleConfigurationLoadError::new(message).at_document(request.manifest_path)
    })?;

    let manifest_directory = request.manifest_path.parent().unwrap_or(Path::new("."));
    let authorized_root = normalize_lexically(request.authorized_root);

    let mut merged_document = serde_json::Map::new();
    merged_document.insert(
        "schema_version".to_string(),
        serde_json::json!(LEGACY_ASSEMBLY_SCHEMA_VERSION),
    );
    merged_document.insert(
        "vehicle_id".to_string(),
        serde_json::json!(manifest.vehicle_identifier),
    );
    merged_document.insert(
        "profile_name".to_string(),
        serde_json::json!(manifest.profile_name),
    );
    merged_document.insert("units".to_string(), serde_json::json!(manifest.units));
    merged_document.insert(
        "coordinate_contract".to_string(),
        serde_json::to_value(&manifest.coordinate_contract).unwrap_or_default(),
    );

    let mut dependencies = Vec::new();
    let mut chassis_declares_mass_inventory = false;

    for (domain_name, reference) in &manifest.domains {
        if !is_physical_domain_name(domain_name) {
            continue;
        }
        let document_path =
            resolve_domain_document_path(&authorized_root, manifest_directory, reference).map_err(
                |error| {
                    let mut contextual = VehicleConfigurationLoadError::new(format!(
                        "domains.{domain_name} reference {reference:?} is invalid: {}",
                        error.message
                    ));
                    contextual.document_path = error.document_path;
                    contextual.field = Some(format!("domains.{domain_name}"));
                    contextual
                },
            )?;
        let document_bytes = std::fs::read(&document_path).map_err(|error| {
            VehicleConfigurationLoadError::new(format!(
                "referenced {domain_name} document could not be read: {error}"
            ))
            .at_document(&document_path)
            .at_field(format!("domains.{domain_name}"))
        })?;
        dependencies.push(VehicleProfileDependencyDigest {
            domain: domain_name.clone(),
            reference: reference.clone(),
            digest_sha256: sha256_hex(&document_bytes),
        });
        let document_text = String::from_utf8(document_bytes).map_err(|error| {
            VehicleConfigurationLoadError::new(format!(
                "{domain_name} document is not valid UTF-8: {error}"
            ))
            .at_document(&document_path)
        })?;
        let document_value: serde_json::Value =
            serde_json::from_str(&document_text).map_err(|error| {
                VehicleConfigurationLoadError::new(format!(
                    "{domain_name} document is invalid JSON: {error}"
                ))
                .at_document(&document_path)
            })?;

        match domain_name.as_str() {
            CHASSIS_DOMAIN_NAME => {
                let typed: ChassisDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "chassis document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                chassis_declares_mass_inventory = typed.physical_mass_inventory.is_some();
                insert_section(&mut merged_document, &document_value, "chassis");
                insert_section(&mut merged_document, &document_value, "geometry");
                insert_section(&mut merged_document, &document_value, "physical_mass_inventory");
            }
            FUEL_DOMAIN_NAME => {
                let _typed: FuelDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "fuel document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "fuel");
            }
            STEERING_DOMAIN_NAME => {
                let _typed: SteeringDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "steering document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "steering");
            }
            POWERTRAIN_DOMAIN_NAME => {
                let _typed: PowertrainDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "powertrain document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "powertrain");
            }
            SUSPENSION_DOMAIN_NAME => {
                let _typed: SuspensionDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "suspension document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "suspension");
            }
            TIRES_DOMAIN_NAME => {
                let _typed: TiresDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "tires document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "tires");
            }
            BRAKES_DOMAIN_NAME => {
                let _typed: BrakesDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "brakes document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "brakes");
            }
            AERODYNAMICS_DOMAIN_NAME => {
                let _typed: AerodynamicsDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "aerodynamics document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "aero");
            }
            DRIVING_ASSISTANCE_DOMAIN_NAME => {
                let _typed: DrivingAssistanceDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "driving assistance document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "aids");
            }
            UNDERFLOOR_CONTACT_DOMAIN_NAME => {
                let _typed: UnderfloorContactDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "underfloor contact document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "underfloor_contact");
            }
            COUPLED_WORLD_DOMAIN_NAME => {
                let _typed: CoupledWorldDomainDocument =
                    serde_json::from_value(document_value.clone()).map_err(|error| {
                        VehicleConfigurationLoadError::new(format!(
                            "coupled world document does not match its contract: {error}"
                        ))
                        .at_document(&document_path)
                    })?;
                insert_section(&mut merged_document, &document_value, "coupled_world");
            }
            _ => {}
        }
    }

    super::consistency_validation::validate_cross_domain(
        &manifest,
        chassis_declares_mass_inventory,
    )
    .map_err(|message| {
        VehicleConfigurationLoadError::new(message).at_document(request.manifest_path)
    })?;

    let assembled = serde_json::Value::Object(merged_document);
    let configuration = crate::VehicleConfig::from_json_str(&assembled.to_string()).map_err(
        |error| {
            VehicleConfigurationLoadError::new(format!(
                "assembled physical configuration is invalid: {error}"
            ))
            .at_document(request.manifest_path)
        },
    )?;

    let global_digest_sha256 = VehicleProfileProvenance::compute_global_digest(
        RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION,
        &manifest_digest_sha256,
        &dependencies,
    );
    let provenance = VehicleProfileProvenance {
        contract_version: RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION,
        manifest_path: request.manifest_path.display().to_string(),
        manifest_digest_sha256,
        vehicle_identifier: manifest.vehicle_identifier,
        profile_name: manifest.profile_name,
        coordinate_contract: manifest.coordinate_contract,
        dependencies,
        global_digest_sha256,
    };

    Ok(LoadedPhysicalConfiguration {
        configuration,
        provenance,
    })
}

pub fn resolve_domain_document_path(
    authorized_root: &Path,
    manifest_directory: &Path,
    reference: &str,
) -> Result<PathBuf, VehicleConfigurationLoadError> {
    if reference.contains("://") {
        return Err(VehicleConfigurationLoadError::new(format!(
            "domain reference {reference:?} must be a relative filesystem path, not a URL"
        )));
    }
    let reference_path = Path::new(reference);
    if reference_path.is_absolute() {
        return Err(VehicleConfigurationLoadError::new(format!(
            "domain reference {reference:?} must be relative to the manifest"
        )));
    }
    let joined = manifest_directory.join(reference_path);
    let normalized_root = normalize_lexically(authorized_root);
    let normalized_joined = normalize_lexically(&joined);
    if !normalized_joined.starts_with(&normalized_root) {
        return Err(VehicleConfigurationLoadError::new(format!(
            "domain reference {reference:?} resolves outside the authorized root {}",
            authorized_root.display()
        ))
        .at_document(&normalized_joined));
    }
    let canonical_root = std::fs::canonicalize(&normalized_root).map_err(|error| {
        VehicleConfigurationLoadError::new(format!(
            "authorized root {} is not accessible: {error}",
            authorized_root.display()
        ))
    })?;
    let canonical_document = std::fs::canonicalize(&joined).map_err(|error| {
        VehicleConfigurationLoadError::new(format!(
            "referenced document could not be resolved: {error}"
        ))
        .at_document(&normalized_joined)
    })?;
    if !canonical_document.starts_with(&canonical_root) {
        return Err(VehicleConfigurationLoadError::new(format!(
            "domain reference {reference:?} resolves outside the authorized root {}",
            authorized_root.display()
        ))
        .at_document(&canonical_document));
    }
    Ok(canonical_document)
}

pub(crate) fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn insert_section(
    merged_document: &mut serde_json::Map<String, serde_json::Value>,
    document_value: &serde_json::Value,
    section_name: &str,
) {
    if let Some(section_value) = document_value.get(section_name) {
        if !section_value.is_null() {
            merged_document.insert(section_name.to_string(), section_value.clone());
        }
    }
}
