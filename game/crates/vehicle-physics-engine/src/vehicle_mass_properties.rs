use crate::suspension_mass_properties::{
    composite_mass_properties, InertiaTensor, MassComponentAttachment,
};
use crate::types::{Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassPropertySource {
    PhysicalInventory,
    UniformBoxFallback,
}

#[derive(Debug, Clone, Copy)]
pub struct VehicleMassProperties {
    pub total_mass_kilograms: f64,
    pub center_of_mass_local_metres: Vec3,
    pub inertia_about_center_of_mass: InertiaTensor,
    pub front_weight_distribution: f64,
    pub wheel_spin_inertia_kilogram_square_metres: [f64; 4],
    pub unsprung_mass_kilograms: [f64; 4],
    pub source: MassPropertySource,
}

impl VehicleMassProperties {
    pub fn principal_moments(&self) -> [f64; 3] {
        let entries = self.inertia_about_center_of_mass.entries_kilogram_square_metres;
        [entries[0][0], entries[1][1], entries[2][2]]
    }

    /// Products of inertia are reported because Godot's rigid body only accepts
    /// a diagonal tensor in body axes; they must stay visible instead of being
    /// discarded silently.
    pub fn products_of_inertia(&self) -> [f64; 3] {
        let entries = self.inertia_about_center_of_mass.entries_kilogram_square_metres;
        [entries[0][1], entries[0][2], entries[1][2]]
    }
}

pub fn resolve_vehicle_mass_properties(configuration: &VehicleConfig) -> VehicleMassProperties {
    let total_mass = configuration
        .chassis
        .total_vehicle_mass(&configuration.tires, &configuration.fuel);
    let front_weight_distribution = configuration
        .chassis
        .effective_front_weight_distribution(&configuration.tires, &configuration.fuel);

    if configuration.chassis.uses_authored_wheel_frame() {
        if let Some((mass, center, inertia)) = inventory_mass_properties(configuration) {
            if (mass - total_mass).abs() <= 1e-6 * total_mass.max(1.0) {
                let resolved_center = Vec3::new(
                    0.0,
                    configuration.chassis.center_of_gravity_height_offset,
                    center.z,
                );
                let rear_axle_z = configuration.chassis.axle_center_z(WheelIndex::RearLeft);
                let resolved_front_share = ((rear_axle_z - resolved_center.z)
                    / configuration.chassis.wheelbase)
                    .clamp(0.0, 1.0);
                return VehicleMassProperties {
                    total_mass_kilograms: mass,
                    center_of_mass_local_metres: resolved_center,
                    inertia_about_center_of_mass: inertia,
                    front_weight_distribution: resolved_front_share,
                    wheel_spin_inertia_kilogram_square_metres:
                        inventory_wheel_spin_inertia(configuration)
                            .unwrap_or_else(|| fallback_wheel_spin_inertia(configuration)),
                    unsprung_mass_kilograms: inventory_unsprung_mass(configuration)
                        .unwrap_or_else(|| fallback_unsprung_mass(configuration)),
                    source: MassPropertySource::PhysicalInventory,
                };
            }
        }
    }

    VehicleMassProperties {
        total_mass_kilograms: total_mass,
        center_of_mass_local_metres: configuration
            .chassis
            .center_of_mass_local(&configuration.tires, &configuration.fuel),
        inertia_about_center_of_mass: uniform_box_inertia(configuration),
        front_weight_distribution,
        wheel_spin_inertia_kilogram_square_metres: fallback_wheel_spin_inertia(configuration),
        unsprung_mass_kilograms: fallback_unsprung_mass(configuration),
        source: MassPropertySource::UniformBoxFallback,
    }
}

pub fn wheel_spin_inertia(configuration: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if configuration.chassis.uses_authored_wheel_frame() {
        if let Some(values) = inventory_wheel_spin_inertia(configuration) {
            return values[wheel as usize];
        }
    }
    fallback_wheel_spin_inertia(configuration)[wheel as usize]
}

pub fn unsprung_mass(configuration: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if configuration.chassis.uses_authored_wheel_frame() {
        if let Some(values) = inventory_unsprung_mass(configuration) {
            return values[wheel as usize];
        }
    }
    fallback_unsprung_mass(configuration)[wheel as usize]
}

fn inventory_mass_properties(configuration: &VehicleConfig) -> Option<(f64, Vec3, InertiaTensor)> {
    let inventory = configuration.chassis.physical_mass_inventory.as_ref()?;
    inventory.validate(configuration).ok()?;
    let base_center = configuration.chassis.base_center_of_mass_local();
    let base_budget = inventory.base_budget_kilograms;

    let mut body_components: Vec<(f64, Vec3, InertiaTensor)> = Vec::new();
    let mut corner_base_components: Vec<(f64, Vec3, InertiaTensor)> = Vec::new();
    let mut wheel_components: Vec<(f64, Vec3, InertiaTensor)> = Vec::new();
    for component in &inventory.components {
        let properties = &component.properties;
        match component.budget {
            crate::vehicle_mass_inventory::VehicleMassBudget::RimAndTire => {
                let wheel = component.rotating_wheel?;
                let center = configuration.chassis.wheel_center_local(wheel)
                    + properties.center_of_mass_in_attachment_metres;
                wheel_components.push((
                    properties.mass_kilograms,
                    center,
                    properties.inertia_about_center_of_mass,
                ));
            }
            crate::vehicle_mass_inventory::VehicleMassBudget::VehicleAndDriver => {
                match &component.properties.attachment {
                    MassComponentAttachment::Body | MassComponentAttachment::SteeringRack => {
                        body_components.push((
                            properties.mass_kilograms,
                            properties.center_of_mass_in_attachment_metres,
                            properties.inertia_about_center_of_mass,
                        ));
                    }
                    other => {
                        let wheel = other.wheel()?;
                        let center = configuration.chassis.wheel_center_local(wheel)
                            + properties.center_of_mass_in_attachment_metres;
                        corner_base_components.push((
                            properties.mass_kilograms,
                            center,
                            properties.inertia_about_center_of_mass,
                        ));
                    }
                }
            }
        }
    }

    let body_mass = body_components.iter().map(|(mass, _, _)| *mass).sum::<f64>();
    if body_mass <= 0.0 {
        return None;
    }
    let (_, body_center, _) = composite_mass_properties(&body_components).ok()?;
    let corner_base_mass = corner_base_components
        .iter()
        .map(|(mass, _, _)| *mass)
        .sum::<f64>();
    let corner_base_center = if corner_base_mass > 0.0 {
        let (_, center, _) = composite_mass_properties(&corner_base_components).ok()?;
        center
    } else {
        Vec3::ZERO
    };
    let translation = (base_center * base_budget - corner_base_center * corner_base_mass)
        / body_mass
        - body_center;

    let mut components: Vec<(f64, Vec3, InertiaTensor)> = body_components
        .into_iter()
        .map(|(mass, center, inertia)| (mass, center + translation, inertia))
        .collect();
    components.extend(corner_base_components);
    components.extend(wheel_components);
    let fuel_mass = configuration.fuel.effective_current_kg();
    if fuel_mass > 0.0 {
        components.push((
            fuel_mass,
            configuration.fuel.tank_position_local_m,
            InertiaTensor {
                entries_kilogram_square_metres: [[0.0; 3]; 3],
            },
        ));
    }
    let (mass, center, inertia) = composite_mass_properties(&components).ok()?;
    Some((mass, center, inertia))
}

fn inventory_wheel_spin_inertia(configuration: &VehicleConfig) -> Option<[f64; 4]> {
    let inventory = configuration.chassis.physical_mass_inventory.as_ref()?;
    inventory.validate(configuration).ok()?;
    let audit = inventory.audit();
    let mut result = [0.0; 4];
    for wheel in WheelIndex::ALL {
        let value = audit.spin_inertia_kilogram_square_metres[wheel as usize];
        if !value.is_finite() || value <= 0.0 {
            return None;
        }
        result[wheel as usize] = value;
    }
    Some(result)
}

fn inventory_unsprung_mass(configuration: &VehicleConfig) -> Option<[f64; 4]> {
    let inventory = configuration.chassis.physical_mass_inventory.as_ref()?;
    inventory.validate(configuration).ok()?;
    let audit = inventory.audit();
    let mut result = [0.0; 4];
    for wheel in WheelIndex::ALL {
        let value = audit.upright_assembly_mass_kilograms[wheel as usize];
        if !value.is_finite() || value <= 0.0 {
            return None;
        }
        result[wheel as usize] = value;
    }
    Some(result)
}

fn fallback_wheel_spin_inertia(configuration: &VehicleConfig) -> [f64; 4] {
    let mut result = [0.0; 4];
    for wheel in WheelIndex::ALL {
        let (mass, radius) = if wheel.is_front() {
            (
                configuration.tires.front_wheel_mass,
                configuration.tires.front_tire_radius,
            )
        } else {
            (
                configuration.tires.rear_wheel_mass,
                configuration.tires.rear_tire_radius,
            )
        };
        result[wheel as usize] = 0.72 * mass.max(1.0) * radius.max(0.05).powi(2);
    }
    result
}

fn fallback_unsprung_mass(configuration: &VehicleConfig) -> [f64; 4] {
    let mut result = [0.0; 4];
    for wheel in WheelIndex::ALL {
        let mass = if wheel.is_front() {
            configuration.tires.front_wheel_mass
        } else {
            configuration.tires.rear_wheel_mass
        };
        result[wheel as usize] = mass.max(1.0) * 1.35;
    }
    result
}

fn uniform_box_inertia(configuration: &VehicleConfig) -> InertiaTensor {
    let width = (configuration.chassis.front_track + configuration.chassis.rear_track) * 0.5;
    let height = 0.75;
    let length = configuration.chassis.wheelbase;
    let mass = configuration
        .chassis
        .total_vehicle_mass(&configuration.tires, &configuration.fuel);
    let multipliers = configuration.chassis.inertia_multipliers;
    InertiaTensor {
        entries_kilogram_square_metres: [
            [
                (mass / 12.0) * (height * height + length * length) * multipliers.x,
                0.0,
                0.0,
            ],
            [
                0.0,
                (mass / 12.0) * (width * width + length * length) * multipliers.y,
                0.0,
            ],
            [
                0.0,
                0.0,
                (mass / 12.0) * (width * width + height * height) * multipliers.z,
            ],
        ],
    }
}
