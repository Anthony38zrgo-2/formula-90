pub(crate) mod aerodynamics;
pub(crate) mod brakes;
pub(crate) mod canonical_profiles;
pub(crate) mod chassis;
pub(crate) mod consistency_validation;
pub(crate) mod coupled_world;
pub(crate) mod driving_assistance;
pub(crate) mod fuel;
pub(crate) mod legacy_profile;
pub(crate) mod powertrain;
pub(crate) mod steering;
pub(crate) mod suspension;
pub(crate) mod tires;
pub(crate) mod underfloor_contact;
pub(crate) mod vehicle_configuration;

pub mod loader;
pub mod manifest;
pub mod resolved_profile;

pub use driving_assistance::configuration::AidsConfig;
pub use fuel::configuration::FuelConfig;
pub use powertrain::configuration::AutomaticShift;
pub use tires::configuration::TireForceProfile;
pub use vehicle_configuration::{VehicleConfig, VehicleConfiguration};

pub use aerodynamics::configuration::AerodynamicsConfiguration;
pub use brakes::configuration::BrakesConfiguration;
pub use chassis::configuration::ChassisConfiguration;
pub use powertrain::configuration::PowertrainConfiguration;
pub use steering::configuration::SteeringConfiguration;
pub use suspension::configuration::SuspensionConfiguration;
pub use tires::configuration::TiresConfiguration;

pub use loader::{
    load_physical_configuration, load_vehicle_configuration, load_vehicle_configuration_from_authorized_root, resolve_domain_document_path,
    LoadedPhysicalConfiguration, VehicleConfigurationLoadError, VehicleProfileLoadRequest,
};
pub use manifest::{
    is_known_domain_name, is_physical_domain_name, VehicleProfileManifest,
    AERODYNAMICS_DOMAIN_NAME, AUDIO_DOMAIN_NAME, BRAKES_DOMAIN_NAME, CHASSIS_DOMAIN_NAME,
    COUPLED_WORLD_DOMAIN_NAME, DRIVING_ASSISTANCE_DOMAIN_NAME, EXTERNALLY_OWNED_DOMAIN_NAMES,
    FUEL_DOMAIN_NAME, OPTIONAL_PHYSICAL_DOMAIN_NAMES, POWERTRAIN_DOMAIN_NAME,
    REQUIRED_PHYSICAL_DOMAIN_NAMES, STEERING_DOMAIN_NAME, SUPPORTED_UNITS,
    SUSPENSION_DOMAIN_NAME, SUSPENSION_PRESENTATION_DOMAIN_NAME, TIRES_DOMAIN_NAME,
    UNDERFLOOR_CONTACT_DOMAIN_NAME, VEHICLE_PROFILE_MANIFEST_SCHEMA_VERSION,
};
pub use resolved_profile::{
    sha256_hex, ResolvedVehicleConfiguration, VehicleProfileDependencyDigest,
    VehicleProfileProvenance, RESOLVED_VEHICLE_CONFIGURATION_DOCUMENT_KIND,
    RESOLVED_VEHICLE_CONFIGURATION_SCHEMA_VERSION,
};

pub(crate) use crate::aero::AeroModelConfig;
pub(crate) use crate::brake_thermals::{
    BrakeAxleThermalConfig, BrakeCoolingProfile, BrakeDuctAxleConfig, BrakeRotorMaterial,
    BrakeRotorVentilation, BrakeThermalConfig, BrakeThermalModelKind,
};
pub(crate) use crate::powertrain_thermals::PowertrainThermalConfig;
pub(crate) use crate::suspension_geo_config::{
    default_suspension_model_version, geometric_from_json_value, geometric_to_json_value,
    GeometricSuspensionConfig, SuspensionModelKind,
};
pub(crate) use crate::tire_thermals::{
    PressureMechanicsSensitivity, TirePressureConfig, TireThermalAxleConfig, TireThermalConfig,
};
pub(crate) use crate::tire_wear::{TireWearAxleConfig, TireWearConfig};
pub(crate) use crate::types::{SurfaceType, Vec3, WheelIndex};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::Path;

