use crate::acoustics::{BandPassNoise, BlockHead, Collector, DcBlocker, Header};
use crate::config::EngineConfig;
use crate::crank::{Crankshaft, CYLINDER_COUNT};
use crate::cylinder::Cylinder;

#[derive(Clone, Copy, Debug)]
pub struct EngineInput {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
}

impl EngineInput {
    pub fn validate(self) -> Result<Self, String> {
        if !self.rpm.is_finite() || !(0.0..=25_000.0).contains(&self.rpm) {
            return Err(format!("rpm out of range: {}", self.rpm));
        }
        if !self.throttle.is_finite() || !(0.0..=1.0).contains(&self.throttle) {
            return Err(format!("throttle out of range: {}", self.throttle));
        }
        if !self.load.is_finite() || !(0.0..=1.0).contains(&self.load) {
            return Err(format!("load out of range: {}", self.load));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineFrame {
    pub combustion_source: f32,
    pub pressure_derivative: f32,
    pub pressure_direct: f32,
    pub crankcase: f32,
    pub block: f32,
    pub head: f32,
    pub block_head: f32,
    pub headers_a: f32,
    pub headers_b: f32,
    pub collector_a: f32,
    pub collector_b: f32,
    pub collector_pressure_a: f32,
    pub collector_pressure_b: f32,
    pub exhaust: f32,
    pub turbulence: f32,
    pub master_pre_limiter: f32,
    pub master: f32,
    pub fired_mask: u16,
    pub crank_phase_deg: f32,
    pub limiter_reduction_db: f32,
}

pub struct V10Engine {
    config: EngineConfig,
    crank: Crankshaft,
    cylinders: [Cylinder; CYLINDER_COUNT],
    block_head: BlockHead,
    headers: [Header; CYLINDER_COUNT],
    collectors: [Collector; 2],
    source_dc: DcBlocker,
    master_dc: DcBlocker,
    input: EngineInput,
    smoothed_energy: f32,
    turbulence_filter: BandPassNoise,
    turbulence_envelope: f32,
    noise_rng: u64,
    sample_clock: u64,
}

impl V10Engine {
    pub fn new(config: EngineConfig) -> Result<Self, String> {
        config.validate()?;
        let sample_rate = config.sample_rate as f32;
        Ok(Self {
            crank: Crankshaft::new(config.sample_rate, config.firing_order),
            cylinders: std::array::from_fn(|index| Cylinder::new(index, &config)),
            block_head: BlockHead::new(sample_rate),
            headers: std::array::from_fn(|index| {
                Header::new(
                    config.header_lengths_m[index],
                    config.exhaust_wave_speed_mps,
                    config.header_reflection,
                    sample_rate,
                )
            }),
            collectors: [
                Collector::new(sample_rate, -3.5),
                Collector::new(sample_rate, 3.5),
            ],
            source_dc: DcBlocker::new(16.0, sample_rate),
            master_dc: DcBlocker::new(16.0, sample_rate),
            input: EngineInput {
                rpm: 0.0,
                throttle: 0.0,
                load: 0.0,
            },
            smoothed_energy: 0.0,
            turbulence_filter: BandPassNoise::new(460.0, 2_050.0, sample_rate),
            turbulence_envelope: 0.0,
            noise_rng: config.seed ^ 0xA17E_5EED_D15C_A11E,
            sample_clock: 0,
            config,
        })
    }

    pub fn set_input(&mut self, input: EngineInput) -> Result<(), String> {
        self.input = input.validate()?;
        Ok(())
    }

    #[inline]
    pub fn render_sample(&mut self) -> EngineFrame {
        let sample_rate = self.config.sample_rate as f32;
        let target_energy = 0.055
            + 0.25 * self.input.throttle
            + 0.695 * self.input.load * (0.35 + 0.65 * self.input.throttle);
        let tau = if target_energy > self.smoothed_energy {
            0.018
        } else {
            0.090
        };
        let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
        self.smoothed_energy += (target_energy - self.smoothed_energy) * alpha;
        // Combustion quality wanders slowly, rather than drawing a new random
        // gain every mechanical cycle. Two incommensurate rates avoid a loop.
        let time = self.sample_clock as f32 / sample_rate;
        let slow_drift = 1.0
            + 0.018 * (std::f32::consts::TAU * 0.83 * time).sin()
            + 0.009 * (std::f32::consts::TAU * 1.17 * time + 1.3).sin();

        let events = self.crank.step(self.input.rpm);
        let deg_per_sample = self.input.rpm.max(0.0) * 6.0 / sample_rate;
        let mut pressure = 0.0;
        let mut derivative = 0.0;
        let mut header_a = 0.0;
        let mut header_b = 0.0;
        let mut turbulence_trigger = 0.0f32;

        for index in 0..CYLINDER_COUNT {
            if events.fired(index) {
                self.cylinders[index].fire(self.config.cycle_variation);
            }
            let cylinder =
                self.cylinders[index].process(
                    deg_per_sample,
                    self.smoothed_energy * slow_drift,
                    &self.config,
                );
            pressure += cylinder.pressure;
            derivative += cylinder.pressure_derivative;
            let header = self.headers[index].process(cylinder.blowdown);
            turbulence_trigger += cylinder.blowdown.abs();
            if index < 5 {
                header_a += header;
            } else {
                header_b += header;
            }
        }

        let structural_derivative = self.source_dc.process(derivative);
        let structure = self.block_head.process(pressure, structural_derivative);
        let block_sum = structure.pressure_direct * self.config.pressure_direct_gain
            + structure.crankcase * self.config.crankcase_gain
            + structure.block * self.config.block_gain
            + structure.head * self.config.head_gain;
        let collector_a = self.collectors[0].process(header_a);
        let collector_b = self.collectors[1].process(header_b);
        let exhaust = (collector_a.radiated + collector_b.radiated) * self.config.exhaust_gain;
        let attack = 0.34;
        let release = 1.0 - (-1.0 / (0.0038 * sample_rate)).exp();
        if turbulence_trigger > self.turbulence_envelope {
            self.turbulence_envelope += attack * (turbulence_trigger - self.turbulence_envelope);
        } else {
            self.turbulence_envelope += release * (turbulence_trigger - self.turbulence_envelope);
        }
        let mut x = self.noise_rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.noise_rng = x;
        let noise_bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        let noise = noise_bits as f32 / ((1u64 << 24) - 1) as f32 * 2.0 - 1.0;
        let turbulence = self.turbulence_filter.process(noise)
            * self.turbulence_envelope.sqrt()
            * self.config.turbulence_gain;
        let pre = self
            .master_dc
            .process((block_sum + exhaust + turbulence) * self.config.master_gain);
        self.sample_clock = self.sample_clock.wrapping_add(1);

        // Safety only. At nominal settings the candidate must stay below this
        // knee; any material reduction is reported and fails the perceptual gate.
        let abs = pre.abs();
        let (master, reduction_db) = if abs <= 0.92 {
            (pre, 0.0)
        } else {
            let excess = abs - 0.92;
            let limited_abs = 0.92 + 0.075 * (excess / 0.075).tanh();
            let limited = pre.signum() * limited_abs.min(0.995);
            let reduction = 20.0 * (limited.abs() / abs.max(1.0e-12)).log10();
            (limited, reduction)
        };

        debug_assert!(master.is_finite());
        EngineFrame {
            combustion_source: pressure,
            pressure_derivative: derivative,
            pressure_direct: structure.pressure_direct,
            crankcase: structure.crankcase,
            block: structure.block,
            head: structure.head,
            block_head: block_sum,
            headers_a: header_a,
            headers_b: header_b,
            collector_a: collector_a.radiated,
            collector_b: collector_b.radiated,
            collector_pressure_a: collector_a.pressure,
            collector_pressure_b: collector_b.pressure,
            exhaust,
            turbulence,
            master_pre_limiter: pre,
            master,
            fired_mask: events.mask,
            crank_phase_deg: events.crank_phase_deg,
            limiter_reduction_db: reduction_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_seed_and_controls_are_bit_deterministic() {
        let config = EngineConfig::default();
        let mut a = V10Engine::new(config.clone()).unwrap();
        let mut b = V10Engine::new(config).unwrap();
        let input = EngineInput {
            rpm: 5_000.0,
            throttle: 0.72,
            load: 0.78,
        };
        a.set_input(input).unwrap();
        b.set_input(input).unwrap();
        for _ in 0..96_000 {
            assert_eq!(
                a.render_sample().master.to_bits(),
                b.render_sample().master.to_bits()
            );
        }
    }

    #[test]
    fn nominal_5000_rpm_is_finite_nonzero_and_not_limiter_driven() {
        let config = EngineConfig::default();
        let mut engine = V10Engine::new(config).unwrap();
        engine
            .set_input(EngineInput {
                rpm: 5_000.0,
                throttle: 0.72,
                load: 0.78,
            })
            .unwrap();
        let mut energy = 0.0f64;
        let mut peak = 0.0f32;
        let mut worst_reduction = 0.0f32;
        for _ in 0..96_000 {
            let frame = engine.render_sample();
            assert!(frame.master.is_finite());
            energy += (frame.master * frame.master) as f64;
            peak = peak.max(frame.master.abs());
            worst_reduction = worst_reduction.min(frame.limiter_reduction_db);
        }
        assert!(energy > 1.0e-6);
        assert!(peak < 1.0);
        assert!(
            worst_reduction > -0.5,
            "limiter reduction {worst_reduction} dB"
        );
    }

    #[test]
    fn invalid_control_fails_before_render() {
        let mut engine = V10Engine::new(EngineConfig::default()).unwrap();
        assert!(engine
            .set_input(EngineInput {
                rpm: f32::NAN,
                throttle: 0.5,
                load: 0.5
            })
            .is_err());
        assert!(engine
            .set_input(EngineInput {
                rpm: 5_000.0,
                throttle: 1.2,
                load: 0.5
            })
            .is_err());
    }
}
