use super::{geometric_damper_element, PreparedGeometricSuspension};
use crate::suspension_geo_config::{AntiRollConfig, GeometricSuspensionConfig};
use crate::suspension_kinematics::{jacobian, solve_corner, KinematicSolution};
use crate::types::{Mat3, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum SuspensionBackend {
    #[default]
    Standard,
    Simplified,
}

impl SuspensionBackend {
    pub(super) fn effective_antiroll_configuration(
        self,
        geometric_configuration: AntiRollConfig,
        axle_ratio: f64,
    ) -> AntiRollConfig {
        match (self, geometric_configuration) {
            (Self::Simplified, AntiRollConfig::LegacyRatio { .. }) => {
                AntiRollConfig::LegacyRatio { ratio: axle_ratio }
            }
            (_, configuration) => configuration,
        }
    }

    pub fn from_execution_environment() -> Self {
        match std::env::var("FORMULA90_SUSPENSION_BACKEND").as_deref() {
            Ok("simplified") => Self::Simplified,
            Ok("standard") | Err(_) => Self::Standard,
            Ok(value) => panic!("Unsupported suspension backend: {value}"),
        }
    }

    pub(super) fn contact_hub_offset_world_metres(
        self,
        _body_orientation: Mat3,
        hub_lateral_metres: f64,
        hub_long_metres: f64,
    ) -> Vec3 {
        let hub_offset_local = Vec3::new(hub_lateral_metres, 0.0, hub_long_metres);
        match self {
            Self::Simplified => Vec3::ZERO,
            Self::Standard => hub_offset_local,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EquivalentWheelParameters {
    reference_spring_compression_metres: f64,
    motion_ratio: f64,
}

#[derive(Debug, Clone)]
pub(super) struct SimplifiedSuspension {
    corners: [EquivalentWheelParameters; 4],
}

impl SimplifiedSuspension {
    pub(super) fn build(
        geometry: &GeometricSuspensionConfig,
        _prepared: &PreparedGeometricSuspension,
    ) -> Self {
        Self {
            corners: std::array::from_fn(|index| {
                let wheel = WheelIndex::ALL[index];
                let corner = geometry.corners.get(wheel);
                let reference_solution = solve_corner(corner, 0.0, 0.0, wheel.is_front(), 0.0, 0.0, 1.0)
                    .expect("Equivalent suspension requires a valid reference pose");
                let motion_ratio = jacobian(corner, 0.0, 0.0, wheel.is_front())
                    .expect("Equivalent suspension requires a valid reference transmission ratio")
                    .motion_ratio;
                assert!(motion_ratio.is_finite() && motion_ratio > 0.0,
                    "Equivalent suspension requires a positive finite transmission ratio");
                EquivalentWheelParameters { reference_spring_compression_metres: reference_solution.damper_compression, motion_ratio }
            }),
        }
    }

    pub(super) fn vertical_reference_at(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<(f64, f64)> {
        if !travel_metres.is_finite() { return None; }
        let parameters = self.corners[wheel as usize];
        Some((parameters.reference_spring_compression_metres
            + parameters.motion_ratio * travel_metres, parameters.motion_ratio))
    }

    pub(super) fn forces_at(
        &self,
        geometry: &GeometricSuspensionConfig,
        wheel: WheelIndex,
        travel_metres: f64,
        velocity_metres_per_second: f64,
    ) -> Option<(f64, f64, f64, f64, f64, bool, f64, Option<KinematicSolution>)> {
        if !velocity_metres_per_second.is_finite() { return None; }
        let (spring_compression_metres, motion_ratio) = self.vertical_reference_at(wheel, travel_metres)?;
        let axle = geometry.axle(wheel);
        let deflection = axle.spring_free_length_m - axle.spring_installed_length_m
            + spring_compression_metres;
        let spring = axle.spring_rate_N_per_m * deflection.max(0.0);
        let damping = geometric_damper_element(axle, motion_ratio * velocity_metres_per_second);
        Some((spring * motion_ratio, damping * motion_ratio, spring_compression_metres,
            motion_ratio, 0.0, deflection <= 0.0, 0.0, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspension::{SuspensionSystem, rest_compression_m};
    use crate::types::{RaycastHit, TriRaycastSample};
    use crate::vehicle_configuration::load_vehicle_configuration_from_authorized_root;

    #[test]
    fn equivalent_vertical_steps_never_solve_mechanical_geometry() {
        let repository_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let configuration = load_vehicle_configuration_from_authorized_root(
            &repository_root.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
            &repository_root).unwrap();
        let mut suspension = SuspensionSystem::new(&configuration);
        suspension.select_backend(&configuration, SuspensionBackend::Simplified);
        let samples = std::array::from_fn(|index| {
            let wheel = WheelIndex::ALL[index];
            let spring_length = super::super::spring_length(&configuration, wheel);
            let tire_radius = super::super::tire_radius(&configuration, wheel);
            let hit = RaycastHit {
                is_colliding: true,
                distance: spring_length + tire_radius - rest_compression_m(&configuration, wheel),
                point: Vec3::new(index as f64, 0.0, 0.0),
                normal: Vec3::UP,
                ..Default::default()
            };
            TriRaycastSample { inner: hit, center: hit, outer: hit }
        });
        crate::suspension_kinematics::counters::reset();
        for _ in 0..120 { suspension.step(&configuration, &samples, 1.0 / 120.0); }
        assert_eq!(crate::suspension_kinematics::counters::solve_corner_calls(), 0);
        assert_eq!(crate::suspension_kinematics::counters::travel_envelope_calls(), 0);
        for state in suspension.wheels {
            assert!(state.total_normal_force > 0.0);
            assert_eq!(state.rod_axial_force_n, 0.0);
            assert_eq!(state.hub_lateral_m, 0.0);
            assert_eq!(state.hub_long_m, 0.0);
        }
    }
}
