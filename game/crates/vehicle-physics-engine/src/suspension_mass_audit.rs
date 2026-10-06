use crate::types::WheelIndex;
use crate::vehicle_config::VehicleConfig;
use crate::wheel_mechanics::{static_wheel_load, wheel_mass, WheelMechanicalTuning};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SuspensionCornerMassAudit {
    pub wheel: String,
    pub configured_rotating_assembly_mass_kilograms: f64,
    pub virtual_contact_filter_mass_kilograms: f64,
    pub static_normal_force_newtons: f64,
}

#[derive(Debug, Serialize)]
pub struct SuspensionMassAudit {
    pub dry_vehicle_mass_kilograms: f64,
    pub configured_body_and_driver_mass_kilograms: Option<f64>,
    pub current_fuel_mass_kilograms: f64,
    pub total_vehicle_mass_kilograms: f64,
    pub total_rotating_assembly_mass_kilograms: f64,
    pub total_virtual_contact_filter_mass_kilograms: f64,
    pub suspended_mass_kilograms: Option<f64>,
    pub corners: Vec<SuspensionCornerMassAudit>,
    pub unresolved_physical_properties: Vec<String>,
}

impl SuspensionMassAudit {
    pub fn from_vehicle_configuration(configuration: &VehicleConfig) -> Result<Self, String> {
        let dry_vehicle_mass_kilograms = configuration.vehicle_mass;
        let current_fuel_mass_kilograms = configuration.fuel.effective_current_kg();
        if !dry_vehicle_mass_kilograms.is_finite() || dry_vehicle_mass_kilograms <= 0.0 {
            return Err("Dry vehicle mass must be finite and positive".into());
        }
        if !current_fuel_mass_kilograms.is_finite() || current_fuel_mass_kilograms < 0.0 {
            return Err("Current fuel mass must be finite and nonnegative".into());
        }
        let mut corners = Vec::with_capacity(4);
        for wheel in WheelIndex::ALL {
            let configured_rotating_assembly_mass_kilograms = wheel_mass(configuration, wheel);
            let static_normal_force_newtons = static_wheel_load(configuration, wheel);
            if !configured_rotating_assembly_mass_kilograms.is_finite()
                || configured_rotating_assembly_mass_kilograms <= 0.0
                || !static_normal_force_newtons.is_finite()
                || static_normal_force_newtons <= 0.0
            {
                return Err(format!("Invalid mass or static load for {wheel:?}"));
            }
            corners.push(SuspensionCornerMassAudit {
                wheel: format!("{wheel:?}"),
                configured_rotating_assembly_mass_kilograms,
                virtual_contact_filter_mass_kilograms: WheelMechanicalTuning::for_wheel(
                    configuration,
                    wheel,
                )
                .unsprung_mass_kg,
                static_normal_force_newtons,
            });
        }
        Ok(Self {
            dry_vehicle_mass_kilograms: configuration.complete_dry_vehicle_mass(),
            configured_body_and_driver_mass_kilograms: configuration
                .vehicle_mass_excludes_wheel_assemblies
                .then_some(configuration.vehicle_mass),
            current_fuel_mass_kilograms,
            total_vehicle_mass_kilograms: configuration.total_vehicle_mass(),
            total_rotating_assembly_mass_kilograms: corners
                .iter()
                .map(|corner| corner.configured_rotating_assembly_mass_kilograms)
                .sum(),
            total_virtual_contact_filter_mass_kilograms: corners
                .iter()
                .map(|corner| corner.virtual_contact_filter_mass_kilograms)
                .sum(),
            suspended_mass_kilograms: None,
            corners,
            unresolved_physical_properties: vec![
                "Hub, upright and brake masses and inertia tensors".into(),
                "Suspension link masses, centers of mass and inertia tensors".into(),
                "Rotating assembly centers of mass and measured inertia tensors".into(),
            ],
        })
    }
}
