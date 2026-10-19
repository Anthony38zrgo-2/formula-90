use super::{geometric_damper_element, PreparedGeometricSuspension};
use crate::suspension_geo_config::{AntiRollConfig, CornerHardpoints, GeometricSuspensionConfig};
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

const TRAVEL_SAMPLE_COUNT: usize = 33;

#[derive(Debug, Clone, Copy)]
struct EquivalentTravelSample {
    travel_metres: f64,
    spring_compression_metres: f64,
    motion_ratio: f64,
    motion_ratio_rate_per_m: f64,
}

#[derive(Debug, Clone)]
struct EquivalentCorner {
    samples: Vec<EquivalentTravelSample>,
}

impl EquivalentCorner {
    fn at(&self, travel_metres: f64) -> Option<(f64, f64, f64)> {
        let first = self.samples.first()?;
        let last = self.samples.last()?;
        if travel_metres <= first.travel_metres {
            return Some((
                first.spring_compression_metres,
                first.motion_ratio,
                first.motion_ratio_rate_per_m,
            ));
        }
        if travel_metres >= last.travel_metres {
            return Some((
                last.spring_compression_metres,
                last.motion_ratio,
                last.motion_ratio_rate_per_m,
            ));
        }
        let upper_index = self
            .samples
            .partition_point(|sample| sample.travel_metres <= travel_metres);
        let lower_index = upper_index.saturating_sub(1).min(self.samples.len() - 2);
        let lower = &self.samples[lower_index];
        let upper = &self.samples[lower_index + 1];
        let span = upper.travel_metres - lower.travel_metres;
        if span <= 1e-12 {
            return Some((
                upper.spring_compression_metres,
                upper.motion_ratio,
                upper.motion_ratio_rate_per_m,
            ));
        }
        let fraction = ((travel_metres - lower.travel_metres) / span).clamp(0.0, 1.0);
        let interpolate = |start: f64, end: f64| start + (end - start) * fraction;
        Some((
            interpolate(
                lower.spring_compression_metres,
                upper.spring_compression_metres,
            ),
            interpolate(lower.motion_ratio, upper.motion_ratio),
            interpolate(lower.motion_ratio_rate_per_m, upper.motion_ratio_rate_per_m),
        ))
    }
}

#[derive(Debug, Clone)]
pub(super) struct SimplifiedSuspension {
    corners: [EquivalentCorner; 4],
}

impl SimplifiedSuspension {
    pub(super) fn build(
        geometry: &GeometricSuspensionConfig,
        prepared: &PreparedGeometricSuspension,
    ) -> Self {
        Self {
            corners: std::array::from_fn(|index| {
                let wheel = WheelIndex::ALL[index];
                let corner = geometry.corners.get(wheel);
                let envelope = prepared.for_wheel(wheel);
                let mut samples = Vec::with_capacity(TRAVEL_SAMPLE_COUNT);
                let span = envelope.q_max - envelope.q_min;
                for sample_index in 0..TRAVEL_SAMPLE_COUNT {
                    let fraction = sample_index as f64 / (TRAVEL_SAMPLE_COUNT - 1) as f64;
                    let travel_metres = envelope.q_min + span * fraction;
                    if let Some(sample) =
                        equivalent_travel_sample(corner, travel_metres, wheel.is_front())
                    {
                        samples.push(sample);
                    }
                }
                assert!(
                    samples.len() >= 2,
                    "Equivalent suspension requires at least two valid travel samples"
                );
                assign_motion_ratio_rates(&mut samples);
                EquivalentCorner { samples }
            }),
        }
    }

    fn corner_at(&self, wheel: WheelIndex, travel_metres: f64) -> Option<(f64, f64, f64)> {
        if !travel_metres.is_finite() {
            return None;
        }
        self.corners[wheel as usize].at(travel_metres)
    }

    pub(super) fn vertical_reference_at(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<(f64, f64)> {
        let (spring_compression_metres, motion_ratio, _) = self.corner_at(wheel, travel_metres)?;
        Some((spring_compression_metres, motion_ratio))
    }

    pub(super) fn motion_ratio_rate_at(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<f64> {
        let (_, _, motion_ratio_rate_per_m) = self.corner_at(wheel, travel_metres)?;
        Some(motion_ratio_rate_per_m)
    }

    pub(super) fn vertical_reference_with_rate_at(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<(f64, f64, f64)> {
        self.corner_at(wheel, travel_metres)
    }

    pub(super) fn forces_at(
        &self,
        geometry: &GeometricSuspensionConfig,
        wheel: WheelIndex,
        travel_metres: f64,
        velocity_metres_per_second: f64,
    ) -> Option<(
        f64,
        f64,
        f64,
        f64,
        f64,
        bool,
        f64,
        Option<KinematicSolution>,
    )> {
        if !velocity_metres_per_second.is_finite() {
            return None;
        }
        let (spring_compression_metres, motion_ratio) =
            self.vertical_reference_at(wheel, travel_metres)?;
        let axle = geometry.axle(wheel);
        let deflection =
            axle.spring_free_length_m - axle.spring_installed_length_m + spring_compression_metres;
        let spring = axle.spring_rate_N_per_m * deflection.max(0.0);
        let damping = geometric_damper_element(axle, motion_ratio * velocity_metres_per_second);
        Some((
            spring * motion_ratio,
            damping * motion_ratio,
            spring_compression_metres,
            motion_ratio,
            0.0,
            deflection <= 0.0,
            0.0,
            None,
        ))
    }
}

fn equivalent_travel_sample(
    corner: &CornerHardpoints,
    travel_metres: f64,
    is_front: bool,
) -> Option<EquivalentTravelSample> {
    let solution = solve_corner(corner, travel_metres, 0.0, is_front, 0.0, 0.0, 1.0)?;
    let transmission = jacobian(corner, travel_metres, 0.0, is_front)?;
    if !transmission.motion_ratio.is_finite()
        || transmission.motion_ratio <= 0.0
        || !solution.converged
        || solution.rocker_clamped
        || solution.steering_clamped
    {
        return None;
    }
    Some(EquivalentTravelSample {
        travel_metres,
        spring_compression_metres: solution.damper_compression,
        motion_ratio: transmission.motion_ratio,
        motion_ratio_rate_per_m: 0.0,
    })
}

fn assign_motion_ratio_rates(samples: &mut [EquivalentTravelSample]) {
    if samples.len() < 2 {
        return;
    }
    for index in 0..samples.len() {
        let rate = if index == 0 {
            (samples[1].motion_ratio - samples[0].motion_ratio)
                / (samples[1].travel_metres - samples[0].travel_metres)
        } else if index + 1 == samples.len() {
            (samples[index].motion_ratio - samples[index - 1].motion_ratio)
                / (samples[index].travel_metres - samples[index - 1].travel_metres)
        } else {
            (samples[index + 1].motion_ratio - samples[index - 1].motion_ratio)
                / (samples[index + 1].travel_metres - samples[index - 1].travel_metres)
        };
        samples[index].motion_ratio_rate_per_m = if rate.is_finite() { rate } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspension::{rest_compression_m, SuspensionSystem};
    use crate::types::{RaycastHit, TriRaycastSample};
    use crate::vehicle_configuration::load_vehicle_configuration_from_authorized_root;

    #[test]
    fn equivalent_vertical_steps_never_solve_mechanical_geometry() {
        let repository_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let configuration = load_vehicle_configuration_from_authorized_root(
            &repository_root.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
            &repository_root,
        )
        .unwrap();
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
            TriRaycastSample {
                inner: hit,
                center: hit,
                outer: hit,
            }
        });
        crate::suspension_kinematics::counters::reset();
        for _ in 0..120 {
            suspension.step(&configuration, &samples, 1.0 / 120.0);
        }
        assert_eq!(
            crate::suspension_kinematics::counters::solve_corner_calls(),
            0
        );
        assert_eq!(
            crate::suspension_kinematics::counters::travel_envelope_calls(),
            0
        );
        for state in suspension.wheels {
            assert!(state.total_normal_force > 0.0);
            assert_eq!(state.rod_axial_force_n, 0.0);
            assert_eq!(state.hub_lateral_m, 0.0);
            assert_eq!(state.hub_long_m, 0.0);
        }
    }

    #[test]
    fn equivalent_travel_table_interpolates_between_valid_samples() {
        let repository_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let configuration = load_vehicle_configuration_from_authorized_root(
            &repository_root
                .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
            &repository_root,
        )
        .unwrap();
        let mut suspension = SuspensionSystem::new(&configuration);
        suspension.select_backend(&configuration, SuspensionBackend::Simplified);
        let prepared = suspension
            .simplified_prepared
            .as_ref()
            .expect("Simplified suspension must be prepared");
        let design = prepared
            .vertical_reference_at(WheelIndex::FrontLeft, 0.0)
            .expect("Design travel must resolve");
        let bump = prepared
            .vertical_reference_at(WheelIndex::FrontLeft, 0.02)
            .expect("Bump travel must resolve");
        let rebound = prepared
            .vertical_reference_at(WheelIndex::FrontLeft, -0.02)
            .expect("Rebound travel must resolve");
        assert!(design.1 > 0.0 && bump.1 > design.1);
        assert!(rebound.1 < design.1);
        assert!(prepared
            .motion_ratio_rate_at(WheelIndex::FrontLeft, 0.02)
            .is_some());
    }
}
