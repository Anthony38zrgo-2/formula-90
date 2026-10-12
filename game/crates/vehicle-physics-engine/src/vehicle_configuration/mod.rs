pub(crate) mod aerodynamics;
pub(crate) mod brakes;
pub(crate) mod canonical_profiles;
pub(crate) mod chassis;
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

pub use driving_assistance::configuration::AidsConfig;
pub use fuel::configuration::FuelConfig;
pub use powertrain::configuration::AutomaticShift;
pub use tires::configuration::TireForceProfile;
pub use vehicle_configuration::VehicleConfig;

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

