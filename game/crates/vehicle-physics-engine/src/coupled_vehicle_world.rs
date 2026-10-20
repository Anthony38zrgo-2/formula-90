use crate::aero::AeroEnvironment;
use crate::coupled_vehicle_dynamics::{CoupledVehicleModel, CoupledVehicleState, GeneralizedVehicleVelocity};
use crate::dynamic_world_objects::{
    DynamicWorldObject, DynamicWorldObjectDefinition, DynamicWorldObjectSnapshot,
    DYNAMIC_WORLD_OBJECT_SLEEP_ANGULAR_SPEED_RADIANS_PER_SECOND,
    DYNAMIC_WORLD_OBJECT_SLEEP_DELAY_SECONDS,
    DYNAMIC_WORLD_OBJECT_SLEEP_LINEAR_SPEED_METRES_PER_SECOND,
    DYNAMIC_WORLD_OBJECT_SUPPORT_NORMAL_MINIMUM_Y,
    DYNAMIC_WORLD_OBJECT_WAKE_RELATIVE_SPEED_METRES_PER_SECOND,
};
use crate::generalized_vehicle_contacts::{solve_generalized_vehicle_contacts_with_fixed_entities, GeneralizedContactConstraint, GeneralizedContactSettings};
use crate::physical_collision_world::{physical_pose, shape_pair_contact_manifolds, PhysicalCollisionWorld};
use crate::simulation::{BodyKinematics, CoupledVehicleForceContext, VehicleSimulator};
use crate::suspension_component_kinematics::wheel_contact_component;
use crate::suspension_coupled_solver::{CoupledSuspensionInput, CoupledSuspensionSettings, CoupledWheelContact};
use crate::suspension_kinematics_table::SuspensionKinematicsTable;
use crate::telemetry::TelemetryFrame;
use crate::types::{Mat3, Quat, RaycastHit, SurfaceType, Transform3D, TriRaycastSample, Vec3, VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use crate::vehicle_dynamics_fidelity::VehicleDynamicsFidelity;
use parry3d_f64::na::{Isometry3, Point3};
use parry3d_f64::bounding_volume::{Aabb, BoundingVolume};
use parry3d_f64::query::{cast_shapes_nonlinear, NonlinearRigidMotion};
use parry3d_f64::shape::SharedShape;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

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
    pub physical_frequency_hertz: u32,
    pub dynamics_fidelity: VehicleDynamicsFidelity,
    pub spawn_position_world_metres: Vec3,
    pub spawn_yaw_radians: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledWorldSnapshot {
    pub entity_identifier: u64,
    pub physical_frequency_hertz: u32,
    #[serde(default)]
    pub dynamics_fidelity: VehicleDynamicsFidelity,
    pub state: CoupledVehicleState,
    pub telemetry: TelemetryFrame,
    pub systems_state: crate::simulation::VehicleState,
    pub steering_rack_metres: f64,
    pub fuel_mass_kilograms: f64,
    pub driver_input: VehicleInput,
    pub driving_aids_mask: u32,
    pub operating_mass_kilograms: f64,
    pub solved_suspension_corners: Vec<crate::suspension_kinematics::KinematicSolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactResponseOwner {
    TireTreadSupport,
    RigidImpact,
    InternalTravelStop,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContactClassificationDiagnostics {
    pub tire_tread_supports: u64,
    pub rigid_impact_contacts: u64,
    pub travel_stop_wheels: u64,
    pub rejected_steep_tire_hits: u64,
    pub duplicate_response_preventions: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContactIslandDiagnostics {
    pub last_island_count: usize,
    pub last_largest_island_entity_count: usize,
    pub last_largest_island_constraint_count: usize,
    pub last_constraint_count: usize,
    pub last_solver_iterations: usize,
    pub last_static_friction_constraint_count: usize,
    pub last_sliding_constraint_count: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DynamicWorldObjectDiagnostics {
    pub registered_total: u64,
    pub removed_total: u64,
    pub reset_total: u64,
    pub rejected_unsupported_class_total: u64,
    pub sleeping_object_count: usize,
    pub sleep_transition_total: u64,
    pub wake_transition_total: u64,
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
    pub manifold_age_steps: u64,
    pub response_owner: ContactResponseOwner,
}

pub const CONTACT_MANIFOLD_RETENTION_STEPS: u64 = 4;
pub const CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL: usize = 3;
pub const CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_SHAPE_PAIR: usize = 4;
pub const CONTACT_MANIFOLD_MINIMUM_CONTACT_SPACING_METRES: f64 = 0.02;
pub const CONTACT_MANIFOLD_MINIMUM_CONTACT_TRIANGLE_AREA_SQUARE_METRES: f64 = 0.0001;
pub const CONTACT_MANIFOLD_SEAM_PROXIMITY_METRES: f64 = 0.01;
pub const CONTACT_MANIFOLD_SEAM_NORMAL_ALIGNMENT: f64 = 0.5;
pub const CONTACT_MANIFOLD_WARM_START_MINIMUM_ALIGNMENT: f64 = 0.7;
pub const CONTACT_MANIFOLD_WARM_START_SCALE: f64 = 0.8;
pub const CONTACT_MANIFOLD_RESTING_WARM_START_SCALE: f64 = 1.0;
pub const CONTACT_MANIFOLD_RESTING_STEPS_BEFORE_FULL_WARM_START: u64 = 2;
pub const CONTACT_MANIFOLD_RESTING_NORMAL_SPEED_METRES_PER_SECOND: f64 = 0.01;
pub const CONTACT_MANIFOLD_RESTING_TANGENTIAL_SPEED_METRES_PER_SECOND: f64 = 0.01;
pub const CONTINUOUS_COLLISION_MINIMUM_SWEEP_SPEED_METRES_PER_SECOND: f64 = 0.1;
pub const CONTINUOUS_COLLISION_MINIMUM_SWEEP_ANGULAR_SPEED_RADIANS_PER_SECOND: f64 = 0.2;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContactFeatureKey {
    pub first_entity_identifier: u64,
    pub second_entity_identifier: Option<u64>,
    pub world_shape_index: Option<usize>,
    pub first_shape_identifier: String,
    pub second_shape_identifier: String,
    pub first_feature_identifier: u32,
    pub second_feature_identifier: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactManifoldEntry {
    pub normal_world: Vec3,
    pub tangent_world: [Vec3; 2],
    pub point_world_metres: Vec3,
    pub first_local_point_metres: Vec3,
    pub second_local_point_metres: Vec3,
    pub separation_metres: f64,
    pub surface_code: u32,
    pub created_step: u64,
    pub age_steps: u64,
    pub last_seen_step: u64,
    pub normal_impulse_newton_seconds: f64,
    pub tangential_impulse_newton_seconds: [f64; 2],
    pub resting_steps: u64,
}

impl ContactManifoldEntry {
    fn new(step: u64) -> Self {
        Self { normal_world: Vec3::ZERO, tangent_world: [Vec3::RIGHT, Vec3::BACK], point_world_metres: Vec3::ZERO,
            first_local_point_metres: Vec3::ZERO, second_local_point_metres: Vec3::ZERO, separation_metres: 0.0,
            surface_code: 0, created_step: step, age_steps: 0, last_seen_step: step,
            normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2], resting_steps: 0 }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContactManifoldDiagnostics {
    pub active_count: usize,
    pub retained_count: usize,
    pub created_total: u64,
    pub warm_started_total: u64,
    pub removed_unseen_total: u64,
    pub removed_reset_total: u64,
    pub removed_entity_total: u64,
    pub removed_normal_rotation_total: u64,
    pub reduced_contact_total: u64,
    pub maximum_age_steps: u64,
    pub resting_contact_count: usize,
    pub full_warm_started_total: u64,
    pub tangential_position_correction_total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactManifoldRefreshSummary {
    pub generated_contact_count: usize,
    pub reduced_contact_count: usize,
    pub warm_started_contact_count: u64,
    pub active_contact_count: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContinuousCollisionDiagnostics {
    pub total_impact_events: u64,
    pub total_event_subdivisions: u64,
    pub maximum_event_subdivisions: usize,
    pub last_impact_time_seconds: Option<f64>,
    pub last_impact_shape_identifier: Option<String>,
    pub last_impact_entity_identifier: Option<u64>,
    pub last_event_interval_seconds: f64,
    pub geometric_miss_count: u64,
}

struct SweptImpact {
    time_seconds: f64,
    shape_identifier: String,
    entity_identifier: u64,
}

#[derive(Debug, Clone)]
pub struct CoupledVehicleWorld {
    pub collision_world: PhysicalCollisionWorld,
    pub configuration: CoupledVehicleWorldConfiguration,
    pub vehicles: BTreeMap<u64, CoupledWorldVehicle>,
    pub dynamic_objects: BTreeMap<u64, DynamicWorldObject>,
    terrain_bounds: Vec<Aabb>,
    pub time_seconds: f64,
    pub accumulated_host_time_seconds: f64,
    pub paused: bool,
    pub collision_events: Vec<CoupledWorldCollisionEvent>,
    pub contact_manifolds: BTreeMap<ContactFeatureKey, ContactManifoldEntry>,
    pub contact_manifold_diagnostics: ContactManifoldDiagnostics,
    pub contact_classification_diagnostics: ContactClassificationDiagnostics,
    pub contact_island_diagnostics: ContactIslandDiagnostics,
    pub dynamic_object_diagnostics: DynamicWorldObjectDiagnostics,
    pub continuous_collision_diagnostics: ContinuousCollisionDiagnostics,
    pub step_profile: WorldStepProfile,
    pub physical_step_index: u64,
    pub physical_interval_index: u64,
    simplified_suspension_kinematics: Option<(u64, Arc<SuspensionKinematicsTable>)>,
    next_entity_identifier: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct WorldStepProfile {
    pub total_seconds: f64,
    pub shape_attachment_seconds: f64,
    pub vehicle_force_seconds: f64,
    pub dynamic_object_seconds: f64,
    pub continuous_collision_seconds: f64,
    pub contact_generation_seconds: f64,
    pub contact_solve_seconds: f64,
    pub contact_correction_seconds: f64,
    pub contact_finalize_seconds: f64,
    pub ai_phase_seconds: f64,
    pub player_phase_seconds: f64,
    pub last_ai_phase_seconds: f64,
    pub last_player_phase_seconds: f64,
    pub ai_phase_count: u64,
    pub player_phase_count: u64,
    pub physical_step_count: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorldContactPhase {
    Player,
    Ai,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorldEntityKind {
    PlayerVehicle,
    AiVehicle,
    DynamicObject,
}

struct AttachedWorldShape {
    entity_index: usize,
    kind: WorldEntityKind,
    identifier: String,
    wheel: Option<WheelIndex>,
    pose: Isometry3<f64>,
    shape: SharedShape,
    bounds: Aabb,
}

struct ContactMetadata {
    point_world_metres: Vec3,
    normal_world: Vec3,
    surface_code: u32,
    penetration_metres: f64,
    separation_metres: f64,
}

struct GeneratedContact {
    constraint: GeneralizedContactConstraint,
    metadata: ContactMetadata,
    key: ContactFeatureKey,
    first_local_point_metres: Vec3,
    second_local_point_metres: Vec3,
    first_shape_identifier: String,
    second_shape_identifier: String,
}

fn contact_axes(normal: Vec3) -> [Vec3; 3] {
    let reference = if normal.dot(Vec3::UP).abs() < 0.9 { Vec3::UP } else { Vec3::RIGHT };
    let tangent = normal.cross(reference).normalized();
    [normal, tangent, normal.cross(tangent)]
}

fn phase_allows_terrain(phase: WorldContactPhase, kind: WorldEntityKind) -> bool {
    match phase {
        WorldContactPhase::Player => matches!(kind, WorldEntityKind::PlayerVehicle | WorldEntityKind::DynamicObject),
        WorldContactPhase::Ai => matches!(kind, WorldEntityKind::AiVehicle),
    }
}

fn phase_allows_pair(phase: WorldContactPhase, first: WorldEntityKind, second: WorldEntityKind) -> bool {
    let has_player = matches!(first, WorldEntityKind::PlayerVehicle)
        || matches!(second, WorldEntityKind::PlayerVehicle);
    let has_ai = matches!(first, WorldEntityKind::AiVehicle)
        || matches!(second, WorldEntityKind::AiVehicle);
    let both_objects = matches!(first, WorldEntityKind::DynamicObject)
        && matches!(second, WorldEntityKind::DynamicObject);
    match phase {
        WorldContactPhase::Player => has_player || both_objects,
        WorldContactPhase::Ai => has_ai && !has_player,
    }
}

fn entity_prescribed_contact_velocity(vehicles: &[&CoupledWorldVehicle], dynamic_objects: &[&DynamicWorldObject],
    entity_index: usize, wheel: Option<WheelIndex>, point_world_metres: Vec3) -> Result<Vec3, String> {
    if let Some(vehicle) = vehicles.get(entity_index) {
        return vehicle.model.prescribed_contact_velocity(&vehicle.state, &vehicle.last_force_input,
            wheel, point_world_metres);
    }
    dynamic_objects.get(entity_index.saturating_sub(vehicles.len()))
        .ok_or_else(|| "Contact references an unknown world entity".to_string())?;
    Ok(Vec3::ZERO)
}

fn entity_contact_jacobian(vehicles: &[&CoupledWorldVehicle], dynamic_objects: &[&DynamicWorldObject],
    entity_index: usize, wheel: Option<WheelIndex>, point_world_metres: Vec3,
    axis_world: Vec3) -> Result<[f64; 14], String> {
    if let Some(vehicle) = vehicles.get(entity_index) {
        return vehicle.model.contact_jacobian(&vehicle.state, &vehicle.last_force_input,
            wheel, point_world_metres, axis_world);
    }
    let object = dynamic_objects.get(entity_index.saturating_sub(vehicles.len()))
        .ok_or("Contact references an unknown dynamic world object")?;
    Ok(object.contact_jacobian(point_world_metres, axis_world))
}

fn entity_surface_code(vehicles: &[&CoupledWorldVehicle], dynamic_objects: &[&DynamicWorldObject],
    entity_index: usize) -> Option<u32> {
    if entity_index < vehicles.len() {
        return None;
    }
    dynamic_objects.get(entity_index.saturating_sub(vehicles.len()))
        .map(|object| object.definition.surface_code)
}

fn contact_island_summary(constraints: &[GeneralizedContactConstraint], entity_count: usize) -> (usize, usize, usize) {
    if constraints.is_empty() || entity_count == 0 {
        return (0, 0, 0);
    }
    let mut parent = (0..entity_count).collect::<Vec<_>>();
    fn root(parent: &mut [usize], index: usize) -> usize {
        if parent[index] != index {
            let resolved = root(parent, parent[index]);
            parent[index] = resolved;
        }
        parent[index]
    }
    for constraint in constraints {
        let first = root(&mut parent, constraint.first_entity);
        if let Some(second_entity) = constraint.second_entity {
            let second = root(&mut parent, second_entity);
            if first != second {
                parent[second] = first;
            }
        }
    }
    let mut entity_counts = vec![0_usize; entity_count];
    let mut constraint_counts = vec![0_usize; entity_count];
    let mut appeared = vec![false; entity_count];
    for constraint in constraints {
        appeared[constraint.first_entity] = true;
        if let Some(second_entity) = constraint.second_entity {
            appeared[second_entity] = true;
        }
        let island = root(&mut parent, constraint.first_entity);
        constraint_counts[island] += 1;
    }
    for index in 0..entity_count {
        if appeared[index] {
            let island = root(&mut parent, index);
            entity_counts[island] += 1;
        }
    }
    let islands = (0..entity_count).filter(|index| entity_counts[*index] > 0).collect::<Vec<_>>();
    let largest_entities = islands.iter().map(|index| entity_counts[*index]).max().unwrap_or(0);
    let largest_constraints = islands.iter().map(|index| constraint_counts[*index]).max().unwrap_or(0);
    (islands.len(), largest_entities, largest_constraints)
}

fn reduce_redundant_manifold_contacts(contacts: &mut Vec<GeneratedContact>) -> usize {    if contacts.len() <= CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL { return 0; }
    let normal_bucket = |normal: Vec3| -> [i32; 3] {
        [(normal.x * 5.0).round() as i32, (normal.y * 5.0).round() as i32, (normal.z * 5.0).round() as i32]
    };
    let mut groups: BTreeMap<(usize, Option<usize>, String, String, [i32; 3]), Vec<usize>> = BTreeMap::new();
    for (index, contact) in contacts.iter().enumerate() {
        groups.entry((contact.constraint.first_entity, contact.constraint.second_entity,
            contact.first_shape_identifier.clone(), contact.second_shape_identifier.clone(),
            normal_bucket(contact.metadata.normal_world))).or_default().push(index);
    }
    let mut discarded = vec![false; contacts.len()];
    for indices in groups.values() {
        if indices.len() <= CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL { continue; }
        let mut order = indices.clone();
        order.sort_by(|first, second| contacts[*second].metadata.penetration_metres
            .partial_cmp(&contacts[*first].metadata.penetration_metres).unwrap_or(std::cmp::Ordering::Equal));
        let mut selected: Vec<usize> = Vec::new();
        if let Some(first) = order.first() { selected.push(*first); }
        if selected.len() < CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL {
            let anchor = contacts[selected[0]].metadata.point_world_metres;
            let farthest = order.iter().skip(1)
                .filter(|index| (contacts[**index].metadata.point_world_metres - anchor).length() >= CONTACT_MANIFOLD_MINIMUM_CONTACT_SPACING_METRES)
                .max_by(|first, second| {
                    let first_distance = (contacts[**first].metadata.point_world_metres - anchor).length();
                    let second_distance = (contacts[**second].metadata.point_world_metres - anchor).length();
                    first_distance.partial_cmp(&second_distance).unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some(second) = farthest { selected.push(*second); }
        }
        if selected.len() == 2 && CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL > 2 {
            let first_point = contacts[selected[0]].metadata.point_world_metres;
            let second_point = contacts[selected[1]].metadata.point_world_metres;
            let best_area = order.iter()
                .filter(|index| !selected.contains(index))
                .filter(|index| {
                    let point = contacts[**index].metadata.point_world_metres;
                    selected.iter().all(|accepted| (contacts[*accepted].metadata.point_world_metres - point).length() >= CONTACT_MANIFOLD_MINIMUM_CONTACT_SPACING_METRES)
                })
                .map(|index| {
                    let point = contacts[*index].metadata.point_world_metres;
                    ((second_point - first_point).cross(point - first_point).length() * 0.5, *index)
                })
                .max_by(|first, second| first.0.partial_cmp(&second.0).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((area, index)) = best_area {
                if area >= CONTACT_MANIFOLD_MINIMUM_CONTACT_TRIANGLE_AREA_SQUARE_METRES { selected.push(index); }
            }
        }
        for index in order {
            if !selected.contains(&index) { discarded[index] = true; }
        }
    }
    let mut pair_groups: BTreeMap<(usize, Option<usize>, String, String), Vec<usize>> = BTreeMap::new();
    for (index, contact) in contacts.iter().enumerate() {
        if discarded[index] { continue; }
        pair_groups.entry((contact.constraint.first_entity, contact.constraint.second_entity,
            contact.first_shape_identifier.clone(), contact.second_shape_identifier.clone())).or_default().push(index);
    }
    for indices in pair_groups.values() {
        if indices.len() <= CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_SHAPE_PAIR { continue; }
        let mut deepest_per_normal: BTreeMap<[i32; 3], usize> = BTreeMap::new();
        for index in indices {
            let bucket = normal_bucket(contacts[*index].metadata.normal_world);
            let candidate = deepest_per_normal.entry(bucket).or_insert(*index);
            if contacts[*index].metadata.penetration_metres > contacts[*candidate].metadata.penetration_metres {
                *candidate = *index;
            }
        }
        let mut selected: Vec<usize> = deepest_per_normal.values().copied().collect();
        let mut remaining = indices.clone();
        remaining.sort_by(|first, second| contacts[*second].metadata.penetration_metres
            .partial_cmp(&contacts[*first].metadata.penetration_metres).unwrap_or(std::cmp::Ordering::Equal));
        for index in remaining {
            if selected.len() >= CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_SHAPE_PAIR { break; }
            if !selected.contains(&index) { selected.push(index); }
        }
        for index in indices {
            if !selected.contains(index) { discarded[*index] = true; }
        }
    }
    for first in 0..contacts.len() {
        if discarded[first] { continue; }
        for second in first + 1..contacts.len() {
            if discarded[second] { continue; }
            if contacts[first].first_shape_identifier != contacts[second].first_shape_identifier
                || contacts[first].second_shape_identifier != contacts[second].second_shape_identifier
                || contacts[first].constraint.first_entity != contacts[second].constraint.first_entity
                || contacts[first].constraint.second_entity != contacts[second].constraint.second_entity { continue; }
            let dominant_axis = |normal: Vec3| normal.x.abs().max(normal.y.abs()).max(normal.z.abs());
            if dominant_axis(contacts[first].metadata.normal_world) >= 0.9
                && dominant_axis(contacts[second].metadata.normal_world) >= 0.9 { continue; }
            let proximity = (contacts[first].metadata.point_world_metres - contacts[second].metadata.point_world_metres).length();
            let alignment = contacts[first].metadata.normal_world.dot(contacts[second].metadata.normal_world);
            if proximity > CONTACT_MANIFOLD_SEAM_PROXIMITY_METRES || alignment >= CONTACT_MANIFOLD_SEAM_NORMAL_ALIGNMENT { continue; }
            if contacts[first].metadata.penetration_metres >= contacts[second].metadata.penetration_metres {
                discarded[second] = true;
            } else {
                discarded[first] = true;
                break;
            }
        }
    }
    let mut write = 0_usize;
    for read in 0..contacts.len() {
        if !discarded[read] {
            if write != read { contacts.swap(write, read); }
            write += 1;
        }
    }
    let removed = contacts.len() - write;
    contacts.truncate(write);
    removed
}

fn surface_from_code(code: u32) -> Result<SurfaceType, String> {
    Ok(match code { 0 => SurfaceType::Road, 1 => SurfaceType::Curb, 2 => SurfaceType::Dirt,
        3 => SurfaceType::Grass, 4 => SurfaceType::Gravel, 5 => SurfaceType::Sand, 6 => SurfaceType::Wall, 7 => SurfaceType::Metal,
        _ => return Err("Unsupported physical surface code".into()) })
}

impl CoupledVehicleWorld {
    pub fn new(collision_world: PhysicalCollisionWorld, configuration: CoupledVehicleWorldConfiguration) -> Result<Self, String> {
        configuration.validate()?;
        let terrain_bounds = collision_world.shapes.iter()
            .map(|shape| shape.shape.compute_aabb(&Isometry3::identity()))
            .collect::<Vec<_>>();
        Ok(Self { collision_world, configuration, vehicles: BTreeMap::new(), dynamic_objects: BTreeMap::new(), terrain_bounds, time_seconds: 0.0,
            accumulated_host_time_seconds: 0.0, paused: false, collision_events: Vec::new(),
            contact_manifolds: BTreeMap::new(), contact_manifold_diagnostics: ContactManifoldDiagnostics::default(),
            contact_classification_diagnostics: ContactClassificationDiagnostics::default(),
            contact_island_diagnostics: ContactIslandDiagnostics::default(),
            dynamic_object_diagnostics: DynamicWorldObjectDiagnostics::default(),
            continuous_collision_diagnostics: ContinuousCollisionDiagnostics::default(),
            step_profile: WorldStepProfile::default(),
            physical_step_index: 0, physical_interval_index: 0,
            simplified_suspension_kinematics: None, next_entity_identifier: 1 })
    }

    pub fn reset_step_profile(&mut self) {
        self.step_profile = WorldStepProfile::default();
    }

    pub fn register_vehicle(&mut self, configuration: VehicleConfig, position: Vec3, yaw_radians: f64) -> Result<u64, String> {
        let world_frequency = self.configuration.physical_frequency_hertz;
        self.register_vehicle_with_frequency(configuration, position, yaw_radians, world_frequency)
    }

    pub fn register_vehicle_with_frequency(&mut self, configuration: VehicleConfig, position: Vec3,
        yaw_radians: f64, physical_frequency_hertz: u32) -> Result<u64, String> {
        self.register_vehicle_with_frequency_and_fidelity(configuration, position, yaw_radians,
            physical_frequency_hertz, VehicleDynamicsFidelity::Full)
    }

    pub fn register_vehicle_with_frequency_and_fidelity(&mut self, configuration: VehicleConfig,
        position: Vec3, yaw_radians: f64, physical_frequency_hertz: u32,
        dynamics_fidelity: VehicleDynamicsFidelity) -> Result<u64, String> {
        if !crate::suspension_mass_properties::vector_is_finite(position) || !yaw_radians.is_finite() {
            return Err("Invalid vehicle spawn pose".into());
        }
        let world_frequency = self.configuration.physical_frequency_hertz;
        if physical_frequency_hertz == 0 || world_frequency % physical_frequency_hertz != 0
            || (physical_frequency_hertz != world_frequency && physical_frequency_hertz != 60) {
            return Err("Only world-frequency player vehicles and 60 Hz AI vehicles are supported".into());
        }
        let selected = configuration.coupled_world.as_ref().ok_or("Vehicle must explicitly select the coupled world")?;
        selected.validate()?;
        if serde_json::to_value(selected).map_err(|error| error.to_string())? != serde_json::to_value(&self.configuration).map_err(|error| error.to_string())? {
            return Err("Vehicles sharing a world require the same world configuration".into());
        }
        let kinematics_table = if dynamics_fidelity.is_simplified() {
            Some(self.simplified_suspension_kinematics_for(&configuration)?)
        } else {
            None
        };
        let mut model = CoupledVehicleModel::new_with_fidelity(configuration.clone(),
            self.configuration.suspension.clone(), dynamics_fidelity, kinematics_table)?;
        if physical_frequency_hertz < world_frequency {
            model.settings.maximum_substep_seconds = 1.0 / physical_frequency_hertz as f64;
        }
        let systems = VehicleSimulator::new_with_fidelity(configuration, position, yaw_radians,
            dynamics_fidelity);
        let mut state = CoupledVehicleState::at_rest(position);
        state.suspension.body_orientation_world = systems.state.orientation;
        state.suspension.simulated_time_seconds = self.time_seconds;
        model.evaluate(&state, &CoupledSuspensionInput::default(), [0.0; 4])?;
        let identifier = self.next_entity_identifier;
        self.next_entity_identifier = identifier.checked_add(1).ok_or("Vehicle identity space exhausted")?;
        let driving_aids_mask = systems.aids.to_bits();
        self.vehicles.insert(identifier, CoupledWorldVehicle { model, state, systems, input: VehicleInput::default(),
            pending_inputs: VecDeque::new(), driving_aids_mask, last_force_input: CoupledSuspensionInput::default(),
            physical_frequency_hertz, dynamics_fidelity,
            spawn_position_world_metres: position, spawn_yaw_radians: yaw_radians });
        Ok(identifier)
    }

    fn simplified_suspension_kinematics_for(&mut self, configuration: &VehicleConfig)
        -> Result<Arc<SuspensionKinematicsTable>, String> {
        use std::hash::{Hash, Hasher};
        let geometry = configuration.suspension.geometric_suspension.as_ref()
            .ok_or("Simplified vehicle dynamics requires geometric hardpoints")?;
        let signature_source = serde_json::to_string(geometry).map_err(|error| error.to_string())?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        signature_source.hash(&mut hasher);
        self.configuration.steering_rack_travel_metres.to_bits().hash(&mut hasher);
        let signature = hasher.finish();
        if let Some((cached_signature, table)) = &self.simplified_suspension_kinematics {
            if *cached_signature == signature {
                return Ok(table.clone());
            }
        }
        let table = Arc::new(SuspensionKinematicsTable::build(configuration,
            self.configuration.steering_rack_travel_metres)?);
        self.simplified_suspension_kinematics = Some((signature, table.clone()));
        Ok(table)
    }

    pub fn remove_vehicle(&mut self, identifier: u64) -> Result<(), String> {
        self.vehicles.remove(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        let removed = self.invalidate_entity_manifolds(identifier);
        self.contact_manifold_diagnostics.removed_entity_total += removed as u64;
        self.refresh_manifold_diagnostics();
        Ok(())
    }

    pub fn register_dynamic_object(&mut self, definition: DynamicWorldObjectDefinition,
        position_world_metres: Vec3, orientation: Quat) -> Result<u64, String> {
        if definition.object_class.rejection_reason().is_some() {
            self.dynamic_object_diagnostics.rejected_unsupported_class_total += 1;
        }
        let object = DynamicWorldObject::new(definition, position_world_metres, orientation)?;
        let identifier = self.next_entity_identifier;
        self.next_entity_identifier = identifier.checked_add(1).ok_or("Entity identity space exhausted")?;
        self.dynamic_objects.insert(identifier, object);
        self.dynamic_object_diagnostics.registered_total += 1;
        Ok(identifier)
    }

    pub fn remove_dynamic_object(&mut self, identifier: u64) -> Result<(), String> {
        self.dynamic_objects.remove(&identifier).ok_or("Stale or unknown dynamic world object identifier")?;
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        let removed = self.invalidate_entity_manifolds(identifier);
        self.contact_manifold_diagnostics.removed_entity_total += removed as u64;
        self.refresh_manifold_diagnostics();
        self.dynamic_object_diagnostics.removed_total += 1;
        self.dynamic_object_diagnostics.sleeping_object_count = self.dynamic_objects.values()
            .filter(|object| object.state.sleeping)
            .count();
        Ok(())
    }

    pub fn reset_dynamic_object(&mut self, identifier: u64) -> Result<(), String> {
        let object = self.dynamic_objects.get_mut(&identifier).ok_or("Stale or unknown dynamic world object identifier")?;
        object.reset()?;
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        let removed = self.invalidate_entity_manifolds(identifier);
        self.contact_manifold_diagnostics.removed_reset_total += removed as u64;
        self.refresh_manifold_diagnostics();
        self.dynamic_object_diagnostics.reset_total += 1;
        self.dynamic_object_diagnostics.sleeping_object_count = self.dynamic_objects.values()
            .filter(|object| object.state.sleeping)
            .count();
        Ok(())
    }

    pub fn dynamic_object_snapshots(&self) -> Vec<DynamicWorldObjectSnapshot> {
        self.dynamic_objects.iter().map(|(identifier, object)| object.snapshot(*identifier)).collect()
    }

    pub fn enqueue_input(&mut self, identifier: u64, input: TimestampedVehicleInput) -> Result<(), String> {
        if !input.time_seconds.is_finite() || input.time_seconds < self.time_seconds - 1e-10
            || input.driving_aids_mask & !255 != 0
            || !(-1.0..=1.0).contains(&input.input.steering)
            || [input.input.throttle, input.input.brake, input.input.handbrake, input.input.clutch].iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        { return Err("Invalid or late timestamped vehicle input".into()); }
        let vehicle = self.vehicles.get_mut(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
        if input.input.gear_request.is_some_and(|gear| gear < -1 || gear as isize > vehicle.model.configuration.powertrain.gear_ratios.len() as isize) {
            return Err("Requested gear lies outside the configured transmission".into());
        }
        if vehicle.pending_inputs.back().is_some_and(|previous| previous.time_seconds >= input.time_seconds) {
            return Err("Vehicle input timestamps must increase strictly".into());
        }
        vehicle.pending_inputs.push_back(input);
        Ok(())
    }

    pub fn reset_vehicle(&mut self, identifier: u64) -> Result<(), String> {
        let (mut configuration, position, yaw, frequency, dynamics_fidelity) = {
            let vehicle = self.vehicles.get(&identifier).ok_or("Stale or unknown coupled vehicle identifier")?;
            (vehicle.model.configuration.clone(), vehicle.spawn_position_world_metres,
                vehicle.spawn_yaw_radians, vehicle.physical_frequency_hertz, vehicle.dynamics_fidelity)
        };
        configuration.fuel.refill_to_initial();
        let kinematics_table = if dynamics_fidelity.is_simplified() {
            Some(self.simplified_suspension_kinematics_for(&configuration)?)
        } else { None };
        let mut model = CoupledVehicleModel::new_with_fidelity(configuration.clone(),
            self.configuration.suspension.clone(), dynamics_fidelity, kinematics_table)?;
        if frequency < self.configuration.physical_frequency_hertz {
            model.settings.maximum_substep_seconds = 1.0 / frequency as f64;
        }
        let systems = VehicleSimulator::new_with_fidelity(configuration, position, yaw,
            dynamics_fidelity);
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
        Self::synchronize_presentation(vehicle);
        self.collision_events.retain(|event| event.first_entity_identifier != identifier && event.second_entity_identifier != Some(identifier));
        let removed = self.invalidate_entity_manifolds(identifier);
        self.contact_manifold_diagnostics.removed_reset_total += removed as u64;
        self.refresh_manifold_diagnostics();
        Ok(())
    }

    fn invalidate_entity_manifolds(&mut self, identifier: u64) -> usize {
        let before = self.contact_manifolds.len();
        self.contact_manifolds.retain(|key, _| key.first_entity_identifier != identifier && key.second_entity_identifier != Some(identifier));
        before - self.contact_manifolds.len()
    }

    pub fn snapshots(&self) -> Result<Vec<CoupledWorldSnapshot>, String> {
        self.vehicles.iter().map(|(identifier, vehicle)| {
            let solved_suspension_corners = WheelIndex::ALL.iter().map(|wheel|
                vehicle.model.corner_solution(*wheel,
                    vehicle.state.suspension.wheel_travel_metres[*wheel as usize],
                    vehicle.last_force_input.steering_rack_metres))
                .collect::<Result<Vec<_>, _>>()?;
            if solved_suspension_corners.iter().any(|corner| !corner.converged) { return Err("Physical snapshot has unconverged suspension geometry".into()); }
            Ok(CoupledWorldSnapshot { entity_identifier: *identifier,
            physical_frequency_hertz: vehicle.physical_frequency_hertz,
            dynamics_fidelity: vehicle.dynamics_fidelity,
            state: vehicle.state.clone(), telemetry: vehicle.systems.build_telemetry_frame(),
            systems_state: vehicle.systems.state.clone(), steering_rack_metres: vehicle.last_force_input.steering_rack_metres,
            fuel_mass_kilograms: vehicle.model.configuration.fuel.effective_current_kg(), driver_input: vehicle.input,
            driving_aids_mask: vehicle.driving_aids_mask,
            operating_mass_kilograms: vehicle.model.configuration.chassis.total_vehicle_mass(&vehicle.model.configuration.tires, &vehicle.model.configuration.fuel), solved_suspension_corners })
        }).collect()
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
        let interval_index = self.physical_interval_index;
        self.physical_interval_index = self.physical_interval_index.wrapping_add(1);
        let mixed = self.vehicles.values()
            .any(|vehicle| vehicle.physical_frequency_hertz < self.configuration.physical_frequency_hertz);
        if mixed {
            self.advance_mixed_physical_step(duration, interval_index)
        } else {
            self.advance_uniform_physical_step(duration)
        }
    }

    fn advance_uniform_physical_step(&mut self, duration: f64) -> Result<(), String> {
        let step_started = Instant::now();
        let start_time = self.time_seconds;
        let mut elapsed = 0.0;
        let mut events = 0;
        let mut classification = self.contact_classification_diagnostics.clone();
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
            let shapes_started = Instant::now();
            let previous_shapes = initial.attached_world_shapes()?;
            self.step_profile.shape_attachment_seconds += shapes_started.elapsed().as_secs_f64();
            let forces_started = Instant::now();
            for vehicle in self.vehicles.values_mut() {
                Self::advance_vehicle_forces(&self.collision_world, &self.configuration, vehicle, interval, &mut classification)?;
            }
            self.step_profile.vehicle_force_seconds += forces_started.elapsed().as_secs_f64();
            let objects_started = Instant::now();
            self.advance_dynamic_objects(interval)?;
            self.step_profile.dynamic_object_seconds += objects_started.elapsed().as_secs_f64();
            let shapes_started = Instant::now();
            let current_shapes = self.attached_world_shapes()?;
            self.step_profile.shape_attachment_seconds += shapes_started.elapsed().as_secs_f64();
            let collision_started = Instant::now();
            let impact = self.earliest_swept_impact(&previous_shapes, &current_shapes, interval, None)?;
            self.step_profile.continuous_collision_seconds += collision_started.elapsed().as_secs_f64();
            let mut shapes_for_contacts = current_shapes;
            let mut localized_impact: Option<SweptImpact> = None;
            let accepted_interval = if let Some(impact) = impact {
                if impact.time_seconds < interval - 1e-10 {
                    events += 1;
                    if events > self.configuration.contacts.maximum_events_per_step {
                        return Err(format!("World collision event budget exceeded at {:.6} s on {}; physical step was not committed",
                            start_time + elapsed + impact.time_seconds, impact.shape_identifier));
                    }
                    *self = initial;
                    let event_interval = impact.time_seconds.max(1e-10);
                    self.continuous_collision_diagnostics.last_impact_time_seconds = Some(start_time + elapsed + impact.time_seconds);
                    self.continuous_collision_diagnostics.last_impact_shape_identifier = Some(impact.shape_identifier.clone());
                    self.continuous_collision_diagnostics.last_impact_entity_identifier = Some(impact.entity_identifier);
                    self.continuous_collision_diagnostics.last_event_interval_seconds = event_interval;
                    localized_impact = Some(impact);
                    let forces_started = Instant::now();
                    for vehicle in self.vehicles.values_mut() {
                        Self::advance_vehicle_forces(&self.collision_world, &self.configuration, vehicle, event_interval, &mut classification)?;
                    }
                    self.step_profile.vehicle_force_seconds += forces_started.elapsed().as_secs_f64();
                    let objects_started = Instant::now();
                    self.advance_dynamic_objects(event_interval)?;
                    self.step_profile.dynamic_object_seconds += objects_started.elapsed().as_secs_f64();
                    let shapes_started = Instant::now();
                    shapes_for_contacts = self.attached_world_shapes()?;
                    self.step_profile.shape_attachment_seconds += shapes_started.elapsed().as_secs_f64();
                    event_interval
                } else { interval }
            } else { interval };
            elapsed += accepted_interval;
            self.time_seconds = start_time + elapsed;
            let collision_events_before = self.collision_events.len();
            self.resolve_world_contacts(accepted_interval, &shapes_for_contacts, None)?;
            if localized_impact.is_some() && self.collision_events.len() == collision_events_before {
                self.continuous_collision_diagnostics.geometric_miss_count += 1;
            }
        }
        self.continuous_collision_diagnostics.total_impact_events += events as u64;
        self.continuous_collision_diagnostics.total_event_subdivisions += events as u64;
        self.continuous_collision_diagnostics.maximum_event_subdivisions =
            self.continuous_collision_diagnostics.maximum_event_subdivisions.max(events);
        classification.rigid_impact_contacts = self.contact_classification_diagnostics.rigid_impact_contacts;
        self.contact_classification_diagnostics = classification;
        self.time_seconds = start_time;
        self.step_profile.total_seconds += step_started.elapsed().as_secs_f64();
        self.step_profile.physical_step_count += 1;
        Ok(())
    }

    fn advance_mixed_physical_step(&mut self, duration: f64, interval_index: u64) -> Result<(), String> {
        let step_started = Instant::now();
        let start_time = self.time_seconds;
        let mut classification = self.contact_classification_diagnostics.clone();
        for vehicle in self.vehicles.values_mut() {
            while vehicle.pending_inputs.front().is_some_and(|input| input.time_seconds <= self.time_seconds + 1e-12) {
                let sample = vehicle.pending_inputs.pop_front().unwrap();
                vehicle.input = sample.input;
                vehicle.driving_aids_mask = sample.driving_aids_mask;
            }
        }
        let world_frequency = self.configuration.physical_frequency_hertz;
        let ai_ratio = (world_frequency / 60) as u64;
        if interval_index % ai_ratio == 0 {
            self.time_seconds = start_time;
            let phase_started = Instant::now();
            self.advance_contact_phase(duration * ai_ratio as f64, WorldContactPhase::Ai, &mut classification)?;
            let elapsed = phase_started.elapsed().as_secs_f64();
            self.step_profile.ai_phase_seconds += elapsed;
            self.step_profile.last_ai_phase_seconds = elapsed;
            self.step_profile.ai_phase_count += 1;
        }
        self.time_seconds = start_time;
        let phase_started = Instant::now();
        self.advance_contact_phase(duration, WorldContactPhase::Player, &mut classification)?;
        let elapsed = phase_started.elapsed().as_secs_f64();
        self.step_profile.player_phase_seconds += elapsed;
        self.step_profile.last_player_phase_seconds = elapsed;
        self.step_profile.player_phase_count += 1;
        classification.rigid_impact_contacts = self.contact_classification_diagnostics.rigid_impact_contacts;
        self.contact_classification_diagnostics = classification;
        self.time_seconds = start_time;
        self.step_profile.total_seconds += step_started.elapsed().as_secs_f64();
        self.step_profile.physical_step_count += 1;
        Ok(())
    }

    fn advance_contact_phase(&mut self, duration: f64, phase: WorldContactPhase,
        classification: &mut ContactClassificationDiagnostics) -> Result<(), String> {
        if !duration.is_finite() || duration <= 0.0 {
            return Err("Contact phase requires a finite positive duration".into());
        }
        let phase_start_time = self.time_seconds;
        let mut remaining = duration;
        let mut events = 0_usize;
        while remaining > duration * 1e-12 {
            let initial = self.clone();
            let previous_shapes = initial.attached_world_shapes()?;
            let forces_started = Instant::now();
            for vehicle in self.vehicles.values_mut() {
                let is_ai = vehicle.physical_frequency_hertz < self.configuration.physical_frequency_hertz;
                let active = match phase {
                    WorldContactPhase::Ai => is_ai,
                    WorldContactPhase::Player => !is_ai,
                };
                if active {
                    Self::advance_vehicle_forces(&self.collision_world, &self.configuration,
                        vehicle, remaining, classification)?;
                }
            }
            self.step_profile.vehicle_force_seconds += forces_started.elapsed().as_secs_f64();
            if phase == WorldContactPhase::Player {
                let objects_started = Instant::now();
                self.advance_dynamic_objects(remaining)?;
                self.step_profile.dynamic_object_seconds += objects_started.elapsed().as_secs_f64();
            }
            let shapes_started = Instant::now();
            let current_shapes = self.attached_world_shapes()?;
            self.step_profile.shape_attachment_seconds += shapes_started.elapsed().as_secs_f64();
            let collision_started = Instant::now();
            let impact = self.earliest_swept_impact(&previous_shapes, &current_shapes, remaining, Some(phase))?;
            self.step_profile.continuous_collision_seconds += collision_started.elapsed().as_secs_f64();
            let mut shapes_for_contacts = current_shapes;
            let mut localized = false;
            let accepted = if let Some(impact) = impact {
                if impact.time_seconds < remaining - 1e-10 {
                    events += 1;
                    if events > self.configuration.contacts.maximum_events_per_step {
                        return Err(format!("World collision event budget exceeded at {:.6} s on {}; physical step was not committed",
                            phase_start_time + (duration - remaining) + impact.time_seconds, impact.shape_identifier));
                    }
                    *self = initial;
                    let event_interval = impact.time_seconds.max(1e-10);
                    self.continuous_collision_diagnostics.last_impact_time_seconds =
                        Some(phase_start_time + (duration - remaining) + impact.time_seconds);
                    self.continuous_collision_diagnostics.last_impact_shape_identifier = Some(impact.shape_identifier.clone());
                    self.continuous_collision_diagnostics.last_impact_entity_identifier = Some(impact.entity_identifier);
                    self.continuous_collision_diagnostics.last_event_interval_seconds = event_interval;
                    localized = true;
                    for vehicle in self.vehicles.values_mut() {
                        let is_ai = vehicle.physical_frequency_hertz < self.configuration.physical_frequency_hertz;
                        let active = match phase {
                            WorldContactPhase::Ai => is_ai,
                            WorldContactPhase::Player => !is_ai,
                        };
                        if active {
                            Self::advance_vehicle_forces(&self.collision_world, &self.configuration,
                                vehicle, event_interval, classification)?;
                        }
                    }
                    if phase == WorldContactPhase::Player {
                        self.advance_dynamic_objects(event_interval)?;
                    }
                    shapes_for_contacts = self.attached_world_shapes()?;
                    event_interval
                } else { remaining }
            } else { remaining };
            let collision_events_before = self.collision_events.len();
            self.resolve_world_contacts(accepted, &shapes_for_contacts, Some(phase))?;
            if localized && self.collision_events.len() == collision_events_before {
                self.continuous_collision_diagnostics.geometric_miss_count += 1;
            }
            remaining -= accepted;
        }
        Ok(())
    }

    fn advance_dynamic_objects(&mut self, duration_seconds: f64) -> Result<(), String> {
        let gravity_world = self.configuration.suspension.gravity_world_metres_per_second_squared;
        for object in self.dynamic_objects.values_mut() {
            object.advance(duration_seconds, gravity_world)?;
        }
        Ok(())
    }

    fn wake_dynamic_objects_for_moving_contacts(&mut self, constraints: &[GeneralizedContactConstraint],
        velocities: &[GeneralizedVehicleVelocity], vehicle_count: usize) -> Result<(), String> {
        if self.dynamic_objects.is_empty() {
            return Ok(());
        }
        let object_identifiers = self.dynamic_objects.keys().copied().collect::<Vec<_>>();
        for constraint in constraints {
            let relative_speed = (0..3)
                .map(|axis| crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                    constraint, axis, velocities))
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            if relative_speed <= DYNAMIC_WORLD_OBJECT_WAKE_RELATIVE_SPEED_METRES_PER_SECOND {
                continue;
            }
            for entity_index in [Some(constraint.first_entity), constraint.second_entity].into_iter().flatten() {
                if entity_index < vehicle_count {
                    continue;
                }
                let object_index = entity_index - vehicle_count;
                let identifier = *object_identifiers.get(object_index)
                    .ok_or("Contact references an unknown dynamic world object")?;
                let object = self.dynamic_objects.get_mut(&identifier)
                    .ok_or("Contact references an unknown dynamic world object")?;
                if object.state.sleeping {
                    object.wake();
                    self.dynamic_object_diagnostics.wake_transition_total += 1;
                }
            }
        }
        Ok(())
    }

    fn update_dynamic_object_sleep_states(&mut self, duration_seconds: f64,
        supported_entities: &[bool]) -> Result<(), String> {
        if !duration_seconds.is_finite() || duration_seconds < 0.0 {
            return Err("Dynamic object sleep update requires a finite non-negative duration".into());
        }
        let vehicle_count = self.vehicles.len();
        let mut sleep_transitions = 0_u64;
        let mut wake_transitions = 0_u64;
        for (object_index, object) in self.dynamic_objects.values_mut().enumerate() {
            let entity_index = vehicle_count + object_index;
            let supported = supported_entities.get(entity_index).copied().unwrap_or(false);
            let linear_speed = object.state.linear_velocity_world_metres_per_second.length();
            let angular_speed = object.state.angular_velocity_world_radians_per_second.length();
            let low_motion = linear_speed <= DYNAMIC_WORLD_OBJECT_SLEEP_LINEAR_SPEED_METRES_PER_SECOND
                && angular_speed <= DYNAMIC_WORLD_OBJECT_SLEEP_ANGULAR_SPEED_RADIANS_PER_SECOND;
            if supported && low_motion {
                object.state.low_motion_time_seconds += duration_seconds;
                if !object.state.sleeping
                    && object.state.low_motion_time_seconds >= DYNAMIC_WORLD_OBJECT_SLEEP_DELAY_SECONDS {
                    object.sleep();
                    sleep_transitions += 1;
                } else if object.state.sleeping {
                    object.state.linear_velocity_world_metres_per_second = Vec3::ZERO;
                    object.state.angular_velocity_world_radians_per_second = Vec3::ZERO;
                }
            } else {
                if object.state.sleeping {
                    wake_transitions += 1;
                }
                object.wake();
            }
        }
        self.dynamic_object_diagnostics.sleep_transition_total += sleep_transitions;
        self.dynamic_object_diagnostics.wake_transition_total += wake_transitions;
        self.dynamic_object_diagnostics.sleeping_object_count = self.dynamic_objects.values()
            .filter(|object| object.state.sleeping)
            .count();
        Ok(())
    }

    fn advance_vehicle_forces(world: &PhysicalCollisionWorld, settings: &CoupledVehicleWorldConfiguration,
        vehicle: &mut CoupledWorldVehicle, duration: f64,
        classification: &mut ContactClassificationDiagnostics) -> Result<(), String> {
        let mut input = CoupledSuspensionInput::default();
        input.steering_rack_metres = vehicle.last_force_input.steering_rack_metres;
        let target_rack = vehicle.input.steering * settings.steering_rack_travel_metres;
        let maximum_rack_change = vehicle.model.configuration.steering.steering_speed * settings.steering_rack_travel_metres * duration;
        input.steering_rack_velocity_metres_per_second = (target_rack - input.steering_rack_metres).clamp(-maximum_rack_change, maximum_rack_change) / duration;
        let geometry = vehicle.model.configuration.suspension.geometric_suspension.as_ref().unwrap();
        let rotation = vehicle.state.suspension.body_orientation_world.to_mat3();
        let mut samples = [TriRaycastSample::default(); 4];
        let mut forwards = [Vec3::FORWARD; 4];
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let solution = vehicle.model.corner_solution(wheel,
                vehicle.state.suspension.wheel_travel_metres[index], input.steering_rack_metres)?;
            if !solution.converged || solution.steering_clamped || solution.rocker_clamped { return Err("Coupled wheel trajectory is outside its supported domain".into()); }
            forwards[index] = rotation.transform_vector(solution.wheel_basis.transform_vector(Vec3::FORWARD));
            let side = if wheel.is_left() { 1.0 } else { -1.0 };
            let static_camber = if wheel.is_front() { vehicle.model.configuration.suspension.front_camber } else { vehicle.model.configuration.suspension.rear_camber };
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
            let radius = if wheel.is_front() { vehicle.model.configuration.tires.front_tire_radius } else { vehicle.model.configuration.tires.rear_tire_radius };
            let direction = -rotation.y;
            if let Some(hit) = world.raycast_driveable(hub.center_of_mass_world_metres + rotation.y * radius,
                direction, radius * 2.0 + settings.road_query_extension_metres)? {
                if hit.normal_world.y >= 0.5 {
                    let distance = hit.distance_metres - radius;
                    input.contacts[index] = Some(CoupledWheelContact { surface_point_world_metres: hit.point_world_metres,
                        surface_normal_world: hit.normal_world, surface_velocity_world_metres_per_second: Vec3::ZERO,
                        tangential_force_world_newtons: Vec3::ZERO });
                    let sample = RaycastHit { is_colliding: true, distance, point: hit.point_world_metres,
                        normal: hit.normal_world, surface: surface_from_code(hit.surface_code)? };
                    samples[index] = TriRaycastSample { inner: sample, center: sample, outer: sample };
                    classification.tire_tread_supports += 1;
                } else {
                    classification.rejected_steep_tire_hits += 1;
                    classification.duplicate_response_preventions += 1;
                }
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
            return Err(format!("Vehicle exceeded the declared continuous collision speed domain: linear {forward_speed} m/s, angular {} rad/s",
                body.angular_velocity.length()));
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
        for wheel in 0..4 {
            let (lower, upper) = vehicle.model.travel_limits_metres[wheel];
            let travel = vehicle.state.suspension.wheel_travel_metres[wheel];
            if travel <= lower + 1e-6 || travel >= upper - 1e-6 {
                classification.travel_stop_wheels += 1;
            }
        }
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
                settings.contacts.velocity_tolerance_metres_per_second / vehicle.model.configuration.tires.front_tire_radius.min(vehicle.model.configuration.tires.rear_tire_radius))?;
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
            vehicle.systems.state.suspension.wheels[wheel].suspension_compression_m = crate::suspension::rest_compression_m(
                &vehicle.systems.config, WheelIndex::ALL[wheel]) + state.wheel_travel_metres[wheel];
            vehicle.systems.state.suspension.wheels[wheel].compression_mm = vehicle.systems.state.suspension.wheels[wheel].suspension_compression_m * 1000.0;
            vehicle.systems.state.suspension.wheels[wheel].unsprung_velocity_m_s = velocity[6 + wheel];
        }
    }

    fn attached_world_shapes(&self) -> Result<Vec<AttachedWorldShape>, String> {
        let mut shapes = Vec::new();
        let world_frequency = self.configuration.physical_frequency_hertz;
        for (entity_index, vehicle) in self.vehicles.values().enumerate() {
            let kind = if vehicle.physical_frequency_hertz < world_frequency {
                WorldEntityKind::AiVehicle
            } else {
                WorldEntityKind::PlayerVehicle
            };
            let rotation = vehicle.state.suspension.body_orientation_world.to_mat3();
            for definition in &self.configuration.body_collision_boxes {
                let pose = physical_pose(vehicle.state.suspension.body_origin_world_metres
                    + rotation.transform_vector(definition.center_local_metres), rotation);
                let shape = SharedShape::cuboid(definition.half_extents_metres.x,
                    definition.half_extents_metres.y, definition.half_extents_metres.z);
                let bounds = shape.compute_aabb(&pose);
                shapes.push(AttachedWorldShape { entity_index, identifier: definition.identifier.clone(), wheel: None,
                    pose, shape, bounds, kind });
            }
            for wheel in WheelIndex::ALL {
                let solution = vehicle.model.corner_solution(wheel,
                    vehicle.state.suspension.wheel_travel_metres[wheel as usize],
                    vehicle.last_force_input.steering_rack_metres)?;
                let local = solution.wheel_basis;
                let basis = Mat3::from_cols(rotation.transform_vector(local.y), rotation.transform_vector(local.x), -rotation.transform_vector(local.z));
                let radius = if wheel.is_front() { vehicle.model.configuration.tires.front_tire_radius } else { vehicle.model.configuration.tires.rear_tire_radius };
                let width = if wheel.is_front() { vehicle.model.configuration.tires.front_tire_width } else { vehicle.model.configuration.tires.rear_tire_width };
                let pose = physical_pose(vehicle.state.suspension.body_origin_world_metres
                    + rotation.transform_vector(solution.hub), basis);
                let shape = SharedShape::cylinder(width * 0.5, radius);
                let bounds = shape.compute_aabb(&pose);
                shapes.push(AttachedWorldShape { entity_index, identifier: format!("wheel_{wheel:?}"), wheel: Some(wheel),
                    pose, shape, bounds, kind });
            }
        }
        let vehicle_count = self.vehicles.len();
        for (object_index, (identifier, object)) in self.dynamic_objects.iter().enumerate() {
            let pose = physical_pose(object.state.position_world_metres, object.state.orientation.to_mat3());
            let shape = object.shared_shape().clone();
            let bounds = shape.compute_aabb(&pose);
            shapes.push(AttachedWorldShape {
                entity_index: vehicle_count + object_index,
                kind: WorldEntityKind::DynamicObject,
                identifier: object.shape_identifier(*identifier),
                wheel: None,
                pose,
                shape,
                bounds,
            });
        }
        Ok(shapes)
    }

    pub fn refresh_contact_manifolds(&mut self) -> Result<ContactManifoldRefreshSummary, String> {
        let (contacts, generated_contact_count, reduced_contact_count, warm_started_contact_count) = self.narrow_phase_contact_batch()?;
        drop(contacts);
        self.prune_contact_manifolds();
        self.refresh_manifold_diagnostics();
        Ok(ContactManifoldRefreshSummary { generated_contact_count, reduced_contact_count,
            warm_started_contact_count, active_contact_count: self.contact_manifold_diagnostics.active_count })
    }

    fn narrow_phase_contact_batch(&mut self) -> Result<(Vec<GeneratedContact>, usize, usize, u64), String> {
        let shapes = self.attached_world_shapes()?;
        self.narrow_phase_contact_batch_with_shapes(&shapes, None)
    }

    fn narrow_phase_contact_batch_with_shapes(&mut self, shapes: &[AttachedWorldShape],
        phase: Option<WorldContactPhase>)
        -> Result<(Vec<GeneratedContact>, usize, usize, u64), String> {
        self.physical_step_index = self.physical_step_index.wrapping_add(1);
        let prediction = self.configuration.contacts.prediction_distance_metres;
        let contact_bounds = shapes.iter().map(|shape| shape.bounds.loosened(prediction)).collect::<Vec<_>>();
        let vehicles = self.vehicles.values().collect::<Vec<_>>();
        let dynamic_objects = self.dynamic_objects.values().collect::<Vec<_>>();
        let entity_identifiers = self.vehicles.keys().copied().chain(self.dynamic_objects.keys().copied()).collect::<Vec<_>>();
        let mut generated = Vec::new();
        let mut generated_index = 0_usize;
        for (shape_index, shape) in shapes.iter().enumerate() {
            if phase.is_some_and(|phase| !phase_allows_terrain(phase, shape.kind)) { continue; }
            let entity_identifier = entity_identifiers[shape.entity_index];
            let shape_bounds = &contact_bounds[shape_index];
            for (terrain_index, terrain) in self.collision_world.shapes.iter().enumerate() {
                if !shape_bounds.intersects(&self.terrain_bounds[terrain_index]) { continue; }
                let collision_shape = if shape.wheel.is_some() && terrain.driveable {
                    let Some(impact_shape) = &terrain.wheel_impact_shape else { continue; };
                    impact_shape
                } else { &terrain.shape };
                for manifold in shape_pair_contact_manifolds(&Isometry3::identity(), collision_shape,
                    &shape.pose, &shape.shape, prediction)
                    .map_err(|error| format!("{error} with world shape {}", terrain.identifier))? {
                    let world_shape_identifier = terrain.identifier.clone();
                    let axes = contact_axes(manifold.normal_toward_first_world);
                    for point in &manifold.points {
                        if point.separation_metres > prediction { continue; }
                        if -point.separation_metres > self.configuration.maximum_initial_overlap_metres {
                            return Err("World entity overlap exceeds the supported initialization/recovery domain".into());
                        }
                        let prescribed_velocity = entity_prescribed_contact_velocity(&vehicles, &dynamic_objects,
                            shape.entity_index, shape.wheel, point.point_on_second_world_metres)?;
                        let mut jacobians = [[0.0; 14]; 3];
                        for axis in 0..3 {
                            jacobians[axis] = entity_contact_jacobian(&vehicles, &dynamic_objects, shape.entity_index,
                                shape.wheel, point.point_on_second_world_metres, axes[axis])?;
                        }
                        generated_index += 1;
                        generated.push(GeneratedContact {
                            constraint: GeneralizedContactConstraint { identifier: format!("world_{:08}_entity_{:020}_{}_{}_{}",
                                    terrain_index, entity_identifier, shape.identifier, point.first_feature_identifier, generated_index),
                                first_entity: shape.entity_index, second_entity: None, first_jacobians: jacobians, second_jacobians: [[0.0; 14]; 3],
                                first_prescribed_velocity_metres_per_second: axes.map(|axis| axis.dot(prescribed_velocity)),
                                second_prescribed_velocity_metres_per_second: [0.0; 3],
                                surface_code: terrain.surface_code,
                                normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] },
                            metadata: ContactMetadata { point_world_metres: point.point_on_first_world_metres, normal_world: axes[0],
                                surface_code: terrain.surface_code, penetration_metres: (-point.separation_metres).max(0.0),
                                separation_metres: point.separation_metres },
                            key: ContactFeatureKey { first_entity_identifier: entity_identifier, second_entity_identifier: None,
                                world_shape_index: Some(terrain_index), first_shape_identifier: world_shape_identifier.clone(),
                                second_shape_identifier: shape.identifier.clone(), first_feature_identifier: point.first_feature_identifier,
                                second_feature_identifier: point.second_feature_identifier },
                            first_local_point_metres: point.point_on_first_local_metres,
                            second_local_point_metres: point.point_on_second_local_metres,
                            first_shape_identifier: world_shape_identifier.clone(),
                            second_shape_identifier: shape.identifier.clone() });
                        self.contact_classification_diagnostics.rigid_impact_contacts += 1;
                    }
                }
            }
        }
        for first in 0..shapes.len() { for second in first + 1..shapes.len() {
            let first_shape = &shapes[first]; let second_shape = &shapes[second];
            if first_shape.entity_index == second_shape.entity_index { continue; }
            if phase.is_some_and(|phase| !phase_allows_pair(phase, first_shape.kind, second_shape.kind)) { continue; }
            if !contact_bounds[first].intersects(&contact_bounds[second]) { continue; }
            let first_entity_identifier = entity_identifiers[first_shape.entity_index];
            let second_entity_identifier = entity_identifiers[second_shape.entity_index];
            let surface_code = entity_surface_code(&vehicles, &dynamic_objects, first_shape.entity_index)
                .or_else(|| entity_surface_code(&vehicles, &dynamic_objects, second_shape.entity_index))
                .unwrap_or(6);
            for manifold in shape_pair_contact_manifolds(&first_shape.pose, &first_shape.shape, &second_shape.pose, &second_shape.shape, prediction)
                .map_err(|error| format!("{error} between {} and {}", first_shape.identifier, second_shape.identifier))? {
                let normal = -manifold.normal_toward_first_world;
                let axes = contact_axes(normal);
                for point in &manifold.points {
                    if point.separation_metres > prediction { continue; }
                    let first_prescribed_velocity = entity_prescribed_contact_velocity(&vehicles, &dynamic_objects,
                        first_shape.entity_index, first_shape.wheel, point.point_on_first_world_metres)?;
                    let second_prescribed_velocity = entity_prescribed_contact_velocity(&vehicles, &dynamic_objects,
                        second_shape.entity_index, second_shape.wheel, point.point_on_second_world_metres)?;
                    let mut first_jacobians = [[0.0; 14]; 3]; let mut second_jacobians = [[0.0; 14]; 3];
                    for axis in 0..3 {
                        first_jacobians[axis] = entity_contact_jacobian(&vehicles, &dynamic_objects,
                            first_shape.entity_index, first_shape.wheel, point.point_on_first_world_metres, axes[axis])?;
                        second_jacobians[axis] = entity_contact_jacobian(&vehicles, &dynamic_objects,
                            second_shape.entity_index, second_shape.wheel, point.point_on_second_world_metres, -axes[axis])?;
                    }
                    generated_index += 1;
                    generated.push(GeneratedContact {
                        constraint: GeneralizedContactConstraint { identifier: format!("pair_{:020}_{:020}_{}_{}_{}_{}",
                                first_entity_identifier, second_entity_identifier, first_shape.identifier, second_shape.identifier,
                                point.first_feature_identifier, generated_index),
                            first_entity: first_shape.entity_index, second_entity: Some(second_shape.entity_index), first_jacobians, second_jacobians,
                            first_prescribed_velocity_metres_per_second: axes.map(|axis| axis.dot(first_prescribed_velocity)),
                            second_prescribed_velocity_metres_per_second: axes.map(|axis| (-axis).dot(second_prescribed_velocity)),
                            surface_code,
                            normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] },
                        metadata: ContactMetadata { point_world_metres: point.point_on_first_world_metres, normal_world: normal, surface_code,
                            penetration_metres: (-point.separation_metres).max(0.0), separation_metres: point.separation_metres },
                        key: ContactFeatureKey { first_entity_identifier, second_entity_identifier: Some(second_entity_identifier),
                            world_shape_index: None, first_shape_identifier: first_shape.identifier.clone(),
                            second_shape_identifier: second_shape.identifier.clone(), first_feature_identifier: point.first_feature_identifier,
                            second_feature_identifier: point.second_feature_identifier },
                        first_local_point_metres: point.point_on_first_local_metres,
                        second_local_point_metres: point.point_on_second_local_metres,
                        first_shape_identifier: first_shape.identifier.clone(),
                        second_shape_identifier: second_shape.identifier.clone() });
                    self.contact_classification_diagnostics.rigid_impact_contacts += 1;
                }
            }
        } }
        let mut warm_started = 0_u64;
        for contact in &mut generated {
            let cached = self.contact_manifolds.get(&contact.key).cloned();
            if let Some(entry) = cached {
                let alignment = entry.normal_world.dot(contact.metadata.normal_world);
                if alignment >= CONTACT_MANIFOLD_WARM_START_MINIMUM_ALIGNMENT {
                    let resting = entry.resting_steps >= CONTACT_MANIFOLD_RESTING_STEPS_BEFORE_FULL_WARM_START;
                    let warm_start_scale = if resting { CONTACT_MANIFOLD_RESTING_WARM_START_SCALE } else { CONTACT_MANIFOLD_WARM_START_SCALE };
                    if resting { self.contact_manifold_diagnostics.full_warm_started_total += 1; }
                    contact.constraint.normal_impulse_newton_seconds = entry.normal_impulse_newton_seconds
                        * alignment.max(0.0) * warm_start_scale;
                    let axes = contact_axes(contact.metadata.normal_world);
                    let tangential_world = entry.tangent_world[0] * entry.tangential_impulse_newton_seconds[0]
                        + entry.tangent_world[1] * entry.tangential_impulse_newton_seconds[1];
                    let mut tangential = [tangential_world.dot(axes[1]) * warm_start_scale,
                        tangential_world.dot(axes[2]) * warm_start_scale];
                    let coefficient = self.configuration.contacts.impact_friction_for_surface(contact.metadata.surface_code);
                    let bound = coefficient * contact.constraint.normal_impulse_newton_seconds;
                    let magnitude = tangential[0].hypot(tangential[1]);
                    if magnitude > bound && magnitude > 0.0 {
                        tangential[0] *= bound / magnitude;
                        tangential[1] *= bound / magnitude;
                    }
                    contact.constraint.tangential_impulse_newton_seconds = tangential;
                    warm_started += 1;
                } else {
                    self.contact_manifolds.remove(&contact.key);
                    self.contact_manifold_diagnostics.removed_normal_rotation_total += 1;
                }
            }
        }
        let generated_count = generated.len();
        let reduced_count = reduce_redundant_manifold_contacts(&mut generated);
        self.contact_manifold_diagnostics.reduced_contact_total += reduced_count as u64;
        self.contact_manifold_diagnostics.warm_started_total += warm_started;
        let step = self.physical_step_index;
        for contact in &generated {
            let tangents = contact_axes(contact.metadata.normal_world);
            let coefficient = self.configuration.contacts.impact_friction_for_surface(contact.metadata.surface_code);
            let entry = self.contact_manifolds.entry(contact.key.clone()).or_insert_with(|| {
                self.contact_manifold_diagnostics.created_total += 1;
                ContactManifoldEntry::new(step)
            });
            let previous_tangential_world = entry.tangent_world[0] * entry.tangential_impulse_newton_seconds[0]
                + entry.tangent_world[1] * entry.tangential_impulse_newton_seconds[1];
            let mut remapped_tangential = [previous_tangential_world.dot(tangents[1]),
                previous_tangential_world.dot(tangents[2])];
            let bound = coefficient * entry.normal_impulse_newton_seconds;
            let magnitude = remapped_tangential[0].hypot(remapped_tangential[1]);
            if magnitude > bound && magnitude > 0.0 {
                remapped_tangential[0] *= bound / magnitude;
                remapped_tangential[1] *= bound / magnitude;
            }
            entry.normal_world = contact.metadata.normal_world;
            entry.tangent_world = [tangents[1], tangents[2]];
            entry.tangential_impulse_newton_seconds = remapped_tangential;
            entry.point_world_metres = contact.metadata.point_world_metres;
            entry.first_local_point_metres = contact.first_local_point_metres;
            entry.second_local_point_metres = contact.second_local_point_metres;
            entry.separation_metres = contact.metadata.separation_metres;
            entry.surface_code = contact.metadata.surface_code;
            entry.last_seen_step = step;
            entry.age_steps += 1;
        }
        generated.sort_by(|first, second| first.constraint.identifier.cmp(&second.constraint.identifier));
        Ok((generated, generated_count, reduced_count, warm_started))
    }

    fn resolve_world_contacts(&mut self, duration_seconds: f64, shapes: &[AttachedWorldShape],
        phase: Option<WorldContactPhase>) -> Result<(), String> {
        let generation_started = Instant::now();
        let (generated, _, _, _) = self.narrow_phase_contact_batch_with_shapes(shapes, phase)?;
        self.step_profile.contact_generation_seconds += generation_started.elapsed().as_secs_f64();
        let entity_count = self.vehicles.len() + self.dynamic_objects.len();
        if generated.is_empty() {
            let finalize_started = Instant::now();
            let no_support = vec![false; entity_count];
            self.contact_island_diagnostics = ContactIslandDiagnostics::default();
            if phase != Some(WorldContactPhase::Ai) {
                self.update_dynamic_object_sleep_states(duration_seconds, &no_support)?;
            }
            self.prune_contact_manifolds();
            self.refresh_manifold_diagnostics();
            self.step_profile.contact_finalize_seconds += finalize_started.elapsed().as_secs_f64();
            return Ok(());
        }
        let solve_started = Instant::now();
        let mut supported_entities = vec![false; entity_count];
        for contact in &generated {
            if contact.metadata.normal_world.y >= DYNAMIC_WORLD_OBJECT_SUPPORT_NORMAL_MINIMUM_Y {
                supported_entities[contact.constraint.first_entity] = true;
            }
            if let Some(second_entity) = contact.constraint.second_entity {
                if -contact.metadata.normal_world.y >= DYNAMIC_WORLD_OBJECT_SUPPORT_NORMAL_MINIMUM_Y {
                    supported_entities[second_entity] = true;
                }
            }
        }
        let vehicles = self.vehicles.values().collect::<Vec<_>>();
        let vehicle_count = vehicles.len();
        let mut keys = Vec::with_capacity(generated.len());
        let mut local_points = Vec::with_capacity(generated.len());
        let mut constraints = Vec::with_capacity(generated.len());
        let mut metadata = Vec::with_capacity(generated.len());
        for contact in generated {
            keys.push(contact.key);
            local_points.push((contact.first_local_point_metres, contact.second_local_point_metres));
            constraints.push(contact.constraint);
            metadata.push(contact.metadata);
        }
        let evaluations = vehicles.iter().map(|vehicle| vehicle.model.evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])).collect::<Result<Vec<_>, _>>()?;
        let mut matrices = evaluations.iter().map(|evaluation| evaluation.mass_matrix).collect::<Vec<_>>();
        let mut velocities = vehicles.iter().map(|vehicle| vehicle.state.velocity()).collect::<Vec<_>>();
        for object in self.dynamic_objects.values() {
            matrices.push(object.mass_matrix()?);
            velocities.push(object.velocity());
        }
        self.wake_dynamic_objects_for_moving_contacts(&constraints, &velocities, vehicle_count)?;
        let mut sleeping_entities = vec![false; entity_count];
        for (object_index, object) in self.dynamic_objects.values().enumerate() {
            sleeping_entities[vehicle_count + object_index] = object.state.sleeping;
        }
        let mut fixed_entities = vec![false; entity_count];
        if phase == Some(WorldContactPhase::Player) {
            let world_frequency = self.configuration.physical_frequency_hertz;
            for (index, vehicle) in self.vehicles.values().enumerate() {
                fixed_entities[index] = vehicle.physical_frequency_hertz < world_frequency;
            }
        }
        let before = velocities.clone();
        let result = solve_generalized_vehicle_contacts_with_fixed_entities(&matrices, &mut velocities,
            &mut constraints, &self.configuration.contacts, &fixed_entities)?;
        let (island_count, largest_island_entities, largest_island_constraints) =
            contact_island_summary(&constraints, entity_count);
        self.contact_island_diagnostics = ContactIslandDiagnostics {
            last_island_count: island_count,
            last_largest_island_entity_count: largest_island_entities,
            last_largest_island_constraint_count: largest_island_constraints,
            last_constraint_count: constraints.len(),
            last_solver_iterations: result.iterations,
            last_static_friction_constraint_count: result.static_friction_constraint_count,
            last_sliding_constraint_count: result.sliding_constraint_count,
        };
        let identities = self.vehicles.keys().copied().chain(self.dynamic_objects.keys().copied()).collect::<Vec<_>>();
        for (index, vehicle) in self.vehicles.values_mut().enumerate() {
            if fixed_entities[index] { continue; }
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
        for (object_index, object) in self.dynamic_objects.values_mut().enumerate() {
            let index = vehicle_count + object_index;
            let previous = crate::coupled_vehicle_dynamics::generalized_vehicle_energy(&matrices[index], before[index]);
            let current = crate::coupled_vehicle_dynamics::generalized_vehicle_energy(&matrices[index], velocities[index]);
            object.state.collision_energy_change_joules += current - previous;
            object.set_velocity(velocities[index])?;
        }
        self.step_profile.contact_solve_seconds += solve_started.elapsed().as_secs_f64();
        let correction_started = Instant::now();
        let factors = matrices.iter().map(crate::coupled_vehicle_dynamics::factor_generalized_vehicle_matrix).collect::<Result<Vec<_>, _>>()?;
        let mut corrections = vec![[0.0; 14]; entity_count];
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
        for (index, constraint) in constraints.iter().enumerate() {
            if result.normal_impulses_newton_seconds[index] <= 0.0 { continue; }
            if sleeping_entities.get(constraint.first_entity).copied().unwrap_or(false)
                || fixed_entities.get(constraint.first_entity).copied().unwrap_or(false)
                || constraint.second_entity.and_then(|entity| sleeping_entities.get(entity).copied()).unwrap_or(false)
                || constraint.second_entity.and_then(|entity| fixed_entities.get(entity).copied()).unwrap_or(false)
            { continue; }
            let age_steps = self.contact_manifolds.get(&keys[index]).map(|entry| entry.age_steps).unwrap_or(0);
            if age_steps == 0 { continue; }
            let after_first = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                constraint, 1, &velocities);
            let after_second = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                constraint, 2, &velocities);
            if after_first.hypot(after_second) > CONTACT_MANIFOLD_RESTING_TANGENTIAL_SPEED_METRES_PER_SECOND {
                continue;
            }
            let before_first = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                constraint, 1, &before);
            let before_second = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                constraint, 2, &before);
            let mut target = [(after_first - before_first) * duration_seconds,
                (after_second - before_second) * duration_seconds];
            let applied_first = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[1], corrections[constraint.first_entity])
                + constraint.second_entity.map_or(0.0, |entity| crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[1], corrections[entity]));
            let applied_second = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[2], corrections[constraint.first_entity])
                + constraint.second_entity.map_or(0.0, |entity| crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[2], corrections[entity]));
            target[0] -= applied_first;
            target[1] -= applied_second;
            let target_length = target[0].hypot(target[1]);
            if target_length > self.configuration.contacts.maximum_position_correction_metres {
                let scale = self.configuration.contacts.maximum_position_correction_metres / target_length;
                target[0] *= scale;
                target[1] *= scale;
            }
            let first_response_first = crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(
                &factors[constraint.first_entity], constraint.first_jacobians[1])?;
            let first_response_second = crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(
                &factors[constraint.first_entity], constraint.first_jacobians[2])?;
            let second_response_first = match constraint.second_entity {
                Some(entity) => Some(crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(
                    &factors[entity], constraint.second_jacobians[1])?),
                None => None,
            };
            let second_response_second = match constraint.second_entity {
                Some(entity) => Some(crate::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(
                    &factors[entity], constraint.second_jacobians[2])?),
                None => None,
            };
            let mut effective_first_first = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[1], first_response_first);
            let mut effective_first_second = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[1], first_response_second);
            let mut effective_second_first = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[2], first_response_first);
            let mut effective_second_second = crate::coupled_vehicle_dynamics::generalized_projection(
                constraint.first_jacobians[2], first_response_second);
            if let (Some(_entity), Some(first_response), Some(second_response)) =
                (constraint.second_entity, second_response_first, second_response_second) {
                effective_first_first += crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[1], first_response);
                effective_first_second += crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[1], second_response);
                effective_second_first += crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[2], first_response);
                effective_second_second += crate::coupled_vehicle_dynamics::generalized_projection(
                    constraint.second_jacobians[2], second_response);
            }
            let determinant = effective_first_first * effective_second_second
                - effective_first_second * effective_second_first;
            if determinant <= 1e-12 || !determinant.is_finite() { continue; }
            let first_weight = (target[0] * effective_second_second - target[1] * effective_first_second) / determinant;
            let second_weight = (target[1] * effective_first_first - target[0] * effective_second_first) / determinant;
            for coordinate in 0..14 {
                corrections[constraint.first_entity][coordinate] += first_response_first[coordinate] * first_weight
                    + first_response_second[coordinate] * second_weight;
                if let (Some(entity), Some(first_response), Some(second_response)) =
                    (constraint.second_entity, second_response_first, second_response_second) {
                    corrections[entity][coordinate] += first_response[coordinate] * first_weight
                        + second_response[coordinate] * second_weight;
                }
            }
            self.contact_manifold_diagnostics.tangential_position_correction_total += 1;
        }
        for (index, vehicle) in self.vehicles.values_mut().enumerate() {
            if fixed_entities[index] { continue; }
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
        let gravity_world = self.configuration.suspension.gravity_world_metres_per_second_squared;
        for (object_index, object) in self.dynamic_objects.values_mut().enumerate() {
            let index = vehicle_count + object_index;
            if object.state.sleeping { continue; }
            if corrections[index].iter().all(|value| *value == 0.0) { continue; }
            let previous = object.mechanical_energy_joules(gravity_world);
            object.apply_position_correction(corrections[index])?;
            object.state.position_correction_energy_joules += object.mechanical_energy_joules(gravity_world) - previous;
        }
        self.step_profile.contact_correction_seconds += correction_started.elapsed().as_secs_f64();
        let finalize_started = Instant::now();
        for index in 0..constraints.len() {
            let axes = contact_axes(metadata[index].normal_world);
            let entry = self.contact_manifolds.entry(keys[index].clone()).or_insert_with(|| {
                self.contact_manifold_diagnostics.created_total += 1;
                ContactManifoldEntry::new(self.physical_step_index)
            });
            entry.normal_world = metadata[index].normal_world;
            entry.tangent_world = [axes[1], axes[2]];
            entry.point_world_metres = metadata[index].point_world_metres;
            entry.first_local_point_metres = local_points[index].0;
            entry.second_local_point_metres = local_points[index].1;
            entry.separation_metres = metadata[index].separation_metres;
            entry.surface_code = metadata[index].surface_code;
            entry.normal_impulse_newton_seconds = result.normal_impulses_newton_seconds[index];
            entry.tangential_impulse_newton_seconds = result.tangential_impulses_newton_seconds[index];
            let normal_speed = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                &constraints[index], 0, &velocities);
            let tangential_speed = crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                &constraints[index], 1, &velocities).hypot(
                crate::generalized_vehicle_contacts::generalized_contact_relative_velocity(
                    &constraints[index], 2, &velocities));
            let resting = (result.normal_impulses_newton_seconds[index] > 0.0
                || sleeping_entities.get(constraints[index].first_entity).copied().unwrap_or(false)
                || constraints[index].second_entity
                    .and_then(|entity| sleeping_entities.get(entity).copied())
                    .unwrap_or(false))
                && normal_speed.abs() <= CONTACT_MANIFOLD_RESTING_NORMAL_SPEED_METRES_PER_SECOND
                && tangential_speed <= CONTACT_MANIFOLD_RESTING_TANGENTIAL_SPEED_METRES_PER_SECOND;
            entry.resting_steps = if resting { entry.resting_steps.saturating_add(1) } else { 0 };
        }
        for (index, constraint) in constraints.iter().enumerate() {
            if result.normal_impulses_newton_seconds[index] <= 0.0 { continue; }
            let manifold_age_steps = self.contact_manifolds.get(&keys[index]).map(|entry| entry.age_steps).unwrap_or(1);
            self.collision_events.push(CoupledWorldCollisionEvent { contact_identifier: constraint.identifier.clone(),
                time_seconds: self.time_seconds, first_entity_identifier: identities[constraint.first_entity],
                second_entity_identifier: constraint.second_entity.map(|entity| identities[entity]),
                point_world_metres: metadata[index].point_world_metres, normal_world: metadata[index].normal_world,
                normal_impulse_newton_seconds: result.normal_impulses_newton_seconds[index],
                tangential_impulse_newton_seconds: result.tangential_impulses_newton_seconds[index],
                surface_code: metadata[index].surface_code, manifold_age_steps,
                response_owner: ContactResponseOwner::RigidImpact });
        }
        if phase != Some(WorldContactPhase::Ai) {
            self.update_dynamic_object_sleep_states(duration_seconds, &supported_entities)?;
        }
        self.prune_contact_manifolds();
        self.refresh_manifold_diagnostics();
        self.step_profile.contact_finalize_seconds += finalize_started.elapsed().as_secs_f64();
        Ok(())
    }

    fn prune_contact_manifolds(&mut self) {
        let step = self.physical_step_index;
        let mut removed = 0_u64;
        self.contact_manifolds.retain(|_, entry| {
            if step.saturating_sub(entry.last_seen_step) <= CONTACT_MANIFOLD_RETENTION_STEPS {
                true
            } else {
                removed += 1;
                false
            }
        });
        self.contact_manifold_diagnostics.removed_unseen_total += removed;
    }

    fn refresh_manifold_diagnostics(&mut self) {
        let step = self.physical_step_index;
        self.contact_manifold_diagnostics.active_count = self.contact_manifolds.values()
            .filter(|entry| entry.last_seen_step == step).count();
        self.contact_manifold_diagnostics.retained_count = self.contact_manifolds.values()
            .filter(|entry| entry.last_seen_step != step).count();
        self.contact_manifold_diagnostics.maximum_age_steps = self.contact_manifolds.values()
            .map(|entry| entry.age_steps).max().unwrap_or(0);
        self.contact_manifold_diagnostics.resting_contact_count = self.contact_manifolds.values()
            .filter(|entry| entry.last_seen_step == step
                && entry.resting_steps >= CONTACT_MANIFOLD_RESTING_STEPS_BEFORE_FULL_WARM_START)
            .count();
    }

    fn earliest_swept_impact(&self, previous_shapes: &[AttachedWorldShape],
        current_shapes: &[AttachedWorldShape], duration: f64,
        phase: Option<WorldContactPhase>) -> Result<Option<SweptImpact>, String> {
        let entity_identifiers = self.vehicles.keys().copied().chain(self.dynamic_objects.keys().copied()).collect::<Vec<_>>();
        if previous_shapes.len() != current_shapes.len() { return Err("Shape attachments changed during a physical step".into()); }
        let persistent_terrain_contacts = self.contact_manifolds.iter()
            .filter(|(_, entry)| entry.last_seen_step == self.physical_step_index)
            .filter_map(|(key, _)| key.world_shape_index.map(|shape_index|
                (key.first_entity_identifier, shape_index, key.second_shape_identifier.clone())))
            .collect::<BTreeSet<_>>();
        let persistent_pair_contacts = self.contact_manifolds.iter()
            .filter(|(_, entry)| entry.last_seen_step == self.physical_step_index)
            .filter_map(|(key, _)| key.second_entity_identifier.map(|second_entity_identifier|
                (key.first_entity_identifier, second_entity_identifier,
                    key.first_shape_identifier.clone(), key.second_shape_identifier.clone())))
            .collect::<BTreeSet<_>>();
        let mut motions = Vec::new();
        let mut swept_bounds = Vec::new();
        let mut sweeping = Vec::new();
        for (previous, current) in previous_shapes.iter().zip(current_shapes.iter()) {
            let linear_velocity = (current.pose.translation.vector - previous.pose.translation.vector) / duration;
            let relative_rotation = current.pose.rotation * previous.pose.rotation.inverse();
            let angular_velocity = relative_rotation.scaled_axis() / duration;
            sweeping.push(linear_velocity.norm() > CONTINUOUS_COLLISION_MINIMUM_SWEEP_SPEED_METRES_PER_SECOND
                || angular_velocity.norm() > CONTINUOUS_COLLISION_MINIMUM_SWEEP_ANGULAR_SPEED_RADIANS_PER_SECOND);
            motions.push(NonlinearRigidMotion::new(previous.pose, Point3::origin(), linear_velocity, angular_velocity));
            let local_bounds = previous.shape.compute_local_aabb();
            let radius = local_bounds.mins.coords.norm().max(local_bounds.maxs.coords.norm());
            let previous_center = Point3::from(previous.pose.translation.vector);
            let current_center = Point3::from(current.pose.translation.vector);
            let extent = parry3d_f64::na::Vector3::repeat(radius + self.configuration.contacts.prediction_distance_metres);
            swept_bounds.push(Aabb::new(previous_center.inf(&current_center) - extent, previous_center.sup(&current_center) + extent));
        }
        let mut earliest: Option<SweptImpact> = None;
        for (index, shape) in previous_shapes.iter().enumerate() {
            if !sweeping[index] { continue; }
            if phase.is_some_and(|phase| !phase_allows_terrain(phase, shape.kind)) { continue; }
            let entity_identifier = entity_identifiers[shape.entity_index];
            for (terrain_index, terrain) in self.collision_world.shapes.iter().enumerate() {
                if persistent_terrain_contacts.contains(&(entity_identifier, terrain_index, shape.identifier.clone())) { continue; }
                if !swept_bounds[index].intersects(&self.terrain_bounds[terrain_index]) { continue; }
                let collision_shape = if shape.wheel.is_some() && terrain.driveable {
                    let Some(impact_shape) = &terrain.wheel_impact_shape else { continue; };
                    impact_shape
                } else { &terrain.shape };
                let result = cast_shapes_nonlinear(&NonlinearRigidMotion::identity(), collision_shape.as_ref(),
                    &motions[index], shape.shape.as_ref(), 0.0, duration, false).map_err(|_| "Unsupported rotating world sweep")?;
                if let Some(hit) = result {
                    if hit.time_of_impact > 1e-10 && earliest.as_ref().is_none_or(|previous| hit.time_of_impact < previous.time_seconds) {
                        earliest = Some(SweptImpact { time_seconds: hit.time_of_impact,
                            shape_identifier: format!("{}|{}", shape.identifier, terrain.identifier),
                            entity_identifier: entity_identifiers[shape.entity_index] });
                    }
                }
            }
        }
        for first in 0..previous_shapes.len() { for second in first + 1..previous_shapes.len() {
            let first_shape = &previous_shapes[first]; let second_shape = &previous_shapes[second];
            if first_shape.entity_index == second_shape.entity_index { continue; }
            if phase.is_some_and(|phase| !phase_allows_pair(phase, first_shape.kind, second_shape.kind)) { continue; }
            if !sweeping[first] && !sweeping[second] { continue; }
            let first_entity_identifier = entity_identifiers[first_shape.entity_index];
            let second_entity_identifier = entity_identifiers[second_shape.entity_index];
            if persistent_pair_contacts.contains(&(first_entity_identifier, second_entity_identifier,
                first_shape.identifier.clone(), second_shape.identifier.clone())) { continue; }
            if !swept_bounds[first].intersects(&swept_bounds[second]) { continue; }
            if let Some(hit) = cast_shapes_nonlinear(&motions[first], first_shape.shape.as_ref(), &motions[second], second_shape.shape.as_ref(),
                0.0, duration, false).map_err(|_| "Unsupported rotating vehicle sweep")? {
                if hit.time_of_impact > 1e-10 && earliest.as_ref().is_none_or(|previous| hit.time_of_impact < previous.time_seconds) {
                    earliest = Some(SweptImpact { time_seconds: hit.time_of_impact,
                        shape_identifier: format!("{}|{}", first_shape.identifier, second_shape.identifier),
                        entity_identifier: entity_identifiers[first_shape.entity_index] });
                }
            }
        } }
        Ok(earliest)
    }
}
