use vehicle_physics_engine::suspension_mass_audit::SuspensionMassAudit;
use vehicle_physics_engine::wheel_mechanics::GRAVITY_M_S2;
use vehicle_physics_engine::VehicleConfig;

fn vehicle_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

#[test]
fn corner_normal_forces_include_fuel_once_and_balance_vehicle_weight() {
    let configuration = vehicle_configuration();
    let audit = SuspensionMassAudit::from_vehicle_configuration(&configuration).unwrap();
    let total_normal_force_newtons: f64 = audit
        .corners
        .iter()
        .map(|corner| corner.static_normal_force_newtons)
        .sum();
    assert!(
        (total_normal_force_newtons - audit.total_vehicle_mass_kilograms * GRAVITY_M_S2).abs()
            < 1e-9
    );
    assert_eq!(
        audit.total_vehicle_mass_kilograms,
        audit.dry_vehicle_mass_kilograms + audit.current_fuel_mass_kilograms
    );
    assert!(audit.suspended_mass_kilograms.is_none());
    assert!(!audit.unresolved_physical_properties.is_empty());
}

#[test]
fn audit_rejects_invalid_physical_masses_before_tuning_clamps_hide_them() {
    let mut configuration = vehicle_configuration();
    configuration.front_wheel_mass = -1.0;
    assert!(SuspensionMassAudit::from_vehicle_configuration(&configuration).is_err());
    configuration.front_wheel_mass = f64::NAN;
    assert!(SuspensionMassAudit::from_vehicle_configuration(&configuration).is_err());
    configuration = vehicle_configuration();
    configuration.vehicle_mass = f64::INFINITY;
    assert!(SuspensionMassAudit::from_vehicle_configuration(&configuration).is_err());
}

#[test]
fn body_mass_contract_adds_each_wheel_once_and_preserves_axle_moments() {
    let mut configuration = vehicle_configuration();
    configuration.fuel.current_kg = 0.0;
    configuration.fuel.initial_kg = 0.0;
    assert_eq!(configuration.complete_dry_vehicle_mass(), 698.0);
    assert_eq!(configuration.total_vehicle_mass(), 698.0);
    let front_supported_mass = configuration.vehicle_mass * configuration.front_weight_distribution
        + 2.0 * configuration.front_wheel_mass;
    assert!(
        (configuration.effective_front_weight_distribution() * 698.0 - front_supported_mass).abs()
            < 1e-10
    );
    let serialized = configuration.to_json_value();
    let reloaded =
        VehicleConfig::from_json_str(&serde_json::to_string(&serialized).unwrap()).unwrap();
    assert!(reloaded.vehicle_mass_excludes_wheel_assemblies);
    assert_eq!(
        reloaded.total_vehicle_mass(),
        configuration.total_vehicle_mass()
    );
}

#[test]
fn profiles_without_new_contract_keep_complete_dry_mass_semantics() {
    let configuration = VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_physics.json"
    ))
    .unwrap();
    assert!(!configuration.vehicle_mass_excludes_wheel_assemblies);
    assert_eq!(
        configuration.complete_dry_vehicle_mass(),
        configuration.vehicle_mass
    );
}
