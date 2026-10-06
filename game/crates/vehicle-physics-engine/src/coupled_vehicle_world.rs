use crate::aero::AeroEnvironment;
use crate::coupled_vehicle_dynamics::{CoupledVehicleModel, CoupledVehicleState};
use crate::generalized_vehicle_contacts::{solve_generalized_vehicle_contacts, GeneralizedContactConstraint, GeneralizedContactSettings};
use crate::physical_collision_world::{physical_pose, vehicle_vector, PhysicalCollisionWorld};
use crate::simulation::{BodyKinematics, CoupledVehicleForceContext, VehicleSimulator};
use crate::suspension_component_kinematics::wheel_contact_component;
use crate::suspension_coupled_solver::{CoupledSuspensionInput, CoupledSuspensionSettings, CoupledWheelContact};
use crate::suspension_kinematics::solve_corner;
use crate::telemetry::TelemetryFrame;
use crate::types::{Mat3, RaycastHit, SurfaceType, Transform3D, TriRaycastSample, Vec3, VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use parry3d_f64::na::{Isometry3, Point3};
use parry3d_f64::bounding_volume::{Aabb, BoundingVolume};
use parry3d_f64::query::{contact, cast_shapes_nonlinear, NonlinearRigidMotion};
use parry3d_f64::shape::SharedShape;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleCollisionBox {
    pub identifier: String,
    pub center_local_metres: Vec3,
    pub half_extents_metres: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledVehicleWorldConfiguration {
    pub schema_version: u32,
    pub integration_owner: String,
    pub physical_frequency_hertz: u32,
    pub suspension: CoupledSuspensionSettings,
    pub contacts: GeneralizedContactSettings,
    pub body_collision_boxes: Vec<VehicleCollisionBox>,
    pub steering_rack_travel_metres: f64,
    pub road_query_extension_metres: f64,
    pub maximum_initial_overlap_metres: f64,
    pub aerodynamic_probe_positions_local_metres: [Vec3; 5],
    pub aerodynamic_probe_length_metres: f64,
}

impl CoupledVehicleWorldConfiguration {
    pub fn validate(&self) -> Result<(), String> {
        self.suspension.validate()?;
        self.contacts.validate()?;
        if self.schema_version != 1 || self.integration_owner != "rust_world_candidate"
            || self.physical_frequency_hertz < 120 || self.physical_frequency_hertz > 4000
            || 1.0 / self.physical_frequency_hertz as f64 > self.suspension.maximum_substep_seconds + 1e-12
            || [self.steering_rack_travel_metres, self.road_query_extension_metres, self.maximum_initial_overlap_metres, self.aerodynamic_probe_length_metres].iter().any(|value| !value.is_finite() || *value <= 0.0)
            || self.aerodynamic_probe_positions_local_metres.iter().any(|position| !crate::suspension_mass_properties::vector_is_finite(*position))
            || self.body_collision_boxes.is_empty()
        { return Err("Invalid coupled world ownership, frequency, domain or collision configuration".into()); }
        let mut identifiers = std::collections::HashSet::new();
        for shape in &self.body_collision_boxes {
            if !identifiers.insert(&shape.identifier) || shape.identifier.trim().is_empty()
                || !crate::suspension_mass_properties::vector_is_finite(shape.center_local_metres)
                || [shape.half_extents_metres.x, shape.half_extents_metres.y, shape.half_extents_metres.z].iter().any(|value| !value.is_finite() || *value <= 0.0)
            { return Err("Invalid or duplicate vehicle collision box".into()); }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimestampedVehicleInput {
    pub time_seconds: f64,
    pub input: VehicleInput,
    pub driving_aids_mask: u32,
}

#[derive(Debug, Clone)]
pub struct CoupledWorldVehicle {
    pub model: CoupledVehicleModel,
    pub state: CoupledVehicleState,
    pub systems: VehicleSimulator,
    pub input: VehicleInput,
    pub pending_inputs: VecDeque<TimestampedVehicleInput>,
    pub driving_aids_mask: u32,
    pub last_force_input: CoupledSuspensionInput,
    pub spawn_position_world_metres: Vec3,
    pub spawn_yaw_radians: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledWorldSnapshot {
    pub entity_identifier: u64,
    pub state: CoupledVehicleState,
    pub telemetry: TelemetryFrame,
    pub systems_state: crate::simulation::VehicleState,
    pub steering_rack_metres: f64,
    pub fuel_mass_kilograms: f64,
    pub driver_input: VehicleInput,
    pub driving_aids_mask: u32,
    pub operating_mass_kilograms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledWorldCollisionEvent {
    pub contact_identifier: String,
    pub time_seconds: f64,
    pub first_entity_identifier: u64,
    pub second_entity_identifier: Option<u64>,
    pub point_world_metres: Vec3,
    pub normal_world: Vec3,
    pub normal_impulse_newton_seconds: f64,
    pub tangential_impulse_newton_seconds: [f64; 2],
    pub surface_code: u32,
}

#[derive(Debug, Clone)]
pub struct CoupledVehicleWorld {
    pub collision_world: PhysicalCollisionWorld,
    pub configuration: CoupledVehicleWorldConfiguration,
    pub vehicles: BTreeMap<u64, CoupledWorldVehicle>,
    pub time_seconds: f64,
    pub accumulated_host_time_seconds: f64,
    pub paused: bool,
    pub collision_events: Vec<CoupledWorldCollisionEvent>,
    next_entity_identifier: u64,
}

struct AttachedVehicleShape {
    entity_index: usize,
    identifier: String,
    wheel: Option<WheelIndex>,
    pose: Isometry3<f64>,
    shape: SharedShape,
}

struct ContactMetadata {
    point_world_metres: Vec3,
    normal_world: Vec3,
    surface_code: u32,
    penetration_metres: f64,
}

fn contact_axes(normal: Vec3) -> [Vec3; 3] {
    let reference = if normal.dot(Vec3::UP).abs() < 0.9 { Vec3::UP } else { Vec3::RIGHT };
    let tangent = normal.cross(reference).normalized();
    [normal, tangent, normal.cross(tangent)]
}

fn surface_from_code(code: u32) -> Result<SurfaceType, String> {
    Ok(match code { 0 => SurfaceType::Road, 1 => SurfaceType::Curb, 2 => SurfaceType::Dirt,
        3 => SurfaceType::Grass, 4 => SurfaceType::Gravel, 5 => SurfaceType::Sand, 6 => SurfaceType::Wall, 7 => SurfaceType::Metal,
        _ => return Err("Unsupported physical surface code".into()) })
}

impl CoupledVehicleWorld {
    pub fn new(collision_world: PhysicalCollisionWorld, configuration: CoupledVehicleWorldConfiguration) -> Result<Self, String> {
        configuration.validate()?;
        Ok(Self { collision_world, configuration, vehicles: BTreeMap::new(), time_seconds: 0.0,
            accumulated_host_time_seconds: 0.0, paused: false, collision_events: Vec::new(), next_entity_identifier: 1 })
    }

    pub fn register_vehicle(&mut self, configuration: VehicleConfig, position: Vec3, yaw_radians: f64) -> Result<u64, String> {
        if !crate::suspension_mass_properties::vector_is_finite(position) || !yaw_radians.is_finite() {
            return Err("Invalid vehicle spawn pose".into());
        }
        let selected = configuration.coupled_world.as_ref().ok_or("Vehicle must explicitly select the coupled world")?;
        selected.validate()?;
        if serde_json::to_value(selected).map_err(|error| error.to_string())? != serde_json::to_value(&self.configuration).map_err(|error| error.to_string())? {
            return Err("Vehicles sharing a world require the same world configuration".into());
        }
        let model = CoupledVehicleModel::new(configuration.clone(), self.configuration.suspension.clone())?;
        let systems = VehicleSimulator::new(configuration, position, yaw_radians);
        let mut state = CoupledVehicleState::at_rest(position);
        state.suspension.body_orientation_world = systems.state.orientation;
        state.suspension.simulated_time_seconds = self.time_seconds;
        model.evaluate(&state, &CoupledSuspensionInput::default(), [0.0; 4])?;
        let identifier = self.next_entity_identifier;
        self.next_entity_identifier = identifier.checked_add(1).ok_or("Vehicle identity space exhausted")?;
        let driving_aids_mask = systems.aids.to_bits();
        self.vehicles.insert(identifier, CoupledWorldVehicle { model, state, systems, input: VehicleInput::default(),
            pending_inputs: VecDeque::new(), driving_aids_mask, last_force_input: CoupledSuspensionInput::default(),
            spawn_position_world_metres: position, spawn_yaw_radians: yaw_radians });
        Ok(identifier)
    }

    pub fn remove_vehicle(&mut self, identifier: u64) -> Result<(), String> {
        self.vehicles.remove(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        Ok(())
    }

    pub fn enqueue_input(&mut self, identifier: u64, input: TimestampedVehicleInput) -> Result<(), String> {
        if !input.time_seconds.is_finite() || input.time_seconds < self.time_seconds - 1e-10
            || input.driving_aids_mask & !255 != 0
            || !(-1.0..=1.0).contains(&input.input.steering)
            || [input.input.throttle, input.input.brake, input.input.handbrake, input.input.clutch].iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        { return Err("Invalid or late timestamped vehicle input".into()); }
        let vehicle = self.vehicles.get_mut(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
        if input.input.gear_request.is_some_and(|gear| gear < -1 || gear as isize > vehicle.model.configuration.gear_ratios.len() as isize) {
            return Err("Requested gear lies outside the configured transmission".into());
        }
        if vehicle.pending_inputs.back().is_some_and(|previous| previous.time_seconds >= input.time_seconds) {
            return Err("Vehicle input timestamps must increase strictly".into());
        }
        vehicle.pending_inputs.push_back(input);
        Ok(())
    }

    pub fn reset_vehicle(&mut self, identifier: u64) -> Result<(), String> {
        let vehicle = self.vehicles.get(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
        let mut configuration = vehicle.model.configuration.clone();
        configuration.fuel.refill_to_initial();
        let position = vehicle.spawn_position_world_metres;
        let yaw = vehicle.spawn_yaw_radians;
        let model = CoupledVehicleModel::new(configuration.clone(), self.configuration.suspension.clone())?;
        let systems = VehicleSimulator::new(configuration, position, yaw);
        let mut state = CoupledVehicleState::at_rest(position);
        state.suspension.body_orientation_world = systems.state.orientation;
        state.suspension.simulated_time_seconds = self.time_seconds;
        let vehicle = self.vehicles.get_mut(&identifier).unwrap();
        vehicle.model = model;
        vehicle.state = state;
        vehicle.systems = systems;
        vehicle.pending_inputs.clear();
        vehicle.input = VehicleInput::default();
        vehicle.last_force_input = CoupledSuspensionInput::default();
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        Ok(())
    }

    pub fn snapshots(&self) -> Vec<CoupledWorldSnapshot> {
        self.vehicles.iter().map(|(identifier, vehicle)| CoupledWorldSnapshot { entity_identifier: *identifier,
            state: vehicle.state.clone(), telemetry: vehicle.systems.build_telemetry_frame(),
            systems_state: vehicle.systems.state.clone(), steering_rack_metres: vehicle.last_force_input.steering_rack_metres,
            fuel_mass_kilograms: vehicle.model.configuration.fuel.effective_current_kg(), driver_input: vehicle.input,
            driving_aids_mask: vehicle.driving_aids_mask,
            operating_mass_kilograms: vehicle.model.configuration.total_vehicle_mass() }).collect()
    }

    pub fn advance_host_interval(&mut self, duration_seconds: f64) -> Result<(), String> {
        if !duration_seconds.is_finite() || duration_seconds < 0.0 || duration_seconds > 1.0 { return Err("Unsupported host interval".into()); }
        if self.paused { return Ok(()); }
        let mut candidate = self.clone();
        candidate.collision_events.clear();
        candidate.accumulated_host_time_seconds += duration_seconds;
        let step_seconds = 1.0 / self.configuration.physical_frequency_hertz as f64;
        while candidate.accumulated_host_time_seconds + 1e-12 >= step_seconds {
            candidate.advance_physical_step(step_seconds)?;
            candidate.accumulated_host_time_seconds = (candidate.accumulated_host_time_seconds - step_seconds).max(0.0);
            candidate.time_seconds += step_seconds;
        }
        *self = candidate;
        Ok(())
    }

    fn advance_physical_step(&mut self, duration: f64) -> Result<(), String> {
        let start_time = self.time_seconds;
        let mut elapsed = 0.0;
        let mut events = 0;
        while duration - elapsed > duration * 1e-12 {
            for vehicle in self.vehicles.values_mut() {
                while vehicle.pending_inputs.front().is_some_and(|input| input.time_seconds <= self.time_seconds + 1e-12) {
                    let sample = vehicle.pending_inputs.pop_front().unwrap();
                    vehicle.input = sample.input;
                    vehicle.driving_aids_mask = sample.driving_aids_mask;
                }
            }
            let interval = self.vehicles.values().filter_map(|vehicle| vehicle.pending_inputs.front())
                .map(|sample| sample.time_seconds - self.time_seconds).fold(duration - elapsed, f64::min);
            let initial = self.clone();
            let previous_shapes = initial.attached_shapes()?;
            for vehicle in self.vehicles.values_mut() {
                Self::advance_vehicle_forces(&self.collision_world, &self.configuration, vehicle, interval)?;
            }
            let impact = self.earliest_swept_impact(&previous_shapes, interval)?;
            let accepted_interval = if let Some(impact) = impact {
                if impact < interval - 1e-10 {
                    events += 1;
                    if events > self.configuration.contacts.maximum_events_per_step { return Err("World collision event budget exceeded; physical step was not committed".into()); }
                    *self = initial;
                    let event_interval = impact.max(1e-10);
                    for vehicle in self.vehicles.values_mut() {
                        Self::advance_vehicle_forces(&self.collision_world, &self.configuration, vehicle, event_interval)?;
                    }
                    event_interval
                } else { interval }
            } else { interval };
            elapsed += accepted_interval;
            self.time_seconds = start_time + elapsed;
            self.resolve_world_contacts()?;
        }
        self.time_seconds = start_time;
        Ok(())
    }

    fn advance_vehicle_forces(world: &PhysicalCollisionWorld, settings: &CoupledVehicleWorldConfiguration,
        vehicle: &mut CoupledWorldVehicle, duration: f64) -> Result<(), String> {
        let mut input = CoupledSuspensionInput::default();
        input.steering_rack_metres = vehicle.last_force_input.steering_rack_metres;
        let target_rack = vehicle.input.steering * settings.steering_rack_travel_metres;
        let maximum_rack_change = vehicle.model.configuration.steering_speed * settings.steering_rack_travel_metres * duration;
        input.steering_rack_velocity_metres_per_second = (target_rack - input.steering_rack_metres).clamp(-maximum_rack_change, maximum_rack_change) / duration;
        let geometry = vehicle.model.configuration.geometric_suspension.as_ref().unwrap();
        let rotation = vehicle.state.suspension.body_orientation_world.to_mat3();
        let mut samples = [TriRaycastSample::default(); 4];
        let mut forwards = [Vec3::FORWARD; 4];
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let solution = solve_corner(geometry.corners.get(wheel), vehicle.state.suspension.wheel_travel_metres[index],
                input.steering_rack_metres, wheel.is_front(),
                if wheel.is_front() { vehicle.model.configuration.front_camber } else { vehicle.model.configuration.rear_camber },
                if wheel.is_front() { vehicle.model.configuration.front_toe } else { vehicle.model.configuration.rear_toe },
                if wheel.is_left() { 1.0 } else { -1.0 }).ok_or("Coupled wheel trajectory failed")?;
            if !solution.converged || solution.steering_clamped || solution.rocker_clamped { return Err("Coupled wheel trajectory is outside its supported domain".into()); }
            forwards[index] = rotation.transform_vector(solution.wheel_basis.transform_vector(Vec3::FORWARD));
            let side = if wheel.is_left() { 1.0 } else { -1.0 };
            let static_camber = if wheel.is_front() { vehicle.model.configuration.front_camber } else { vehicle.model.configuration.rear_camber };
            let kinematic_camber = static_camber + side * solution.upright_basis.x.y.clamp(-1.0, 1.0).asin();
            let suspension = &mut vehicle.systems.state.suspension.wheels[index];
            suspension.geometric_camber_rad = kinematic_camber;
            suspension.kinematic_camber_rad = kinematic_camber;
            suspension.dynamic_camber = kinematic_camber;
            suspension.effective_contact_camber_rad = kinematic_camber;
            suspension.geometric_toe_rad = side * solution.wheel_basis.z.x.clamp(-1.0, 1.0).asin();
            suspension.hub_lateral_m = solution.hub.x - geometry.corners.get(wheel).hub_center.x;
            suspension.hub_long_m = solution.hub.z - geometry.corners.get(wheel).hub_center.z;
            let hub = wheel_contact_component(&vehicle.model.configuration, wheel, vehicle.model.kinematic_input(&vehicle.state, &input))?;
            let radius = if wheel.is_front() { vehicle.model.configuration.front_tire_radius } else { vehicle.model.configuration.rear_tire_radius };
            let direction = -rotation.y;
            if let Some(hit) = world.raycast_driveable(hub.center_of_mass_world_metres + rotation.y * radius,
                direction, radius * 2.0 + settings.road_query_extension_metres)? {
                let distance = hit.distance_metres - radius;
                input.contacts[index] = Some(CoupledWheelContact { surface_point_world_metres: hit.point_world_metres,
                    surface_normal_world: hit.normal_world, surface_velocity_world_metres_per_second: Vec3::ZERO,
                    tangential_force_world_newtons: Vec3::ZERO });
                let sample = RaycastHit { is_colliding: true, distance, point: hit.point_world_metres,
                    normal: hit.normal_world, surface: surface_from_code(hit.surface_code)? };
                samples[index] = TriRaycastSample { inner: sample, center: sample, outer: sample };
            }
        }
        let support = vehicle.model.evaluate(&vehicle.state, &input, [0.0; 4])?;
        let velocity = vehicle.state.suspension.generalized_velocity;
        let body = BodyKinematics { transform: Transform3D::new(vehicle.state.suspension.body_origin_world_metres, rotation),
            orientation: vehicle.state.suspension.body_orientation_world,
            linear_velocity: Vec3::new(velocity[0], velocity[1], velocity[2]), angular_velocity: Vec3::new(velocity[3], velocity[4], velocity[5]) };
        let forward_speed = body.linear_velocity.length();
        if forward_speed > settings.contacts.maximum_supported_speed_metres_per_second
            || body.angular_velocity.length() > settings.contacts.maximum_supported_angular_speed_radians_per_second {
            return Err("Vehicle exceeded the declared continuous collision speed domain".into());
        }
        vehicle.systems.coupled_force_context = Some(CoupledVehicleForceContext {
            normal_force_newtons: support.suspension_diagnostics.normal_force_newtons,
            hub_velocity_world_metres_per_second: support.suspension_diagnostics.hub_velocity_world_metres_per_second,
            surface_velocity_world_metres_per_second: [Vec3::ZERO; 4],
            wheel_travel_metres: vehicle.state.suspension.wheel_travel_metres,
            wheel_travel_velocity_metres_per_second: [velocity[6], velocity[7], velocity[8], velocity[9]], wheel_forward_world: forwards });
        vehicle.systems.aids = crate::simulation::AidsMask::from_bits(vehicle.driving_aids_mask);
        for wheel in 0..4 { vehicle.systems.state.tires.wheels[wheel].spin = -vehicle.state.wheel_angular_velocity_radians_per_second[wheel]; }
        let mut clearance = [0.0; 5];
        let mut valid_mask = 0;
        for (index, probe) in settings.aerodynamic_probe_positions_local_metres.iter().enumerate() {
            let origin = body.transform.origin + rotation.transform_vector(*probe);
            if let Some(hit) = world.raycast_driveable(origin, -rotation.y, settings.aerodynamic_probe_length_metres)? {
                clearance[index] = hit.distance_metres;
                valid_mask |= 1 << index;
            }
        }
        let environment = AeroEnvironment { clearance_m: clearance, valid_mask,
            rake_rad: 0.0, roll_rad: 0.0, bottoming_mask: 0, contact_confidence: 1.0 };
        let (external, _) = vehicle.systems.solve_external_with_aero(body, &vehicle.input, &samples, &environment, duration);
        input.body_force_world_newtons = external.force_world;
        let center_world_offset = rotation.transform_vector(crate::simulation::center_of_mass_local(&vehicle.systems.config));
        input.body_torque_about_origin_world_newton_metres = external.torque_world + center_world_offset.cross(external.force_world);
        let mut torques = [0.0; 4];
        let mut brake_torque_limits = [0.0; 4];
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let spin = vehicle.state.wheel_angular_velocity_radians_per_second[index];
            let brake = vehicle.systems.state.powertrain.brake_torques[index];
            torques[index] = -vehicle.systems.state.powertrain.drive_torques[index];
            brake_torque_limits[index] = brake;
            if let Some(contact) = &mut input.contacts[index] {
                let normal = contact.surface_normal_world;
                let forward = (forwards[index] - normal * forwards[index].dot(normal)).normalized();
                let right = forward.cross(normal).normalized();
                let tire = &vehicle.systems.state.tires.wheels[index];
                contact.tangential_force_world_newtons = right * tire.lateral_force + forward * tire.longitudinal_force;
                input.body_torque_about_origin_world_newton_metres += normal * tire.aligning_torque;
            }
            vehicle.systems.state.tires.wheels[index].spin = -spin;
        }
        vehicle.model.update_fuel_mass(&mut vehicle.state, &input, vehicle.systems.config.fuel.effective_current_kg())?;
        vehicle.model.advance(&mut vehicle.state, &input, torques, duration, settings.contacts.maximum_events_per_step)?;
        input.steering_rack_metres += input.steering_rack_velocity_metres_per_second * duration;
        let wheel_spin_before_braking = vehicle.state.wheel_angular_velocity_radians_per_second.map(|spin| -spin);
        let mut dissipated_brake_energy_joules = [0.0; 4];
        let mut applied_brake_torque_newton_metres = [0.0; 4];
        if brake_torque_limits.iter().any(|torque| *torque > 0.0) {
            let evaluation = vehicle.model.evaluate(&vehicle.state, &input, [0.0; 4])?;
            let mut velocity = vehicle.state.velocity();
            let unconstrained_velocity = velocity;
            let result = crate::coupled_vehicle_dynamics::apply_generalized_wheel_braking(&evaluation.mass_matrix, &mut velocity,
                brake_torque_limits, duration, settings.contacts.maximum_iterations,
                settings.contacts.velocity_tolerance_metres_per_second / vehicle.model.configuration.front_tire_radius.min(vehicle.model.configuration.rear_tire_radius))?;
            let velocity_change = std::array::from_fn(|coordinate| velocity[coordinate] - unconstrained_velocity[coordinate]);
            vehicle.state.suspension.steering_actuator_work_joules += crate::coupled_vehicle_dynamics::generalized_projection(
                evaluation.prescribed_generalized_momentum, velocity_change);
            vehicle.state.brake_dissipated_energy_joules += result.dissipated_energy_joules;
            dissipated_brake_energy_joules = result.dissipated_energy_joules_by_wheel.map(|energy| energy.max(0.0));
            applied_brake_torque_newton_metres = result.impulses_newton_metre_seconds.map(|impulse| impulse.abs() / duration);
            vehicle.state.set_velocity(velocity);
        }
        vehicle.last_force_input = input;
        Self::synchronize_presentation(vehicle);
        vehicle.systems.finish_coupled_thermal_interval(dissipated_brake_energy_joules,
            applied_brake_torque_newton_metres, wheel_spin_before_braking)?;
        vehicle.systems.state.linear_acceleration = (vehicle.systems.state.linear_velocity - body.linear_velocity) / duration;
        vehicle.systems.state.angular_acceleration = (vehicle.systems.state.angular_velocity - body.angular_velocity) / duration;
        Ok(())
    }

    fn synchronize_presentation(vehicle: &mut CoupledWorldVehicle) {
        let state = &vehicle.state.suspension;
        let rotation = state.body_orientation_world.to_mat3();
        let velocity = state.generalized_velocity;
        vehicle.systems.state.transform = Transform3D::new(state.body_origin_world_metres, rotation);
        vehicle.systems.state.orientation = state.body_orientation_world;
        vehicle.systems.state.linear_velocity = Vec3::new(velocity[0], velocity[1], velocity[2]);
        vehicle.systems.state.angular_velocity = Vec3::new(velocity[3], velocity[4], velocity[5]);
        vehicle.systems.state.sim_time = state.simulated_time_seconds;
        for wheel in 0..4 {
            vehicle.systems.state.tires.wheels[wheel].spin = -vehicle.state.wheel_angular_velocity_radians_per_second[wheel];
        }
    }

    fn attached_shapes(&self) -> Result<Vec<AttachedVehicleShape>, String> {
        let mut shapes = Vec::new();
        for (entity_index, vehicle) in self.vehicles.values().enumerate() {
            let rotation = vehicle.state.suspension.body_orientation_world.to_mat3();
            for definition in &self.configuration.body_collision_boxes {
                shapes.push(AttachedVehicleShape { entity_index, identifier: definition.identifier.clone(), wheel: None,
                    pose: physical_pose(vehicle.state.suspension.body_origin_world_metres + rotation.transform_vector(definition.center_local_metres), rotation),
                    shape: SharedShape::cuboid(definition.half_extents_metres.x, definition.half_extents_metres.y, definition.half_extents_metres.z) });
            }
            let geometry = vehicle.model.configuration.geometric_suspension.as_ref().unwrap();
            for wheel in WheelIndex::ALL {
                let solution = solve_corner(geometry.corners.get(wheel), vehicle.state.suspension.wheel_travel_metres[wheel as usize],
                    vehicle.last_force_input.steering_rack_metres, wheel.is_front(),
                    if wheel.is_front() { vehicle.model.configuration.front_camber } else { vehicle.model.configuration.rear_camber },
                    if wheel.is_front() { vehicle.model.configuration.front_toe } else { vehicle.model.configuration.rear_toe },
                    if wheel.is_left() { 1.0 } else { -1.0 }).ok_or("Collision wheel trajectory failed")?;
                let local = solution.wheel_basis;
                let basis = Mat3::from_cols(rotation.transform_vector(local.y), rotation.transform_vector(local.x), -rotation.transform_vector(local.z));
                let radius = if wheel.is_front() { vehicle.model.configuration.front_tire_radius } else { vehicle.model.configuration.rear_tire_radius };
                let width = if wheel.is_front() { vehicle.model.configuration.front_tire_width } else { vehicle.model.configuration.rear_tire_width };
                shapes.push(AttachedVehicleShape { entity_index, identifier: format!("wheel_{wheel:?}"), wheel: Some(wheel),
                    pose: physical_pose(vehicle.state.suspension.body_origin_world_metres + rotation.transform_vector(solution.hub), basis), shape: SharedShape::cylinder(width * 0.5, radius) });
            }
        }
        Ok(shapes)
    }

    fn resolve_world_contacts(&mut self) -> Result<(), String> {
        let shapes = self.attached_shapes()?;
        let shape_bounds = shapes.iter().map(|shape| shape.shape.compute_aabb(&shape.pose)
            .loosened(self.configuration.contacts.prediction_distance_metres)).collect::<Vec<_>>();
        let vehicles = self.vehicles.values().collect::<Vec<_>>();
        let entity_identifiers = self.vehicles.keys().copied().collect::<Vec<_>>();
        let mut indexed_contacts = Vec::new();
        for shape in &shapes {
            let vehicle = vehicles[shape.entity_index];
            for detected in self.collision_world.contacts(&shape.pose, &shape.shape, self.configuration.contacts.prediction_distance_metres, shape.wheel.is_some())? {
                if -detected.signed_distance_metres > self.configuration.maximum_initial_overlap_metres { return Err("Vehicle/world overlap exceeds the supported initialization/recovery domain".into()); }
                let axes = contact_axes(detected.normal_toward_vehicle_world);
                let prescribed_velocity = vehicle.model.prescribed_contact_velocity(&vehicle.state, &vehicle.last_force_input,
                    shape.wheel, detected.point_on_vehicle_metres)?;
                let mut jacobians = [[0.0; 14]; 3];
                for axis in 0..3 { jacobians[axis] = vehicle.model.contact_jacobian(&vehicle.state, &vehicle.last_force_input,
                    shape.wheel, detected.point_on_vehicle_metres, axes[axis])?; }
                indexed_contacts.push((GeneralizedContactConstraint { identifier: format!("world_{:08}_vehicle_{:020}_{}", detected.shape_index, entity_identifiers[shape.entity_index], shape.identifier),
                    first_entity: shape.entity_index, second_entity: None, first_jacobians: jacobians, second_jacobians: [[0.0; 14]; 3],
                    first_prescribed_velocity_metres_per_second: axes.map(|axis| axis.dot(prescribed_velocity)),
                    second_prescribed_velocity_metres_per_second: [0.0; 3],
                    normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] },
                    ContactMetadata { point_world_metres: detected.point_on_vehicle_metres, normal_world: axes[0], surface_code: detected.surface_code,
                        penetration_metres: (-detected.signed_distance_metres).max(0.0) }));
            }
        }
        for first in 0..shapes.len() { for second in first + 1..shapes.len() {
            let first_shape = &shapes[first]; let second_shape = &shapes[second];
            if first_shape.entity_index == second_shape.entity_index { continue; }
            if !shape_bounds[first].intersects(&shape_bounds[second]) { continue; }
            let detected = contact(&first_shape.pose, first_shape.shape.as_ref(), &second_shape.pose, second_shape.shape.as_ref(),
                self.configuration.contacts.prediction_distance_metres).map_err(|_| "Unsupported vehicle collision shapes")?;
            if let Some(detected) = detected {
                let normal = -vehicle_vector(*detected.normal1);
                let axes = contact_axes(normal);
                let first_vehicle = vehicles[first_shape.entity_index]; let second_vehicle = vehicles[second_shape.entity_index];
                let first_prescribed_velocity = first_vehicle.model.prescribed_contact_velocity(&first_vehicle.state,
                    &first_vehicle.last_force_input, first_shape.wheel, vehicle_vector(detected.point1.coords))?;
                let second_prescribed_velocity = second_vehicle.model.prescribed_contact_velocity(&second_vehicle.state,
                    &second_vehicle.last_force_input, second_shape.wheel, vehicle_vector(detected.point2.coords))?;
                let mut first_jacobians = [[0.0; 14]; 3]; let mut second_jacobians = [[0.0; 14]; 3];
                for axis in 0..3 {
                    first_jacobians[axis] = first_vehicle.model.contact_jacobian(&first_vehicle.state, &first_vehicle.last_force_input,
                        first_shape.wheel, vehicle_vector(detected.point1.coords), axes[axis])?;
                    second_jacobians[axis] = second_vehicle.model.contact_jacobian(&second_vehicle.state, &second_vehicle.last_force_input,
                        second_shape.wheel, vehicle_vector(detected.point2.coords), -axes[axis])?;
                }
                indexed_contacts.push((GeneralizedContactConstraint { identifier: format!("pair_{:020}_{:020}_{}_{}", entity_identifiers[first_shape.entity_index], entity_identifiers[second_shape.entity_index], first_shape.identifier, second_shape.identifier),
                    first_entity: first_shape.entity_index, second_entity: Some(second_shape.entity_index), first_jacobians, second_jacobians,
                    first_prescribed_velocity_metres_per_second: axes.map(|axis| axis.dot(first_prescribed_velocity)),
                    second_prescribed_velocity_metres_per_second: axes.map(|axis| (-axis).dot(second_prescribed_velocity)),
                    normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] },
                    ContactMetadata { point_world_metres: vehicle_vector(detected.point1.coords), normal_world: normal, surface_code: 6,
                        penetration_metres: (-detected.dist).max(0.0) }));
            }
        } }
        indexed_contacts.sort_by(|first, second| first.0.identifier.cmp(&second.0.identifier));
        let (mut constraints, metadata): (Vec<_>, Vec<_>) = indexed_contacts.into_iter().unzip();
        if constraints.is_empty() { return Ok(()); }
        let evaluations = vehicles.iter().map(|vehicle| vehicle.model.evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])).collect::<Result<Vec<_>, _>>()?;
        let matrices = evaluations.iter().map(|evaluation| evaluation.mass_matrix).collect::<Vec<_>>();
        let mut velocities = vehicles.iter().map(|vehicle| vehicle.state.velocity()).collect::<Vec<_>>();
        let before = velocities.clone();
        let result = solve_generalized_vehicle_contacts(&matrices, &mut velocities, &mut constraints, &self.configuration.contacts)?;
        let identities = self.vehicles.keys().copied().collect::<Vec<_>>();
        for (index, vehicle) in self.vehicles.values_mut().enumerate() {
            let previous = crate::coupled_vehicle_dynamics::generalized_vehicle_energy(&matrices[index], before[index]);
            let current = crate::coupled_vehicle_dynamics::generalized_vehicle_energy(&matrices[index], velocities[index]);
            let velocity_change = std::array::from_fn(|coordinate| velocities[index][coordinate] - before[index][coordinate]);
            let prescribed_inertial_work = crate::coupled_vehicle_dynamics::generalized_projection(
                evaluations[index].prescribed_generalized_momentum, velocity_change);
            let prescribed_work = result.prescribed_actuator_work_joules_by_entity[index];
            vehicle.state.suspension.collision_energy_change_joules += current - previous - prescribed_work;
            vehicle.state.suspension.steering_actuator_work_joules += prescribed_inertial_work + prescribed_work;
            vehicle.state.set_velocity(velocities[index]);
            Self::synchronize_presentation(vehicle);
        }
        let factors = matrices.iter().map(crate::coupled_vehicle_dynamics::factor_generalized_vehicle_matrix).collect::<Result<Vec<_>, _>>()?;
        let mut corrections = vec![[0.0; 14]; identities.len()];
        for (index, constraint) in constraints.iter().enumerate() {
            let penetration = metadata[index].penetration_metres;
            if penetration <= self.configuration.contacts.prediction_distance_metres { continue; }
            let first_response = crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(&factors[constraint.first_entity], constraint.first_jacobians[0])?;
            let mut inverse_mass = crate::coupled_vehicle_dynamics::generalized_projection(constraint.first_jacobians[0], first_response);
            let second_response = if let Some(entity) = constraint.second_entity {
                let response = crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(&factors[entity], constraint.second_jacobians[0])?;
                inverse_mass += crate::coupled_vehicle_dynamics::generalized_projection(constraint.second_jacobians[0], response);
                Some(response)
            } else { None };
            let applied_correction = crate::coupled_vehicle_dynamics::generalized_projection(constraint.first_jacobians[0], corrections[constraint.first_entity])
                + constraint.second_entity.map_or(0.0, |entity| crate::coupled_vehicle_dynamics::generalized_projection(constraint.second_jacobians[0], corrections[entity]));
            let correction = (penetration - self.configuration.contacts.prediction_distance_metres - applied_correction)
                .clamp(0.0, self.configuration.contacts.maximum_position_correction_metres) / inverse_mass;
            for coordinate in 0..14 {
                corrections[constraint.first_entity][coordinate] += first_response[coordinate] * correction;
                if let (Some(entity), Some(response)) = (constraint.second_entity, second_response) { corrections[entity][coordinate] += response[coordinate] * correction; }
            }
        }
        for (index, vehicle) in self.vehicles.values_mut().enumerate() {
            if corrections[index].iter().all(|value| *value == 0.0) { continue; }
            let previous = vehicle.model.evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])?;
            let correction = corrections[index];
            vehicle.state.suspension.body_origin_world_metres += Vec3::new(correction[0], correction[1], correction[2]);
            let angular_local = vehicle.state.suspension.body_orientation_world.to_mat3().inverse_transform_vector(Vec3::new(correction[3], correction[4], correction[5]));
            vehicle.state.suspension.body_orientation_world = vehicle.state.suspension.body_orientation_world.integrate_angular_velocity(angular_local, 1.0).normalized();
            for wheel in 0..4 {
                let (lower, upper) = vehicle.model.travel_limits_metres[wheel];
                vehicle.state.suspension.wheel_travel_metres[wheel] = (vehicle.state.suspension.wheel_travel_metres[wheel] + correction[6 + wheel]).clamp(lower, upper);
                vehicle.state.wheel_spin_angle_radians[wheel] = (vehicle.state.wheel_spin_angle_radians[wheel] + correction[10 + wheel]).rem_euclid(std::f64::consts::TAU);
            }
            let current = vehicle.model.evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])?;
            let energy = |evaluation: &crate::coupled_vehicle_dynamics::CoupledVehicleEvaluation| evaluation.kinetic_energy_joules
                + evaluation.suspension_diagnostics.gravitational_potential_energy_joules + evaluation.suspension_diagnostics.elastic_potential_energy_joules;
            vehicle.state.position_correction_energy_joules += energy(&current) - energy(&previous);
            Self::synchronize_presentation(vehicle);
        }
        for (index, constraint) in constraints.iter().enumerate() {
            if result.normal_impulses_newton_seconds[index] <= 0.0 { continue; }
            self.collision_events.push(CoupledWorldCollisionEvent { contact_identifier: constraint.identifier.clone(),
                time_seconds: self.time_seconds, first_entity_identifier: identities[constraint.first_entity],
                second_entity_identifier: constraint.second_entity.map(|entity| identities[entity]),
                point_world_metres: metadata[index].point_world_metres, normal_world: metadata[index].normal_world,
                normal_impulse_newton_seconds: result.normal_impulses_newton_seconds[index],
                tangential_impulse_newton_seconds: result.tangential_impulses_newton_seconds[index], surface_code: metadata[index].surface_code });
        }
        Ok(())
    }

    fn earliest_swept_impact(&self, previous_shapes: &[AttachedVehicleShape], duration: f64) -> Result<Option<f64>, String> {
        let current_shapes = self.attached_shapes()?;
        if previous_shapes.len() != current_shapes.len() { return Err("Shape attachments changed during a physical step".into()); }
        let mut motions = Vec::new();
        let mut swept_bounds = Vec::new();
        for (previous, current) in previous_shapes.iter().zip(current_shapes.iter()) {
            let linear_velocity = (current.pose.translation.vector - previous.pose.translation.vector) / duration;
            let relative_rotation = current.pose.rotation * previous.pose.rotation.inverse();
            let angular_velocity = relative_rotation.scaled_axis() / duration;
            motions.push(NonlinearRigidMotion::new(previous.pose, Point3::origin(), linear_velocity, angular_velocity));
            let local_bounds = previous.shape.compute_local_aabb();
            let radius = local_bounds.mins.coords.norm().max(local_bounds.maxs.coords.norm());
            let previous_center = Point3::from(previous.pose.translation.vector);
            let current_center = Point3::from(current.pose.translation.vector);
            let extent = parry3d_f64::na::Vector3::repeat(radius + self.configuration.contacts.prediction_distance_metres);
            swept_bounds.push(Aabb::new(previous_center.inf(&current_center) - extent, previous_center.sup(&current_center) + extent));
        }
        let mut earliest: Option<f64> = None;
        for (index, shape) in previous_shapes.iter().enumerate() {
            for terrain in &self.collision_world.shapes {
                if !swept_bounds[index].intersects(&terrain.shape.compute_aabb(&Isometry3::identity())) { continue; }
                let collision_shape = if shape.wheel.is_some() && terrain.driveable {
                    let Some(impact_shape) = &terrain.wheel_impact_shape else { continue; };
                    impact_shape
                } else { &terrain.shape };
                let result = cast_shapes_nonlinear(&NonlinearRigidMotion::identity(), collision_shape.as_ref(),
                    &motions[index], shape.shape.as_ref(), 0.0, duration, false).map_err(|_| "Unsupported rotating world sweep")?;
                if let Some(hit) = result {
                    if hit.time_of_impact > 1e-10 && earliest.is_none_or(|previous| hit.time_of_impact < previous) { earliest = Some(hit.time_of_impact); }
                }
            }
        }
        for first in 0..previous_shapes.len() { for second in first + 1..previous_shapes.len() {
            let first_shape = &previous_shapes[first]; let second_shape = &previous_shapes[second];
            if first_shape.entity_index == second_shape.entity_index { continue; }
            if !swept_bounds[first].intersects(&swept_bounds[second]) { continue; }
            if let Some(hit) = cast_shapes_nonlinear(&motions[first], first_shape.shape.as_ref(), &motions[second], second_shape.shape.as_ref(),
                0.0, duration, false).map_err(|_| "Unsupported rotating vehicle sweep")? {
                if hit.time_of_impact > 1e-10 && earliest.is_none_or(|previous| hit.time_of_impact < previous) { earliest = Some(hit.time_of_impact); }
            }
        } }
        Ok(earliest)
    }
}
