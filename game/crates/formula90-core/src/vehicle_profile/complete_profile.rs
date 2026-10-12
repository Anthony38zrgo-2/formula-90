use std::path::Path;

use vehicle_audio_engine::vehicle_audio_configuration::{
    VehicleAudioConfigurationLoadError, VehicleAudioDocument,
};
use vehicle_physics_engine::vehicle_configuration::{
    load_physical_configuration, resolve_domain_document_path, sha256_hex,
    ResolvedVehicleConfiguration, VehicleConfigurationLoadError, VehicleProfileDependencyDigest,
    VehicleProfileLoadRequest, VehicleProfileManifest, VehicleProfileProvenance,
    AUDIO_DOMAIN_NAME,
};

pub const COMPLETE_VEHICLE_PROFILE_CONTRACT_VERSION: u32 = 1;

#[derive(Debug)]
pub enum VehicleProfileCoordinationError {
    Manifest(VehicleConfigurationLoadError),
    Physical(VehicleConfigurationLoadError),
    Audio(VehicleAudioConfigurationLoadError),
}

impl std::fmt::Display for VehicleProfileCoordinationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manifest(error) => write!(formatter, "vehicle profile manifest: {error}"),
            Self::Physical(error) => write!(formatter, "physical configuration: {error}"),
            Self::Audio(error) => write!(formatter, "audio configuration: {error}"),
        }
    }
}

impl std::error::Error for VehicleProfileCoordinationError {}

#[derive(Debug, Clone)]
pub struct CompleteVehicleProfileProvenance {
    pub contract_version: u32,
    pub physical: VehicleProfileProvenance,
    pub audio: Option<VehicleProfileDependencyDigest>,
    pub global_digest_sha256: String,
}

#[derive(Debug, Clone)]
pub struct CompleteVehicleProfile {
    pub physical_configuration: ResolvedVehicleConfiguration,
    pub audio_document: Option<VehicleAudioDocument>,
    pub provenance: CompleteVehicleProfileProvenance,
}

pub fn load_complete_vehicle_profile(
    manifest_path: &Path,
    authorized_root: &Path,
) -> Result<CompleteVehicleProfile, VehicleProfileCoordinationError> {
    let manifest_text = std::fs::read_to_string(manifest_path).map_err(|error| {
        VehicleProfileCoordinationError::Manifest(
            VehicleConfigurationLoadError::new(format!(
                "vehicle profile manifest could not be read: {error}"
            ))
            .at_document(manifest_path),
        )
    })?;
    let manifest: VehicleProfileManifest = serde_json::from_str(&manifest_text).map_err(|error| {
        VehicleProfileCoordinationError::Manifest(
            VehicleConfigurationLoadError::new(format!(
                "vehicle profile manifest is invalid JSON: {error}"
            ))
            .at_document(manifest_path),
        )
    })?;

    let loaded_physical =
        load_physical_configuration(&VehicleProfileLoadRequest::new(manifest_path, authorized_root))
            .map_err(VehicleProfileCoordinationError::Physical)?;
    let physical_configuration = ResolvedVehicleConfiguration::from_loaded(loaded_physical);

    let mut audio_document = None;
    let mut audio_dependency = None;
    if let Some(audio_reference) = manifest.domains.get(AUDIO_DOMAIN_NAME) {
        let manifest_directory = manifest_path.parent().unwrap_or(Path::new("."));
        let audio_path =
            resolve_domain_document_path(authorized_root, manifest_directory, audio_reference)
                .map_err(|error| {
                    let mut contextual = VehicleAudioConfigurationLoadError::new(format!(
                        "domains.{AUDIO_DOMAIN_NAME} reference {audio_reference:?} is invalid: {}",
                        error.message
                    ));
                    contextual.document_path = error.document_path;
                    contextual.field = Some(format!("domains.{AUDIO_DOMAIN_NAME}"));
                    VehicleProfileCoordinationError::Audio(contextual)
                })?;
        let audio_bytes = std::fs::read(&audio_path).map_err(|error| {
            VehicleProfileCoordinationError::Audio(
                VehicleAudioConfigurationLoadError::new(format!(
                    "audio document could not be read: {error}"
                ))
                .at_document(&audio_path)
                .at_field(format!("domains.{AUDIO_DOMAIN_NAME}")),
            )
        })?;
        let digest_sha256 = sha256_hex(&audio_bytes);
        let document_text = String::from_utf8(audio_bytes).map_err(|error| {
            VehicleProfileCoordinationError::Audio(
                VehicleAudioConfigurationLoadError::new(format!(
                    "audio document is not valid UTF-8: {error}"
                ))
                .at_document(&audio_path),
            )
        })?;
        let value: serde_json::Value = serde_json::from_str(&document_text).map_err(|error| {
            VehicleProfileCoordinationError::Audio(
                VehicleAudioConfigurationLoadError::new(format!(
                    "audio document is invalid JSON: {error}"
                ))
                .at_document(&audio_path),
            )
        })?;
        let document = VehicleAudioDocument::from_value(value).map_err(|mut error| {
            error.document_path = Some(audio_path.clone());
            VehicleProfileCoordinationError::Audio(error)
        })?;
        audio_document = Some(document);
        audio_dependency = Some(VehicleProfileDependencyDigest {
            domain: AUDIO_DOMAIN_NAME.to_string(),
            reference: audio_reference.clone(),
            digest_sha256,
        });
    }

    let global_digest_sha256 = compute_complete_global_digest(
        &physical_configuration.provenance.global_digest_sha256,
        audio_dependency.as_ref(),
    );
    let provenance = CompleteVehicleProfileProvenance {
        contract_version: COMPLETE_VEHICLE_PROFILE_CONTRACT_VERSION,
        physical: physical_configuration.provenance.clone(),
        audio: audio_dependency,
        global_digest_sha256,
    };

    Ok(CompleteVehicleProfile {
        physical_configuration,
        audio_document,
        provenance,
    })
}

fn compute_complete_global_digest(
    physical_global_digest: &str,
    audio_dependency: Option<&VehicleProfileDependencyDigest>,
) -> String {
    let mut text = format!(
        "complete_vehicle_profile_v{COMPLETE_VEHICLE_PROFILE_CONTRACT_VERSION}\nphysical\t{physical_global_digest}\n"
    );
    if let Some(audio) = audio_dependency {
        text.push_str(&format!(
            "audio\t{}\t{}\n",
            audio.reference, audio.digest_sha256
        ));
    }
    sha256_hex(text.as_bytes())
}
