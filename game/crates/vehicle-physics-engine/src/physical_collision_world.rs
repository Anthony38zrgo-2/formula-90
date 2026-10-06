use crate::suspension_mass_properties::vector_is_finite;
use crate::types::{Mat3, Vec3};
use parry3d_f64::na::{Isometry3, Matrix3, Point3, Rotation3, Translation3, UnitQuaternion, Vector3};
use parry3d_f64::query::{contact, cast_shapes, Ray, ShapeCastOptions};
use parry3d_f64::bounding_volume::BoundingVolume;
use parry3d_f64::shape::{SharedShape, TriMesh};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalWorldShapeDefinition {
    pub identifier: String,
    pub surface_code: u32,
    pub collision_role: String,
    pub vertices_world_metres: Vec<[f64; 3]>,
    pub triangle_indices: Vec<[u32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalWorldPackage {
    pub schema_version: u32,
    pub source_path: String,
    pub source_digest: String,
    pub geometry_digest: String,
    pub coordinate_contract: String,
    pub shapes: Vec<PhysicalWorldShapeDefinition>,
}

#[derive(Debug, Clone)]
pub struct PhysicalWorldShape {
    pub identifier: String,
    pub surface_code: u32,
    pub driveable: bool,
    pub shape: SharedShape,
    pub wheel_impact_shape: Option<SharedShape>,
}

#[derive(Debug, Clone)]
pub struct PhysicalCollisionWorld {
    pub source_digest: String,
    pub shapes: Vec<PhysicalWorldShape>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PhysicalWorldRayHit {
    pub point_world_metres: Vec3,
    pub normal_world: Vec3,
    pub distance_metres: f64,
    pub surface_code: u32,
    pub shape_index: usize,
    pub triangle_identifier: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalShapeContact {
    pub point_on_world_metres: Vec3,
    pub point_on_vehicle_metres: Vec3,
    pub normal_toward_vehicle_world: Vec3,
    pub signed_distance_metres: f64,
    pub surface_code: u32,
    pub shape_index: usize,
}

pub fn physical_vector(vector: Vec3) -> Vector3<f64> { Vector3::new(vector.x, vector.y, vector.z) }
pub fn vehicle_vector(vector: Vector3<f64>) -> Vec3 { Vec3::new(vector.x, vector.y, vector.z) }

pub fn physical_pose(position: Vec3, rotation: Mat3) -> Isometry3<f64> {
    let matrix = Matrix3::from_columns(&[physical_vector(rotation.x), physical_vector(rotation.y), physical_vector(rotation.z)]);
    Isometry3::from_parts(Translation3::from(physical_vector(position)),
        UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix_unchecked(matrix)))
}

pub fn physical_geometry_digest(shapes: &[PhysicalWorldShapeDefinition]) -> String {
    let mut digest = Sha256::new();
    for shape in shapes {
        for label in [&shape.identifier, &shape.collision_role] {
            digest.update((label.len() as u64).to_le_bytes());
            digest.update(label.as_bytes());
        }
        digest.update(shape.surface_code.to_le_bytes());
        digest.update((shape.vertices_world_metres.len() as u64).to_le_bytes());
        for coordinate in shape.vertices_world_metres.iter().flatten() { digest.update(coordinate.to_le_bytes()); }
        digest.update((shape.triangle_indices.len() as u64).to_le_bytes());
        for index in shape.triangle_indices.iter().flatten() { digest.update(index.to_le_bytes()); }
    }
    format!("{digest:x}", digest = digest.finalize())
}

impl PhysicalCollisionWorld {
    pub fn from_package(package: PhysicalWorldPackage, repository_root: &Path) -> Result<Self, String> {
        if package.schema_version != 1 || package.coordinate_contract != "right_positive_x_up_positive_y_forward_negative_z_metres" {
            return Err("Unsupported physical-world package schema or coordinate contract".into());
        }
        if physical_geometry_digest(&package.shapes) != package.geometry_digest {
            return Err("Physical package geometry digest does not match its contents".into());
        }
        let source_path = Path::new(&package.source_path);
        if source_path.is_absolute() || source_path.components().any(|part| matches!(part, std::path::Component::ParentDir)) {
            return Err("Physical source path must stay within the repository".into());
        }
        let bytes = std::fs::read(repository_root.join(source_path)).map_err(|error| format!("Cannot verify physical source: {error}"))?;
        if format!("{:x}", Sha256::digest(bytes)) != package.source_digest {
            return Err("Stale physical world geometry: source digest does not match".into());
        }
        let mut names = HashSet::new();
        let mut shapes = Vec::new();
        for definition in package.shapes {
            if definition.identifier.is_empty() || !names.insert(definition.identifier.clone())
                || !matches!(definition.collision_role.as_str(), "driveable" | "solid")
                || definition.surface_code > 7 || definition.triangle_indices.is_empty()
                || definition.vertices_world_metres.iter().flatten().any(|value| !value.is_finite())
                || definition.triangle_indices.iter().flatten().any(|index| *index as usize >= definition.vertices_world_metres.len())
            { return Err(format!("Invalid physical shape: {}", definition.identifier)); }
            let vertices: Vec<_> = definition.vertices_world_metres.iter().map(|position| Point3::new(position[0], position[1], position[2])).collect();
            let wheel_impact_triangles = definition.triangle_indices.iter().copied().filter(|indices| {
                let normal = (vertices[indices[1] as usize] - vertices[indices[0] as usize])
                    .cross(&(vertices[indices[2] as usize] - vertices[indices[0] as usize]));
                normal.norm() > 1e-12 && normal.y.abs() < normal.norm() * 0.5
            }).collect::<Vec<_>>();
            let wheel_impact_shape = if definition.collision_role == "driveable" && !wheel_impact_triangles.is_empty() {
                Some(SharedShape::new(TriMesh::new(vertices.clone(), wheel_impact_triangles)
                    .map_err(|error| format!("Invalid wheel impact triangle mesh: {error:?}"))?))
            } else { None };
            let shape = TriMesh::new(vertices, definition.triangle_indices).map_err(|error| format!("Invalid physical triangle mesh: {error:?}"))?;
            shapes.push(PhysicalWorldShape { identifier: definition.identifier,
                surface_code: definition.surface_code, driveable: definition.collision_role == "driveable", shape: SharedShape::new(shape), wheel_impact_shape });
        }
        if shapes.is_empty() { return Err("Physical collision world has no shapes".into()); }
        shapes.sort_by(|first, second| first.identifier.cmp(&second.identifier));
        Ok(Self { source_digest: package.source_digest, shapes })
    }

    pub fn raycast_driveable(&self, origin: Vec3, direction: Vec3, length: f64) -> Result<Option<PhysicalWorldRayHit>, String> {
        if !vector_is_finite(origin) || !vector_is_finite(direction) || (direction.length() - 1.0).abs() > 1e-8 || !length.is_finite() || length <= 0.0 {
            return Err("Invalid physical-world ray query".into());
        }
        let ray = Ray::new(Point3::from(physical_vector(origin)), physical_vector(direction));
        let mut nearest: Option<PhysicalWorldRayHit> = None;
        for (shape_index, shape) in self.shapes.iter().enumerate().filter(|(_, shape)| shape.driveable) {
            if let Some(hit) = shape.shape.cast_ray_and_get_normal(&Isometry3::identity(), &ray, length, false) {
                if nearest.is_none_or(|previous| hit.time_of_impact < previous.distance_metres) {
                    let mut normal = vehicle_vector(hit.normal);
                    if normal.dot(direction) > 0.0 { normal = -normal; }
                    nearest = Some(PhysicalWorldRayHit { point_world_metres: origin + direction * hit.time_of_impact,
                        normal_world: normal, distance_metres: hit.time_of_impact, surface_code: shape.surface_code,
                        shape_index, triangle_identifier: match hit.feature {
                            parry3d_f64::shape::FeatureId::Face(identifier) => identifier,
                            _ => return Err("Triangle query returned an unsupported feature identity".into()),
                        } });
                }
            }
        }
        Ok(nearest)
    }

    pub fn contacts(&self, vehicle_pose: &Isometry3<f64>, vehicle_shape: &SharedShape,
        prediction_metres: f64, wheel_tread: bool) -> Result<Vec<PhysicalShapeContact>, String> {
        let mut contacts = Vec::new();
        let bounds = vehicle_shape.compute_aabb(vehicle_pose).loosened(prediction_metres);
        for (shape_index, shape) in self.shapes.iter().enumerate() {
            if !bounds.intersects(&shape.shape.compute_aabb(&Isometry3::identity())) { continue; }
            let collision_shape = if wheel_tread && shape.driveable {
                let Some(impact_shape) = &shape.wheel_impact_shape else { continue; };
                impact_shape
            } else { &shape.shape };
            let result = contact(&Isometry3::identity(), collision_shape.as_ref(), vehicle_pose, vehicle_shape.as_ref(), prediction_metres)
                .map_err(|_| format!("Unsupported collision pair with {}", shape.identifier))?;
            if let Some(result) = result {
                contacts.push(PhysicalShapeContact { point_on_world_metres: vehicle_vector(result.point1.coords),
                    point_on_vehicle_metres: vehicle_vector(result.point2.coords),
                    normal_toward_vehicle_world: vehicle_vector(*result.normal1),
                    signed_distance_metres: result.dist, surface_code: shape.surface_code, shape_index });
            }
        }
        Ok(contacts)
    }

    pub fn earliest_translation_impact(&self, vehicle_pose: &Isometry3<f64>, vehicle_shape: &SharedShape,
        velocity_world: Vec3, duration_seconds: f64, wheel_tread: bool) -> Result<Option<f64>, String> {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 || !vector_is_finite(velocity_world) {
            return Err("Invalid physical shape sweep".into());
        }
        let mut earliest: Option<f64> = None;
        for shape in &self.shapes {
            let collision_shape = if wheel_tread && shape.driveable {
                let Some(impact_shape) = &shape.wheel_impact_shape else { continue; };
                impact_shape
            } else { &shape.shape };
            let options = ShapeCastOptions { max_time_of_impact: duration_seconds,
                stop_at_penetration: false, ..Default::default() };
            if let Some(hit) = cast_shapes(&Isometry3::identity(), &Vector3::zeros(), collision_shape.as_ref(),
                vehicle_pose, &physical_vector(velocity_world), vehicle_shape.as_ref(), options).map_err(|_| "Unsupported continuous collision query")? {
                if hit.time_of_impact > 1e-10 && earliest.is_none_or(|previous| hit.time_of_impact < previous) {
                    earliest = Some(hit.time_of_impact);
                }
            }
        }
        Ok(earliest)
    }
}
