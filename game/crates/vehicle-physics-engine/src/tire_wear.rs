use crate::types::WheelIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TireWearConfig {
    pub initial_tread_depth_millimeters: f64,
    pub minimum_tread_depth_millimeters: f64,
    pub tread_wear_per_megajoule_millimeters: f64,
    pub cold_tread_temperature_celsius: f64,
    pub optimal_tread_temperature_celsius: f64,
    pub overheat_tread_temperature_celsius: f64,
    pub extreme_tread_temperature_celsius: f64,
    pub cold_temperature_wear_multiplier: f64,
    pub overheat_temperature_wear_multiplier: f64,
    pub extreme_temperature_wear_multiplier: f64,
    pub friction_reference_coefficient: f64,
    pub friction_sensitivity_exponent: f64,
    pub inner_shoulder_wear_multiplier: f64,
    pub center_wear_multiplier: f64,
    pub outer_shoulder_wear_multiplier: f64,
    pub cliff_start_wear_fraction: f64,
    pub end_of_life_grip_scale: f64,
    pub progressive_grip_loss_fraction: f64,
}

impl Default for TireWearConfig {
    fn default() -> Self {
        Self {
            initial_tread_depth_millimeters: 6.0,
            minimum_tread_depth_millimeters: 0.5,
            tread_wear_per_megajoule_millimeters: 0.0,
            cold_tread_temperature_celsius: 60.0,
            optimal_tread_temperature_celsius: 95.0,
            overheat_tread_temperature_celsius: 120.0,
            extreme_tread_temperature_celsius: 160.0,
            cold_temperature_wear_multiplier: 0.45,
            overheat_temperature_wear_multiplier: 1.6,
            extreme_temperature_wear_multiplier: 3.0,
            friction_reference_coefficient: 2.7,
            friction_sensitivity_exponent: 0.6,
            inner_shoulder_wear_multiplier: 1.0,
            center_wear_multiplier: 0.85,
            outer_shoulder_wear_multiplier: 1.0,
            cliff_start_wear_fraction: 0.75,
            end_of_life_grip_scale: 0.55,
            progressive_grip_loss_fraction: 0.05,
        }
    }
}

impl TireWearConfig {
    pub fn is_enabled(&self) -> bool {
        self.tread_wear_per_megajoule_millimeters > 0.0
            && self.initial_tread_depth_millimeters > self.minimum_tread_depth_millimeters
    }

    pub fn usable_tread_millimeters(&self) -> f64 {
        (self.initial_tread_depth_millimeters - self.minimum_tread_depth_millimeters).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TireWearAxleConfig {
    pub front: TireWearConfig,
    pub rear: TireWearConfig,
}

impl Default for TireWearAxleConfig {
    fn default() -> Self {
        Self {
            front: TireWearConfig::default(),
            rear: TireWearConfig::default(),
        }
    }
}

impl TireWearAxleConfig {
    pub fn for_wheel(&self, wheel: WheelIndex) -> &TireWearConfig {
        if wheel.is_front() {
            &self.front
        } else {
            &self.rear
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelTireWearState {
    pub tread_inner_millimeters: f64,
    pub tread_center_millimeters: f64,
    pub tread_outer_millimeters: f64,
    pub inner_wear_fraction: f64,
    pub center_wear_fraction: f64,
    pub outer_wear_fraction: f64,
    pub wear_grip_scale: f64,
    pub accumulated_friction_work_joules: f64,
    pub last_friction_power_watts: f64,
    pub last_temperature_wear_multipliers: [f64; 3],
}

impl WheelTireWearState {
    fn new(config: &TireWearConfig) -> Self {
        let depth = config.initial_tread_depth_millimeters.max(0.0);
        Self {
            tread_inner_millimeters: depth,
            tread_center_millimeters: depth,
            tread_outer_millimeters: depth,
            inner_wear_fraction: 0.0,
            center_wear_fraction: 0.0,
            outer_wear_fraction: 0.0,
            wear_grip_scale: 1.0,
            accumulated_friction_work_joules: 0.0,
            last_friction_power_watts: 0.0,
            last_temperature_wear_multipliers: [1.0; 3],
        }
    }

    pub fn tread_zone_millimeters(&self) -> [f64; 3] {
        [
            self.tread_inner_millimeters,
            self.tread_center_millimeters,
            self.tread_outer_millimeters,
        ]
    }

    pub fn wear_fractions(&self) -> [f64; 3] {
        [
            self.inner_wear_fraction,
            self.center_wear_fraction,
            self.outer_wear_fraction,
        ]
    }

    pub fn remaining_tread_fraction(&self) -> f64 {
        let wear_fractions = self.wear_fractions();
        (1.0 - wear_fractions.iter().sum::<f64>() / 3.0).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireWearSystem {
    pub wheels: [WheelTireWearState; 4],
}

impl TireWearSystem {
    pub fn new_with_axles(config: &TireWearAxleConfig) -> Self {
        Self {
            wheels: [
                WheelTireWearState::new(&config.front),
                WheelTireWearState::new(&config.front),
                WheelTireWearState::new(&config.rear),
                WheelTireWearState::new(&config.rear),
            ],
        }
    }

    pub fn reset_with_axles(&mut self, config: &TireWearAxleConfig) {
        *self = Self::new_with_axles(config);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn step_after_forces(
        &mut self,
        wheel: WheelIndex,
        config: &TireWearConfig,
        surface_friction_coefficient: f64,
        contact_zone_weights: [f64; 3],
        tread_zone_temperatures_celsius: [f64; 3],
        longitudinal_force_newtons: f64,
        lateral_force_newtons: f64,
        slip_velocity_longitudinal_m_per_s: f64,
        slip_velocity_lateral_m_per_s: f64,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        if !config.is_enabled() {
            state.wear_grip_scale = 1.0;
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        let friction_power_watts = longitudinal_force_newtons.abs()
            * slip_velocity_longitudinal_m_per_s.abs()
            + lateral_force_newtons.abs() * slip_velocity_lateral_m_per_s.abs();
        state.accumulated_friction_work_joules += friction_power_watts * dt;
        state.last_friction_power_watts = friction_power_watts;

        let friction_factor = (surface_friction_coefficient.max(0.0)
            / config.friction_reference_coefficient.max(1e-3))
        .powf(config.friction_sensitivity_exponent.clamp(0.0, 4.0))
        .clamp(0.05, 4.0);
        let zone_wear_multipliers = [
            config.inner_shoulder_wear_multiplier.max(0.0),
            config.center_wear_multiplier.max(0.0),
            config.outer_shoulder_wear_multiplier.max(0.0),
        ];

        let mut treads = state.tread_zone_millimeters();
        let mut temperature_multipliers = [0.0; 3];
        for zone in 0..3 {
            let temperature_multiplier =
                temperature_wear_multiplier(tread_zone_temperatures_celsius[zone], config);
            temperature_multipliers[zone] = temperature_multiplier;
            let zone_friction_power_watts =
                friction_power_watts * contact_zone_weights[zone].max(0.0);
            let wear_millimeters = config.tread_wear_per_megajoule_millimeters
                * (zone_friction_power_watts * dt / 1_000_000.0)
                * temperature_multiplier
                * friction_factor
                * zone_wear_multipliers[zone];
            treads[zone] =
                (treads[zone] - wear_millimeters).max(config.minimum_tread_depth_millimeters);
        }
        state.tread_inner_millimeters = treads[0];
        state.tread_center_millimeters = treads[1];
        state.tread_outer_millimeters = treads[2];
        state.last_temperature_wear_multipliers = temperature_multipliers;
        update_wear_fractions(state, config);

        let weight_sum: f64 = contact_zone_weights.iter().map(|w| w.max(0.0)).sum();
        if weight_sum > 1e-9 {
            let zone_grips = state
                .wear_fractions()
                .map(|fraction| zone_wear_grip_scale(fraction, config));
            state.wear_grip_scale = (0..3)
                .map(|zone| {
                    contact_zone_weights[zone].max(0.0) / weight_sum * zone_grips[zone]
                })
                .sum();
        }
    }
}

fn update_wear_fractions(state: &mut WheelTireWearState, config: &TireWearConfig) {
    let usable = config.usable_tread_millimeters().max(1e-6);
    state.inner_wear_fraction = ((config.initial_tread_depth_millimeters
        - state.tread_inner_millimeters)
        / usable)
        .clamp(0.0, 1.0);
    state.center_wear_fraction = ((config.initial_tread_depth_millimeters
        - state.tread_center_millimeters)
        / usable)
        .clamp(0.0, 1.0);
    state.outer_wear_fraction = ((config.initial_tread_depth_millimeters
        - state.tread_outer_millimeters)
        / usable)
        .clamp(0.0, 1.0);
}

pub fn temperature_wear_multiplier(temperature_celsius: f64, config: &TireWearConfig) -> f64 {
    let cold = config.cold_tread_temperature_celsius;
    let optimal = config.optimal_tread_temperature_celsius.max(cold + 1.0);
    let overheat = config.overheat_tread_temperature_celsius.max(optimal + 1.0);
    let extreme = config.extreme_tread_temperature_celsius.max(overheat + 1.0);
    let cold_multiplier = config.cold_temperature_wear_multiplier.max(0.0);
    let overheat_multiplier = config.overheat_temperature_wear_multiplier.max(0.0);
    let extreme_multiplier = config.extreme_temperature_wear_multiplier.max(0.0);

    if temperature_celsius <= cold {
        cold_multiplier
    } else if temperature_celsius <= optimal {
        lerp(
            cold_multiplier,
            1.0,
            (temperature_celsius - cold) / (optimal - cold),
        )
    } else if temperature_celsius <= overheat {
        lerp(
            1.0,
            overheat_multiplier,
            (temperature_celsius - optimal) / (overheat - optimal),
        )
    } else if temperature_celsius <= extreme {
        lerp(
            overheat_multiplier,
            extreme_multiplier,
            (temperature_celsius - overheat) / (extreme - overheat),
        )
    } else {
        extreme_multiplier
    }
}

pub fn zone_wear_grip_scale(wear_fraction: f64, config: &TireWearConfig) -> f64 {
    let wear = wear_fraction.clamp(0.0, 1.0);
    let end_of_life = config.end_of_life_grip_scale.clamp(0.0, 1.0);
    let progressive = config
        .progressive_grip_loss_fraction
        .clamp(0.0, 1.0 - end_of_life);
    let cliff_start = config.cliff_start_wear_fraction.clamp(0.0, 1.0);
    let cliff_span = (1.0 - cliff_start).max(1e-6);
    let raw_cliff = ((wear - cliff_start) / cliff_span).clamp(0.0, 1.0);
    let cliff_progress = raw_cliff * raw_cliff * (3.0 - 2.0 * raw_cliff);
    let cliff_loss = cliff_progress * (1.0 - progressive - end_of_life).max(0.0);
    (1.0 - progressive * wear - cliff_loss).clamp(0.0, 1.0)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled_and_identity_grip() {
        let config = TireWearConfig::default();
        assert!(!config.is_enabled());
        let mut system = TireWearSystem::new_with_axles(&TireWearAxleConfig::default());
        system.step_after_forces(
            WheelIndex::FrontLeft,
            &config,
            2.7,
            [0.25, 0.5, 0.25],
            [120.0, 120.0, 120.0],
            5000.0,
            5000.0,
            10.0,
            10.0,
            1.0,
        );
        let state = &system.wheels[WheelIndex::FrontLeft as usize];
        assert_eq!(state.wear_grip_scale, 1.0);
        assert_eq!(state.inner_wear_fraction, 0.0);
        assert_eq!(state.tread_inner_millimeters, config.initial_tread_depth_millimeters);
    }

    #[test]
    fn temperature_multiplier_is_monotone_and_bounded() {
        let config = TireWearConfig {
            tread_wear_per_megajoule_millimeters: 1.0,
            ..TireWearConfig::default()
        };
        let mut previous = f64::NEG_INFINITY;
        for step in 0..=200 {
            let temperature = 20.0 + step as f64 * 1.0;
            let multiplier = temperature_wear_multiplier(temperature, &config);
            assert!(multiplier.is_finite());
            assert!(multiplier >= previous - 1e-12, "must be non-decreasing");
            previous = multiplier;
        }
        assert!(
            temperature_wear_multiplier(20.0, &config)
                < temperature_wear_multiplier(95.0, &config)
        );
        assert!(
            temperature_wear_multiplier(95.0, &config)
                < temperature_wear_multiplier(140.0, &config)
        );
        assert_eq!(
            temperature_wear_multiplier(200.0, &config),
            config.extreme_temperature_wear_multiplier
        );
    }

    #[test]
    fn wear_grip_scale_is_monotone_bounded_and_cliffs() {
        let config = TireWearConfig {
            tread_wear_per_megajoule_millimeters: 1.0,
            ..TireWearConfig::default()
        };
        let mut previous = f64::INFINITY;
        let mut slope_before_cliff = 0.0;
        let mut slope_after_cliff = 0.0;
        for step in 0..=1000 {
            let wear = step as f64 / 1000.0;
            let grip = zone_wear_grip_scale(wear, &config);
            assert!(grip <= previous + 1e-12, "must be non-increasing");
            assert!((0.0..=1.0).contains(&grip));
            if (0.1..0.2).contains(&wear) {
                slope_before_cliff = (previous - grip) * 1000.0;
            }
            if (0.85..0.95).contains(&wear) {
                slope_after_cliff = (previous - grip) * 1000.0;
            }
            previous = grip;
        }
        assert!(
            (zone_wear_grip_scale(1.0, &config) - config.end_of_life_grip_scale).abs() < 1e-9
        );
        assert_eq!(zone_wear_grip_scale(0.0, &config), 1.0);
        assert!(
            slope_after_cliff > slope_before_cliff * 3.0,
            "cliff must steepen the grip loss: before={slope_before_cliff} after={slope_after_cliff}"
        );
    }
}
