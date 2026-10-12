use std::path::{Path, PathBuf};

use super::document::VehicleAudioDocument;
use super::validation;

#[derive(Debug, Clone)]
pub struct VehicleAudioConfigurationLoadError {
    pub document_path: Option<PathBuf>,
    pub field: Option<String>,
    pub message: String,
}

impl VehicleAudioConfigurationLoadError {
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

impl std::fmt::Display for VehicleAudioConfigurationLoadError {
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

impl std::error::Error for VehicleAudioConfigurationLoadError {}

pub fn load_vehicle_audio_document(
    document_path: &Path,
) -> Result<VehicleAudioDocument, VehicleAudioConfigurationLoadError> {
    let bytes = std::fs::read(document_path).map_err(|error| {
        VehicleAudioConfigurationLoadError::new(format!(
            "vehicle audio document could not be read: {error}"
        ))
        .at_document(document_path)
    })?;
    let text = String::from_utf8(bytes).map_err(|error| {
        VehicleAudioConfigurationLoadError::new(format!(
            "vehicle audio document is not valid UTF-8: {error}"
        ))
        .at_document(document_path)
    })?;
    let document: VehicleAudioDocument = serde_json::from_str(&text).map_err(|error| {
        VehicleAudioConfigurationLoadError::new(format!(
            "vehicle audio document is invalid JSON: {error}"
        ))
        .at_document(document_path)
    })?;
    document.validate().map_err(|message| {
        VehicleAudioConfigurationLoadError::new(message).at_document(document_path)
    })?;
    Ok(document)
}

impl VehicleAudioDocument {
    pub fn validate(&self) -> Result<(), String> {
        validation::validate(self)
    }

    pub fn from_value(
        value: serde_json::Value,
    ) -> Result<Self, VehicleAudioConfigurationLoadError> {
        let document: Self = serde_json::from_value(value).map_err(|error| {
            VehicleAudioConfigurationLoadError::new(format!(
                "vehicle audio document does not match its contract: {error}"
            ))
        })?;
        document
            .validate()
            .map_err(VehicleAudioConfigurationLoadError::new)?;
        Ok(document)
    }
}
