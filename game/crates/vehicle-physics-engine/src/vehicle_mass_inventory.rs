use crate::suspension_mass_properties::{MassComponentAttachment, SuspensionMassComponent, SuspensionMassInventory};
use crate::types::WheelIndex;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleMassBudget {
    VehicleAndDriver,
    RimAndTire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentMassUncertainty {
    pub lower_mass_kilograms: f64,
    pub upper_mass_kilograms: f64,
    pub center_uncertainty_metres: f64,
    pub inertia_relative_uncertainty: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleMassComponent {
    pub properties: SuspensionMassComponent,
    pub budget: VehicleMassBudget,
    pub rotating_wheel: Option<WheelIndex>,
    pub uncertainty: ComponentMassUncertainty,
    pub component_perimeter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleMassInventory {
    pub schema_version: u32,
    pub base_budget_kilograms: f64,
    pub rim_and_tire_mass_kilograms: [f64; 4],
    pub components: Vec<VehicleMassComponent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VehicleMassInventoryAudit {
    pub base_mass_kilograms: f64,
    pub rim_and_tire_mass_kilograms: [f64; 4],
    pub complete_unfueled_mass_kilograms: f64,
    pub body_fixed_mass_kilograms: f64,
    pub moving_member_mass_kilograms: f64,
    pub upright_assembly_mass_kilograms: [f64; 4],
    pub spin_inertia_kilogram_square_metres: [f64; 4],
    pub independent_lower_mass_kilograms: f64,
    pub independent_upper_mass_kilograms: f64,
}

impl VehicleMassInventory {
    pub fn budget_constrained_mass_variant(&self, configuration: &VehicleConfig,
        component_directions: &[f64]) -> Result<Self, String> {
        self.validate(configuration)?;
        if component_directions.len() != self.components.len()
            || component_directions.iter().any(|direction| !direction.is_finite() || !(-1.0..=1.0).contains(direction))
        {
            return Err("Mass sensitivity requires one finite direction between minus one and one per component".into());
        }
        let provisional_masses = self.components.iter().zip(component_directions).map(|(component, direction)| {
            let mass = component.properties.mass_kilograms;
            mass + direction * if *direction >= 0.0 {
                component.uncertainty.upper_mass_kilograms - mass
            } else { mass - component.uncertainty.lower_mass_kilograms }
        }).collect::<Vec<_>>();
        let mut variant = self.clone();
        for membership in 0..5 {
            let indices = self.components.iter().enumerate().filter_map(|(index, component)| {
                let belongs = if membership == 0 { component.budget == VehicleMassBudget::VehicleAndDriver }
                    else { component.budget == VehicleMassBudget::RimAndTire
                        && component.rotating_wheel == Some(WheelIndex::ALL[membership - 1]) };
                belongs.then_some(index)
            }).collect::<Vec<_>>();
            let target = if membership == 0 { self.base_budget_kilograms }
                else { self.rim_and_tire_mass_kilograms[membership - 1] };
            let projected_mass = |index: usize, multiplier: f64| {
                let uncertainty = &self.components[index].uncertainty;
                (provisional_masses[index] + multiplier
                    * (uncertainty.upper_mass_kilograms - uncertainty.lower_mass_kilograms))
                    .clamp(uncertainty.lower_mass_kilograms, uncertainty.upper_mass_kilograms)
            };
            let mut lower_multiplier = -2.0;
            let mut upper_multiplier = 2.0;
            for _ in 0..80 {
                let multiplier = (lower_multiplier + upper_multiplier) * 0.5;
                let mass = indices.iter().map(|index| projected_mass(*index, multiplier)).sum::<f64>();
                if mass < target { lower_multiplier = multiplier; } else { upper_multiplier = multiplier; }
            }
            let multiplier = (lower_multiplier + upper_multiplier) * 0.5;
            for index in indices {
                let mass = projected_mass(index, multiplier);
                let mass_ratio = mass / self.components[index].properties.mass_kilograms;
                let properties = &mut variant.components[index].properties;
                properties.mass_kilograms = mass;
                for entry in properties.inertia_about_center_of_mass.entries_kilogram_square_metres.iter_mut().flatten() {
                    *entry *= mass_ratio;
                }
            }
        }
        variant.validate(configuration)?;
        Ok(variant)
    }

    pub fn validate(&self, configuration: &VehicleConfig) -> Result<(), String> {
        if self.schema_version != 2 {
            return Err("Vehicle mass inventory schema_version must be 2".into());
        }
        if !configuration.vehicle_mass_excludes_wheel_assemblies {
            return Err("Physical inventory requires explicit wheel-excluded base semantics".into());
        }
        if !self.base_budget_kilograms.is_finite()
            || (self.base_budget_kilograms - configuration.vehicle_mass).abs() > 0.000001
        {
            return Err("Inventory base budget disagrees with vehicle configuration".into());
        }
        self.reference_inventory().validate()?;
        let mut names = HashSet::new();
        for component in &self.components {
            let properties = &component.properties;
            if !names.insert(&properties.name) || component.component_perimeter.trim().is_empty() {
                return Err(format!("Missing perimeter or duplicate identity: {}", properties.name));
            }
            let uncertainty = &component.uncertainty;
            if [uncertainty.lower_mass_kilograms, uncertainty.upper_mass_kilograms,
                uncertainty.center_uncertainty_metres, uncertainty.inertia_relative_uncertainty]
                .iter().any(|value| !value.is_finite() || *value < 0.0)
                || uncertainty.lower_mass_kilograms <= 0.0
                || uncertainty.lower_mass_kilograms > properties.mass_kilograms
                || uncertainty.upper_mass_kilograms < properties.mass_kilograms
                || uncertainty.rationale.trim().is_empty()
            {
                return Err(format!("Invalid uncertainty bounds: {}", properties.name));
            }
            if let Some(wheel) = component.rotating_wheel {
                if !matches!(properties.attachment, MassComponentAttachment::WheelRotor { wheel: attached } if attached == wheel) {
                    return Err(format!("Wheel rotation requires the matching aligned rotor attachment: {}", properties.name));
                }
            }
            if component.budget == VehicleMassBudget::RimAndTire && component.rotating_wheel.is_none() {
                return Err(format!("Rim and tire must declare wheel rotation: {}", properties.name));
            }
        }
        let audit = self.audit();
        if (audit.base_mass_kilograms - self.base_budget_kilograms).abs() > 0.000001 {
            return Err("Vehicle-and-driver membership does not close the base budget".into());
        }
        for wheel in WheelIndex::ALL {
            let expected = if wheel.is_front() { configuration.front_wheel_mass } else { configuration.rear_wheel_mass };
            let declared = self.rim_and_tire_mass_kilograms[wheel as usize];
            if !declared.is_finite() || declared <= 0.0
                || (declared - expected).abs() > 0.000001
                || (audit.rim_and_tire_mass_kilograms[wheel as usize] - declared).abs() > 0.000001
                || audit.spin_inertia_kilogram_square_metres[wheel as usize] <= 0.0
            {
                return Err(format!("Wheel mass or spin inertia does not close: {wheel:?}"));
            }
        }
        Ok(())
    }

    pub fn reference_inventory(&self) -> SuspensionMassInventory {
        SuspensionMassInventory {
            schema_version: 1,
            declared_complete_dry_mass_kilograms: self.base_budget_kilograms + self.rim_and_tire_mass_kilograms.iter().sum::<f64>(),
            components: self.components.iter().map(|component| component.properties.clone()).collect(),
        }
    }

    pub fn audit(&self) -> VehicleMassInventoryAudit {
        let mut audit = VehicleMassInventoryAudit {
            base_mass_kilograms: 0.0, rim_and_tire_mass_kilograms: [0.0; 4],
            complete_unfueled_mass_kilograms: 0.0, body_fixed_mass_kilograms: 0.0,
            moving_member_mass_kilograms: 0.0, upright_assembly_mass_kilograms: [0.0; 4],
            spin_inertia_kilogram_square_metres: [0.0; 4],
            independent_lower_mass_kilograms: 0.0, independent_upper_mass_kilograms: 0.0,
        };
        for component in &self.components {
            let properties = &component.properties;
            let mass = properties.mass_kilograms;
            audit.complete_unfueled_mass_kilograms += mass;
            audit.independent_lower_mass_kilograms += component.uncertainty.lower_mass_kilograms;
            audit.independent_upper_mass_kilograms += component.uncertainty.upper_mass_kilograms;
            match component.budget {
                VehicleMassBudget::VehicleAndDriver => audit.base_mass_kilograms += mass,
                VehicleMassBudget::RimAndTire => {
                    if let Some(wheel) = component.rotating_wheel { audit.rim_and_tire_mass_kilograms[wheel as usize] += mass; }
                }
            }
            match properties.attachment {
                MassComponentAttachment::Body => audit.body_fixed_mass_kilograms += mass,
                MassComponentAttachment::Upright { wheel } | MassComponentAttachment::WheelRotor { wheel } => audit.upright_assembly_mass_kilograms[wheel as usize] += mass,
                _ => audit.moving_member_mass_kilograms += mass,
            }
            if let Some(wheel) = component.rotating_wheel {
                audit.spin_inertia_kilogram_square_metres[wheel as usize] += properties.inertia_about_center_of_mass.entries_kilogram_square_metres[0][0];
            }
        }
        audit
    }
}
