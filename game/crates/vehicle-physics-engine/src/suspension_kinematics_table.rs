use crate::suspension_component_kinematics::attachment_frame_from_solution;
use crate::suspension_kinematics::{solve_corner, travel_envelope, KinematicSolution};
use crate::suspension_mass_properties::AttachmentFrameKind;
use crate::types::{Mat3, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;

pub const SUSPENSION_KINEMATICS_TABLE_VERSION: u32 = 1;
pub const TRAVEL_NODE_COUNT: usize = 257;
pub const RACK_NODE_COUNT: usize = 65;
pub const RACK_TABLE_RANGE_FRACTION: f64 = 0.90;

#[derive(Debug, Clone)]
pub struct SuspensionKinematicsTable {
    corners: [CornerKinematicsTable; 4],
}

#[derive(Debug, Clone)]
struct CornerKinematicsTable {
    travel_min_metres: f64,
    travel_max_metres: f64,
    rack_min_metres: f64,
    rack_max_metres: f64,
    travel_node_count: usize,
    rack_node_count: usize,
    wheel_frames: Vec<Option<KinematicSolution>>,
    attachment_frames: Vec<Option<(Vec3, Mat3)>>,
}

impl SuspensionKinematicsTable {
    pub fn build(configuration: &VehicleConfig, steering_rack_half_range_metres: f64)
        -> Result<Self, String> {
        if !steering_rack_half_range_metres.is_finite() || steering_rack_half_range_metres <= 0.0 {
            return Err("Suspension kinematics table requires a positive rack half range".into());
        }
        let geometry = configuration.suspension.geometric_suspension.as_ref()
            .ok_or("Suspension kinematics table requires geometric hardpoints")?;
        let corners: [Result<CornerKinematicsTable, String>; 4] = std::array::from_fn(|index| {
            let wheel = WheelIndex::ALL[index];
            let axle = geometry.axle(wheel);
            let (travel_min_metres, travel_max_metres) = travel_envelope(
                geometry.corners.get(wheel), axle.wheel_droop_m, axle.wheel_bump_m, 0.0,
                wheel.is_front());
            CornerKinematicsTable::build(configuration, wheel, travel_min_metres,
                travel_max_metres, steering_rack_half_range_metres)
        });
        let mut built = Vec::with_capacity(4);
        for corner in corners { built.push(corner?); }
        Ok(Self { corners: built.try_into().map_err(|_| "Suspension kinematics table corner count mismatch")? })
    }

    pub fn wheel_frame(&self, wheel: WheelIndex, travel_metres: f64, steering_rack_metres: f64)
        -> Option<KinematicSolution> {
        self.corners[wheel as usize].sample_wheel_frame(travel_metres, steering_rack_metres)
    }

    pub fn attachment_frame(&self, wheel: WheelIndex, kind: AttachmentFrameKind,
        travel_metres: f64, steering_rack_metres: f64) -> Option<(Vec3, Mat3)> {
        self.corners[wheel as usize].sample_attachment_frame(kind, travel_metres, steering_rack_metres)
    }

    pub fn travel_limits_metres(&self, wheel: WheelIndex) -> (f64, f64) {
        let corner = &self.corners[wheel as usize];
        (corner.travel_min_metres, corner.travel_max_metres)
    }
}

impl CornerKinematicsTable {
    fn build(configuration: &VehicleConfig, wheel: WheelIndex, travel_min_metres: f64,
        travel_max_metres: f64, steering_rack_half_range_metres: f64) -> Result<Self, String> {
        if !(travel_max_metres > travel_min_metres) {
            return Err(format!("{wheel:?}: empty suspension travel envelope"));
        }
        let geometry = configuration.suspension.geometric_suspension.as_ref()
            .ok_or("Suspension kinematics table requires geometric hardpoints")?;
        let corner = geometry.corners.get(wheel);
        let rack_node_count = if wheel.is_front() { RACK_NODE_COUNT } else { 1 };
        let table_rack_half_range_metres = steering_rack_half_range_metres * RACK_TABLE_RANGE_FRACTION;
        let rack_min_metres = if wheel.is_front() { -table_rack_half_range_metres } else { 0.0 };
        let rack_max_metres = if wheel.is_front() { table_rack_half_range_metres } else { 0.0 };
        let travel_node_count = TRAVEL_NODE_COUNT;
        let node_count = travel_node_count * rack_node_count;
        let mut wheel_frames = vec![None; node_count];
        let mut attachment_frames = vec![None; node_count * AttachmentFrameKind::ALL.len()];
        let mut valid_node_count = 0_usize;
        for travel_index in 0..travel_node_count {
            let travel = travel_min_metres + (travel_max_metres - travel_min_metres)
                * travel_index as f64 / (travel_node_count - 1) as f64;
            for rack_index in 0..rack_node_count {
                let rack = if rack_node_count == 1 { 0.0 } else {
                    rack_min_metres + (rack_max_metres - rack_min_metres)
                        * rack_index as f64 / (rack_node_count - 1) as f64
                };
                let Some(solution) = solve_corner(corner, travel, rack, wheel.is_front(), 0.0, 0.0, 1.0)
                    else { continue; };
                if !solution.converged || solution.rocker_clamped || solution.steering_clamped
                    || solution.travel_limited {
                    continue;
                }
                let node_index = travel_index * rack_node_count + rack_index;
                wheel_frames[node_index] = Some(solution);
                for kind in AttachmentFrameKind::ALL {
                    if let Ok(frame) = attachment_frame_from_solution(configuration, wheel, kind,
                        &solution, rack) {
                        attachment_frames[kind.index() * node_count + node_index] = Some(frame);
                    }
                }
                valid_node_count += 1;
            }
        }
        if valid_node_count < 2 {
            return Err(format!("{wheel:?}: suspension kinematics table has no valid nodes"));
        }
        Ok(Self { travel_min_metres, travel_max_metres, rack_min_metres, rack_max_metres,
            travel_node_count, rack_node_count, wheel_frames, attachment_frames })
    }

    fn grid_position(&self, travel_metres: f64, steering_rack_metres: f64)
        -> Option<(usize, f64, usize, f64)> {
        if !travel_metres.is_finite() || !steering_rack_metres.is_finite()
            || travel_metres < self.travel_min_metres || travel_metres > self.travel_max_metres {
            return None;
        }
        if self.rack_node_count == 1 {
            let travel_position = (travel_metres - self.travel_min_metres)
                / (self.travel_max_metres - self.travel_min_metres)
                * (self.travel_node_count - 1) as f64;
            if travel_position < 1.0 || travel_position > (self.travel_node_count - 2) as f64 {
                return None;
            }
            let travel_index = travel_position.floor() as usize;
            return Some((travel_index, travel_position - travel_index as f64, 0, 0.0));
        }
        if steering_rack_metres < self.rack_min_metres || steering_rack_metres > self.rack_max_metres {
            return None;
        }
        let travel_position = (travel_metres - self.travel_min_metres)
            / (self.travel_max_metres - self.travel_min_metres)
            * (self.travel_node_count - 1) as f64;
        let rack_position = (steering_rack_metres - self.rack_min_metres)
            / (self.rack_max_metres - self.rack_min_metres)
            * (self.rack_node_count - 1) as f64;
        if travel_position < 1.0 || travel_position > (self.travel_node_count - 2) as f64
            || rack_position < 1.0 || rack_position > (self.rack_node_count - 2) as f64 {
            return None;
        }
        let travel_index = travel_position.floor() as usize;
        let rack_index = rack_position.floor() as usize;
        Some((travel_index, travel_position - travel_index as f64,
            rack_index, rack_position - rack_index as f64))
    }

    fn neighbourhood_indices(&self, travel_index: usize, rack_index: usize)
        -> ([usize; 4], [usize; 4]) {
        let travel = std::array::from_fn(|offset| {
            (travel_index as isize + offset as isize - 1)
                .clamp(0, self.travel_node_count as isize - 1) as usize
        });
        let rack = std::array::from_fn(|offset| {
            (rack_index as isize + offset as isize - 1)
                .clamp(0, self.rack_node_count as isize - 1) as usize
        });
        (travel, rack)
    }

    fn sample_wheel_frame(&self, travel_metres: f64, steering_rack_metres: f64)
        -> Option<KinematicSolution> {
        let (travel_index, travel_fraction, rack_index, rack_fraction) =
            self.grid_position(travel_metres, steering_rack_metres)?;
        let (travel_indices, rack_indices) = self.neighbourhood_indices(travel_index, rack_index);
        let mut samples = [[None; 4]; 4];
        for (travel_offset, travel_node) in travel_indices.iter().enumerate() {
            for (rack_offset, rack_node) in rack_indices.iter().enumerate() {
                let index = travel_node * self.rack_node_count + rack_node;
                let sample = self.wheel_frames.get(index).copied().flatten()?;
                if !sample.converged || sample.rocker_clamped || sample.steering_clamped
                    || sample.travel_limited {
                    return None;
                }
                samples[travel_offset][rack_offset] = Some(sample);
            }
        }
        let nearest = samples[1][1]?;
        let mut result = nearest;
        result.hub = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.hub);
        result.hub_target = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.hub_target);
        result.lbj = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.lbj);
        result.ubj = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.ubj);
        result.upright_basis = sample_mat3(samples, travel_fraction, rack_fraction, |sample| sample.upright_basis);
        result.wheel_basis = sample_mat3(samples, travel_fraction, rack_fraction, |sample| sample.wheel_basis);
        result.rocker_angle = sample_scalar(samples, travel_fraction, rack_fraction, |sample| sample.rocker_angle);
        result.damper_length = sample_scalar(samples, travel_fraction, rack_fraction, |sample| sample.damper_length);
        result.damper_compression = sample_scalar(samples, travel_fraction, rack_fraction, |sample| sample.damper_compression);
        result.rocker_end = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.rocker_end);
        result.pushrod_outer = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.pushrod_outer);
        result.damper_end = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.damper_end);
        result.trackrod_outer = sample_vec3(samples, travel_fraction, rack_fraction, |sample| sample.trackrod_outer);
        result.requested_travel = travel_metres;
        result.solved_travel = sample_scalar(samples, travel_fraction, rack_fraction, |sample| sample.solved_travel);
        Some(result)
    }

    fn sample_attachment_frame(&self, kind: AttachmentFrameKind, travel_metres: f64,
        steering_rack_metres: f64) -> Option<(Vec3, Mat3)> {
        let (travel_index, travel_fraction, rack_index, rack_fraction) =
            self.grid_position(travel_metres, steering_rack_metres)?;
        let (travel_indices, rack_indices) = self.neighbourhood_indices(travel_index, rack_index);
        let frame_offset = kind.index() * self.travel_node_count * self.rack_node_count;
        let mut samples = [[None; 4]; 4];
        for (travel_offset, travel_node) in travel_indices.iter().enumerate() {
            for (rack_offset, rack_node) in rack_indices.iter().enumerate() {
                let index = frame_offset + travel_node * self.rack_node_count + rack_node;
                samples[travel_offset][rack_offset] = self.attachment_frames.get(index).copied().flatten();
                samples[travel_offset][rack_offset]?;
            }
        }
        let origin = sample_vec3(samples, travel_fraction, rack_fraction, |frame| frame.0);
        let orientation = sample_mat3(samples, travel_fraction, rack_fraction, |frame| frame.1);
        Some((origin, orientation))
    }
}

fn cubic_weights(fraction: f64) -> [f64; 4] {
    let squared = fraction * fraction;
    let cubed = squared * fraction;
    [
        -0.5 * fraction + squared - 0.5 * cubed,
        1.0 - 2.5 * squared + 1.5 * cubed,
        0.5 * fraction + 2.0 * squared - 1.5 * cubed,
        -0.5 * squared + 0.5 * cubed,
    ]
}

fn sample_grid<T: Copy>(samples: [[Option<T>; 4]; 4], travel_fraction: f64,
    rack_fraction: f64, extract: impl Fn(T) -> f64) -> f64 {
    let travel_weights = cubic_weights(travel_fraction);
    let rack_weights = cubic_weights(rack_fraction);
    let mut result = 0.0;
    for travel_offset in 0..4 {
        for rack_offset in 0..4 {
            result += travel_weights[travel_offset] * rack_weights[rack_offset]
                * extract(samples[travel_offset][rack_offset].unwrap());
        }
    }
    result
}

fn sample_scalar<T: Copy>(samples: [[Option<T>; 4]; 4], travel_fraction: f64,
    rack_fraction: f64, extract: impl Fn(T) -> f64) -> f64 {
    sample_grid(samples, travel_fraction, rack_fraction, extract)
}

fn sample_vec3<T: Copy>(samples: [[Option<T>; 4]; 4], travel_fraction: f64,
    rack_fraction: f64, extract: impl Fn(T) -> Vec3) -> Vec3 {
    Vec3::new(
        sample_grid(samples, travel_fraction, rack_fraction, |sample| extract(sample).x),
        sample_grid(samples, travel_fraction, rack_fraction, |sample| extract(sample).y),
        sample_grid(samples, travel_fraction, rack_fraction, |sample| extract(sample).z),
    )
}

fn sample_mat3<T: Copy>(samples: [[Option<T>; 4]; 4], travel_fraction: f64,
    rack_fraction: f64, extract: impl Fn(T) -> Mat3) -> Mat3 {
    let x = sample_vec3(samples, travel_fraction, rack_fraction, |sample| extract(sample).x);
    let y = sample_vec3(samples, travel_fraction, rack_fraction, |sample| extract(sample).y);
    orthonormalize(x, y)
}

fn orthonormalize(first: Vec3, second: Vec3) -> Mat3 {
    let mut x = if first.length_squared() > 1e-12 { first.normalized() } else { Vec3::RIGHT };
    let mut y = second - x * second.dot(x);
    if y.length_squared() < 1e-12 {
        y = Vec3::UP - x * Vec3::UP.dot(x);
        if y.length_squared() < 1e-12 { y = Vec3::FORWARD; }
    }
    y = y.normalized();
    let z = x.cross(y).normalized();
    x = y.cross(z).normalized();
    Mat3::from_cols(x, y, z)
}
