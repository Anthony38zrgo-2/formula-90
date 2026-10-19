use crate::coupled_vehicle_dynamics::{
    GeneralizedVehicleMatrix, GeneralizedVehicleVelocity, VEHICLE_GENERALIZED_VELOCITY_COUNT,
};
use crate::suspension_mass_properties::vector_is_finite;
use crate::types::{Quat, Vec3};
use parry3d_f64::shape::SharedShape;
use serde::{Deserialize, Serialize};

pub const DYNAMIC_WORLD_OBJECT_SCHEMA_VERSION: u32 = 1;
pub const DYNAMIC_WORLD_OBJECT_FREE_VELOCITY_COORDINATE_COUNT: usize = 6;
pub const DYNAMIC_WORLD_OBJECT_SLEEP_LINEAR_SPEED_METRES_PER_SECOND: f64 = 0.02;
pub const DYNAMIC_WORLD_OBJECT_SLEEP_ANGULAR_SPEED_RADIANS_PER_SECOND: f64 = 0.05;
pub const DYNAMIC_WORLD_OBJECT_SLEEP_DELAY_SECONDS: f64 = 0.5;
pub const DYNAMIC_WORLD_OBJECT_WAKE_RELATIVE_SPEED_METRES_PER_SECOND: f64 = 0.05;
pub const DYNAMIC_WORLD_OBJECT_SUPPORT_NORMAL_MINIMUM_Y: f64 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicWorldObjectClass {
    TrackMarkerCone,
    PortableBarrier,
    LegacyGodotOwnedVehicle,
}

impl DynamicWorldObjectClass {
    pub fn uses_shared_rust_dynamics(self) -> bool {
        matches!(self, Self::TrackMarkerCone | Self::PortableBarrier)
    }

    pub fn rejection_reason(self) -> Option<&'static str> {
        match self {
            Self::LegacyGodotOwnedVehicle => Some(
                "Legacy Godot-owned vehicles remain on the explicit legacy route and cannot participate in shared physical contact",
            ),
            Self::TrackMarkerCone | Self::PortableBarrier => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DynamicWorldObjectShapeDefinition {
    Cuboid { half_extents_metres: Vec3 },
    Cylinder { half_height_metres: f64, radius_metres: f64 },
    Sphere { radius_metres: f64 },
}

impl DynamicWorldObjectShapeDefinition {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Cuboid { half_extents_metres } => {
                if !vector_is_finite(*half_extents_metres)
                    || [half_extents_metres.x, half_extents_metres.y, half_extents_metres.z]
                        .iter()
                        .any(|extent| *extent <= 0.0)
                {
                    return Err("Dynamic object cuboid requires finite positive half extents".into());
                }
            }
            Self::Cylinder { half_height_metres, radius_metres } => {
                if !half_height_metres.is_finite()
                    || !radius_metres.is_finite()
                    || *half_height_metres <= 0.0
                    || *radius_metres <= 0.0
                {
                    return Err("Dynamic object cylinder requires a finite positive half height and radius".into());
                }
            }
            Self::Sphere { radius_metres } => {
                if !radius_metres.is_finite() || *radius_metres <= 0.0 {
                    return Err("Dynamic object sphere requires a finite positive radius".into());
                }
            }
        }
        Ok(())
    }

    pub fn shared_shape(&self) -> SharedShape {
        match self {
            Self::Cuboid { half_extents_metres } => {
                SharedShape::cuboid(half_extents_metres.x, half_extents_metres.y, half_extents_metres.z)
            }
            Self::Cylinder { half_height_metres, radius_metres } => {
                SharedShape::cylinder(*half_height_metres, *radius_metres)
            }
            Self::Sphere { radius_metres } => SharedShape::ball(*radius_metres),
        }
    }

    pub fn inertia_diagonal_kilogram_square_metres(&self, mass_kilograms: f64) -> Vec3 {
        match self {
            Self::Cuboid { half_extents_metres } => {
                let third = mass_kilograms / 3.0;
                Vec3::new(
                    third * (half_extents_metres.y * half_extents_metres.y
                        + half_extents_metres.z * half_extents_metres.z),
                    third * (half_extents_metres.x * half_extents_metres.x
                        + half_extents_metres.z * half_extents_metres.z),
                    third * (half_extents_metres.x * half_extents_metres.x
                        + half_extents_metres.y * half_extents_metres.y),
                )
            }
            Self::Cylinder { half_height_metres, radius_metres } => {
                let transverse = mass_kilograms
                    * (3.0 * radius_metres * radius_metres + 4.0 * half_height_metres * half_height_metres)
                    / 12.0;
                Vec3::new(
                    transverse,
                    0.5 * mass_kilograms * radius_metres * radius_metres,
                    transverse,
                )
            }
            Self::Sphere { radius_metres } => {
                let inertia = 0.4 * mass_kilograms * radius_metres * radius_metres;
                Vec3::new(inertia, inertia, inertia)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicWorldObjectDefinition {
    pub schema_version: u32,
    pub identifier: String,
    pub object_class: DynamicWorldObjectClass,
    pub shape: DynamicWorldObjectShapeDefinition,
    pub mass_kilograms: f64,
    pub surface_code: u32,
    #[serde(default)]
    pub linear_damping_per_second: f64,
    #[serde(default)]
    pub angular_damping_per_second: f64,
}

impl DynamicWorldObjectDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(reason) = self.object_class.rejection_reason() {
            return Err(reason.into());
        }
        if self.schema_version != DYNAMIC_WORLD_OBJECT_SCHEMA_VERSION {
            return Err("Unsupported dynamic world object schema version".into());
        }
        if self.identifier.trim().is_empty() || self.identifier.len() > 128 {
            return Err("Dynamic world object requires a non-empty bounded identifier".into());
        }
        if !self.mass_kilograms.is_finite() || self.mass_kilograms <= 0.0 {
            return Err("Dynamic world object requires finite positive mass".into());
        }
        if self.surface_code > 7 {
            return Err("Dynamic world object surface code is outside the declared domain".into());
        }
        if !self.linear_damping_per_second.is_finite()
            || !self.angular_damping_per_second.is_finite()
            || self.linear_damping_per_second < 0.0
            || self.angular_damping_per_second < 0.0
            || self.linear_damping_per_second > 100.0
            || self.angular_damping_per_second > 100.0
        {
            return Err("Dynamic world object damping must be finite and bounded".into());
        }
        self.shape.validate()?;
        let shape_is_supported = match self.object_class {
            DynamicWorldObjectClass::TrackMarkerCone => {
                matches!(self.shape, DynamicWorldObjectShapeDefinition::Cylinder { .. })
            }
            DynamicWorldObjectClass::PortableBarrier => {
                matches!(self.shape, DynamicWorldObjectShapeDefinition::Cuboid { .. })
            }
            DynamicWorldObjectClass::LegacyGodotOwnedVehicle => false,
        };
        if !shape_is_supported {
            return Err("Dynamic world object class and collision proxy shape are incompatible".into());
        }
        let inertia = self.shape.inertia_diagonal_kilogram_square_metres(self.mass_kilograms);
        if [inertia.x, inertia.y, inertia.z].iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return Err("Dynamic world object requires finite positive principal inertia".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicWorldObjectState {
    pub position_world_metres: Vec3,
    pub orientation: Quat,
    pub linear_velocity_world_metres_per_second: Vec3,
    pub angular_velocity_world_radians_per_second: Vec3,
    pub collision_energy_change_joules: f64,
    pub position_correction_energy_joules: f64,
    pub sleeping: bool,
    pub low_motion_time_seconds: f64,
}

impl DynamicWorldObjectState {
    pub fn at_rest(position_world_metres: Vec3, orientation: Quat) -> Self {
        Self {
            position_world_metres,
            orientation,
            linear_velocity_world_metres_per_second: Vec3::ZERO,
            angular_velocity_world_radians_per_second: Vec3::ZERO,
            collision_energy_change_joules: 0.0,
            position_correction_energy_joules: 0.0,
            sleeping: false,
            low_motion_time_seconds: 0.0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !vector_is_finite(self.position_world_metres)
            || !vector_is_finite(self.linear_velocity_world_metres_per_second)
            || !vector_is_finite(self.angular_velocity_world_radians_per_second)
            || !self.collision_energy_change_joules.is_finite()
            || !self.position_correction_energy_joules.is_finite()
            || !self.low_motion_time_seconds.is_finite()
            || self.low_motion_time_seconds < 0.0
        {
            return Err("Dynamic world object state requires finite pose, velocity and energy values".into());
        }
        if !self.orientation.x.is_finite()
            || !self.orientation.y.is_finite()
            || !self.orientation.z.is_finite()
            || !self.orientation.w.is_finite()
            || self.orientation.length_squared() < 1e-12
        {
            return Err("Dynamic world object state requires a finite non-degenerate orientation".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DynamicWorldObjectSnapshot {
    pub entity_identifier: u64,
    pub definition_identifier: String,
    pub object_class: DynamicWorldObjectClass,
    pub state: DynamicWorldObjectState,
    pub mass_kilograms: f64,
    pub surface_code: u32,
}

#[derive(Debug, Clone)]
pub struct DynamicWorldObject {
    pub definition: DynamicWorldObjectDefinition,
    pub state: DynamicWorldObjectState,
    pub spawn_position_world_metres: Vec3,
    pub spawn_orientation: Quat,
    shape: SharedShape,
    inertia_diagonal_kilogram_square_metres: Vec3,
}

impl DynamicWorldObject {
    pub fn new(
        definition: DynamicWorldObjectDefinition,
        position_world_metres: Vec3,
        orientation: Quat,
    ) -> Result<Self, String> {
        definition.validate()?;
        if !vector_is_finite(position_world_metres) {
            return Err("Dynamic world object requires a finite spawn position".into());
        }
        if !orientation.x.is_finite()
            || !orientation.y.is_finite()
            || !orientation.z.is_finite()
            || !orientation.w.is_finite()
            || orientation.length_squared() < 1e-12
        {
            return Err("Dynamic world object requires a finite non-degenerate spawn orientation".into());
        }
        let normalized_orientation = orientation.normalized();
        let inertia_diagonal_kilogram_square_metres =
            definition.shape.inertia_diagonal_kilogram_square_metres(definition.mass_kilograms);
        let shape = definition.shape.shared_shape();
        let object = Self {
            definition,
            state: DynamicWorldObjectState::at_rest(position_world_metres, normalized_orientation),
            spawn_position_world_metres: position_world_metres,
            spawn_orientation: normalized_orientation,
            shape,
            inertia_diagonal_kilogram_square_metres,
        };
        object.state.validate()?;
        Ok(object)
    }

    pub fn shape_identifier(&self, entity_identifier: u64) -> String {
        format!("dynamic_object_{entity_identifier:020}_{}", self.definition.identifier)
    }

    pub fn shared_shape(&self) -> &SharedShape {
        &self.shape
    }

    pub fn mass_matrix(&self) -> Result<GeneralizedVehicleMatrix, String> {
        let mass = self.definition.mass_kilograms;
        if !mass.is_finite() || mass <= 0.0 {
            return Err("Dynamic world object mass matrix requires positive finite mass".into());
        }
        let mut matrix = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        for axis in 0..3 {
            matrix[axis][axis] = mass;
        }
        for coordinate in DYNAMIC_WORLD_OBJECT_FREE_VELOCITY_COORDINATE_COUNT..VEHICLE_GENERALIZED_VELOCITY_COUNT {
            matrix[coordinate][coordinate] = 1.0;
        }
        let inertia_tensor = self.inertia_tensor_world();
        for row in 0..3 {
            for column in 0..3 {
                matrix[3 + row][3 + column] = inertia_tensor[row][column];
            }
        }
        for coordinate in DYNAMIC_WORLD_OBJECT_FREE_VELOCITY_COORDINATE_COUNT..VEHICLE_GENERALIZED_VELOCITY_COUNT {
            if matrix[coordinate][coordinate] <= 0.0 || !matrix[coordinate][coordinate].is_finite() {
                return Err("Dynamic world object mass matrix is not positive definite".into());
            }
        }
        Ok(matrix)
    }

    pub fn velocity(&self) -> GeneralizedVehicleVelocity {
        let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        let linear = self.state.linear_velocity_world_metres_per_second;
        let angular = self.state.angular_velocity_world_radians_per_second;
        velocity[0] = linear.x;
        velocity[1] = linear.y;
        velocity[2] = linear.z;
        velocity[3] = angular.x;
        velocity[4] = angular.y;
        velocity[5] = angular.z;
        velocity
    }

    pub fn set_velocity(&mut self, velocity: GeneralizedVehicleVelocity) -> Result<(), String> {
        if velocity.iter().any(|value| !value.is_finite()) {
            return Err("Dynamic world object velocity must be finite".into());
        }
        self.state.linear_velocity_world_metres_per_second = Vec3::new(velocity[0], velocity[1], velocity[2]);
        self.state.angular_velocity_world_radians_per_second = Vec3::new(velocity[3], velocity[4], velocity[5]);
        Ok(())
    }

    pub fn contact_jacobian(&self, point_world_metres: Vec3, axis_world: Vec3) -> GeneralizedVehicleVelocity {
        let mut jacobian = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        let arm = point_world_metres - self.state.position_world_metres;
        let angular = arm.cross(axis_world);
        jacobian[0] = axis_world.x;
        jacobian[1] = axis_world.y;
        jacobian[2] = axis_world.z;
        jacobian[3] = angular.x;
        jacobian[4] = angular.y;
        jacobian[5] = angular.z;
        jacobian
    }

    pub fn advance(&mut self, duration_seconds: f64, gravity_world: Vec3) -> Result<(), String> {
        if !duration_seconds.is_finite() || duration_seconds < 0.0 || !vector_is_finite(gravity_world) {
            return Err("Dynamic world object advance requires a finite duration and gravity".into());
        }
        if duration_seconds == 0.0 || self.state.sleeping {
            return Ok(());
        }
        self.state.linear_velocity_world_metres_per_second += gravity_world * duration_seconds;
        if self.definition.linear_damping_per_second > 0.0 {
            self.state.linear_velocity_world_metres_per_second *=
                (-self.definition.linear_damping_per_second * duration_seconds).exp();
        }
        if self.definition.angular_damping_per_second > 0.0 {
            self.state.angular_velocity_world_radians_per_second *=
                (-self.definition.angular_damping_per_second * duration_seconds).exp();
        }
        self.state.position_world_metres +=
            self.state.linear_velocity_world_metres_per_second * duration_seconds;
        let local_angular_velocity = self
            .state
            .orientation
            .to_mat3()
            .inverse_transform_vector(self.state.angular_velocity_world_radians_per_second);
        self.state.orientation = self
            .state
            .orientation
            .integrate_angular_velocity(local_angular_velocity, duration_seconds)
            .normalized();
        self.state.validate()
    }

    pub fn apply_position_correction(&mut self, correction: GeneralizedVehicleVelocity) -> Result<(), String> {
        if correction.iter().any(|value| !value.is_finite()) {
            return Err("Dynamic world object position correction must be finite".into());
        }
        self.state.position_world_metres += Vec3::new(correction[0], correction[1], correction[2]);
        let angular_correction_world = Vec3::new(correction[3], correction[4], correction[5]);
        let angular_correction_local = self
            .state
            .orientation
            .to_mat3()
            .inverse_transform_vector(angular_correction_world);
        self.state.orientation = self
            .state
            .orientation
            .integrate_angular_velocity(angular_correction_local, 1.0)
            .normalized();
        self.state.validate()
    }

    pub fn mechanical_energy_joules(&self, gravity_world: Vec3) -> f64 {
        let linear = self.state.linear_velocity_world_metres_per_second;
        let angular = self.state.angular_velocity_world_radians_per_second;
        let inertia_tensor = self.inertia_tensor_world();
        let angular_momentum = [
            inertia_tensor[0][0] * angular.x + inertia_tensor[0][1] * angular.y + inertia_tensor[0][2] * angular.z,
            inertia_tensor[1][0] * angular.x + inertia_tensor[1][1] * angular.y + inertia_tensor[1][2] * angular.z,
            inertia_tensor[2][0] * angular.x + inertia_tensor[2][1] * angular.y + inertia_tensor[2][2] * angular.z,
        ];
        0.5 * self.definition.mass_kilograms * linear.dot(linear)
            + 0.5 * (angular.x * angular_momentum[0] + angular.y * angular_momentum[1] + angular.z * angular_momentum[2])
            - self.definition.mass_kilograms * gravity_world.dot(self.state.position_world_metres)
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.state = DynamicWorldObjectState::at_rest(self.spawn_position_world_metres, self.spawn_orientation);
        self.state.validate()
    }

    pub fn sleep(&mut self) {
        self.state.sleeping = true;
        self.state.low_motion_time_seconds = DYNAMIC_WORLD_OBJECT_SLEEP_DELAY_SECONDS;
        self.state.linear_velocity_world_metres_per_second = Vec3::ZERO;
        self.state.angular_velocity_world_radians_per_second = Vec3::ZERO;
    }

    pub fn wake(&mut self) {
        self.state.sleeping = false;
        self.state.low_motion_time_seconds = 0.0;
    }

    pub fn snapshot(&self, entity_identifier: u64) -> DynamicWorldObjectSnapshot {
        DynamicWorldObjectSnapshot {
            entity_identifier,
            definition_identifier: self.definition.identifier.clone(),
            object_class: self.definition.object_class,
            state: self.state.clone(),
            mass_kilograms: self.definition.mass_kilograms,
            surface_code: self.definition.surface_code,
        }
    }

    fn inertia_tensor_world(&self) -> [[f64; 3]; 3] {
        let rotation = self.state.orientation.to_mat3();
        let axes = [
            [rotation.x.x, rotation.x.y, rotation.x.z],
            [rotation.y.x, rotation.y.y, rotation.y.z],
            [rotation.z.x, rotation.z.y, rotation.z.z],
        ];
        let diagonal = [
            self.inertia_diagonal_kilogram_square_metres.x,
            self.inertia_diagonal_kilogram_square_metres.y,
            self.inertia_diagonal_kilogram_square_metres.z,
        ];
        let mut tensor = [[0.0; 3]; 3];
        for axis_index in 0..3 {
            for row in 0..3 {
                for column in 0..3 {
                    tensor[row][column] +=
                        diagonal[axis_index] * axes[axis_index][row] * axes[axis_index][column];
                }
            }
        }
        tensor
    }
}
