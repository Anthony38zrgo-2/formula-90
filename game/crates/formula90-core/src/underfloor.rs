//! Deterministic fusion of five underfloor clearance probes and rigid contacts.

use serde::{Deserialize, Serialize};
use vehicle_physics_engine::{BodyKinematics, Vec3};

pub const UNDERFLOOR_RAY_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, Default)]
pub struct UnderfloorRayHit {
    pub valid: bool,
    pub clearance_m: f64,
    pub point_world: [f64; 3],
    pub normal_world: [f64; 3],
    pub surface_code: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnderfloorRigidContact {
    pub confirmed: bool,
    pub local_position: [f64; 3],
    pub normal_impulse_ns: f64,
    pub tangential_speed_m_s: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnderfloorSample {
    pub rays: [UnderfloorRayHit; UNDERFLOOR_RAY_COUNT],
    pub rigid_contact: UnderfloorRigidContact,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrapePhase {
    #[default]
    Clear = 0,
    Approaching = 1,
    Impact = 2,
    Scraping = 3,
    Releasing = 4,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomingPhase {
    #[default]
    Clear = 0,
    Approaching = 1,
    Loaded = 2,
    RigidContact = 3,
    Releasing = 4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnderfloorContactConfig {
    pub enabled: bool,
    pub approach_clearance_m: f64,
    pub activation_clearance_m: f64,
    pub release_clearance_m: f64,
    pub linear_rate_n_m: f64,
    pub progressive_rate_n_m2: f64,
    pub damping_n_s_m: f64,
    pub max_force_per_probe_n: f64,
    pub rigid_contact_spring_scale: f64,
    pub normal_min_y: f64,
}

impl Default for UnderfloorContactConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            approach_clearance_m: 0.020,
            activation_clearance_m: 0.008,
            release_clearance_m: 0.016,
            linear_rate_n_m: 450_000.0,
            progressive_rate_n_m2: 40_000_000.0,
            damping_n_s_m: 12_000.0,
            max_force_per_probe_n: 12_000.0,
            rigid_contact_spring_scale: 0.25,
            normal_min_y: 0.55,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnderfloorState {
    pub raw_clearance_m: [f64; UNDERFLOOR_RAY_COUNT],
    pub filtered_clearance_m: [f64; UNDERFLOOR_RAY_COUNT],
    pub clearance_velocity_m_s: [f64; UNDERFLOOR_RAY_COUNT],
    pub valid_mask: u32,
    pub minimum_clearance_m: f64,
    pub rake_rad: f64,
    pub roll_rad: f64,
    pub contact_confidence: f64,
    pub scrape_phase: ScrapePhase,
    pub scrape_intensity: f64,
    pub onset_strength: f64,
    pub compression_m: [f64; UNDERFLOOR_RAY_COUNT],
    pub closing_speed_m_s: [f64; UNDERFLOOR_RAY_COUNT],
    pub normal_force_n: [f64; UNDERFLOOR_RAY_COUNT],
    pub bottoming_phase: [BottomingPhase; UNDERFLOOR_RAY_COUNT],
    pub active_probe_mask: u32,
    pub total_normal_force_n: f64,
    pub max_probe_force_n: f64,
    pub force_world: [f64; 3],
    pub torque_world: [f64; 3],
    pub force_center_local: [f64; 3],
    pub dissipated_energy_j: f64,
    pub rigid_contact_blend: f64,
    hold_s: f64,
    initialized: bool,
}

impl Default for UnderfloorState {
    fn default() -> Self {
        Self {
            raw_clearance_m: [0.35; UNDERFLOOR_RAY_COUNT],
            filtered_clearance_m: [0.35; UNDERFLOOR_RAY_COUNT],
            clearance_velocity_m_s: [0.0; UNDERFLOOR_RAY_COUNT],
            valid_mask: 0,
            minimum_clearance_m: 0.35,
            rake_rad: 0.0,
            roll_rad: 0.0,
            contact_confidence: 0.0,
            scrape_phase: ScrapePhase::Clear,
            scrape_intensity: 0.0,
            onset_strength: 0.0,
            compression_m: [0.0; UNDERFLOOR_RAY_COUNT],
            closing_speed_m_s: [0.0; UNDERFLOOR_RAY_COUNT],
            normal_force_n: [0.0; UNDERFLOOR_RAY_COUNT],
            bottoming_phase: [BottomingPhase::Clear; UNDERFLOOR_RAY_COUNT],
            active_probe_mask: 0,
            total_normal_force_n: 0.0,
            max_probe_force_n: 0.0,
            force_world: [0.0; 3],
            torque_world: [0.0; 3],
            force_center_local: [0.0; 3],
            dissipated_energy_j: 0.0,
            rigid_contact_blend: 0.0,
            hold_s: 0.0,
            initialized: false,
        }
    }
}

impl UnderfloorState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn step(
        &mut self,
        sample: &UnderfloorSample,
        body: Option<&BodyKinematics>,
        config: &UnderfloorContactConfig,
        dt: f64,
    ) {
        const MAX_CLEARANCE: f64 = 0.35;
        const PROXIMITY_ENTER: f64 = 0.025;
        const CONTACT_ENTER: f64 = 0.008;
        const CONTACT_EXIT: f64 = 0.016;
        const HOLD_SECONDS: f64 = 0.060;

        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        let alpha = 1.0 - (-dt / 0.015).exp();
        let previous = self.filtered_clearance_m;
        self.valid_mask = 0;
        for (i, ray) in sample.rays.iter().enumerate() {
            let raw = if ray.valid && ray.clearance_m.is_finite() {
                self.valid_mask |= 1 << i;
                ray.clearance_m.clamp(0.0, MAX_CLEARANCE)
            } else {
                MAX_CLEARANCE
            };
            self.raw_clearance_m[i] = raw;
            if !self.initialized {
                self.filtered_clearance_m[i] = raw;
            } else {
                self.filtered_clearance_m[i] += (raw - self.filtered_clearance_m[i]) * alpha;
            }
            self.clearance_velocity_m_s[i] =
                ((self.filtered_clearance_m[i] - previous[i]) / dt).clamp(-12.0, 12.0);
        }
        self.initialized = true;
        self.minimum_clearance_m = self
            .filtered_clearance_m
            .iter()
            .enumerate()
            .filter(|(i, _)| self.valid_mask & (1 << i) != 0)
            .map(|(_, v)| *v)
            .fold(MAX_CLEARANCE, f64::min);
        self.roll_rad =
            ((self.filtered_clearance_m[0] - self.filtered_clearance_m[1]) / 0.90).atan();
        // Center is mounted 40 mm lower than the diffuser-exit probe. Remove that
        // fixed chassis geometry so this signal represents road-relative pitch,
        // not the diffuser's built-in ramp.
        const CENTER_TO_EXIT_Y_OFFSET_M: f64 = 0.040;
        self.rake_rad = ((self.filtered_clearance_m[2] - self.filtered_clearance_m[4]
            + CENTER_TO_EXIT_Y_OFFSET_M)
            / 1.65)
            .atan();

        self.step_bottoming(sample, body, config, dt);

        let close_rays = self
            .filtered_clearance_m
            .iter()
            .enumerate()
            .filter(|(i, v)| self.valid_mask & (1 << i) != 0 && **v <= CONTACT_ENTER)
            .count();
        let geometry_confidence = (close_rays as f64 / 2.0).clamp(0.0, 1.0);
        self.contact_confidence = geometry_confidence.max(if sample.rigid_contact.confirmed {
            1.0
        } else {
            0.0
        });
        let previous_phase = self.scrape_phase;
        let approaching = self.minimum_clearance_m <= PROXIMITY_ENTER;
        let contact_now = sample.rigid_contact.confirmed
            || self.active_probe_mask != 0
            || (self.minimum_clearance_m <= CONTACT_ENTER && close_rays >= 2);
        let separating =
            self.minimum_clearance_m >= CONTACT_EXIT && !sample.rigid_contact.confirmed;
        let approach_speed = self
            .clearance_velocity_m_s
            .iter()
            .copied()
            .map(|v| (-v).max(0.0))
            .fold(0.0, f64::max);
        self.onset_strength = 0.0;
        if contact_now {
            self.hold_s = HOLD_SECONDS;
            if matches!(
                previous_phase,
                ScrapePhase::Clear | ScrapePhase::Approaching | ScrapePhase::Releasing
            ) {
                self.scrape_phase = ScrapePhase::Impact;
                self.onset_strength = (approach_speed / 1.5
                    + sample.rigid_contact.normal_impulse_ns / 1200.0)
                    .clamp(0.0, 1.0);
            } else {
                self.scrape_phase = ScrapePhase::Scraping;
            }
        } else if self.hold_s > 0.0 && !separating {
            self.hold_s = (self.hold_s - dt).max(0.0);
            self.scrape_phase = ScrapePhase::Scraping;
        } else if approaching {
            self.hold_s = (self.hold_s - dt).max(0.0);
            self.scrape_phase = ScrapePhase::Approaching;
        } else if matches!(previous_phase, ScrapePhase::Impact | ScrapePhase::Scraping) {
            self.hold_s = 0.0;
            self.scrape_phase = ScrapePhase::Releasing;
        } else {
            self.scrape_phase = ScrapePhase::Clear;
        }

        let speed_factor = (sample.rigid_contact.tangential_speed_m_s / 35.0).clamp(0.0, 1.0);
        let clearance_factor =
            ((CONTACT_EXIT - self.minimum_clearance_m) / CONTACT_EXIT).clamp(0.0, 1.0);
        let impulse_factor = (sample.rigid_contact.normal_impulse_ns / 900.0).clamp(0.0, 1.0);
        let force_factor = (self.total_normal_force_n / 18_000.0).clamp(0.0, 1.0);
        let target = if matches!(
            self.scrape_phase,
            ScrapePhase::Impact | ScrapePhase::Scraping
        ) {
            self.contact_confidence
                * (0.15
                    + 0.35 * speed_factor
                    + 0.20 * clearance_factor
                    + 0.10 * impulse_factor
                    + 0.20 * force_factor)
        } else {
            0.0
        };
        let tau = if target > self.scrape_intensity {
            0.020
        } else {
            0.150
        };
        self.scrape_intensity += (target - self.scrape_intensity) * (1.0 - (-dt / tau).exp());
        if self.scrape_phase == ScrapePhase::Clear && self.scrape_intensity < 1e-4 {
            self.scrape_intensity = 0.0;
        }
    }

    fn step_bottoming(
        &mut self,
        sample: &UnderfloorSample,
        body: Option<&BodyKinematics>,
        config: &UnderfloorContactConfig,
        dt: f64,
    ) {
        const LOCAL_POSITIONS: [[f64; 3]; 5] = [
            [-0.45, -0.205, -0.90],
            [0.45, -0.205, -0.90],
            [0.0, -0.205, 0.0],
            [0.0, -0.165, 0.85],
            [0.0, -0.165, 1.65],
        ];
        self.active_probe_mask = 0;
        self.total_normal_force_n = 0.0;
        self.max_probe_force_n = 0.0;
        self.force_world = [0.0; 3];
        self.torque_world = [0.0; 3];
        self.force_center_local = [0.0; 3];
        let target_blend = if sample.rigid_contact.confirmed {
            1.0
        } else {
            0.0
        };
        let blend_tau = if target_blend > self.rigid_contact_blend {
            0.020
        } else {
            0.040
        };
        self.rigid_contact_blend +=
            (target_blend - self.rigid_contact_blend) * (1.0 - (-dt / blend_tau).exp());
        let mut total_force = Vec3::ZERO;
        let mut total_torque = Vec3::ZERO;
        let mut weighted_center = Vec3::ZERO;

        for i in 0..UNDERFLOOR_RAY_COUNT {
            let ray = sample.rays[i];
            let previous_phase = self.bottoming_phase[i];
            let normal_min_y = config.normal_min_y.clamp(-1.0, 1.0);
            let valid = config.enabled
                && ray.valid
                && ray.normal_world.iter().all(|v| v.is_finite())
                && ray.normal_world[1] >= normal_min_y;
            let clearance = self.filtered_clearance_m[i];
            let compression = if valid {
                (config.activation_clearance_m.max(0.0) - clearance).max(0.0)
            } else {
                0.0
            };
            let normal = Vec3::new(
                ray.normal_world[0],
                ray.normal_world[1],
                ray.normal_world[2],
            )
            .normalized();
            let point = Vec3::new(ray.point_world[0], ray.point_world[1], ray.point_world[2]);
            let lever = body.map(|b| point - b.transform.origin).unwrap_or_else(|| {
                let p = LOCAL_POSITIONS[i];
                Vec3::new(p[0], p[1], p[2])
            });
            let kinematic_closing = body
                .map(|b| {
                    let point_velocity = b.linear_velocity + b.angular_velocity.cross(lever);
                    (-point_velocity.dot(normal)).max(0.0)
                })
                .unwrap_or(0.0);
            let closing = kinematic_closing
                .max((-self.clearance_velocity_m_s[i]).max(0.0))
                .clamp(0.0, 6.0);
            self.compression_m[i] = compression;
            self.closing_speed_m_s[i] = if valid { closing } else { 0.0 };
            let spring = config.linear_rate_n_m.max(0.0) * compression
                + config.progressive_rate_n_m2.max(0.0) * compression * compression;
            let damper = if compression > 0.0 {
                config.damping_n_s_m.max(0.0) * closing
            } else {
                0.0
            };
            let rigid_scale = 1.0
                - self.rigid_contact_blend
                    * (1.0 - config.rigid_contact_spring_scale.clamp(0.0, 1.0));
            let force = if valid {
                ((spring + damper) * rigid_scale).clamp(0.0, config.max_force_per_probe_n.max(0.0))
            } else {
                0.0
            };
            self.normal_force_n[i] = force;
            self.bottoming_phase[i] = if !valid || clearance >= config.release_clearance_m {
                if matches!(
                    previous_phase,
                    BottomingPhase::Loaded | BottomingPhase::RigidContact
                ) {
                    BottomingPhase::Releasing
                } else {
                    BottomingPhase::Clear
                }
            } else if sample.rigid_contact.confirmed && compression > 0.0 {
                BottomingPhase::RigidContact
            } else if compression > 0.0 {
                BottomingPhase::Loaded
            } else if clearance <= config.approach_clearance_m {
                BottomingPhase::Approaching
            } else {
                BottomingPhase::Clear
            };
            if force > 0.0 {
                self.active_probe_mask |= 1 << i;
                self.total_normal_force_n += force;
                self.max_probe_force_n = self.max_probe_force_n.max(force);
                let local = LOCAL_POSITIONS[i];
                weighted_center += Vec3::new(local[0], local[1], local[2]) * force;
                let f = normal * force;
                total_force += f;
                total_torque += lever.cross(f);
                self.dissipated_energy_j += damper.min(force) * closing * dt;
            }
        }
        if self.total_normal_force_n > 1e-6 {
            let center = weighted_center / self.total_normal_force_n;
            self.force_center_local = [center.x, center.y, center.z];
        }
        self.force_world = [total_force.x, total_force.y, total_force.z];
        self.torque_world = [total_torque.x, total_torque.y, total_torque.z];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flat_sample(clearance: f64) -> UnderfloorSample {
        UnderfloorSample {
            rays: [UnderfloorRayHit {
                valid: true,
                clearance_m: clearance,
                normal_world: [0.0, 1.0, 0.0],
                ..Default::default()
            }; UNDERFLOOR_RAY_COUNT],
            ..Default::default()
        }
    }
    #[test]
    fn clear_road_does_not_scrape() {
        let mut state = UnderfloorState::default();
        state.step(
            &flat_sample(0.12),
            None,
            &UnderfloorContactConfig::default(),
            1.0 / 120.0,
        );
        assert_eq!(state.scrape_phase, ScrapePhase::Clear);
        assert_eq!(state.scrape_intensity, 0.0);
    }
    #[test]
    fn rake_removes_fixed_diffuser_probe_height_offset() {
        let mut state = UnderfloorState::default();
        let mut sample = flat_sample(0.100);
        sample.rays[4].clearance_m = 0.140;
        state.step(
            &sample,
            None,
            &UnderfloorContactConfig::default(),
            1.0 / 120.0,
        );
        assert!(state.rake_rad.abs() < 1e-12, "rake={}", state.rake_rad);
    }

    #[test]
    fn localized_front_left_force_creates_roll_and_pitch_torque() {
        let mut state = UnderfloorState::default();
        let mut sample = UnderfloorSample::default();
        sample.rays[0] = UnderfloorRayHit {
            valid: true,
            clearance_m: 0.004,
            point_world: [-0.45, -0.205, -0.90],
            normal_world: [0.0, 1.0, 0.0],
            ..Default::default()
        };
        state.step(
            &sample,
            None,
            &UnderfloorContactConfig::default(),
            1.0 / 120.0,
        );
        assert!(state.normal_force_n[0] > 0.0);
        assert!(state.torque_world[0] > 0.0);
        assert!(state.torque_world[2] < 0.0);
        assert_eq!(state.active_probe_mask, 1);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
    }

    #[test]
    fn rigid_contact_reduces_supplemental_force() {
        let cfg = UnderfloorContactConfig::default();
        let sample = flat_sample(0.004);
        let mut free = UnderfloorState::default();
        free.step(&sample, None, &cfg, 1.0 / 120.0);
        let mut rigid = UnderfloorState::default();
        let mut rigid_sample = sample;
        rigid_sample.rigid_contact.confirmed = true;
        for _ in 0..12 {
            rigid.step(&rigid_sample, None, &cfg, 1.0 / 120.0);
        }
        assert!(rigid.total_normal_force_n < free.total_normal_force_n);
        assert!(rigid.rigid_contact_blend > 0.9);
    }

    #[test]
    fn close_rays_create_one_impact_then_sustain() {
        let mut state = UnderfloorState::default();
        state.step(
            &flat_sample(0.004),
            None,
            &UnderfloorContactConfig::default(),
            1.0 / 120.0,
        );
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        state.step(
            &flat_sample(0.004),
            None,
            &UnderfloorContactConfig::default(),
            1.0 / 120.0,
        );
        assert_eq!(state.scrape_phase, ScrapePhase::Scraping);
        assert_eq!(state.onset_strength, 0.0);
    }
}
