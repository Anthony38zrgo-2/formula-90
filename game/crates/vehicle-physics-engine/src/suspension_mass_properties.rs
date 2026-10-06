use crate::types::{Mat3, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InertiaTensor {
    pub entries_kilogram_square_metres: [[f64; 3]; 3],
}

impl InertiaTensor {
    pub fn is_point_mass(&self) -> bool {
        self.entries_kilogram_square_metres
            .iter()
            .flatten()
            .all(|entry| *entry == 0.0)
    }
    pub fn validate(&self) -> Result<(), String> {
        let entries = self.entries_kilogram_square_metres;
        for row in 0..3 {
            for column in 0..3 {
                if !entries[row][column].is_finite() {
                    return Err("Inertia tensor contains nonfinite values".into());
                }
                if (entries[row][column] - entries[column][row]).abs() > 1e-10 {
                    return Err("Inertia tensor must be symmetric".into());
                }
            }
        }
        let mut lower_triangle = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..=row {
                let remainder = entries[row][column]
                    - (0..column)
                        .map(|previous| {
                            lower_triangle[row][previous] * lower_triangle[column][previous]
                        })
                        .sum::<f64>();
                if row == column {
                    if remainder <= 0.0 {
                        return Err("Inertia tensor must be positive definite".into());
                    }
                    lower_triangle[row][column] = remainder.sqrt();
                } else {
                    lower_triangle[row][column] = remainder / lower_triangle[column][column];
                }
            }
        }
        let half_trace = 0.5 * (entries[0][0] + entries[1][1] + entries[2][2]);
        let mut second_moment = [[0.0; 3]; 3];
        let tolerance = 1e-10 * half_trace.max(1.0);
        for row in 0..3 {
            for column in 0..3 {
                second_moment[row][column] =
                    if row == column { half_trace } else { 0.0 } - entries[row][column];
            }
            if second_moment[row][row] < -tolerance {
                return Err("Inertia tensor violates physical moment inequalities".into());
            }
        }
        for row in 0..3 {
            for column in row + 1..3 {
                if second_moment[row][row] * second_moment[column][column]
                    - second_moment[row][column].powi(2)
                    < -tolerance * half_trace
                {
                    return Err("Inertia tensor has an impossible second moment".into());
                }
            }
        }
        let determinant = second_moment[0][0]
            * (second_moment[1][1] * second_moment[2][2] - second_moment[1][2].powi(2))
            - second_moment[0][1]
                * (second_moment[0][1] * second_moment[2][2]
                    - second_moment[0][2] * second_moment[1][2])
            + second_moment[0][2]
                * (second_moment[0][1] * second_moment[1][2]
                    - second_moment[0][2] * second_moment[1][1]);
        if determinant < -tolerance * half_trace.powi(2) {
            return Err("Inertia tensor has an impossible second moment determinant".into());
        }
        Ok(())
    }

    pub fn multiply(&self, vector: Vec3) -> Vec3 {
        let entries = self.entries_kilogram_square_metres;
        Vec3::new(
            entries[0][0] * vector.x + entries[0][1] * vector.y + entries[0][2] * vector.z,
            entries[1][0] * vector.x + entries[1][1] * vector.y + entries[1][2] * vector.z,
            entries[2][0] * vector.x + entries[2][1] * vector.y + entries[2][2] * vector.z,
        )
    }

    pub fn rotated(&self, rotation: Mat3) -> Self {
        let axes = [Vec3::RIGHT, Vec3::UP, Vec3::BACK];
        let mut entries = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                entries[row][column] = rotation
                    .inverse_transform_vector(axes[row])
                    .dot(self.multiply(rotation.inverse_transform_vector(axes[column])));
            }
        }
        Self {
            entries_kilogram_square_metres: entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MassPropertyOrigin {
    Measured,
    Documented,
    UserDeclared,
    ProfileDeclared,
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MassPropertyProvenance {
    pub mass_origin: MassPropertyOrigin,
    pub center_of_mass_origin: MassPropertyOrigin,
    pub inertia_origin: MassPropertyOrigin,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MassComponentAttachment {
    Body,
    Upright { wheel: WheelIndex },
    WheelRotor { wheel: WheelIndex },
    LowerWishbone { wheel: WheelIndex },
    UpperWishbone { wheel: WheelIndex },
    TrackRod { wheel: WheelIndex },
    ActuationRod { wheel: WheelIndex },
    Rocker { wheel: WheelIndex },
    DamperCylinder { wheel: WheelIndex },
    DamperPiston { wheel: WheelIndex },
    Driveshaft { wheel: WheelIndex },
    SteeringRack,
}

impl MassComponentAttachment {
    pub fn wheel(&self) -> Option<WheelIndex> {
        match self {
            Self::Body | Self::SteeringRack => None,
            Self::Upright { wheel }
            | Self::WheelRotor { wheel }
            | Self::LowerWishbone { wheel }
            | Self::UpperWishbone { wheel }
            | Self::TrackRod { wheel }
            | Self::ActuationRod { wheel }
            | Self::Rocker { wheel }
            | Self::DamperCylinder { wheel }
            | Self::DamperPiston { wheel }
            | Self::Driveshaft { wheel } => Some(*wheel),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionMassComponent {
    pub name: String,
    pub mass_kilograms: f64,
    pub attachment: MassComponentAttachment,
    pub center_of_mass_in_attachment_metres: Vec3,
    pub inertia_about_center_of_mass: InertiaTensor,
    pub provenance: MassPropertyProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionMassInventory {
    pub schema_version: u32,
    pub declared_complete_dry_mass_kilograms: f64,
    pub components: Vec<SuspensionMassComponent>,
}

impl SuspensionMassInventory {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported suspension mass inventory version".into());
        }
        if !self.declared_complete_dry_mass_kilograms.is_finite()
            || self.declared_complete_dry_mass_kilograms <= 0.0
        {
            return Err("Declared complete dry mass must be finite and positive".into());
        }
        let mut names = HashSet::new();
        let mut total_mass = 0.0;
        let mut body_present = false;
        let mut upright_present = [false; 4];
        for component in &self.components {
            if component.name.trim().is_empty() || !names.insert(&component.name) {
                return Err("Component names must be nonempty and unique".into());
            }
            if component.provenance.source.trim().is_empty() {
                return Err(format!("Missing provenance source for {}", component.name));
            }
            if !component.mass_kilograms.is_finite() || component.mass_kilograms <= 0.0 {
                return Err(format!("Invalid mass for {}", component.name));
            }
            if !vector_is_finite(component.center_of_mass_in_attachment_metres) {
                return Err(format!("Invalid center of mass for {}", component.name));
            }
            component
                .inertia_about_center_of_mass
                .validate()
                .map_err(|error| format!("{}: {error}", component.name))?;
            total_mass += component.mass_kilograms;
            match component.attachment {
                MassComponentAttachment::Body => body_present = true,
                MassComponentAttachment::Upright { wheel } => {
                    upright_present[wheel as usize] = true
                }
                _ => {}
            }
        }
        if !body_present || upright_present.iter().any(|present| !present) {
            return Err("Inventory requires a body and all four upright assemblies".into());
        }
        if !total_mass.is_finite()
            || (total_mass - self.declared_complete_dry_mass_kilograms).abs()
                > 0.000001
        {
            return Err("Component masses do not close the declared dry mass budget".into());
        }
        Ok(())
    }
}

pub fn vector_is_finite(vector: Vec3) -> bool {
    vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
}

pub fn composite_mass_properties(
    components: &[(f64, Vec3, InertiaTensor)],
) -> Result<(f64, Vec3, InertiaTensor), String> {
    let mut total_mass = 0.0;
    let mut weighted_center = Vec3::ZERO;
    for (mass, center, inertia) in components {
        if !mass.is_finite() || *mass <= 0.0 || !vector_is_finite(*center) {
            return Err("Invalid component in composite mass properties".into());
        }
        if !inertia.is_point_mass() {
            inertia.validate()?;
        }
        total_mass += mass;
        weighted_center += *center * *mass;
    }
    if !total_mass.is_finite() || total_mass <= 0.0 {
        return Err("Composite body requires finite positive mass".into());
    }
    let center = weighted_center / total_mass;
    let mut entries = [[0.0; 3]; 3];
    for (mass, component_center, inertia) in components {
        let offset = *component_center - center;
        let coordinates = [offset.x, offset.y, offset.z];
        for row in 0..3 {
            for column in 0..3 {
                entries[row][column] += inertia.entries_kilogram_square_metres[row][column]
                    + mass
                        * (if row == column {
                            offset.length_squared()
                        } else {
                            0.0
                        } - coordinates[row] * coordinates[column]);
            }
        }
    }
    Ok((
        total_mass,
        center,
        InertiaTensor {
            entries_kilogram_square_metres: entries,
        },
    ))
}
