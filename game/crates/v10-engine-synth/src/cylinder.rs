use crate::config::EngineConfig;

#[derive(Clone, Copy, Debug, Default)]
pub struct CylinderFrame {
    pub pressure: f32,
    pub pressure_derivative: f32,
    pub blowdown: f32,
    pub valve_lift: f32,
}

pub struct Cylinder {
    age_deg: f32,
    last_pressure: f32,
    last_exhaust_flow: f32,
    cycle_gain: f32,
    fixed_gain: f32,
    rng: u64,
}

impl Cylinder {
    pub fn new(index: usize, config: &EngineConfig) -> Self {
        let centered = index as f32 - 4.5;
        let fixed_gain =
            (1.0 + centered / 4.5 * config.cylinder_spread) * config.cylinder_signature[index];
        Self {
            age_deg: 1.0e9,
            last_pressure: 0.0,
            last_exhaust_flow: 0.0,
            cycle_gain: 1.0,
            fixed_gain,
            rng: config.seed ^ ((index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        }
    }

    #[inline]
    fn next_signed(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        bits as f32 / ((1u64 << 24) - 1) as f32 * 2.0 - 1.0
    }

    pub fn fire(&mut self, variation: f32) {
        self.age_deg = 0.0;
        self.cycle_gain = 1.0 + self.next_signed() * variation;
    }

    #[inline]
    fn pressure_shape(age: f32, rise_deg: f32, expansion_decay_deg: f32) -> f32 {
        if age < 0.0 {
            return 0.0;
        }
        if age <= rise_deg {
            let u = (age / rise_deg).clamp(0.0, 1.0);
            let burn = 1.0 - (-6.0 * u.powf(2.35)).exp();
            burn * (-0.55 * u).exp()
        } else {
            let peak = (1.0 - (-6.0f32).exp()) * (-0.55f32).exp();
            peak * (-(age - rise_deg) / expansion_decay_deg).exp()
        }
    }

    pub fn process(
        &mut self,
        deg_per_sample: f32,
        energy: f32,
        config: &EngineConfig,
    ) -> CylinderFrame {
        let shifted_age = self.age_deg - config.combustion_start_deg;
        let shape = Self::pressure_shape(
            shifted_age,
            config.combustion_rise_deg,
            config.expansion_decay_deg,
        );
        let pressure = shape * energy * self.cycle_gain * self.fixed_gain;
        let pressure_derivative = pressure - self.last_pressure;
        self.last_pressure = pressure;

        let valve_age = self.age_deg - config.exhaust_open_deg;
        let valve_lift = if (0.0..config.exhaust_duration_deg).contains(&valve_age) {
            let opening = (valve_age / config.exhaust_opening_ramp_deg).clamp(0.0, 1.0);
            let closing_start = config.exhaust_duration_deg - 24.0;
            let closing = if valve_age > closing_start {
                ((config.exhaust_duration_deg - valve_age) / 24.0).clamp(0.0, 1.0)
            } else {
                1.0
            };
            (opening * std::f32::consts::FRAC_PI_2).sin() * closing
        } else {
            0.0
        };
        // The pipe is acoustically excited by the change in mass-flow proxy,
        // not by a slowly varying unipolar flow. The fast but finite valve ramp
        // supplies the real blowdown edge without inventing broadband noise.
        let exhaust_flow = pressure * valve_lift.powf(1.35);
        let blowdown = exhaust_flow - self.last_exhaust_flow;
        self.last_exhaust_flow = exhaust_flow;
        self.age_deg += deg_per_sample;
        CylinderFrame {
            pressure,
            pressure_derivative,
            blowdown,
            valve_lift,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blowdown_cannot_precede_valve_opening() {
        let config = EngineConfig::default();
        let mut cylinder = Cylinder::new(0, &config);
        cylinder.fire(0.0);
        let deg = 1.0;
        for age in 0..config.exhaust_open_deg as usize {
            let frame = cylinder.process(deg, 1.0, &config);
            assert_eq!(frame.blowdown, 0.0, "blowdown at age {age}");
        }
    }

    #[test]
    fn pressure_event_is_finite_and_decays() {
        let config = EngineConfig::default();
        let mut cylinder = Cylinder::new(0, &config);
        cylinder.fire(0.0);
        let mut peak = 0.0f32;
        let mut last = 0.0;
        for _ in 0..1_500 {
            let frame = cylinder.process(0.5, 1.0, &config);
            assert!(frame.pressure.is_finite() && frame.pressure_derivative.is_finite());
            peak = peak.max(frame.pressure);
            last = frame.pressure;
        }
        assert!(peak > 0.5);
        assert!(last < peak * 0.02, "tail {last} peak {peak}");
    }
}
