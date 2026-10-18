use crate::suspension::SuspensionBackend;
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
    resolve_vehicle_mass_properties_for_suspension_backend(configuration, SuspensionBackend::Standard)
}

pub fn resolve_vehicle_mass_properties_for_suspension_backend(
    configuration: &VehicleConfig,
    suspension_backend: SuspensionBackend,
) -> VehicleMassProperties {
    let total_mass = configuration
        .chassis
        .total_vehicle_mass(&configuration.tires, &configuration.fuel);
    let front_weight_distribution = configuration
        .chassis
        .effective_front_weight_distribution(&configuration.tires, &configuration.fuel);

    if configuration.chassis.uses_authored_wheel_frame() {
        if let Some((mass, center, inertia)) =
            inventory_mass_properties_for_backend(configuration, suspension_backend)
        {
            if (mass - total_mass).abs() <= 1e-6 * total_mass.max(1.0) {
                let resolved_center = match suspension_backend {
                    SuspensionBackend::Simplified => center,
                    SuspensionBackend::Standard => Vec3::new(
                        0.0,
                        configuration.chassis.center_of_gravity_height_offset,
                        center.z,
                    ),
                };
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedMassComponentKind {
    Body,
    CornerAttachment,
    Wheel,
    Fuel,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedMassComponent {
    pub kind: ResolvedMassComponentKind,
    pub mass_kilograms: f64,
    pub center_of_mass_local_metres: Vec3,
    pub inertia_about_center_of_mass: InertiaTensor,
}

pub fn resolved_mass_components(
    configuration: &VehicleConfig,
) -> Option<Vec<ResolvedMassComponent>> {
    let inventory = configuration.chassis.physical_mass_inventory.as_ref()?;
    inventory.validate(configuration).ok()?;
    let base_center = configuration.chassis.base_center_of_mass_local();
    let base_budget = inventory.base_budget_kilograms;

    let resolved_component = |component: &crate::vehicle_mass_inventory::VehicleMassComponent,
                              center: Vec3,
                              kind: ResolvedMassComponentKind| {
        ResolvedMassComponent {
            kind,
            mass_kilograms: component.properties.mass_kilograms,
            center_of_mass_local_metres: center,
            inertia_about_center_of_mass: component.properties.inertia_about_center_of_mass,
        }
    };
    let mut body_components: Vec<ResolvedMassComponent> = Vec::new();
    let mut corner_base_components: Vec<ResolvedMassComponent> = Vec::new();
    let mut wheel_components: Vec<ResolvedMassComponent> = Vec::new();
    for component in &inventory.components {
        let properties = &component.properties;
        let center_of_mass_in_attachment = properties.center_of_mass_in_attachment_metres;
        match component.budget {
            crate::vehicle_mass_inventory::VehicleMassBudget::RimAndTire => {
                let wheel = component.rotating_wheel?;
                let center =
                    configuration.chassis.wheel_center_local(wheel) + center_of_mass_in_attachment;
                wheel_components.push(resolved_component(
                    component,
                    center,
                    ResolvedMassComponentKind::Wheel,
                ));
            }
            crate::vehicle_mass_inventory::VehicleMassBudget::VehicleAndDriver => {
                match &properties.attachment {
                    MassComponentAttachment::Body | MassComponentAttachment::SteeringRack => {
                        body_components.push(resolved_component(
                            component,
                            center_of_mass_in_attachment,
                            ResolvedMassComponentKind::Body,
                        ));
                    }
                    other => {
                        let wheel = other.wheel()?;
                        let center = configuration.chassis.wheel_center_local(wheel)
                            + center_of_mass_in_attachment;
                        corner_base_components.push(resolved_component(
                            component,
                            center,
                            ResolvedMassComponentKind::CornerAttachment,
                        ));
                    }
                }
            }
        }
    }

    let body_mass = body_components
        .iter()
        .map(|component| component.mass_kilograms)
        .sum::<f64>();
    if body_mass <= 0.0 {
        return None;
    }
    let (_, body_center, _) =
        composite_mass_properties(&mass_component_tuples(&body_components)).ok()?;
    let corner_base_mass = corner_base_components
        .iter()
        .map(|component| component.mass_kilograms)
        .sum::<f64>();
    let corner_base_center = if corner_base_mass > 0.0 {
        let (_, center, _) =
            composite_mass_properties(&mass_component_tuples(&corner_base_components)).ok()?;
        center
    } else {
        Vec3::ZERO
    };
    let translation = (base_center * base_budget - corner_base_center * corner_base_mass)
        / body_mass
        - body_center;

    let mut components: Vec<ResolvedMassComponent> = body_components
        .into_iter()
        .map(|component| ResolvedMassComponent {
            center_of_mass_local_metres: component.center_of_mass_local_metres + translation,
            ..component
        })
        .collect();
    components.extend(corner_base_components);
    components.extend(wheel_components);
    let fuel_mass = configuration.fuel.effective_current_kg();
    if fuel_mass > 0.0 {
        components.push(ResolvedMassComponent {
            kind: ResolvedMassComponentKind::Fuel,
            mass_kilograms: fuel_mass,
            center_of_mass_local_metres: configuration.fuel.tank_position_local_m,
            inertia_about_center_of_mass: InertiaTensor {
                entries_kilogram_square_metres: [[0.0; 3]; 3],
            },
        });
    }
    Some(components)
}

pub fn resolved_mass_components_for_suspension_backend(
    configuration: &VehicleConfig,
    suspension_backend: SuspensionBackend,
) -> Option<Vec<ResolvedMassComponent>> {
    let mut components = resolved_mass_components(configuration)?;
    if suspension_backend == SuspensionBackend::Simplified {
        apply_reference_load_height_calibration(configuration, &mut components)?;
    }
    Some(components)
}

fn apply_reference_load_height_calibration(
    configuration: &VehicleConfig,
    components: &mut [ResolvedMassComponent],
) -> Option<()> {
    let target_height = configuration.chassis.center_of_gravity_height_offset;
    let reference_load = configuration.fuel.initial_kg;
    let tank_height = configuration.fuel.tank_position_local_m.y;
    if !target_height.is_finite()
        || !reference_load.is_finite()
        || reference_load < 0.0
        || !tank_height.is_finite()
    {
        return None;
    }
    let mut body_mass_kilograms = 0.0;
    let mut total_mass_kilograms = 0.0;
    let mut total_height_moment = 0.0;
    let mut fuel_mass_kilograms = 0.0;
    for component in components.iter() {
        total_mass_kilograms += component.mass_kilograms;
        total_height_moment += component.mass_kilograms * component.center_of_mass_local_metres.y;
        match component.kind {
            ResolvedMassComponentKind::Body => body_mass_kilograms += component.mass_kilograms,
            ResolvedMassComponentKind::Fuel => fuel_mass_kilograms += component.mass_kilograms,
            ResolvedMassComponentKind::CornerAttachment | ResolvedMassComponentKind::Wheel => {}
        }
    }
    if body_mass_kilograms <= 0.0 || total_mass_kilograms <= 0.0 {
        return None;
    }
    let non_fuel_mass_kilograms = total_mass_kilograms - fuel_mass_kilograms;
    let non_fuel_height_moment = total_height_moment - fuel_mass_kilograms * tank_height;
    let reference_total_mass_kilograms = non_fuel_mass_kilograms + reference_load;
    if reference_total_mass_kilograms <= 0.0 {
        return None;
    }
    let reference_composite_height =
        (non_fuel_height_moment + reference_load * tank_height) / reference_total_mass_kilograms;
    let needed_composite_shift = target_height - reference_composite_height;
    let body_height_shift =
        needed_composite_shift * reference_total_mass_kilograms / body_mass_kilograms;
    if !body_height_shift.is_finite() {
        return None;
    }
    for component in components.iter_mut() {
        if component.kind == ResolvedMassComponentKind::Body {
            component.center_of_mass_local_metres.y += body_height_shift;
        }
    }
    Some(())
}

fn mass_component_tuples(
    components: &[ResolvedMassComponent],
) -> Vec<(f64, Vec3, InertiaTensor)> {
    components
        .iter()
        .map(|component| {
            (
                component.mass_kilograms,
                component.center_of_mass_local_metres,
                component.inertia_about_center_of_mass,
            )
        })
        .collect()
}

fn inventory_mass_properties_for_backend(
    configuration: &VehicleConfig,
    suspension_backend: SuspensionBackend,
) -> Option<(f64, Vec3, InertiaTensor)> {
    let components =
        resolved_mass_components_for_suspension_backend(configuration, suspension_backend)?;
    composite_mass_properties(&mass_component_tuples(&components)).ok()
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
