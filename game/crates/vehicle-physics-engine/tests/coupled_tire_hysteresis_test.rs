use parry3d_f64::shape::SharedShape;
use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, TimestampedVehicleInput};
use vehicle_physics_engine::physical_collision_world::{
    PhysicalCollisionWorld, PhysicalWorldShape,
};
use vehicle_physics_engine::{Vec3, VehicleConfig, VehicleInput};

#[test]
fn coupled_carcass_heat_matches_radial_damper_loss_without_elastic_work() {
    for velocity in [-0.01, 0.0, 0.01] {
        let mut configuration = VehicleConfig::from_json_str(include_str!(
            "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
        ))
        .unwrap();
        configuration.fuel.current_kg = 0.0;
        configuration.fuel.initial_kg = 0.0;
        configuration.tires.tire_thermal.front.tread_to_carcass_w_k = 0.0;
        configuration.tires.tire_thermal.front.carcass_to_air_w_k = 0.0;
        configuration.tires.tire_thermal.front.carcass_to_gas_w_k = 0.0;
        configuration.brakes.brake_thermal.front.rim_to_tire_carcass_w_k = 0.0;
        let settings = configuration.coupled_world.clone().unwrap();
        let duration = 1.0 / settings.physical_frequency_hertz as f64;
        let damping = settings
            .suspension
            .tire_vertical_damping_newton_seconds_per_metre[0];
        let capacity = configuration.tires.tire_thermal.front.carcass_heat_capacity_j_k;
        let fraction = configuration
            .tires
            .tire_thermal
            .front
            .carcass_hysteresis_efficiency;
        let shape = SharedShape::cuboid(50.0, 1.0, 50.0);
        let terrain = PhysicalCollisionWorld {
            source_digest: "analytical_hysteresis_fixture".into(),
            shapes: vec![PhysicalWorldShape {
                identifier: "analytical_ground".into(),
                surface_code: 0,
                driveable: true,
                shape,
                wheel_impact_shape: None,
            }],
        };
        let height = 1.0 + configuration.tires.front_tire_radius - 0.015;
        let mut world = CoupledVehicleWorld::new(terrain, settings).unwrap();
        let identifier = world
            .register_vehicle(configuration, Vec3::new(0.0, height, 0.0), 0.0)
            .unwrap();
        let vehicle = world.vehicles.get_mut(&identifier).unwrap();
        vehicle.state.suspension.generalized_velocity[1] = velocity;
        let initial_temperature = vehicle.systems.state.tire_thermal.wheels[0].carcass_c;
        world
            .enqueue_input(
                identifier,
                TimestampedVehicleInput {
                    time_seconds: 0.0,
                    input: VehicleInput::default(),
                    driving_aids_mask: 0,
                },
            )
            .unwrap();
        world.advance_host_interval(duration).unwrap();
        let vehicle = &world.vehicles[&identifier];
        let deflection_velocity =
            vehicle.systems.state.suspension.wheels[0].tire_deflection_velocity_m_s;
        assert!((deflection_velocity + velocity).abs() < 1e-10);
        let heat = (vehicle.systems.state.tire_thermal.wheels[0].carcass_c - initial_temperature)
            * capacity;
        let expected_heat = damping * velocity * velocity * duration * fraction;
        assert!(
            (heat - expected_heat).abs() < 1e-8,
            "{heat} differs from {expected_heat}"
        );
    }
}
