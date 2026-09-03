use std::f32::consts::TAU;

#[derive(Clone)]
pub struct DcBlocker {
    x1: f32,
    y1: f32,
    r: f32,
}

#[derive(Clone)]
pub struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

pub struct BandPassNoise {
    low_cut: OnePoleLowPass,
    high_cut_a: OnePoleLowPass,
    high_cut_b: OnePoleLowPass,
}

impl BandPassNoise {
    pub fn new(low_hz: f32, high_hz: f32, sample_rate: f32) -> Self {
        Self {
            low_cut: OnePoleLowPass::new(low_hz, sample_rate),
            high_cut_a: OnePoleLowPass::new(high_hz, sample_rate),
            high_cut_b: OnePoleLowPass::new(high_hz, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let below_high = self.high_cut_b.process(self.high_cut_a.process(input));
        below_high - self.low_cut.process(below_high)
    }
}

impl OnePoleLowPass {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        assert!(cutoff_hz > 0.0 && cutoff_hz < sample_rate * 0.48);
        Self {
            alpha: 1.0 - (-TAU * cutoff_hz / sample_rate).exp(),
            state: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

impl DcBlocker {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            r: (-TAU * cutoff_hz / sample_rate).exp(),
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

#[derive(Clone)]
pub struct Resonator {
    a1: f32,
    a2: f32,
    gain: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    pub fn new(frequency_hz: f32, decay_seconds: f32, gain: f32, sample_rate: f32) -> Self {
        assert!(frequency_hz > 0.0 && frequency_hz < sample_rate * 0.48);
        assert!(decay_seconds > 0.0005 && decay_seconds < 2.0);
        let radius = (-1.0 / (decay_seconds * sample_rate)).exp();
        let omega = TAU * frequency_hz / sample_rate;
        Self {
            a1: 2.0 * radius * omega.cos(),
            a2: -(radius * radius),
            // Normalize the excitation by pole radius. Without this term, a
            // longer decay also multiplies steady-state level and turns a
            // physical resonance into an uncontrolled narrow tonal boost.
            gain: gain * (1.0 - radius),
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, excitation: f32) -> f32 {
        let y = excitation * self.gain + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = if y.abs() < 1.0e-20 { 0.0 } else { y };
        self.y1
    }
}

pub struct ModalBank {
    modes: Vec<Resonator>,
}

impl ModalBank {
    pub fn new(spec: &[(f32, f32, f32)], sample_rate: f32) -> Self {
        Self {
            modes: spec
                .iter()
                .map(|&(f, d, g)| Resonator::new(f, d, g, sample_rate))
                .collect(),
        }
    }

    #[inline]
    pub fn process(&mut self, excitation: f32) -> f32 {
        self.modes
            .iter_mut()
            .map(|mode| mode.process(excitation))
            .sum()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StructureFrame {
    pub pressure_direct: f32,
    pub crankcase: f32,
    pub block: f32,
    pub head: f32,
}

pub struct BlockHead {
    pressure_dc: DcBlocker,
    pressure_lp_a: OnePoleLowPass,
    pressure_lp_b: OnePoleLowPass,
    crankcase: ModalBank,
    block: ModalBank,
    head: ModalBank,
}

impl BlockHead {
    pub fn new(sample_rate: f32) -> Self {
        // Broad, quickly damped structural families. These are not oscillators:
        // they produce output only when pressure derivative excites them.
        Self {
            pressure_dc: DcBlocker::new(28.0, sample_rate),
            pressure_lp_a: OnePoleLowPass::new(720.0, sample_rate),
            pressure_lp_b: OnePoleLowPass::new(720.0, sample_rate),
            crankcase: ModalBank::new(
                &[
                    (86.0, 0.012, 0.92),
                    (132.0, 0.010, 0.84),
                    (196.0, 0.008, 0.72),
                    (278.0, 0.006, 0.58),
                ],
                sample_rate,
            ),
            block: ModalBank::new(
                &[
                    (335.0, 0.0055, 0.46),
                    (470.0, 0.0048, 0.48),
                    (620.0, 0.0042, 0.56),
                    (790.0, 0.0038, 0.58),
                    (980.0, 0.0033, 0.52),
                    (1_225.0, 0.0029, 0.46),
                ],
                sample_rate,
            ),
            head: ModalBank::new(
                &[
                    (760.0, 0.0035, 0.48),
                    (940.0, 0.0031, 0.52),
                    (1_160.0, 0.0028, 0.50),
                    (1_410.0, 0.0025, 0.46),
                    (1_720.0, 0.0022, 0.41),
                    (2_080.0, 0.0019, 0.34),
                    (2_520.0, 0.0016, 0.23),
                    (3_150.0, 0.0013, 0.12),
                ],
                sample_rate,
            ),
        }
    }

    #[inline]
    pub fn process(&mut self, pressure: f32, derivative: f32) -> StructureFrame {
        // Direct pressure supplies mass/body. Two gentle low-passes keep it from
        // becoming another edge layer; the DC blocker removes the mean pressure
        // of overlapping cylinders. The soft compression is pressure-dependent
        // and memoryless, not a master limiter.
        let pressure_ac = self.pressure_dc.process(pressure);
        let pressure_low = self
            .pressure_lp_b
            .process(self.pressure_lp_a.process(pressure_ac));
        let pressure_direct = (pressure_low * 1.8).tanh() / 1.8;
        StructureFrame {
            pressure_direct,
            crankcase: self.crankcase.process(pressure_ac),
            block: self.block.process(derivative),
            head: self.head.process(derivative),
        }
    }
}

pub struct Collector {
    body: ModalBank,
    dc: DcBlocker,
    chamber_pressure: f32,
    chamber_leak: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CollectorFrame {
    pub pressure: f32,
    pub radiated: f32,
}

impl Collector {
    pub fn new(sample_rate: f32, bank_offset: f32) -> Self {
        Self {
            body: ModalBank::new(
                &[
                    (164.0 + bank_offset, 0.022, 1.05),
                    (328.0 + bank_offset * 1.7, 0.016, 0.88),
                    (585.0 + bank_offset * 2.1, 0.008, 0.58),
                    (820.0 + bank_offset * 2.4, 0.006, 0.48),
                    (1_080.0 + bank_offset * 2.8, 0.005, 0.48),
                    (1_390.0 + bank_offset * 2.9, 0.0042, 0.42),
                    (1_720.0 + bank_offset * 3.0, 0.0036, 0.36),
                    (2_080.0 + bank_offset * 3.0, 0.0030, 0.30),
                    (3_250.0 + bank_offset * 4.0, 0.0020, 0.20),
                    (5_200.0 + bank_offset * 5.0, 0.0012, 0.10),
                ],
                sample_rate,
            ),
            dc: DcBlocker::new(18.0, sample_rate),
            chamber_pressure: 0.0,
            // Roughly 3.5 ms pressure relaxation. The chamber integrates the
            // flow derivative arriving from the headers and leaks toward zero.
            chamber_leak: 1.0 - (-1.0 / (0.0035 * sample_rate)).exp(),
        }
    }

    /// Process incoming waves from the 5 individual primary runners into the collector.
    ///
    /// Models pulse interference at the 5-in-1 junction (PHY-071):
    /// - Individual runner pulses collide at the junction, exhibiting constructive/destructive interference.
    /// - Nonlinear confluence resistance attenuates extreme shock spikes as pulses merge into the common volume.
    /// - The chamber pressure integrates the net incident flow derivative and decays with acoustic leak.
    /// - The radiated output combines the transmitted junction wave, the chamber mass pressure, and the collector modal body.
    #[inline]
    pub fn process_bank(&mut self, runners: &[f32; 5]) -> CollectorFrame {
        let sum: f32 = runners.iter().sum();
        let nonlinear_loss = 0.08 * sum * sum.abs();
        let junction_input = sum - nonlinear_loss;

        self.chamber_pressure += 0.42 * junction_input - self.chamber_leak * self.chamber_pressure;
        let pressure = self.chamber_pressure / (1.0 + 0.35 * self.chamber_pressure.abs());
        let resonant = self.body.process(junction_input + pressure * 0.12);
        CollectorFrame {
            pressure,
            // The chamber contributes weight, while the arriving travelling
            // wave preserves the blowdown edge. Making chamber pressure the
            // whole output collapses the collector back into one firing tone.
            radiated: self
                .dc
                .process(junction_input * 0.62 + pressure * 0.18 + resonant),
        }
    }

    #[inline]
    pub fn process(&mut self, header_sum: f32) -> CollectorFrame {
        let runners = [header_sum * 0.2; 5];
        self.process_bank(&runners)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonator_decays_without_free_running_output() {
        let mut resonator = Resonator::new(800.0, 0.01, 0.1, 48_000.0);
        let mut last = 0.0;
        for i in 0..96_000 {
            last = resonator.process(if i == 0 { 1.0 } else { 0.0 });
            assert!(last.is_finite());
        }
        assert!(last.abs() < 1.0e-10, "tail={last}");
    }

    #[test]
    fn collector_bank_interference_produces_finite_bounded_pressure() {
        let mut collector = Collector::new(48_000.0, 0.0);
        let pulses = [0.1, -0.05, 0.08, -0.02, 0.04];
        for _ in 0..1_000 {
            let frame = collector.process_bank(&pulses);
            assert!(frame.pressure.is_finite());
            assert!(frame.radiated.is_finite());
        }
    }

    #[test]
    fn opposing_pulses_interfere_destructively_at_collector_junction() {
        let mut c1 = Collector::new(48_000.0, 0.0);
        let mut c2 = Collector::new(48_000.0, 0.0);
        // Constructive: two in-phase pulses
        let f1 = c1.process_bank(&[0.5, 0.5, 0.0, 0.0, 0.0]);
        // Destructive: two anti-phase pulses cancel at the junction
        let f2 = c2.process_bank(&[0.5, -0.5, 0.0, 0.0, 0.0]);
        assert!(f1.pressure > f2.pressure, "in-phase pulses must create higher junction pressure");
        assert!(f1.radiated.abs() > f2.radiated.abs(), "in-phase pulses must radiate higher amplitude");
    }
}
