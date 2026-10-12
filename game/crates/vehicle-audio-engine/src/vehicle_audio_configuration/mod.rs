pub mod document;
pub mod loader;
pub mod validation;

pub use document::{
    CppDspDocument, GrandPrixSamplerDocument, VehicleAudioDocument,
    GRAND_PRIX_SAMPLER_DEFAULT_MANIFEST, V10_GF509_DEFAULT_MANIFEST,
};
pub use loader::{load_vehicle_audio_document, VehicleAudioConfigurationLoadError};
