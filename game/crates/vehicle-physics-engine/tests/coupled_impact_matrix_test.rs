use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use vehicle_physics_engine::coupled_vehicle_world::*;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::types::{Mat3, Quat, Vec3};
use vehicle_physics_engine::VehicleConfig;

#[derive(Default)]
struct ImpactMetrics {
    minimum_up_y: f64,
    maximum_penetration_metres: f64,
    maximum_vertical_speed_metres_per_second: f64,
    maximum_island_entity_count: usize,
    event_count: usize,
    finite: bool,
}

impl ImpactMetrics {
    fn new() -> Self {
        Self {
            minimum_up_y: 1.0,
            maximum_penetration_metres: 0.0,
            maximum_vertical_speed_metres_per_second: 0.0,
            maximum_island_entity_count: 0,
            event_count: 0,
            finite: true,
        }
    }

    fn observe(&mut self, world: &CoupledVehicleWorld) {
        for vehicle in world.vehicles.values() {
            let state = &vehicle.state.suspension;
            let up_y = state.body_orientation_world.rotate_vec3(Vec3::UP).y;
            self.minimum_up_y = self.minimum_up_y.min(up_y);
            let velocity = state.generalized_velocity;
            self.maximum_vertical_speed_metres_per_second =
                self.maximum_vertical_speed_metres_per_second.max(velocity[1].abs());
            self.finite &= up_y.is_finite()
                && state.body_origin_world_metres.x.is_finite()
                && state.body_origin_world_metres.y.is_finite()
                && state.body_origin_world_metres.z.is_finite()
                && state.body_orientation_world.x.is_finite()
                && state.body_orientation_world.y.is_finite()
                && state.body_orientation_world.z.is_finite()
                && state.body_orientation_world.w.is_finite()
                && velocity.iter().all(|value| value.is_finite());
        }
        for entry in world.contact_manifolds.values()
            .filter(|entry| entry.last_seen_step == world.physical_step_index) {
            self.maximum_penetration_metres =
                self.maximum_penetration_metres.max((-entry.separation_metres).max(0.0));
        }
        self.maximum_island_entity_count = self.maximum_island_entity_count
            .max(world.contact_island_diagnostics.last_largest_island_entity_count);
        self.event_count += world.collision_events.len();
    }
}

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn flat_floor_shape() -> PhysicalWorldShape {
    let shape = SharedShape::new(TriMesh::new(
        vec![
            Point3::new(-80.0, 0.0, -80.0),
            Point3::new(80.0, 0.0, -80.0),
            Point3::new(80.0, 0.0, 80.0),
            Point3::new(-80.0, 0.0, 80.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    ).unwrap());
    PhysicalWorldShape {
        identifier: "flat_floor".into(),
        surface_code: 0,
        driveable: true,
        shape,
        wheel_impact_shape: None,
    }
}

fn flat_floor_world() -> PhysicalCollisionWorld {
    PhysicalCollisionWorld {
        source_digest: "analytical_impact_floor".into(),
        shapes: vec![flat_floor_shape()],
    }
}

fn box_shape(center: Vec3, half_extents: Vec3) -> SharedShape {
    let x = half_extents.x;
    let y = half_extents.y;
    let z = half_extents.z;
    let vertices = vec![
        Point3::new(center.x - x, center.y - y, center.z - z),
        Point3::new(center.x + x, center.y - y, center.z - z),
        Point3::new(center.x + x, center.y + y, center.z - z),
        Point3::new(center.x - x, center.y + y, center.z - z),
        Point3::new(center.x - x, center.y - y, center.z + z),
        Point3::new(center.x + x, center.y - y, center.z + z),
        Point3::new(center.x + x, center.y + y, center.z + z),
        Point3::new(center.x - x, center.y + y, center.z + z),
    ];
    let triangles = vec![
        [0, 2, 1], [0, 3, 2],
        [4, 5, 6], [4, 6, 7],
        [0, 1, 5], [0, 5, 4],
        [3, 7, 6], [3, 6, 2],
        [0, 4, 7], [0, 7, 3],
        [1, 2, 6], [1, 6, 5],
    ];
    SharedShape::new(TriMesh::new(vertices, triangles).unwrap())
}

fn wall_world(z_position: f64, half_thickness_metres: f64) -> PhysicalCollisionWorld {
    let mut world = flat_floor_world();
    world.shapes.push(PhysicalWorldShape {
        identifier: "impact_wall".into(),
        surface_code: 6,
        driveable: false,
        shape: box_shape(Vec3::new(0.0, 2.0, z_position), Vec3::new(8.0, 2.0, half_thickness_metres)),
        wheel_impact_shape: None,
    });
    world.source_digest = format!("analytical_impact_wall_{z_position}");
    world
}

fn curb_world(center_x: f64, half_x: f64, z_position: f64, height_metres: f64) -> PhysicalCollisionWorld {
    let mut world = flat_floor_world();
    world.shapes.push(PhysicalWorldShape {
        identifier: "partial_curb".into(),
        surface_code: 1,
        driveable: false,
        shape: box_shape(
            Vec3::new(center_x, height_metres * 0.5, z_position),
            Vec3::new(half_x, height_metres * 0.5, 0.3),
        ),
        wheel_impact_shape: None,
    });
    world.source_digest = format!("analytical_partial_curb_{center_x}_{z_position}");
    world
}

fn sidewall_world(x_position: f64) -> PhysicalCollisionWorld {
    let mut world = flat_floor_world();
    world.shapes.push(PhysicalWorldShape {
        identifier: "side_wall".into(),
        surface_code: 6,
        driveable: false,
        shape: box_shape(Vec3::new(x_position + 0.1, 1.5, 0.0), Vec3::new(0.1, 1.5, 20.0)),
        wheel_impact_shape: None,
    });
    world.source_digest = format!("analytical_sidewall_{x_position}");
    world
}

fn scenario_configuration(body_box_identifier: Option<&str>) -> VehicleConfig {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    settings.contacts.maximum_supported_angular_speed_radians_per_second = 60.0;
    if let Some(identifier) = body_box_identifier {
        settings.body_collision_boxes.retain(|shape| shape.identifier == identifier);
    }
    configuration.coupled_world = Some(settings);
    configuration
}

fn position_vehicles(world: &mut CoupledVehicleWorld, configuration: &VehicleConfig,
    placements: &[(Vec3, Vec3)]) -> Vec<u64> {
    placements.iter().map(|(position, velocity)| {
        let identifier = world.register_vehicle(configuration.clone(), *position, 0.0).unwrap();
        let state = &mut world.vehicles.get_mut(&identifier).unwrap().state.suspension;
        state.generalized_velocity[0] = velocity.x;
        state.generalized_velocity[1] = velocity.y;
        state.generalized_velocity[2] = velocity.z;
        identifier
    }).collect()
}

#[test]
fn frontal_wall_impact_stays_upright_and_does_not_tunnel() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(wall_world(-4.0, 0.1), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[2] = -60.0;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..30 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.5, "vehicle rolled over: up_y {}", metrics.minimum_up_y);
    assert!(state.body_origin_world_metres.z > -3.0, "vehicle tunneled through the wall: {}", state.body_origin_world_metres.z);
    assert!(metrics.maximum_penetration_metres < 0.05, "penetration {} m", metrics.maximum_penetration_metres);
    assert!(metrics.event_count > 0, "impact produced no collision event");
    assert!(world.continuous_collision_diagnostics.total_impact_events > 0);
    assert!(state.generalized_velocity[2].abs() < 60.0, "vehicle retained full impact speed");
    assert!(metrics.maximum_vertical_speed_metres_per_second < 15.0,
        "vehicle was launched vertically at {} m/s", metrics.maximum_vertical_speed_metres_per_second);
    let collision_energy: f64 = world.vehicles.values()
        .map(|vehicle| vehicle.state.suspension.collision_energy_change_joules).sum();
    assert!(collision_energy <= 1e-6, "passive impact added energy: {collision_energy}");
}

#[test]
fn oblique_wall_impact_keeps_lateral_response_bounded() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(wall_world(-4.0, 0.1), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    {
        let state = &mut world.vehicles.get_mut(&identifier).unwrap().state.suspension;
        state.generalized_velocity[0] = -12.0;
        state.generalized_velocity[2] = -25.0;
    }
    let mut metrics = ImpactMetrics::new();
    for _ in 0..30 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.3, "oblique impact rolled the vehicle: {}", metrics.minimum_up_y);
    assert!(state.body_origin_world_metres.z > -3.0, "oblique impact tunneled: {}", state.body_origin_world_metres.z);
    assert!(state.body_origin_world_metres.x.abs() < 20.0, "lateral response unbounded: {}", state.body_origin_world_metres.x);
    assert!(metrics.event_count > 0);
    assert!(metrics.maximum_penetration_metres < 0.05);
}

#[test]
fn three_hundred_kilometres_per_hour_impact_does_not_tunnel() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(wall_world(-6.0, 0.05), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[2] = -300.0 / 3.6;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..24 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(state.body_origin_world_metres.z > -5.0, "300 km/h impact tunneled: {}", state.body_origin_world_metres.z);
    assert!(metrics.minimum_up_y > 0.3);
    assert!(metrics.event_count > 0);
    assert!(world.continuous_collision_diagnostics.total_impact_events > 0);
    assert!(metrics.maximum_penetration_metres < 0.05);
}

#[test]
fn partial_curb_crossing_stays_upright_and_reports_contact() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(curb_world(1.0, 1.0, -3.0, 0.10), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[2] = -20.0;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..36 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.3, "partial curb rolled the vehicle: {}", metrics.minimum_up_y);
    assert!(metrics.event_count > 0, "partial curb produced no contact event");
    assert!(metrics.maximum_penetration_metres < 0.05);
    assert!(metrics.maximum_vertical_speed_metres_per_second < 10.0,
        "curb launched the vehicle at {} m/s", metrics.maximum_vertical_speed_metres_per_second);
    assert!(state.body_origin_world_metres.y.is_finite());
}

#[test]
fn landing_from_height_hits_travel_stops_without_rollover() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 3.0, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[1] = -10.0;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..60 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.5, "landing rolled the vehicle: {}", metrics.minimum_up_y);
    assert!(state.body_origin_world_metres.y > 0.2 && state.body_origin_world_metres.y < 0.6,
        "landing height {}", state.body_origin_world_metres.y);
    assert!(metrics.event_count > 0, "landing produced no contact event");
    assert!(metrics.maximum_penetration_metres < 0.05);
    assert!(world.contact_classification_diagnostics.travel_stop_wheels > 0,
        "hard landing did not reach the travel-stop diagnostic");
}

#[test]
fn sidewall_impact_keeps_lateral_velocity_bounded() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(sidewall_world(1.5), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    {
        let state = &mut world.vehicles.get_mut(&identifier).unwrap().state.suspension;
        state.generalized_velocity[0] = 15.0;
        state.generalized_velocity[2] = -10.0;
    }
    let mut metrics = ImpactMetrics::new();
    for _ in 0..30 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    let state = &world.vehicles[&identifier].state.suspension;
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.3, "side impact rolled the vehicle: {}", metrics.minimum_up_y);
    assert!(state.body_origin_world_metres.x < 1.6, "sidewall tunneled: {}", state.body_origin_world_metres.x);
    assert!(state.generalized_velocity[0].abs() < 15.0, "side impact retained full lateral speed");
    assert!(metrics.event_count > 0);
    assert!(metrics.maximum_penetration_metres < 0.05);
}

#[test]
fn nose_down_asphalt_impact_stays_upright_and_bounded() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.8, 0.0), 0.0).unwrap();
    {
        let state = &mut world.vehicles.get_mut(&identifier).unwrap().state.suspension;
        state.body_orientation_world = Quat::from_mat3(&Mat3::from_axis_angle(
            Vec3::RIGHT, -12.0_f64.to_radians()));
        state.generalized_velocity[1] = -3.0;
        state.generalized_velocity[2] = -30.0;
        state.generalized_velocity[3] = -1.5;
    }
    let mut metrics = ImpactMetrics::new();
    for _ in 0..45 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.3, "nose-down impact flipped the vehicle: {}", metrics.minimum_up_y);
    assert!(metrics.event_count > 0, "nose-down asphalt impact produced no event");
    assert!(metrics.maximum_penetration_metres < 0.05);
    assert!(metrics.maximum_vertical_speed_metres_per_second < 15.0,
        "nose-down impact launched the vehicle at {} m/s", metrics.maximum_vertical_speed_metres_per_second);
    let collision_energy: f64 = world.vehicles.values()
        .map(|vehicle| vehicle.state.suspension.collision_energy_change_joules).sum();
    assert!(collision_energy <= 1e-6, "nose-down impact added energy: {collision_energy}");
}

#[test]
fn bottoming_platform_impact_is_classified_and_bounded() {
    let configuration = scenario_configuration(None);
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(curb_world(0.0, 3.0, -3.0, 0.15), settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[2] = -30.0;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..30 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    assert!(metrics.finite);
    assert!(metrics.minimum_up_y > 0.3, "bottoming impact rolled the vehicle: {}", metrics.minimum_up_y);
    assert!(metrics.event_count > 0, "bottoming impact produced no event");
    assert!(world.contact_classification_diagnostics.rigid_impact_contacts > 0);
    assert!(metrics.maximum_penetration_metres < 0.05);
    assert!(metrics.maximum_vertical_speed_metres_per_second < 15.0,
        "bottoming impact launched the vehicle at {} m/s", metrics.maximum_vertical_speed_metres_per_second);
}

#[test]
fn wheel_to_wheel_contact_is_reported_in_the_pair_event() {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    settings.contacts.maximum_supported_angular_speed_radians_per_second = 60.0;
    settings.body_collision_boxes = vec![VehicleCollisionBox {
        identifier: "sensor".into(),
        center_local_metres: Vec3::new(0.0, 5.0, 0.0),
        half_extents_metres: Vec3::new(0.05, 0.05, 0.05),
    }];
    configuration.coupled_world = Some(settings.clone());
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let first = world.register_vehicle(configuration.clone(), Vec3::new(-1.1, 0.35, 0.0), 0.0).unwrap();
    let second = world.register_vehicle(configuration, Vec3::new(1.1, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&first).unwrap().state.suspension.generalized_velocity[0] = 3.0;
    world.vehicles.get_mut(&second).unwrap().state.suspension.generalized_velocity[0] = -3.0;
    let mut wheel_event = false;
    let mut metrics = ImpactMetrics::new();
    for _ in 0..20 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
        wheel_event |= world.collision_events.iter().any(|event| event.contact_identifier.contains("wheel_"));
    }
    assert!(metrics.finite);
    assert!(wheel_event, "wheel-to-wheel contact did not publish a wheel event");
    assert!(world.vehicles[&first].state.suspension.generalized_velocity[0] < 3.0);
    assert!(world.vehicles[&second].state.suspension.generalized_velocity[0] > -3.0);
}

#[test]
fn three_vehicle_pileup_forms_one_contact_island() {
    let configuration = scenario_configuration(Some("monocoque"));
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let placements = [
        (Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.0, 0.0, -20.0)),
        (Vec3::new(0.0, 0.35, -3.0), Vec3::new(0.0, 0.0, -10.0)),
        (Vec3::new(0.0, 0.35, -6.0), Vec3::new(0.0, 0.0, 0.0)),
    ];
    let identifiers = position_vehicles(&mut world, &configuration, &placements);
    let mut metrics = ImpactMetrics::new();
    for _ in 0..45 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    assert!(metrics.finite);
    assert!(metrics.event_count > 0, "three-car pileup produced no event");
    assert!(metrics.maximum_island_entity_count >= 3,
        "pileup island had {} entities", metrics.maximum_island_entity_count);
    assert!(identifiers.iter().all(|identifier| {
        world.vehicles[identifier].state.suspension.generalized_velocity.iter().all(|value| value.is_finite())
    }));
}

#[test]
fn four_vehicle_pileup_stays_finite_and_reports_the_island() {
    let mut configuration = scenario_configuration(Some("monocoque"));
    configuration.coupled_world.as_mut().unwrap().contacts.velocity_tolerance_metres_per_second = 1e-5;
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let placements = [
        (Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.0, 0.0, -30.0)),
        (Vec3::new(0.0, 0.35, -2.5), Vec3::new(0.0, 0.0, -20.0)),
        (Vec3::new(0.0, 0.35, -5.0), Vec3::new(0.0, 0.0, -10.0)),
        (Vec3::new(0.0, 0.35, -7.5), Vec3::new(0.0, 0.0, 0.0)),
    ];
    let _identifiers = position_vehicles(&mut world, &configuration, &placements);
    let mut metrics = ImpactMetrics::new();
    for _ in 0..50 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        metrics.observe(&world);
    }
    assert!(metrics.finite);
    assert!(metrics.event_count > 0, "four-car pileup produced no event");
    assert!(metrics.maximum_island_entity_count >= 4,
        "pileup island had {} entities", metrics.maximum_island_entity_count);
}
