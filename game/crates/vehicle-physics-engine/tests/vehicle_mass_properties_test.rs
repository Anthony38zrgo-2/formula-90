use vehicle_physics_engine::vehicle_mass_properties::{
    resolve_vehicle_mass_properties, unsprung_mass, wheel_spin_inertia, MassPropertySource,
};
use vehicle_physics_engine::{VehicleConfig, WheelIndex};

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .expect("Williams profile")
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
