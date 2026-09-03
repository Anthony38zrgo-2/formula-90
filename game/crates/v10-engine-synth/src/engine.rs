use crate::acoustics::{BandPassNoise, BlockHead, Collector, DcBlocker};
use crate::config::EngineConfig;
use crate::crank::{Crankshaft, CYLINDER_COUNT};
use crate::cylinder::Cylinder;
use crate::exhaust::runner::RunnerWaveguide;

const COAST_IDLE_RPM: f32 = 1_000.0;
const COAST_MAX_RPM: f32 = 15_000.0;
const COAST_BASE_ENERGY: f32 = 0.12;
const COAST_RPM_ENERGY: f32 = 0.20;
const ENERGY_ATTACK_SECONDS: f32 = 0.018;
const ENERGY_RELEASE_SECONDS: f32 = 0.413;

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

#[inline]
fn target_acoustic_energy(input: EngineInput) -> f32 {
    let rpm_norm = ((input.rpm - COAST_IDLE_RPM) / (COAST_MAX_RPM - COAST_IDLE_RPM))
        .clamp(0.0, 1.0);
    let coast_energy = COAST_BASE_ENERGY + COAST_RPM_ENERGY * rpm_norm;
    let powered_energy =
        0.25 * input.throttle + 0.63 * input.load * (0.35 + 0.65 * input.throttle);
    (coast_energy + powered_energy).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineFrame {
    pub throttle: f32,
    pub load: f32,
    pub combustion_source: f32,
    pub pressure_derivative: f32,
    pub pressure_derivative_a: f32,
    pub pressure_derivative_b: f32,
    pub cylinder_pressure: [f32; CYLINDER_COUNT],
    pub cylinder_pressure_derivative: [f32; CYLINDER_COUNT],
    pub cylinder_blowdown: [f32; CYLINDER_COUNT],
    pub cylinder_headers: [f32; CYLINDER_COUNT],
    pub cylinder_valve_lift: [f32; CYLINDER_COUNT],
    pub cylinder_valve_area: [f32; CYLINDER_COUNT],
    /// Per-cylinder physical exhaust mass flow (kg/s) from the runner boundary.
    pub cylinder_mass_flow: [f32; CYLINDER_COUNT],
    /// Per-cylinder exhaust-runner gas pressure (Pa).
    pub cylinder_runner_pressure: [f32; CYLINDER_COUNT],
    /// Per-cylinder dynamic acoustic pressure perturbation in the runner waveguide (PHY-062).
    pub cylinder_runner_acoustic_pressure: [f32; CYLINDER_COUNT],
    /// Per-cylinder exhaust-runner gas temperature (K), the PHY-061 wave-speed source.
    pub cylinder_runner_temperature_k: [f32; CYLINDER_COUNT],
    /// Per-cylinder normalised header excitation fed into the runner waveguide.
    pub cylinder_exhaust_excitation: [f32; CYLINDER_COUNT],
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
    headers: [RunnerWaveguide; CYLINDER_COUNT],
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
                RunnerWaveguide::new(
                    config.header_lengths_m[index],
                    config.exhaust_wave_speed_mps,
                    config.header_reflection,
                    sample_rate,
                    config.use_temperature_dependent_wave_speed,
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
        let target_energy = target_acoustic_energy(self.input);
        let tau = if target_energy > self.smoothed_energy {
            ENERGY_ATTACK_SECONDS
        } else {
            ENERGY_RELEASE_SECONDS
        };
        let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
        self.smoothed_energy += (target_energy - self.smoothed_energy) * alpha;
        // Combustion quality wanders slowly, rather than drawing a new random
        // gain every mechanical cycle. Two incommensurate rates avoid a loop.
        let time = self.sample_clock as f32 / sample_rate;
        let slow_drift = 1.0
            + 0.010 * (std::f32::consts::TAU * 0.83 * time).sin()
            + 0.005 * (std::f32::consts::TAU * 1.17 * time + 1.3).sin();

        let events = self.crank.step(self.input.rpm);
        let deg_per_sample = self.input.rpm.max(0.0) * 6.0 / sample_rate;
        let mut pressure = 0.0;
        let mut derivative = 0.0;
        let mut derivative_a = 0.0;
        let mut derivative_b = 0.0;
        let mut runners_a = [0.0f32; 5];
        let mut runners_b = [0.0f32; 5];
        let mut header_a = 0.0;
        let mut header_b = 0.0;
        let mut turbulence_trigger = 0.0f32;
        let mut cylinder_pressure = [0.0; CYLINDER_COUNT];
        let mut cylinder_pressure_derivative = [0.0; CYLINDER_COUNT];
        let mut cylinder_blowdown = [0.0; CYLINDER_COUNT];
        let mut cylinder_headers = [0.0; CYLINDER_COUNT];
        let mut cylinder_valve_lift = [0.0; CYLINDER_COUNT];
        let mut cylinder_valve_area = [0.0; CYLINDER_COUNT];
        let mut cylinder_mass_flow = [0.0; CYLINDER_COUNT];
        let mut cylinder_runner_pressure = [0.0; CYLINDER_COUNT];
        let mut cylinder_runner_acoustic_pressure = [0.0; CYLINDER_COUNT];
        let mut cylinder_runner_temperature_k = [0.0; CYLINDER_COUNT];
        let mut cylinder_exhaust_excitation = [0.0; CYLINDER_COUNT];

        for index in 0..CYLINDER_COUNT {
            if events.fired(index) {
                self.cylinders[index].fire(self.config.cycle_variation);
            }
            let cylinder = self.cylinders[index].process(
                deg_per_sample,
                self.smoothed_energy * slow_drift,
                &self.config,
            );
            pressure += cylinder.pressure;
            derivative += cylinder.pressure_derivative;
            if index < 5 {
                derivative_a += cylinder.pressure_derivative;
            } else {
                derivative_b += cylinder.pressure_derivative;
            }
            let header = self
                .headers[index]
                .process(cylinder.exhaust_excitation, cylinder.runner_temperature_k);
            cylinder_pressure[index] = cylinder.pressure;
            cylinder_pressure_derivative[index] = cylinder.pressure_derivative;
            cylinder_blowdown[index] = cylinder.blowdown;
            cylinder_headers[index] = header;
            cylinder_valve_lift[index] = cylinder.valve_lift;
            cylinder_valve_area[index] = cylinder.effective_area_m2;
            cylinder_mass_flow[index] = cylinder.exhaust_mass_flow_kg_s;
            cylinder_runner_pressure[index] = cylinder.runner_pressure_pa;
            cylinder_runner_acoustic_pressure[index] = self.headers[index].acoustic_pressure();
            cylinder_runner_temperature_k[index] = cylinder.runner_temperature_k;
            cylinder_exhaust_excitation[index] = cylinder.exhaust_excitation;
            turbulence_trigger += cylinder.blowdown.abs();
            if index < 5 {
                runners_a[index] = header;
                header_a += header;
            } else {
                runners_b[index - 5] = header;
                header_b += header;
            }
        }

        let structural_derivative = self.source_dc.process(derivative);
        let structure = self.block_head.process(pressure, structural_derivative);
        let block_sum = structure.pressure_direct * self.config.pressure_direct_gain
            + structure.crankcase * self.config.crankcase_gain
            + structure.block * self.config.block_gain
            + structure.head * self.config.head_gain;
        let collector_a = self.collectors[0].process_bank(&runners_a);
        let collector_b = self.collectors[1].process_bank(&runners_b);
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
            throttle: self.input.throttle,
            load: self.input.load,
            combustion_source: pressure,
            pressure_derivative: derivative,
            pressure_derivative_a: derivative_a,
            pressure_derivative_b: derivative_b,
            cylinder_pressure,
            cylinder_pressure_derivative,
            cylinder_blowdown,
            cylinder_headers,
            cylinder_valve_lift,
            cylinder_valve_area,
            cylinder_mass_flow,
            cylinder_runner_pressure,
            cylinder_runner_acoustic_pressure,
            cylinder_runner_temperature_k,
            cylinder_exhaust_excitation,
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
    fn coast_energy_retains_rpm_dependent_engine_drag() {
        let closed = |rpm| {
            target_acoustic_energy(EngineInput {
                rpm,
                throttle: 0.0,
                load: 0.0,
            })
        };

        assert!((closed(1_000.0) - 0.12).abs() < 1.0e-6);
        assert!((closed(8_000.0) - 0.22).abs() < 1.0e-6);
        assert!((closed(15_000.0) - 0.32).abs() < 1.0e-6);
    }

    #[test]
    fn lift_off_energy_release_uses_413_ms_time_constant() {
        let config = EngineConfig::default();
        let sample_rate = config.sample_rate as usize;
        let mut engine = V10Engine::new(config).unwrap();
        engine.smoothed_energy = 1.0;
        engine
            .set_input(EngineInput {
                rpm: 15_000.0,
                throttle: 0.0,
                load: 0.0,
            })
            .unwrap();

        for _ in 0..(sample_rate * 413 / 1_000) {
            engine.render_sample();
        }

        let expected_after_one_tau = 0.32 + (1.0 - 0.32) / std::f32::consts::E;
        assert!((engine.smoothed_energy - expected_after_one_tau).abs() < 2.0e-4);
    }

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
    fn ten_cylinder_signals_remain_independent_before_bank_sum() {
        let baseline = EngineConfig::default();
        let mut altered = baseline.clone();
        altered.cylinder_signature[3] = 0.75;
        let mut engine = V10Engine::new(baseline).unwrap();
        let mut isolated = V10Engine::new(altered).unwrap();
        let input = EngineInput {
            rpm: 7_499.0,
            throttle: 0.72,
            load: 0.66,
        };
        engine.set_input(input).unwrap();
        isolated.set_input(input).unwrap();
        let mut energy = [0.0f64; CYLINDER_COUNT];
        let mut altered_target = false;
        for _ in 0..48_000 {
            let frame = engine.render_sample();
            let isolated_frame = isolated.render_sample();
            let bank_a: f32 = frame.cylinder_pressure_derivative[..5].iter().sum();
            let bank_b: f32 = frame.cylinder_pressure_derivative[5..].iter().sum();
            assert!((bank_a - frame.pressure_derivative_a).abs() < 1.0e-6);
            assert!((bank_b - frame.pressure_derivative_b).abs() < 1.0e-6);
            for (total, sample) in energy
                .iter_mut()
                .zip(frame.cylinder_pressure_derivative.iter())
            {
                *total += (*sample * *sample) as f64;
            }
            for index in 0..CYLINDER_COUNT {
                if index == 3 {
                    altered_target |= frame.cylinder_pressure_derivative[index].to_bits()
                        != isolated_frame.cylinder_pressure_derivative[index].to_bits();
                } else {
                    assert_eq!(
                        frame.cylinder_pressure_derivative[index].to_bits(),
                        isolated_frame.cylinder_pressure_derivative[index].to_bits(),
                        "cylinder {index} changed when only cylinder 3 was altered"
                    );
                    assert_eq!(
                        frame.cylinder_headers[index].to_bits(),
                        isolated_frame.cylinder_headers[index].to_bits(),
                        "header {index} changed when only cylinder 3 was altered"
                    );
                }
            }
        }
        assert!(energy.iter().all(|value| *value > 1.0e-8), "{energy:?}");
        assert!(altered_target, "altered cylinder never diverged");
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
