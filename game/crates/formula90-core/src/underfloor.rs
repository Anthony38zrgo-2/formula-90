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

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UnderfloorRawContact {
    pub local_position: [f64; 3],
    pub local_normal: [f64; 3],
    pub impulse_newton_seconds: f64,
    pub tangential_speed_metres_per_second: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnderfloorSample {
    pub rays: [UnderfloorRayHit; UNDERFLOOR_RAY_COUNT],
    pub rigid_contact: UnderfloorRigidContact,
    pub raw_contacts: [UnderfloorRawContact; 32],
    pub raw_contact_count: usize,
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

pub use vehicle_physics_engine::underfloor_contact_configuration::UnderfloorContactConfig;

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
    /// Last latched rigid-contact diagnostics (probe-local Y, impulse and
    /// tangential speed), kept across debounce frames for telemetry.
    pub rigid_local_position: [f64; 3],
    pub rigid_normal_impulse_ns: f64,
    pub rigid_tangential_speed_m_s: f64,
    hold_s: f64,
    /// Debounce hold for the raw (frame-flickering) rigid contact flag.
    rigid_hold_s: f64,
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
            rigid_local_position: [0.0; 3],
            rigid_normal_impulse_ns: 0.0,
            rigid_tangential_speed_m_s: 0.0,
            hold_s: 0.0,
            rigid_hold_s: 0.0,
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
        self.step_with_probe_layout(sample, body, config, dt, false);
    }

    pub fn step_with_probe_layout(
        &mut self,
        sample: &UnderfloorSample,
        body: Option<&BodyKinematics>,
        config: &UnderfloorContactConfig,
        dt: f64,
        two_longitudinal_probes: bool,
    ) {
        const MAX_CLEARANCE: f64 = 0.35;

        if !config.enabled {
            self.hold_s = 0.0;
            self.rigid_hold_s = 0.0;
        }
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        let alpha = 1.0 - (-dt / config.clearance_filter_seconds).exp();
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
        if two_longitudinal_probes {
            self.roll_rad = 0.0;
            self.rake_rad = if self.valid_mask & 0b01001 == 0b01001 {
                ((self.filtered_clearance_m[0] - self.filtered_clearance_m[3] + 0.040) / 1.75).atan()
            } else {
                0.0
            };
        }

        // The host reports raw floor contacts that flicker frame-to-frame while
        // the body rides a few millimetres off the ground. Latch them with a
        // hold so confidence, phase and scrape intensity stay stable instead of
        // pulsing at frame rate.
        let strongest_contact = sample.raw_contacts[..sample.raw_contact_count.min(32)].iter()
            .filter(|contact| config.enabled
                && contact.local_position.iter().chain(contact.local_normal.iter()).all(|value| value.is_finite())
                && contact.local_position[1] <= config.rigid_maximum_local_height_metres
                && contact.local_normal[1] >= config.normal_min_y
                && contact.impulse_newton_seconds.is_finite()
                && contact.impulse_newton_seconds >= config.rigid_minimum_impulse_newton_seconds
                && contact.tangential_speed_metres_per_second.is_finite())
            .max_by(|first, second| first.impulse_newton_seconds.total_cmp(&second.impulse_newton_seconds));
        let rigid_contact = strongest_contact.map(|contact| UnderfloorRigidContact {
            confirmed: true,
            local_position: contact.local_position,
            normal_impulse_ns: contact.impulse_newton_seconds,
            tangential_speed_m_s: contact.tangential_speed_metres_per_second,
        }).unwrap_or(sample.rigid_contact);
        let rigid_confirmed = config.enabled && rigid_contact.confirmed;
        if rigid_confirmed {
            self.rigid_hold_s = config.rigid_hold_seconds;
        } else {
            self.rigid_hold_s = (self.rigid_hold_s - dt).max(0.0);
        }
        let rigid_latched = rigid_confirmed || self.rigid_hold_s > 0.0;
        if rigid_confirmed {
            // Keep the last confirmed diagnostics across the hold so telemetry
            // does not blank out between flickering contact frames.
            self.rigid_local_position = rigid_contact.local_position;
            self.rigid_normal_impulse_ns = rigid_contact.normal_impulse_ns;
            self.rigid_tangential_speed_m_s = rigid_contact.tangential_speed_m_s;
        }

        self.step_bottoming(sample, body, config, dt, rigid_latched, two_longitudinal_probes);

        let geometry_confidence: f64 = if self.active_probe_mask != 0 { 1.0 } else { 0.0 };
        self.contact_confidence = geometry_confidence.max(if rigid_latched { 1.0 } else { 0.0 });
        let previous_phase = self.scrape_phase;
        let approaching = config.enabled && self.minimum_clearance_m <= config.approach_clearance_m;
        let contact_now = config.enabled && (rigid_latched || self.active_probe_mask != 0);
        let separating = self.minimum_clearance_m >= config.release_clearance_m && !rigid_latched;
        let approach_speed = self
            .clearance_velocity_m_s
            .iter()
            .copied()
            .map(|v| (-v).max(0.0))
            .fold(0.0, f64::max);
        self.onset_strength = 0.0;
        if contact_now {
            self.hold_s = config.contact_hold_seconds;
            if matches!(
                previous_phase,
                ScrapePhase::Clear | ScrapePhase::Approaching | ScrapePhase::Releasing
            ) {
                self.scrape_phase = ScrapePhase::Impact;
                self.onset_strength = (approach_speed / 1.5
                    + self.rigid_normal_impulse_ns / 1200.0)
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

        let sliding_speed = body.map(|kinematics| kinematics.linear_velocity.length()).unwrap_or(self.rigid_tangential_speed_m_s);
        let speed_factor = (sliding_speed / 35.0).clamp(0.0, 1.0);
        let clearance_span = config.release_clearance_m.max(f64::EPSILON);
        let clearance_factor =
            ((clearance_span - self.minimum_clearance_m) / clearance_span).clamp(0.0, 1.0);
        let impulse_factor = (self.rigid_normal_impulse_ns / 900.0).clamp(0.0, 1.0);
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
        rigid_latched: bool,
        two_longitudinal_probes: bool,
    ) {
        let mut local_positions: [[f64; 3]; 5] = [
            [-0.45, -0.205, -0.90],
            [0.45, -0.205, -0.90],
            [0.0, -0.205, 0.0],
            [0.0, -0.165, 0.85],
            [0.0, -0.165, 1.65],
        ];
        if two_longitudinal_probes {
            local_positions[0][0] = 0.0;
        }
        self.active_probe_mask = 0;
        self.total_normal_force_n = 0.0;
        self.max_probe_force_n = 0.0;
        self.force_world = [0.0; 3];
        self.torque_world = [0.0; 3];
        self.force_center_local = [0.0; 3];
        let target_blend = if rigid_latched { 1.0 } else { 0.0 };
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
            let clearance = self.raw_clearance_m[i];
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
                let p = local_positions[i];
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
            } else if rigid_latched && compression > 0.0 {
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
                let local = local_positions[i];
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
    fn two_longitudinal_probes_preserve_rake_without_artificial_roll() {
        let mut state = UnderfloorState::default();
        let mut sample = UnderfloorSample::default();
        sample.rays[0] = UnderfloorRayHit {
            valid: true,
            clearance_m: 0.100,
            normal_world: [0.0, 1.0, 0.0],
            ..Default::default()
        };
        sample.rays[3] = UnderfloorRayHit {
            valid: true,
            clearance_m: 0.140,
            normal_world: [0.0, 1.0, 0.0],
            ..Default::default()
        };
        state.step_with_probe_layout(&sample, None, &UnderfloorContactConfig::default(), 1.0 / 120.0, true);
        assert!(state.rake_rad.abs() < 1e-12);
        assert_eq!(state.roll_rad, 0.0);
        assert_eq!(state.valid_mask, 0b01001);
    }

    #[test]
    fn front_center_bottoming_does_not_create_lateral_torque() {
        let mut state = UnderfloorState::default();
        let mut sample = UnderfloorSample::default();
        sample.rays[0] = UnderfloorRayHit {
            valid: true,
            clearance_m: 0.004,
            point_world: [0.0, -0.205, -0.90],
            normal_world: [0.0, 1.0, 0.0],
            ..Default::default()
        };
        state.step_with_probe_layout(&sample, None, &UnderfloorContactConfig::default(), 1.0 / 120.0, true);
        assert!(state.normal_force_n[0] > 0.0);
        assert!(state.torque_world[0] > 0.0);
        assert_eq!(state.torque_world[2], 0.0);
        assert_eq!(state.force_center_local[0], 0.0);
    }

    #[test]
    fn rigid_contact_latches_across_frame_flicker() {
        let cfg = UnderfloorContactConfig::default();
        let mut state = UnderfloorState::default();
        // Clearance is above the spring band: only the rigid flag can drive
        // contact, which isolates the debounce latch.
        let mut sample = flat_sample(0.030);
        sample.rigid_contact.confirmed = true;
        sample.rigid_contact.local_position = [0.0, -0.18, 0.0];
        sample.rigid_contact.normal_impulse_ns = 500.0;
        state.step(&sample, None, &cfg, 1.0 / 120.0);
        assert!(state.contact_confidence > 0.99);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        assert!(state.rigid_normal_impulse_ns > 0.0);
        assert!(state.total_normal_force_n == 0.0);

        // The raw flag clears on the next frame; the hold must keep the contact
        // stable for several frames instead of toggling at frame rate.
        let clear = flat_sample(0.030);
        state.step(&clear, None, &cfg, 1.0 / 120.0);
        assert!(
            state.contact_confidence > 0.99,
            "latch must hold confidence: {}",
            state.contact_confidence
        );
        assert!(
            state.rigid_normal_impulse_ns > 0.0,
            "latched diagnostics must persist"
        );

        for _ in 0..12 {
            state.step(&clear, None, &cfg, 1.0 / 120.0);
        }
        assert!(
            state.contact_confidence < 0.01,
            "latch must release after the hold window: {}",
            state.contact_confidence
        );
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
    #[test]
    fn localized_loaded_probe_above_old_threshold_has_audible_intensity() {
        let configuration = UnderfloorContactConfig::default();
        let mut sample = flat_sample(0.12);
        sample.rays[0].clearance_m = 0.020;
        let mut state = UnderfloorState::default();
        state.step(&sample, None, &configuration, 1.0 / 120.0);
        assert_eq!(state.active_probe_mask, 1);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        assert_eq!(state.contact_confidence, 1.0);
        assert!(state.scrape_intensity > 0.0);
    }

    #[test]
    fn raw_floor_contact_survives_stronger_side_contact_and_releases() {
        let configuration = UnderfloorContactConfig::default();
        let mut sample = flat_sample(0.12);
        sample.raw_contact_count = 2;
        sample.raw_contacts[0] = UnderfloorRawContact {
            local_position: [0.0, 0.0, 0.0], local_normal: [1.0, 0.0, 0.0],
            impulse_newton_seconds: 500.0, tangential_speed_metres_per_second: 30.0,
        };
        sample.raw_contacts[1] = UnderfloorRawContact {
            local_position: [0.0, -0.18, 0.0], local_normal: [0.0, 1.0, 0.0],
            impulse_newton_seconds: 10.0, tangential_speed_metres_per_second: 30.0,
        };
        let mut state = UnderfloorState::default();
        state.step(&sample, None, &configuration, 1.0 / 120.0);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        assert_eq!(state.rigid_normal_impulse_ns, 10.0);
        for _ in 0..120 {
            state.step(&flat_sample(0.12), None, &configuration, 1.0 / 120.0);
        }
        assert_eq!(state.scrape_phase, ScrapePhase::Clear);
        assert!(state.scrape_intensity < 0.001);
    }

    #[test]
    fn side_contacts_and_disabled_configuration_do_not_activate_scrape() {
        let mut sample = flat_sample(0.12);
        sample.raw_contact_count = 1;
        sample.raw_contacts[0] = UnderfloorRawContact {
            local_position: [0.0, -0.18, 0.0], local_normal: [1.0, 0.0, 0.0],
            impulse_newton_seconds: 500.0, tangential_speed_metres_per_second: 30.0,
        };
        let mut state = UnderfloorState::default();
        let mut configuration = UnderfloorContactConfig::default();
        state.step(&sample, None, &configuration, 1.0 / 120.0);
        assert_eq!(state.scrape_phase, ScrapePhase::Clear);
        configuration.enabled = false;
        state.step(&flat_sample(0.004), None, &configuration, 1.0 / 120.0);
        assert_eq!(state.scrape_intensity, 0.0);
        assert_eq!(state.total_normal_force_n, 0.0);
    }

    #[test]
    fn configuration_rejects_inverted_thresholds_and_nonfinite_rates() {
        let mut configuration = UnderfloorContactConfig::default();
        configuration.release_clearance_m = configuration.activation_clearance_m;
        assert!(configuration.validate().is_err());
        configuration = UnderfloorContactConfig::default();
        configuration.linear_rate_n_m = f64::NAN;
        assert!(configuration.validate().is_err());
    }

    #[test]
    fn one_frame_probe_contact_is_not_hidden_by_clearance_filter() {
        let configuration = UnderfloorContactConfig::default();
        let mut state = UnderfloorState::default();
        state.step(&flat_sample(0.12), None, &configuration, 1.0 / 120.0);
        let mut sample = flat_sample(0.12);
        sample.rays[0].clearance_m = 0.020;
        state.step(&sample, None, &configuration, 1.0 / 120.0);
        assert!(state.filtered_clearance_m[0] > configuration.activation_clearance_m);
        assert_eq!(state.active_probe_mask, 1);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        assert!(state.scrape_intensity > 0.0);
    }

}
