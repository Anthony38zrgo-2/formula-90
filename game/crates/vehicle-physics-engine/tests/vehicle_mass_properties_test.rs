use vehicle_physics_engine::suspension_mass_properties::{composite_mass_properties, InertiaTensor};
use vehicle_physics_engine::vehicle_mass_properties::{
    resolve_vehicle_mass_properties, resolve_vehicle_mass_properties_for_suspension_backend,
    resolved_mass_components, resolved_mass_components_for_suspension_backend, unsprung_mass,
    wheel_spin_inertia, MassPropertySource, ResolvedMassComponentKind,
};
use vehicle_physics_engine::wheel_mechanics::static_wheel_load;
use vehicle_physics_engine::{
    center_of_mass_world, center_of_mass_world_for_suspension_backend, Mat3, SuspensionBackend,
    Transform3D, Vec3, VehicleConfig, WheelIndex,
};

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .expect("Williams profile")
}

fn raw_components_with_centers(
    configuration: &VehicleConfig,
) -> Vec<(f64, Vec3, InertiaTensor)> {
    resolved_mass_components(configuration)
        .expect("Williams profile carries a physical mass inventory")
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

fn calibrated_components_with_centers(
    configuration: &VehicleConfig,
) -> Vec<(f64, Vec3, InertiaTensor)> {
    resolved_mass_components_for_suspension_backend(configuration, SuspensionBackend::Simplified)
        .expect("Williams profile carries a physical mass inventory")
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

fn composite_center_of(components: &[(f64, Vec3, InertiaTensor)]) -> (f64, Vec3, InertiaTensor) {
    composite_mass_properties(components).expect("composite mass properties")
}

fn parallel_axis_about(
    inertia_about_center: InertiaTensor,
    mass_kilograms: f64,
    displacement: Vec3,
) -> InertiaTensor {
    let mut entries = inertia_about_center.entries_kilogram_square_metres;
    let coordinates = [displacement.x, displacement.y, displacement.z];
    for row in 0..3 {
        for column in 0..3 {
            entries[row][column] += mass_kilograms
                * (if row == column {
                    displacement.length_squared()
                } else {
                    0.0
                } - coordinates[row] * coordinates[column]);
        }
    }
    InertiaTensor {
        entries_kilogram_square_metres: entries,
    }
}

fn composite_about_point(
    components: &[(f64, Vec3, InertiaTensor)],
    point: Vec3,
) -> InertiaTensor {
    let mut entries = [[0.0; 3]; 3];
    for (mass_kilograms, center_of_mass, inertia) in components {
        let offset = *center_of_mass - point;
        let coordinates = [offset.x, offset.y, offset.z];
        for row in 0..3 {
            for column in 0..3 {
                entries[row][column] += inertia.entries_kilogram_square_metres[row][column]
                    + mass_kilograms
                        * (if row == column {
                            offset.length_squared()
                        } else {
                            0.0
                        } - coordinates[row] * coordinates[column]);
            }
        }
    }
    InertiaTensor {
        entries_kilogram_square_metres: entries,
    }
}

fn maximum_entry_difference(first: InertiaTensor, second: InertiaTensor) -> f64 {
    (0..3)
        .flat_map(|row| (0..3).map(move |column| (row, column)))
        .map(|(row, column)| {
            (first.entries_kilogram_square_metres[row][column]
                - second.entries_kilogram_square_metres[row][column])
                .abs()
        })
        .fold(0.0_f64, f64::max)
}

#[test]
fn williams_mass_properties_use_the_authored_axle_frame() {
    let configuration = williams_configuration();
    let properties = resolve_vehicle_mass_properties(&configuration);
    assert_eq!(properties.source, MassPropertySource::PhysicalInventory);
    assert!((properties.total_mass_kilograms - 612.6).abs() < 0.001);
    assert!((properties.center_of_mass_local_metres.z - 0.4578363).abs() < 0.0005);
    assert!((properties.front_weight_distribution - 0.44447).abs() < 0.0005);
    println!(
        "WILLIAMS_MASS {}",
        serde_json::json!({
            "center_of_mass_local_metres": [
                properties.center_of_mass_local_metres.x,
                properties.center_of_mass_local_metres.y,
                properties.center_of_mass_local_metres.z
            ],
            "front_weight_distribution": properties.front_weight_distribution,
            "principal_moments": properties.principal_moments(),
            "products_of_inertia": properties.products_of_inertia(),
            "wheel_spin_inertia": properties.wheel_spin_inertia_kilogram_square_metres,
            "unsprung_mass": properties.unsprung_mass_kilograms,
        })
    );
}

#[test]
fn wheel_spin_inertia_and_unsprung_mass_come_from_the_inventory() {
    let configuration = williams_configuration();
    assert!((wheel_spin_inertia(&configuration, WheelIndex::FrontLeft) - 0.632529).abs() < 1e-6);
    assert!((wheel_spin_inertia(&configuration, WheelIndex::RearRight) - 0.829554).abs() < 1e-6);
    assert!((unsprung_mass(&configuration, WheelIndex::FrontLeft) - 15.0).abs() < 1e-9);
    assert!((unsprung_mass(&configuration, WheelIndex::RearRight) - 18.0).abs() < 1e-9);
}

#[test]
fn legacy_profile_keeps_the_symmetric_frame_and_box_inertia() {
    let configuration = VehicleConfig::f1_94_canonical();
    let properties = resolve_vehicle_mass_properties(&configuration);
    assert_eq!(properties.source, MassPropertySource::UniformBoxFallback);
    let expected_z = (0.5 - configuration.chassis.front_weight_distribution)
        * configuration.chassis.wheelbase;
    assert!((properties.center_of_mass_local_metres.z - expected_z).abs() < 1e-9);
}

#[test]
fn simplified_calibrates_the_reference_load_to_the_target_height() {
    let mut configuration = williams_configuration();
    let target_height = configuration.chassis.center_of_gravity_height_offset;
    let reference_load = configuration.fuel.initial_kg;
    for (fuel_kilograms, expected_raw_height) in
        [(0.0, -0.241493), (7.6, -0.239737), (105.0, -0.220568)]
    {
        configuration.fuel.set_current_kg(fuel_kilograms);
        let (_, raw_center, _) = composite_center_of(&raw_components_with_centers(&configuration));
        assert!(
            (raw_center.y - expected_raw_height).abs() < 1.0e-4,
            "raw composite height {} differs from the audited {expected_raw_height}",
            raw_center.y
        );
        let standard = resolve_vehicle_mass_properties(&configuration);
        assert_eq!(standard.center_of_mass_local_metres.y, target_height);
        let simplified = resolve_vehicle_mass_properties_for_suspension_backend(
            &configuration,
            SuspensionBackend::Simplified,
        );
        assert_eq!(simplified.total_mass_kilograms, standard.total_mass_kilograms);
        assert!(
            (simplified.center_of_mass_local_metres.z - standard.center_of_mass_local_metres.z)
                .abs()
                < 1.0e-9
        );
        assert!(simplified.inertia_about_center_of_mass.validate().is_ok());
    }

    configuration.fuel.set_current_kg(reference_load);
    let reference = resolve_vehicle_mass_properties_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
    );
    assert!(
        (reference.center_of_mass_local_metres.y - target_height).abs() < 1.0e-9,
        "the composite height at the reference load must land on the calibrated target"
    );

    configuration.fuel.set_current_kg(0.0);
    let empty = resolve_vehicle_mass_properties_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
    );
    configuration.fuel.set_current_kg(105.0);
    let full = resolve_vehicle_mass_properties_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
    );
    assert!(empty.center_of_mass_local_metres.y < target_height);
    assert!(full.center_of_mass_local_metres.y > target_height + 0.015);
}

#[test]
fn calibration_keeps_the_tensor_about_the_published_center() {
    let mut configuration = williams_configuration();
    for fuel_kilograms in [0.0, 7.6, 105.0] {
        configuration.fuel.set_current_kg(fuel_kilograms);
        let properties = resolve_vehicle_mass_properties_for_suspension_backend(
            &configuration,
            SuspensionBackend::Simplified,
        );
        let components = calibrated_components_with_centers(&configuration);
        let (mass, center, inertia) = composite_center_of(&components);
        assert_eq!(mass, properties.total_mass_kilograms);
        assert_eq!(center, properties.center_of_mass_local_metres);
        assert_eq!(inertia, properties.inertia_about_center_of_mass);
    }
}

#[test]
fn mass_components_close_the_budget_without_duplicating_wheels_or_fuel() {
    let mut configuration = williams_configuration();
    configuration.fuel.set_current_kg(105.0);
    let inventory = configuration
        .chassis
        .physical_mass_inventory
        .as_ref()
        .expect("Williams inventory");
    let components = resolved_mass_components(&configuration).expect("resolved components");
    assert_eq!(components.len(), inventory.components.len() + 1);
    let component_mass: f64 = components
        .iter()
        .map(|component| component.mass_kilograms)
        .sum();
    let declared_dry_mass = configuration
        .chassis
        .complete_dry_vehicle_mass(&configuration.tires);
    let fuel_mass = configuration.fuel.effective_current_kg();
    assert!((component_mass - declared_dry_mass - fuel_mass).abs() < 1e-9);

    let wheel_components = components
        .iter()
        .filter(|component| component.kind == ResolvedMassComponentKind::Wheel)
        .count();
    let fuel_components = components
        .iter()
        .filter(|component| component.kind == ResolvedMassComponentKind::Fuel)
        .count();
    assert_eq!(wheel_components, 4, "each wheel must enter exactly once");
    assert_eq!(fuel_components, 1, "the fuel must enter exactly once");

    let properties = resolve_vehicle_mass_properties_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
    );
    assert!((properties.total_mass_kilograms - component_mass).abs() < 1e-9);
}

#[test]
fn simplified_tensor_is_expressed_about_the_published_center() {
    let mut configuration = williams_configuration();
    configuration.fuel.set_current_kg(7.6);
    let components = calibrated_components_with_centers(&configuration);
    let (mass, center, inertia) = composite_center_of(&components);

    let translation = Vec3::new(0.25, -0.40, 0.75);
    let translated_components: Vec<(f64, Vec3, InertiaTensor)> = components
        .iter()
        .map(|(component_mass, component_center, component_inertia)| {
            (
                *component_mass,
                *component_center + translation,
                *component_inertia,
            )
        })
        .collect();
    let (translated_mass, translated_center, translated_inertia) =
        composite_center_of(&translated_components);
    assert_eq!(translated_mass, mass);
    assert!((translated_center - (center + translation)).length() < 1e-12);
    assert!(maximum_entry_difference(translated_inertia, inertia) < 1e-9);

    let expected_about_shifted_point = parallel_axis_about(inertia, mass, translation);
    let composed_about_shifted_point = composite_about_point(&components, center + translation);
    let difference =
        maximum_entry_difference(composed_about_shifted_point, expected_about_shifted_point);
    assert!(
        difference < 1e-9,
        "tensor reference translation mismatch {difference}"
    );
}

#[test]
fn simplified_static_axle_share_matches_the_published_center() {
    let mut configuration = williams_configuration();
    for fuel_kilograms in [0.0, 7.6, 105.0] {
        configuration.fuel.set_current_kg(fuel_kilograms);
        let properties = resolve_vehicle_mass_properties_for_suspension_backend(
            &configuration,
            SuspensionBackend::Simplified,
        );
        let configured_share = configuration
            .chassis
            .effective_front_weight_distribution(&configuration.tires, &configuration.fuel);
        assert!(
            (properties.front_weight_distribution - configured_share).abs() < 1e-5,
            "axle share {} differs from the configured {configured_share}",
            properties.front_weight_distribution
        );
        let front_force = static_wheel_load(&configuration, WheelIndex::FrontLeft)
            + static_wheel_load(&configuration, WheelIndex::FrontRight);
        let rear_force = static_wheel_load(&configuration, WheelIndex::RearLeft)
            + static_wheel_load(&configuration, WheelIndex::RearRight);
        let total_force = front_force + rear_force;
        assert!(
            (front_force / total_force - properties.front_weight_distribution).abs() < 1e-5,
            "static axle reactions disagree with the published center"
        );
    }
}

#[test]
fn world_center_queries_follow_the_suspension_backend() {
    let mut configuration = williams_configuration();
    configuration.fuel.set_current_kg(0.0);
    let transform = Transform3D::new(Vec3::new(1.0, 2.0, 3.0), Mat3::IDENTITY);
    let standard = center_of_mass_world(&configuration, &transform);
    let simplified = center_of_mass_world_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
        &transform,
    );
    assert!((simplified.y - standard.y).abs() > 0.001);
    assert_eq!(simplified.x, standard.x);
    assert_eq!(simplified.z, standard.z);
}
