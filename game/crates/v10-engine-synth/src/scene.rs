use crate::acoustics::{BandPassNoise, DcBlocker, ModalBank, OnePoleLowPass};
use crate::engine::EngineFrame;

use crate::crank::CYLINDER_COUNT;

#[derive(Clone, Copy, Debug)]
pub struct AcousticSceneConfig {
    pub dry_low_gain: f32,
    pub dry_mid_gain: f32,
    pub dry_high_gain: f32,
    pub metal_gain: f32,
    pub gearbox_gain: f32,
    pub head_cover_gain: f32,
    pub airbox_gain: f32,
    pub engine_cover_gain: f32,
    pub rear_exhaust_gain: f32,
    pub mount_monocoque_gain: f32,
    pub under_seat_gain: f32,
    pub cockpit_cavity_gain: f32,
    pub low_mid_parallel_gain: f32,
    pub load_saturation_gain: f32,
    pub event_residual_gain: f32,
    pub output_gain: f32,
}

impl Default for AcousticSceneConfig {
    fn default() -> Self {
        Self {
            dry_low_gain: 0.18,
            dry_mid_gain: 0.32,
            dry_high_gain: 0.14,
            metal_gain: 1.00,
            gearbox_gain: 0.60,
            head_cover_gain: 0.70,
            airbox_gain: 0.85,
            engine_cover_gain: 0.42,
            rear_exhaust_gain: 0.75,
            mount_monocoque_gain: 0.06,
            under_seat_gain: 0.04,
            cockpit_cavity_gain: 0.22,
            low_mid_parallel_gain: 0.14,
            load_saturation_gain: 0.10,
            event_residual_gain: 0.055,
            output_gain: 2.90,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AcousticFrame {
    pub engine_dry: f32,
    pub engine_air: f32,
    pub metallic_structure: f32,
    pub gearbox_housing: f32,
    pub head_cover_a: f32,
    pub head_cover_b: f32,
    pub cylinder_head_covers: f32,
    pub airbox_plenum: f32,
    pub engine_cover: f32,
    pub rear_exhaust_a: f32,
    pub rear_exhaust_b: f32,
    pub rear_exhaust: f32,
    pub engine_mounts: f32,
    pub monocoque_seat: f32,
    pub mount_monocoque: f32,
    pub under_seat_vibration: f32,
    pub cockpit_cavity: f32,
    pub low_mid_parallel: f32,
    pub load_saturation: f32,
    pub event_residual: f32,
    pub cylinder_mechanical: [f32; CYLINDER_COUNT],
    pub cylinder_mechanical_sum: f32,
    pub output: f32,
}

pub struct GearboxHousing {
    casing_low: ModalBank,
    casing_second: ModalBank,
    casing_treble: ModalBank,
    gear_mesh: ModalBank,
    transmission_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
    casing_lowpass: OnePoleLowPass,
}

/// One short structural route per cylinder. These paths retain cylinder
/// identity until after propagation; collectors and the large vehicle
/// structures remain shared.
pub struct CylinderMechanicalPath {
    signature: ModalBank,
    propagation_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
    radiation_lowpass: OnePoleLowPass,
    polarity: f32,
}

impl CylinderMechanicalPath {
    fn new(index: usize, sample_rate: f32) -> Self {
        let bank = if index < 5 { 0.0 } else { 1.0 };
        let position = (index % 5) as f32;
        let spread = position - 2.0;
        let base = 236.0 + spread * 11.5 + bank * 8.0;
        let delay_ms = 0.16 + position * 0.085 + bank * 0.055;
        Self {
            signature: ModalBank::new(
                &[
                    (base, 0.0095 + position * 0.00035, 0.115),
                    (base * (1.47 + bank * 0.018), 0.0072, -0.090),
                    (base * (2.08 - position * 0.012), 0.0050, 0.062),
                ],
                sample_rate,
            ),
            propagation_delay: vec![
                0.0;
                (delay_ms * 0.001 * sample_rate).round().max(1.0) as usize
            ],
            delay_cursor: 0,
            dc: DcBlocker::new(72.0 + position * 4.0, sample_rate),
            radiation_lowpass: OnePoleLowPass::new(1_350.0 + position * 85.0, sample_rate),
            polarity: if (index + index / 5).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            },
        }
    }

    #[inline]
    fn process(&mut self, derivative: f32, blowdown: f32, header: f32) -> f32 {
        let excitation = derivative * 0.72 + blowdown * 0.22 + header * 0.16;
        let arrived = self.propagation_delay[self.delay_cursor];
        self.propagation_delay[self.delay_cursor] = excitation;
        self.delay_cursor += 1;
        if self.delay_cursor == self.propagation_delay.len() {
            self.delay_cursor = 0;
        }
        let resonant = self.signature.process(arrived * self.polarity);
        let radiated = self
            .radiation_lowpass
            .process(self.dc.process(arrived * 0.018 + resonant));
        (radiated * 10.0).tanh() / 4.0
    }
}

pub struct CylinderHeadCovers {
    bank_a_low: ModalBank,
    bank_a_high: ModalBank,
    bank_b_low: ModalBank,
    bank_b_high: ModalBank,
    delay_a: Vec<f32>,
    delay_b: Vec<f32>,
    cursor_a: usize,
    cursor_b: usize,
    dc_a: DcBlocker,
    dc_b: DcBlocker,
}

pub struct AirboxPlenum {
    helmholtz: ModalBank,
    runners: ModalBank,
    chamber_lowpass: OnePoleLowPass,
    chamber_pressure: f32,
    chamber_leak: f32,
    air_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
}

pub struct EngineCover {
    broad_panels: ModalBank,
    upper_skin: ModalBank,
    propagation_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
    radiation_lowpass: OnePoleLowPass,
}

pub struct RearExhaustCapture {
    bank_a_low: ModalBank,
    bank_a_high: ModalBank,
    bank_b_low: ModalBank,
    bank_b_high: ModalBank,
    delay_a: Vec<f32>,
    delay_b: Vec<f32>,
    cursor_a: usize,
    cursor_b: usize,
    dc_a: DcBlocker,
    dc_b: DcBlocker,
    lowpass_a: OnePoleLowPass,
    lowpass_b: OnePoleLowPass,
}

pub struct EngineMountMonocoque {
    mount_modes: ModalBank,
    monocoque_modes: ModalBank,
    mount_delay: Vec<f32>,
    monocoque_delay: Vec<f32>,
    mount_cursor: usize,
    monocoque_cursor: usize,
    mount_dc: DcBlocker,
    monocoque_dc: DcBlocker,
    mount_lowpass: OnePoleLowPass,
    monocoque_lowpass: OnePoleLowPass,
}

pub struct UnderSeatVibration {
    seat_modes: ModalBank,
    transmission_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
    lowpass: OnePoleLowPass,
}

pub struct CockpitCavity {
    cavity_modes: ModalBank,
    reflection_delay: Vec<f32>,
    delay_cursor: usize,
    taps: [(usize, f32); 4],
    dc: DcBlocker,
    absorption_lowpass_a: OnePoleLowPass,
    absorption_lowpass_b: OnePoleLowPass,
}

pub struct LowMidParallelCompressor {
    band_lowpass_a: OnePoleLowPass,
    band_lowpass_b: OnePoleLowPass,
    band_highpass: OnePoleLowPass,
    envelope: f32,
    attack: f32,
    release: f32,
    threshold: f32,
    ratio: f32,
    makeup: f32,
    dc: DcBlocker,
}

pub struct LoadDependentSaturation {
    pre_lowpass_a: OnePoleLowPass,
    pre_lowpass_b: OnePoleLowPass,
    low_reject: OnePoleLowPass,
    post_lowpass: OnePoleLowPass,
    dc: DcBlocker,
}

/// RPM-tracking rejection used only on structural routes that over-capture
/// the V10 firing family. The dry engine and final mix never pass through it.
pub struct TrackingOrderNotch {
    sample_rate: f32,
    q: f32,
    wet: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl TrackingOrderNotch {
    fn new(sample_rate: f32, q: f32, wet: f32) -> Self {
        Self {
            sample_rate,
            q,
            wet,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32, frequency_hz: f32) -> f32 {
        self.process_with_wet(input, frequency_hz, self.wet)
    }

    #[inline]
    fn process_with_wet(&mut self, input: f32, frequency_hz: f32, wet: f32) -> f32 {
        if frequency_hz < 40.0 || frequency_hz >= self.sample_rate * 0.45 {
            return input;
        }
        let omega = std::f32::consts::TAU * frequency_hz / self.sample_rate;
        let alpha = omega.sin() / (2.0 * self.q);
        let a0_recip = 1.0 / (1.0 + alpha);
        let b0 = a0_recip;
        let b1 = -2.0 * omega.cos() * a0_recip;
        let b2 = a0_recip;
        let a1 = b1;
        let a2 = (1.0 - alpha) * a0_recip;
        let notched = b0 * input + b1 * self.x1 + b2 * self.x2 - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = notched;
        input + (notched - input) * wet.clamp(0.0, 1.0)
    }
}

pub struct EventResidual {
    noise: BandPassNoise,
    envelope: f32,
    release: f32,
    rng: u64,
}

impl EventResidual {
    fn new(sample_rate: f32) -> Self {
        Self {
            noise: BandPassNoise::new(2_450.0, 6_200.0, sample_rate),
            envelope: 0.0,
            release: 1.0 - (-1.0 / (0.0028 * sample_rate)).exp(),
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    #[inline]
    fn process(&mut self, frame: &EngineFrame) -> f32 {
        if frame.fired_mask != 0 {
            let impact = frame.pressure_derivative.abs().min(1.0);
            self.envelope = self.envelope.max(0.10 + impact * 0.90);
        } else {
            self.envelope += self.release * (0.0 - self.envelope);
        }
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        let white = bits as f32 / ((1u64 << 24) - 1) as f32 * 2.0 - 1.0;
        self.noise.process(white) * self.envelope * (0.18 + 0.82 * frame.load)
    }
}

impl LoadDependentSaturation {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            pre_lowpass_a: OnePoleLowPass::new(680.0, sample_rate),
            pre_lowpass_b: OnePoleLowPass::new(680.0, sample_rate),
            low_reject: OnePoleLowPass::new(105.0, sample_rate),
            post_lowpass: OnePoleLowPass::new(920.0, sample_rate),
            dc: DcBlocker::new(72.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, structural_bus: f32, throttle: f32, load: f32) -> f32 {
        let below_680 = self
            .pre_lowpass_b
            .process(self.pre_lowpass_a.process(structural_bus));
        let band = below_680 - self.low_reject.process(below_680);
        let torque = load * (0.28 + 0.72 * throttle);
        let drive = 1.0 + 2.35 * torque;
        // A small even-order bias represents unequal compression and tension
        // in mounts/castings. DC removal follows immediately.
        let driven = band * drive * 6.0;
        let asymmetric = driven + 0.075 * driven.abs();
        let colored = asymmetric.tanh() / 6.0;
        self.post_lowpass.process(self.dc.process(colored))
    }
}

impl LowMidParallelCompressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            band_lowpass_a: OnePoleLowPass::new(560.0, sample_rate),
            band_lowpass_b: OnePoleLowPass::new(560.0, sample_rate),
            band_highpass: OnePoleLowPass::new(150.0, sample_rate),
            envelope: 0.0,
            attack: 1.0 - (-1.0 / (0.014 * sample_rate)).exp(),
            release: 1.0 - (-1.0 / (0.075 * sample_rate)).exp(),
            threshold: 0.030,
            ratio: 1.8,
            makeup: 1.55,
            dc: DcBlocker::new(90.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, structural_bus: f32) -> f32 {
        let below_560 = self
            .band_lowpass_b
            .process(self.band_lowpass_a.process(structural_bus));
        let low_tail = self.band_highpass.process(below_560);
        let band = below_560 - low_tail;
        let detector = band.abs();
        let coefficient = if detector > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope += coefficient * (detector - self.envelope);

        let gain = if self.envelope > self.threshold {
            let compressed_level =
                self.threshold * (self.envelope / self.threshold).powf(1.0 / self.ratio);
            compressed_level / self.envelope.max(1.0e-12)
        } else {
            1.0
        };
        self.dc.process(band * gain * self.makeup)
    }
}

impl CockpitCavity {
    pub fn new(sample_rate: f32) -> Self {
        let tap = |milliseconds: f32| (milliseconds * 0.001 * sample_rate).round() as usize;
        let taps = [
            (tap(2.15), 0.38),
            (tap(3.85), -0.24),
            (tap(5.70), 0.17),
            (tap(7.90), -0.11),
        ];
        let maximum = taps.iter().map(|&(delay, _)| delay).max().unwrap();
        Self {
            cavity_modes: ModalBank::new(
                &[
                    (168.0, 0.024, 0.20),
                    (226.0, 0.020, -0.18),
                    (271.0, 0.017, 0.17),
                    (338.0, 0.014, -0.15),
                    (401.0, 0.012, 0.13),
                    (482.0, 0.010, -0.11),
                    (596.0, 0.008, 0.075),
                    (742.0, 0.006, -0.050),
                ],
                sample_rate,
            ),
            reflection_delay: vec![0.0; maximum + 1],
            delay_cursor: 0,
            taps,
            dc: DcBlocker::new(38.0, sample_rate),
            absorption_lowpass_a: OnePoleLowPass::new(1_600.0, sample_rate),
            absorption_lowpass_b: OnePoleLowPass::new(1_600.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(
        &mut self,
        engine_air: f32,
        airbox: f32,
        engine_cover: f32,
        rear_exhaust: f32,
        cylinder_structure: f32,
    ) -> f32 {
        let airborne = engine_air * 0.62
            + airbox * 0.27
            + engine_cover * 0.12
            + rear_exhaust * 0.10
            + cylinder_structure * 0.065;
        self.reflection_delay[self.delay_cursor] = airborne;
        let mut reflected = 0.0;
        for &(delay, gain) in &self.taps {
            let index = (self.delay_cursor + self.reflection_delay.len() - delay)
                % self.reflection_delay.len();
            reflected += self.reflection_delay[index] * gain;
        }
        self.delay_cursor += 1;
        if self.delay_cursor == self.reflection_delay.len() {
            self.delay_cursor = 0;
        }

        let modes = self.cavity_modes.process(airborne * 0.34 + reflected);
        let absorbed = self
            .absorption_lowpass_b
            .process(self.absorption_lowpass_a.process(reflected * 0.42 + modes));
        let cavity = self.dc.process(absorbed);
        (cavity * 12.0).tanh() / 3.2
    }
}

impl UnderSeatVibration {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = (0.00145 * sample_rate).round().max(1.0) as usize;
        Self {
            seat_modes: ModalBank::new(
                &[
                    (92.0, 0.032, 0.13),
                    (117.0, 0.028, -0.18),
                    (146.0, 0.024, 0.20),
                    (179.0, 0.020, -0.18),
                    (216.0, 0.016, 0.15),
                    (263.0, 0.012, -0.09),
                ],
                sample_rate,
            ),
            transmission_delay: vec![0.0; delay_samples + 1],
            delay_cursor: 0,
            dc: DcBlocker::new(28.0, sample_rate),
            lowpass: OnePoleLowPass::new(360.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, mounts: f32, monocoque: f32, load: f32) -> f32 {
        let input = (mounts * 0.34 + monocoque * 0.72) * (0.48 + 0.52 * load);
        let arrived = self.transmission_delay[self.delay_cursor];
        self.transmission_delay[self.delay_cursor] = input;
        self.delay_cursor += 1;
        if self.delay_cursor == self.transmission_delay.len() {
            self.delay_cursor = 0;
        }
        let resonance = self.seat_modes.process(arrived);
        let vibration = self
            .lowpass
            .process(self.dc.process(arrived * 0.045 + resonance));
        // The seat path is deliberately soft and dense. It should be felt as
        // transmitted mass, never heard as an isolated sub oscillator.
        (vibration * 9.0).tanh() / 3.4
    }
}

impl EngineMountMonocoque {
    pub fn new(sample_rate: f32) -> Self {
        let delay = |milliseconds: f32| {
            vec![0.0; (milliseconds * 0.001 * sample_rate).round().max(1.0) as usize + 1]
        };
        Self {
            mount_modes: ModalBank::new(
                &[
                    (148.0, 0.022, 0.10),
                    (193.0, 0.020, -0.12),
                    (247.0, 0.018, 0.31),
                    (318.0, 0.016, -0.34),
                    (402.0, 0.013, 0.30),
                    (515.0, 0.010, -0.25),
                ],
                sample_rate,
            ),
            monocoque_modes: ModalBank::new(
                &[
                    (171.0, 0.021, -0.09),
                    (226.0, 0.019, 0.13),
                    (291.0, 0.017, -0.30),
                    (367.0, 0.014, 0.32),
                    (454.0, 0.012, -0.28),
                    (548.0, 0.009, 0.22),
                ],
                sample_rate,
            ),
            mount_delay: delay(0.18),
            monocoque_delay: delay(0.82),
            mount_cursor: 0,
            monocoque_cursor: 0,
            mount_dc: DcBlocker::new(48.0, sample_rate),
            monocoque_dc: DcBlocker::new(42.0, sample_rate),
            mount_lowpass: OnePoleLowPass::new(920.0, sample_rate),
            monocoque_lowpass: OnePoleLowPass::new(760.0, sample_rate),
        }
    }

    #[inline]
    fn delayed(delay: &mut [f32], cursor: &mut usize, input: f32) -> f32 {
        let arrived = delay[*cursor];
        delay[*cursor] = input;
        *cursor += 1;
        if *cursor == delay.len() {
            *cursor = 0;
        }
        arrived
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, cylinder_structure: f32) -> (f32, f32) {
        let torque_transfer = 0.42 + 0.58 * frame.load * (0.30 + 0.70 * frame.throttle);
        let block_force = (frame.pressure_direct * 0.52
            + frame.block_head * 0.68
            + frame.block * 0.32
            + (frame.collector_pressure_a + frame.collector_pressure_b) * 0.14
            + frame.pressure_derivative * 0.035
            + cylinder_structure * 0.24)
            * torque_transfer;
        let mount_arrival =
            Self::delayed(&mut self.mount_delay, &mut self.mount_cursor, block_force);
        let mount_resonance = self.mount_modes.process(mount_arrival);
        let engine_mounts = self.mount_lowpass.process(
            self.mount_dc
                .process(mount_arrival * 0.10 + mount_resonance),
        );

        let chassis_force = Self::delayed(
            &mut self.monocoque_delay,
            &mut self.monocoque_cursor,
            engine_mounts * 0.84 + frame.block_head * 0.055,
        );
        let shell = self.monocoque_modes.process(chassis_force);
        let monocoque_seat = self
            .monocoque_lowpass
            .process(self.monocoque_dc.process(chassis_force * 0.08 + shell));

        (
            (engine_mounts * 13.0).tanh() / 3.0,
            (monocoque_seat * 15.0).tanh() / 3.2,
        )
    }
}

impl RearExhaustCapture {
    pub fn new(sample_rate: f32) -> Self {
        let delay = |milliseconds: f32| {
            vec![0.0; (milliseconds * 0.001 * sample_rate).round().max(1.0) as usize + 1]
        };
        Self {
            bank_a_low: ModalBank::new(
                &[
                    (584.0, 0.0100, 0.24),
                    (773.0, 0.0085, -0.22),
                    (1_013.0, 0.0070, 0.20),
                    (1_307.0, 0.0058, -0.18),
                    (1_674.0, 0.0047, 0.16),
                ],
                sample_rate,
            ),
            bank_a_high: ModalBank::new(
                &[
                    (2_129.0, 0.0038, -0.135),
                    (2_697.0, 0.0030, 0.105),
                    (3_421.0, 0.0023, -0.078),
                    (4_317.0, 0.0017, 0.052),
                ],
                sample_rate,
            ),
            bank_b_low: ModalBank::new(
                &[
                    (607.0, 0.0096, -0.23),
                    (806.0, 0.0081, 0.215),
                    (1_049.0, 0.0067, -0.195),
                    (1_352.0, 0.0055, 0.175),
                    (1_729.0, 0.0045, -0.15),
                ],
                sample_rate,
            ),
            bank_b_high: ModalBank::new(
                &[
                    (2_196.0, 0.0036, 0.128),
                    (2_781.0, 0.0029, -0.10),
                    (3_519.0, 0.0022, 0.074),
                    (4_449.0, 0.0016, -0.048),
                ],
                sample_rate,
            ),
            delay_a: delay(2.915),
            delay_b: delay(3.265),
            cursor_a: 0,
            cursor_b: 0,
            dc_a: DcBlocker::new(115.0, sample_rate),
            dc_b: DcBlocker::new(125.0, sample_rate),
            lowpass_a: OnePoleLowPass::new(6_200.0, sample_rate),
            lowpass_b: OnePoleLowPass::new(6_000.0, sample_rate),
        }
    }

    #[inline]
    fn delayed(delay: &mut [f32], cursor: &mut usize, input: f32) -> f32 {
        let arrived = delay[*cursor];
        delay[*cursor] = input;
        *cursor += 1;
        if *cursor == delay.len() {
            *cursor = 0;
        }
        arrived
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, high_rpm: f32) -> (f32, f32) {
        let load_radiation = 0.32 + 0.68 * frame.load * (0.35 + 0.65 * frame.throttle);
        let excite_a = (frame.collector_a * 1.18 + frame.headers_a * 0.20) * load_radiation;
        let excite_b = (frame.collector_b * 1.18 + frame.headers_b * 0.20) * load_radiation;
        let arrived_a = Self::delayed(&mut self.delay_a, &mut self.cursor_a, excite_a);
        let arrived_b = Self::delayed(&mut self.delay_b, &mut self.cursor_b, excite_b);
        let resonant_a = self.bank_a_low.process(arrived_a) * (1.0 + 0.29 * high_rpm)
            + self.bank_a_high.process(arrived_a) * (1.0 - 0.72 * high_rpm);
        let resonant_b = self.bank_b_low.process(arrived_b) * (1.0 + 0.29 * high_rpm)
            + self.bank_b_high.process(arrived_b) * (1.0 - 0.72 * high_rpm);
        let direct_a = arrived_a * 0.62 * (1.0 - 0.45 * high_rpm);
        let direct_b = arrived_b * 0.62 * (1.0 - 0.45 * high_rpm);
        let radiated_a = self
            .lowpass_a
            .process(self.dc_a.process(direct_a + resonant_a * 1.25));
        let radiated_b = self
            .lowpass_b
            .process(self.dc_b.process(direct_b + resonant_b * 1.25));
        (
            (radiated_a * 12.0).tanh() / 3.1,
            (radiated_b * 12.0).tanh() / 3.1,
        )
    }
}

impl EngineCover {
    pub fn new(sample_rate: f32) -> Self {
        // Panel radiation faces the cockpit microphone ~0.45 m away.
        let delay_samples = (0.001309 * sample_rate).round().max(1.0) as usize;
        Self {
            broad_panels: ModalBank::new(
                &[
                    (684.0, 0.0130, 0.25),
                    (817.0, 0.0110, -0.23),
                    (1_036.0, 0.0092, 0.22),
                    (1_291.0, 0.0077, -0.20),
                    (1_603.0, 0.0063, 0.18),
                    (1_982.0, 0.0051, -0.16),
                ],
                sample_rate,
            ),
            upper_skin: ModalBank::new(
                &[
                    (2_438.0, 0.0042, 0.14),
                    (2_997.0, 0.0034, -0.12),
                    (3_611.0, 0.0027, 0.095),
                    (4_283.0, 0.0021, -0.070),
                    (5_071.0, 0.0016, 0.045),
                ],
                sample_rate,
            ),
            propagation_delay: vec![0.0; delay_samples + 1],
            delay_cursor: 0,
            dc: DcBlocker::new(420.0, sample_rate),
            radiation_lowpass: OnePoleLowPass::new(6_400.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, airbox: f32, high_rpm: f32) -> f32 {
        // The cover is a lightly coupled skin. It receives structure from the
        // heads plus airborne pressure from the plenum and engine bay.
        let excitation =
            frame.head * 0.58 + frame.block * 0.16 + airbox * 0.34 + frame.turbulence * 0.12;
        let arrived = self.propagation_delay[self.delay_cursor];
        self.propagation_delay[self.delay_cursor] = excitation;
        self.delay_cursor += 1;
        if self.delay_cursor == self.propagation_delay.len() {
            self.delay_cursor = 0;
        }

        let broad = self.broad_panels.process(arrived) * (1.0 + 0.28 * high_rpm);
        let skin = self
            .upper_skin
            .process(arrived + frame.pressure_derivative * 0.08)
            * (1.0 - 0.68 * high_rpm);
        let radiated = self
            .dc
            .process(self.radiation_lowpass.process(broad + skin * 1.35));
        (radiated * 21.0).tanh() / 3.15
    }
}

impl AirboxPlenum {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = (0.0026 * sample_rate).round().max(1.0) as usize;
        Self {
            helmholtz: ModalBank::new(
                &[
                    (176.0, 0.031, 0.19),
                    (263.0, 0.023, -0.18),
                    (358.0, 0.018, 0.22),
                    (471.0, 0.014, -0.21),
                ],
                sample_rate,
            ),
            runners: ModalBank::new(
                &[
                    (612.0, 0.012, 0.48),
                    (748.0, 0.010, -0.44),
                    (927.0, 0.0085, 0.40),
                    (1_143.0, 0.0070, -0.35),
                    (1_397.0, 0.0057, 0.30),
                    (1_704.0, 0.0046, -0.23),
                    (2_083.0, 0.0036, 0.15),
                ],
                sample_rate,
            ),
            chamber_lowpass: OnePoleLowPass::new(2_900.0, sample_rate),
            chamber_pressure: 0.0,
            chamber_leak: 1.0 - (-1.0 / (0.0065 * sample_rate)).exp(),
            air_delay: vec![0.0; delay_samples + 1],
            delay_cursor: 0,
            dc: DcBlocker::new(55.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, high_rpm: f32) -> f32 {
        // Intake roar is driven mostly by the changing cylinder demand and
        // turbulent flow. Direct combustion pressure is deliberately absent.
        let demand = frame.pressure_derivative * 0.46
            + frame.turbulence * 0.82
            + (frame.headers_a + frame.headers_b) * 0.055;
        self.chamber_pressure += demand * 0.18 - self.chamber_leak * self.chamber_pressure;
        let chamber = self.chamber_pressure / (1.0 + self.chamber_pressure.abs() * 0.8);
        // The Helmholtz cavity is a 250-600 Hz boom; above 6000 rpm it must
        // shrink so the runner band can carry the intake voice instead.
        let body = self.helmholtz.process(chamber + demand * 0.20) * (1.0 - 0.55 * high_rpm);
        let runner = self.runners.process(demand + chamber * 0.12) * (1.0 + 0.28 * high_rpm);
        let radiated = self.dc.process(
            self.chamber_lowpass
                .process(body + runner + chamber * 0.012),
        );

        let arrived = self.air_delay[self.delay_cursor];
        self.air_delay[self.delay_cursor] = (radiated * 13.0).tanh() / 3.0;
        self.delay_cursor += 1;
        if self.delay_cursor == self.air_delay.len() {
            self.delay_cursor = 0;
        }
        arrived
    }
}

impl CylinderHeadCovers {
    pub fn new(sample_rate: f32) -> Self {
        let delay = |milliseconds: f32| {
            vec![0.0; (milliseconds * 0.001 * sample_rate).round().max(1.0) as usize + 1]
        };
        Self {
            bank_a_low: ModalBank::new(
                &[
                    (1_184.0, 0.0072, 0.22),
                    (1_427.0, 0.0061, -0.20),
                    (1_731.0, 0.0051, 0.18),
                ],
                sample_rate,
            ),
            bank_a_high: ModalBank::new(
                &[
                    (2_086.0, 0.0043, -0.16),
                    (2_511.0, 0.0036, 0.14),
                    (3_018.0, 0.0030, -0.12),
                    (3_647.0, 0.0024, 0.095),
                    (4_382.0, 0.0019, -0.070),
                    (5_126.0, 0.0015, 0.048),
                ],
                sample_rate,
            ),
            bank_b_low: ModalBank::new(
                &[
                    (1_219.0, 0.0069, -0.21),
                    (1_471.0, 0.0059, 0.195),
                    (1_784.0, 0.0049, -0.175),
                ],
                sample_rate,
            ),
            bank_b_high: ModalBank::new(
                &[
                    (2_151.0, 0.0041, 0.155),
                    (2_583.0, 0.0035, -0.135),
                    (3_109.0, 0.0029, 0.115),
                    (3_729.0, 0.0023, -0.090),
                    (4_501.0, 0.0018, 0.066),
                    (5_287.0, 0.0014, -0.044),
                ],
                sample_rate,
            ),
            delay_a: delay(0.72),
            delay_b: delay(0.96),
            cursor_a: 0,
            cursor_b: 0,
            dc_a: DcBlocker::new(620.0, sample_rate),
            dc_b: DcBlocker::new(650.0, sample_rate),
        }
    }

    #[inline]
    fn delayed(delay: &mut [f32], cursor: &mut usize, input: f32) -> f32 {
        let arrived = delay[*cursor];
        delay[*cursor] = input;
        *cursor += 1;
        if *cursor == delay.len() {
            *cursor = 0;
        }
        arrived
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, high_rpm: f32) -> (f32, f32) {
        let excite_a =
            frame.pressure_derivative_a * 1.15 + frame.headers_a * 0.20 + frame.turbulence * 0.045;
        let excite_b =
            frame.pressure_derivative_b * 1.15 + frame.headers_b * 0.20 - frame.turbulence * 0.045;
        let arrived_a = Self::delayed(&mut self.delay_a, &mut self.cursor_a, excite_a);
        let arrived_b = Self::delayed(&mut self.delay_b, &mut self.cursor_b, excite_b);
        // The 1.18-1.78 kHz head-shell family radiates the firing pulse that
        // the reference shows; the 2.1 kHz and up cap ticking does not and
        // instead fuels the overbright second-harmonic bed.
        let cover_a = self.dc_a.process(
            self.bank_a_low.process(arrived_a) * (1.0 + 0.52 * high_rpm)
                + self.bank_a_high.process(arrived_a) * (1.0 - 0.82 * high_rpm),
        );
        let cover_b = self.dc_b.process(
            self.bank_b_low.process(arrived_b) * (1.0 + 0.52 * high_rpm)
                + self.bank_b_high.process(arrived_b) * (1.0 - 0.82 * high_rpm),
        );
        ((cover_a * 32.0).tanh() / 3.2, (cover_b * 32.0).tanh() / 3.2)
    }
}

impl GearboxHousing {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = (0.00038 * sample_rate).round().max(1.0) as usize;
        Self {
            casing_low: ModalBank::new(
                &[
                    (246.0, 0.024, 0.40),
                    (278.0, 0.022, -0.24),
                    (344.0, 0.020, 0.35),
                    (431.0, 0.017, -0.34),
                ],
                sample_rate,
            ),
            casing_second: ModalBank::new(
                &[(522.0, 0.014, 0.32), (588.0, 0.012, -0.26)],
                sample_rate,
            ),
            casing_treble: ModalBank::new(
                &[
                    (742.0, 0.009, 0.10),
                    (914.0, 0.007, -0.07),
                    (1_147.0, 0.005, 0.04),
                ],
                sample_rate,
            ),
            gear_mesh: ModalBank::new(
                &[
                    (1_423.0, 0.0060, 0.080),
                    (1_769.0, 0.0048, -0.065),
                    (2_183.0, 0.0038, 0.052),
                    (2_677.0, 0.0030, -0.038),
                    (3_241.0, 0.0023, 0.024),
                ],
                sample_rate,
            ),
            transmission_delay: vec![0.0; delay_samples + 1],
            delay_cursor: 0,
            dc: DcBlocker::new(32.0, sample_rate),
            casing_lowpass: OnePoleLowPass::new(1_500.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, high_rpm: f32) -> f32 {
        // The bellhousing is driven through the rear face of the block and by
        // collector pressure, not by a copy of the final engine microphone.
        let excitation = frame.block_head * 0.25
            + frame.block * 0.15
            + (frame.collector_pressure_a + frame.collector_pressure_b) * 1.20
            + frame.exhaust * 0.03;
        let arrived = self.transmission_delay[self.delay_cursor];
        self.transmission_delay[self.delay_cursor] = excitation;
        self.delay_cursor += 1;
        if self.delay_cursor == self.transmission_delay.len() {
            self.delay_cursor = 0;
        }

        // At high RPM the hum of the casing's first-order/crank-coupled mass
        // (246-431 Hz) is a wash that buries the midrange, so it is throttled
        // progressively while the second-order shell and the mid cast panels
        // above 700 Hz are allowed to carry the useful engine body.
        let casing = self.casing_low.process(arrived) * (1.0 - 0.60 * high_rpm)
            + self.casing_second.process(arrived) * (1.0 - 0.38 * high_rpm)
            + self.casing_treble.process(arrived) * (1.0 + 0.57 * high_rpm);
        let mesh = self
            .gear_mesh
            .process(arrived + frame.pressure_derivative * 0.075);
        let housing = self
            .casing_lowpass
            .process(self.dc.process(casing + mesh * 0.42));
        // Thick cast alloy has more mass and less hard clipping than the thin
        // panels represented by `MetallicStructure`.
        (housing * 10.0).tanh() / 2.9
    }
}

struct AirPath {
    delay: Vec<f32>,
    cursor: usize,
    taps: [(usize, f32); 3],
}

impl AirPath {
    fn new(sample_rate: f32) -> Self {
        let tap = |milliseconds: f32| (milliseconds * 0.001 * sample_rate).round() as usize;
        let taps = [(tap(2.11), 0.56), (tap(3.91), 0.28), (tap(6.31), 0.13)];
        let maximum = taps.iter().map(|&(delay, _)| delay).max().unwrap();
        Self {
            delay: vec![0.0; maximum + 1],
            cursor: 0,
            taps,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.delay[self.cursor] = input;
        let mut output = 0.0;
        for &(delay, gain) in &self.taps {
            let index = (self.cursor + self.delay.len() - delay) % self.delay.len();
            output += self.delay[index] * gain;
        }
        self.cursor += 1;
        if self.cursor == self.delay.len() {
            self.cursor = 0;
        }
        output
    }
}

/// Short, event-driven resonances of the engine castings, bellhousing and
/// nearby sheet metal. Nothing here free-runs: all modes are excited by the
/// physical stems emitted by `V10Engine`.
pub struct MetallicStructure {
    heavy_low: ModalBank,
    heavy_high: ModalBank,
    bank_signature: ModalBank,
    bellhousing: ModalBank,
    thin_panels: ModalBank,
    panel_lowpass: OnePoleLowPass,
    panel_highpass: DcBlocker,
    dc: DcBlocker,
    // Cast bulk and surface shells arrive at the microphone through different
    // path lengths, so their order-2 and order-5 tones do not share one phase.
    cast_delay: Vec<f32>,
    cast_cursor: usize,
    bell_delay: Vec<f32>,
    bell_cursor: usize,
    panel_delay: Vec<f32>,
    panel_cursor: usize,
}

impl MetallicStructure {
    pub fn new(sample_rate: f32) -> Self {
        let delay = |milliseconds: f32| {
            vec![0.0; (milliseconds * 0.001 * sample_rate).round().max(1.0) as usize + 1]
        };
        Self {
            heavy_low: ModalBank::new(
                &[
                    (214.0, 0.024, 0.34),
                    (287.0, 0.019, -0.28),
                    (373.0, 0.016, 0.31),
                    (468.0, 0.013, -0.25),
                ],
                sample_rate,
            ),
            heavy_high: ModalBank::new(
                &[
                    (535.0, 0.0090, 0.060),
                    (592.0, 0.0085, 0.200),
                    (665.0, 0.0075, -0.060),
                    (735.0, 0.0065, 0.050),
                    (810.0, 0.0055, -0.040),
                ],
                sample_rate,
            ),
            bank_signature: ModalBank::new(
                &[
                    (188.0, 0.018, 0.18),
                    (314.0, 0.014, -0.17),
                    (438.0, 0.011, 0.15),
                    (562.0, 0.008, -0.11),
                ],
                sample_rate,
            ),
            bellhousing: ModalBank::new(
                &[
                    (704.0, 0.010, 0.25),
                    (843.0, 0.0085, -0.24),
                    (1_027.0, 0.0072, 0.23),
                    (1_246.0, 0.0061, -0.21),
                    (1_493.0, 0.0052, 0.19),
                    (1_781.0, 0.0045, -0.17),
                    (2_117.0, 0.0038, 0.15),
                ],
                sample_rate,
            ),
            thin_panels: ModalBank::new(
                &[
                    (2_431.0, 0.0033, 0.28),
                    (2_789.0, 0.0029, -0.26),
                    (3_226.0, 0.0025, 0.23),
                    (3_741.0, 0.0021, -0.20),
                    (4_337.0, 0.0018, 0.16),
                    (5_083.0, 0.0015, -0.12),
                ],
                sample_rate,
            ),
            panel_lowpass: OnePoleLowPass::new(6_200.0, sample_rate),
            panel_highpass: DcBlocker::new(1_650.0, sample_rate),
            dc: DcBlocker::new(45.0, sample_rate),
            cast_delay: delay(3.036),
            cast_cursor: 0,
            bell_delay: delay(0.145),
            bell_cursor: 0,
            panel_delay: delay(0.145),
            panel_cursor: 0,
        }
    }

    #[inline]
    fn delayed(delay: &mut [f32], cursor: &mut usize, input: f32) -> f32 {
        let arrived = delay[*cursor];
        delay[*cursor] = input;
        *cursor += 1;
        if *cursor == delay.len() {
            *cursor = 0;
        }
        arrived
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame, cylinder_structure: f32, high_rpm: f32) -> f32 {
        // Mounts mostly conduct the block; the bellhousing also sees collector
        // pressure; thin covers respond to fast head/pressure changes.
        let mount_excitation = frame.block * 1.15
            + frame.block_head * 0.48
            + (frame.collector_pressure_a + frame.collector_pressure_b) * 0.12
            + cylinder_structure * 0.52;
        let bell_excitation =
            frame.head * 1.05 + frame.pressure_derivative * 0.38 + frame.exhaust * 0.10;
        let panel_excitation =
            frame.head * 0.72 + frame.pressure_derivative * 0.55 + frame.turbulence * 0.08;

        let cast_arrived = Self::delayed(
            &mut self.cast_delay,
            &mut self.cast_cursor,
            mount_excitation,
        );
        // The cast block's 214-468 Hz clatter is cabin boomy noise; at high
        // engine speed the 535 Hz and up cast/bell mass carries the body.
        let heavy = self.heavy_low.process(cast_arrived) * (1.0 - 0.41 * high_rpm)
            + self.heavy_high.process(cast_arrived) * (1.0 + 0.12 * high_rpm);
        // Opposing banks do not load the chassis identically. Keeping their
        // difference separate restores half/integer crank signatures that a
        // summed firing pulse erases.
        let bank_difference = frame.pressure_derivative_a - frame.pressure_derivative_b;
        let bank_body = self.bank_signature.process(bank_difference) * (1.0 - 0.30 * high_rpm);
        let bell_arrived =
            Self::delayed(&mut self.bell_delay, &mut self.bell_cursor, bell_excitation);
        let panel_arrived = Self::delayed(
            &mut self.panel_delay,
            &mut self.panel_cursor,
            panel_excitation,
        );
        let bell = self.bellhousing.process(bell_arrived) * (1.0 + 0.50 * high_rpm);
        let panel_modes = self
            .panel_lowpass
            .process(self.thin_panels.process(panel_arrived))
            * (1.0 - 0.82 * high_rpm);
        // A small high-passed strain component preserves the initial metallic
        // tick that modal ringing alone tends to smear away.
        let panel_edge = self.panel_highpass.process(panel_arrived) * (1.0 - 0.72 * high_rpm);
        let metal = self.dc.process(
            heavy * 1.00 + bank_body * 0.42 + bell * 1.20 + panel_modes * 2.35 + panel_edge * 0.16,
        );

        // Local strain compression keeps coincident modes from producing a
        // bell-like spike. This is coloration of the structure, not a limiter.
        (metal * 8.0).tanh() / 2.35
    }
}

pub struct AcousticScene {
    config: AcousticSceneConfig,
    metal: MetallicStructure,
    gearbox: GearboxHousing,
    head_covers: CylinderHeadCovers,
    airbox: AirboxPlenum,
    engine_cover: EngineCover,
    rear_exhaust: RearExhaustCapture,
    mount_monocoque: EngineMountMonocoque,
    under_seat: UnderSeatVibration,
    cockpit_cavity: CockpitCavity,
    low_mid_parallel: LowMidParallelCompressor,
    load_saturation: LoadDependentSaturation,
    event_residual: EventResidual,
    cylinder_paths: [CylinderMechanicalPath; CYLINDER_COUNT],
    metal_order_notch: TrackingOrderNotch,
    gearbox_order_notch: TrackingOrderNotch,
    parallel_order_notch: TrackingOrderNotch,
    head_cover_order_notch: TrackingOrderNotch,
    dry_lowpass: OnePoleLowPass,
    dry_midpass: OnePoleLowPass,
    air_path: AirPath,
    onboard_highpass: DcBlocker,
    metal_envelope: f32,
    envelope_attack: f32,
    envelope_release: f32,
    sample_rate: f32,
    previous_crank_phase: f32,
    firing_frequency_hz: f32,
    sample_clock: u64,
}

impl AcousticScene {
    pub fn new(sample_rate: f32, config: AcousticSceneConfig) -> Result<Self, String> {
        if !(0.0..=1.5).contains(&config.dry_low_gain)
            || !(0.0..=1.5).contains(&config.dry_mid_gain)
            || !(0.0..=1.5).contains(&config.dry_high_gain)
            || !(0.0..=1.5).contains(&config.metal_gain)
            || !(0.0..=1.5).contains(&config.gearbox_gain)
            || !(0.0..=1.5).contains(&config.head_cover_gain)
            || !(0.0..=1.5).contains(&config.airbox_gain)
            || !(0.0..=1.5).contains(&config.engine_cover_gain)
            || !(0.0..=1.5).contains(&config.rear_exhaust_gain)
            || !(0.0..=1.5).contains(&config.mount_monocoque_gain)
            || !(0.0..=1.5).contains(&config.under_seat_gain)
            || !(0.0..=1.5).contains(&config.cockpit_cavity_gain)
            || !(0.0..=1.5).contains(&config.low_mid_parallel_gain)
            || !(0.0..=1.5).contains(&config.load_saturation_gain)
            || !(0.0..=1.5).contains(&config.event_residual_gain)
            || !(0.25..=5.0).contains(&config.output_gain)
        {
            return Err("acoustic scene gain outside supported range".into());
        }
        Ok(Self {
            config,
            metal: MetallicStructure::new(sample_rate),
            gearbox: GearboxHousing::new(sample_rate),
            head_covers: CylinderHeadCovers::new(sample_rate),
            airbox: AirboxPlenum::new(sample_rate),
            engine_cover: EngineCover::new(sample_rate),
            rear_exhaust: RearExhaustCapture::new(sample_rate),
            mount_monocoque: EngineMountMonocoque::new(sample_rate),
            under_seat: UnderSeatVibration::new(sample_rate),
            cockpit_cavity: CockpitCavity::new(sample_rate),
            low_mid_parallel: LowMidParallelCompressor::new(sample_rate),
            load_saturation: LoadDependentSaturation::new(sample_rate),
            event_residual: EventResidual::new(sample_rate),
            cylinder_paths: std::array::from_fn(|index| {
                CylinderMechanicalPath::new(index, sample_rate)
            }),
            metal_order_notch: TrackingOrderNotch::new(sample_rate, 3.2, 0.64),
            gearbox_order_notch: TrackingOrderNotch::new(sample_rate, 3.0, 0.48),
            parallel_order_notch: TrackingOrderNotch::new(sample_rate, 2.7, 0.70),
            head_cover_order_notch: TrackingOrderNotch::new(sample_rate, 12.0, 0.0),
            dry_lowpass: OnePoleLowPass::new(360.0, sample_rate),
            dry_midpass: OnePoleLowPass::new(2_650.0, sample_rate),
            air_path: AirPath::new(sample_rate),
            onboard_highpass: DcBlocker::new(75.0, sample_rate),
            metal_envelope: 0.0,
            envelope_attack: 1.0 - (-1.0 / (0.0012 * sample_rate)).exp(),
            envelope_release: 1.0 - (-1.0 / (0.026 * sample_rate)).exp(),
            sample_rate,
            previous_crank_phase: 0.0,
            firing_frequency_hz: 0.0,
            sample_clock: 0,
        })
    }

    #[inline]
    pub fn process(&mut self, engine: &EngineFrame) -> AcousticFrame {
        let crank_delta = (engine.crank_phase_deg - self.previous_crank_phase).rem_euclid(720.0);
        self.previous_crank_phase = engine.crank_phase_deg;
        let measured_firing = crank_delta * self.sample_rate / 360.0 * 5.0;
        if measured_firing.is_finite() && measured_firing < self.sample_rate * 0.45 {
            self.firing_frequency_hz += 0.04 * (measured_firing - self.firing_frequency_hz);
        }
        // HR-2: the low-body capture of the gearbox/empty casing and the
        // overbright second-harmonic shell are speed-sensitive, so the whole
        // redistribution rides one continuous high-RPM ramp (8500 -> 14500).
        let rpm = self.firing_frequency_hz * 60.0 / 5.0;
        let high_rpm = ((rpm - 8_500.0) / 6_000.0).clamp(0.0, 1.0);
        let mut cylinder_mechanical = [0.0; CYLINDER_COUNT];
        for (index, path) in self.cylinder_paths.iter_mut().enumerate() {
            cylinder_mechanical[index] = path.process(
                engine.cylinder_pressure_derivative[index],
                engine.cylinder_blowdown[index],
                engine.cylinder_headers[index],
            );
        }
        let cylinder_mechanical_sum = cylinder_mechanical.iter().sum::<f32>() * 0.34;
        let metallic_structure = self.metal_order_notch.process(
            self.metal
                .process(engine, cylinder_mechanical_sum, high_rpm),
            self.firing_frequency_hz,
        );
        let gearbox_housing = self.gearbox_order_notch.process(
            self.gearbox.process(engine, high_rpm),
            self.firing_frequency_hz,
        );
        let (head_cover_a, head_cover_b) = self.head_covers.process(engine, high_rpm);
        let cylinder_head_covers = self.head_cover_order_notch.process_with_wet(
            head_cover_a + head_cover_b,
            self.firing_frequency_hz,
            0.20 + 0.50 * high_rpm,
        );
        let airbox_plenum = self.airbox.process(engine, high_rpm);
        let engine_cover = self.engine_cover.process(engine, airbox_plenum, high_rpm);
        let (rear_exhaust_a, rear_exhaust_b) = self.rear_exhaust.process(engine, high_rpm);
        let rear_exhaust = rear_exhaust_a + rear_exhaust_b;
        let (engine_mounts, monocoque_seat) = self
            .mount_monocoque
            .process(engine, cylinder_mechanical_sum);
        let mount_monocoque = engine_mounts * 0.58 + monocoque_seat;
        let under_seat_vibration =
            self.under_seat
                .process(engine_mounts, monocoque_seat, engine.load);
        let detector = metallic_structure.abs();
        let envelope_coefficient = if detector > self.metal_envelope {
            self.envelope_attack
        } else {
            self.envelope_release
        };
        self.metal_envelope += envelope_coefficient * (detector - self.metal_envelope);

        let dry_low = self.dry_lowpass.process(engine.master);
        let below_mid = self.dry_midpass.process(engine.master);
        let dry_mid = below_mid - dry_low;
        let dry_high = engine.master - below_mid;
        // Only the competing mid band ducks. Low-frequency mass remains stable
        // and the high-frequency air is rolled off as the intake charge pulse
        // shortens at high engine speed.
        let duck = 1.0 - 0.52 * (self.metal_envelope * 10.0).clamp(0.0, 1.0);
        let filtered_dry = dry_low * self.config.dry_low_gain
            + dry_mid * self.config.dry_mid_gain * duck
            + dry_high * self.config.dry_high_gain * (1.0 - 0.72 * high_rpm);
        let engine_air = self.air_path.process(filtered_dry);
        let cockpit_cavity = self.cockpit_cavity.process(
            engine_air,
            airbox_plenum,
            engine_cover,
            rear_exhaust,
            cylinder_mechanical_sum,
        );
        let structural_low_mid_bus = metallic_structure * 0.08
            + gearbox_housing * 0.38
            + mount_monocoque * 1.00
            + under_seat_vibration * 0.62
            + cockpit_cavity * 0.72;
        let parallel_input = self
            .parallel_order_notch
            .process(structural_low_mid_bus, self.firing_frequency_hz);
        let low_mid_parallel = self.low_mid_parallel.process(parallel_input);
        let load_saturation = self.load_saturation.process(
            parallel_input * 0.66 + low_mid_parallel * 0.24,
            engine.throttle,
            engine.load,
        );
        let event_residual = self.event_residual.process(engine);
        let time = self.sample_clock as f32 / self.sample_rate;
        self.sample_clock = self.sample_clock.wrapping_add(1);
        let slow_scene_drift = 1.0
            + 0.007 * (std::f32::consts::TAU * 0.79 * time + 0.4).sin()
            + 0.003 * (std::f32::consts::TAU * 1.09 * time + 2.1).sin();
        AcousticFrame {
            engine_dry: engine.master,
            engine_air,
            metallic_structure,
            gearbox_housing,
            head_cover_a,
            head_cover_b,
            cylinder_head_covers,
            airbox_plenum,
            engine_cover,
            rear_exhaust_a,
            rear_exhaust_b,
            rear_exhaust,
            engine_mounts,
            monocoque_seat,
            mount_monocoque,
            under_seat_vibration,
            cockpit_cavity,
            low_mid_parallel,
            load_saturation,
            event_residual,
            cylinder_mechanical,
            cylinder_mechanical_sum,
            output: self.onboard_highpass.process(
                ((engine_air
                    + metallic_structure * self.config.metal_gain * slow_scene_drift
                    + gearbox_housing * self.config.gearbox_gain * slow_scene_drift
                    + cylinder_head_covers * self.config.head_cover_gain
                    + airbox_plenum * self.config.airbox_gain
                    + engine_cover * self.config.engine_cover_gain
                    + rear_exhaust * self.config.rear_exhaust_gain
                    + mount_monocoque * self.config.mount_monocoque_gain
                    + under_seat_vibration * self.config.under_seat_gain
                    + cockpit_cavity * self.config.cockpit_cavity_gain
                    + low_mid_parallel * self.config.low_mid_parallel_gain
                    + load_saturation * self.config.load_saturation_gain)
                    + event_residual * self.config.event_residual_gain)
                    * self.config.output_gain,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_engine_cannot_create_free_running_metal() {
        let mut scene = AcousticScene::new(48_000.0, AcousticSceneConfig::default()).unwrap();
        for _ in 0..96_000 {
            let frame = scene.process(&EngineFrame::default());
            assert_eq!(frame.metallic_structure, 0.0);
            assert_eq!(frame.output, 0.0);
        }
    }

    #[test]
    fn filtered_air_path_is_audible_when_metal_is_disabled() {
        let mut scene = AcousticScene::new(
            48_000.0,
            AcousticSceneConfig {
                dry_low_gain: 1.0,
                dry_mid_gain: 1.0,
                dry_high_gain: 1.0,
                metal_gain: 0.0,
                gearbox_gain: 0.0,
                head_cover_gain: 0.0,
                airbox_gain: 0.0,
                engine_cover_gain: 0.0,
                rear_exhaust_gain: 0.0,
                mount_monocoque_gain: 0.0,
                under_seat_gain: 0.0,
                cockpit_cavity_gain: 0.0,
                low_mid_parallel_gain: 0.0,
                load_saturation_gain: 0.0,
                event_residual_gain: 0.0,
                output_gain: 1.0,
            },
        )
        .unwrap();
        let source = EngineFrame {
            master: 0.375,
            ..EngineFrame::default()
        };
        // Air propagation deliberately adds latency; after its longest tap the
        // all-passband dry signal is reconstructed without a direct shortcut.
        let mut heard = false;
        for _ in 0..400 {
            let output = scene.process(&source).output;
            heard |= output != 0.0;
        }
        assert!(heard);
    }
}
