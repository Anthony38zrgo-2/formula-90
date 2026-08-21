//! Deterministic fusion of five underfloor clearance probes and rigid contacts.

use serde::{Deserialize, Serialize};

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
            hold_s: 0.0,
            initialized: false,
        }
    }
}

impl UnderfloorState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn step(&mut self, sample: &UnderfloorSample, dt: f64) {
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
        self.rake_rad =
            ((self.filtered_clearance_m[2] - self.filtered_clearance_m[4]) / 1.65).atan();

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
        let target = if matches!(
            self.scrape_phase,
            ScrapePhase::Impact | ScrapePhase::Scraping
        ) {
            self.contact_confidence
                * (0.20 + 0.45 * speed_factor + 0.25 * clearance_factor + 0.10 * impulse_factor)
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
        state.step(&flat_sample(0.12), 1.0 / 120.0);
        assert_eq!(state.scrape_phase, ScrapePhase::Clear);
        assert_eq!(state.scrape_intensity, 0.0);
    }
    #[test]
    fn close_rays_create_one_impact_then_sustain() {
        let mut state = UnderfloorState::default();
        state.step(&flat_sample(0.004), 1.0 / 120.0);
        assert_eq!(state.scrape_phase, ScrapePhase::Impact);
        state.step(&flat_sample(0.004), 1.0 / 120.0);
        assert_eq!(state.scrape_phase, ScrapePhase::Scraping);
        assert_eq!(state.onset_strength, 0.0);
    }
}
