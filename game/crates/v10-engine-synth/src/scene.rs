use crate::acoustics::{DcBlocker, ModalBank, OnePoleLowPass};
use crate::engine::EngineFrame;

use crate::crank::CYLINDER_COUNT;

#[derive(Clone, Copy, Debug)]
pub struct AcousticSceneConfig {
    pub dry_low_gain: f32,
    pub dry_mid_gain: f32,
    pub dry_high_gain: f32,
    pub metal_gain: f32,
    pub airbox_gain: f32,
    pub engine_cover_gain: f32,
    pub mount_monocoque_gain: f32,
    pub under_seat_gain: f32,
    pub output_gain: f32,
}

impl Default for AcousticSceneConfig {
    fn default() -> Self {
        Self {
            dry_low_gain: 0.18,
            dry_mid_gain: 0.46,
            dry_high_gain: 0.14,
            metal_gain: 1.00,
            airbox_gain: 0.85,
            engine_cover_gain: 0.42,
            mount_monocoque_gain: 0.06,
            under_seat_gain: 0.04,
            output_gain: 2.90,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AcousticFrame {
    pub engine_dry: f32,
    pub engine_air: f32,
    pub metallic_structure: f32,
    pub airbox_plenum: f32,
    pub engine_cover: f32,
    pub engine_mounts: f32,
    pub monocoque_seat: f32,
    pub mount_monocoque: f32,
    pub under_seat_vibration: f32,
    pub cylinder_mechanical: [f32; CYLINDER_COUNT],
    pub cylinder_mechanical_sum: f32,
    pub output: f32,
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

struct AirPath {
    delay: Vec<f32>,
    cursor: usize,
    taps: [(usize, f32); 3],
}

impl AirPath {
    fn new(sample_rate: f32) -> Self {
        let tap = |milliseconds: f32| (milliseconds * 0.001 * sample_rate).round() as usize;
        let taps = [(tap(2.11), 0.56), (tap(3.91), 0.28), (tap(6.31), 0.13)];
        let maximum = taps.iter().map(|&(delay, _)| delay).max().unwrap_or(0);
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
    airbox: AirboxPlenum,
    engine_cover: EngineCover,
    mount_monocoque: EngineMountMonocoque,
    under_seat: UnderSeatVibration,
    cylinder_paths: [CylinderMechanicalPath; CYLINDER_COUNT],
    metal_order_notch: TrackingOrderNotch,
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
            || !(0.0..=1.5).contains(&config.airbox_gain)
            || !(0.0..=1.5).contains(&config.engine_cover_gain)
            || !(0.0..=1.5).contains(&config.mount_monocoque_gain)
            || !(0.0..=1.5).contains(&config.under_seat_gain)
            || !(0.25..=5.0).contains(&config.output_gain)
        {
            return Err("acoustic scene gain outside supported range".into());
        }
        Ok(Self {
            config,
            metal: MetallicStructure::new(sample_rate),
            airbox: AirboxPlenum::new(sample_rate),
            engine_cover: EngineCover::new(sample_rate),
            mount_monocoque: EngineMountMonocoque::new(sample_rate),
            under_seat: UnderSeatVibration::new(sample_rate),
            cylinder_paths: std::array::from_fn(|index| {
                CylinderMechanicalPath::new(index, sample_rate)
            }),
            metal_order_notch: TrackingOrderNotch::new(sample_rate, 3.2, 0.64),
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
        let airbox_plenum = self.airbox.process(engine, high_rpm);
        let engine_cover = self.engine_cover.process(engine, airbox_plenum, high_rpm);
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
        let duck = 1.0 - 0.10 * (self.metal_envelope * 10.0).clamp(0.0, 1.0);
        let filtered_dry = dry_low * self.config.dry_low_gain
            + dry_mid * self.config.dry_mid_gain * duck
            + dry_high * self.config.dry_high_gain * (1.0 - 0.72 * high_rpm);
        let engine_air = self.air_path.process(filtered_dry);
        let time = self.sample_clock as f32 / self.sample_rate;
        self.sample_clock = self.sample_clock.wrapping_add(1);
        let slow_scene_drift = 1.0
            + 0.007 * (std::f32::consts::TAU * 0.79 * time + 0.4).sin()
            + 0.003 * (std::f32::consts::TAU * 1.09 * time + 2.1).sin();
        AcousticFrame {
            engine_dry: engine.master,
            engine_air,
            metallic_structure,
            airbox_plenum,
            engine_cover,
            engine_mounts,
            monocoque_seat,
            mount_monocoque,
            under_seat_vibration,
            cylinder_mechanical,
            cylinder_mechanical_sum,
            output: self.onboard_highpass.process(
                (engine_air
                    + metallic_structure * self.config.metal_gain * slow_scene_drift
                    + airbox_plenum * self.config.airbox_gain
                    + engine_cover * self.config.engine_cover_gain
                    + mount_monocoque * self.config.mount_monocoque_gain
                    + under_seat_vibration * self.config.under_seat_gain)
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
                airbox_gain: 0.0,
                engine_cover_gain: 0.0,
                mount_monocoque_gain: 0.0,
                under_seat_gain: 0.0,
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
