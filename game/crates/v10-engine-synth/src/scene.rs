use crate::acoustics::{DcBlocker, ModalBank, OnePoleLowPass};
use crate::engine::EngineFrame;

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
    pub output_gain: f32,
}

impl Default for AcousticSceneConfig {
    fn default() -> Self {
        Self {
            dry_low_gain: 0.45,
            dry_mid_gain: 0.18,
            dry_high_gain: 0.12,
            metal_gain: 1.00,
            gearbox_gain: 0.72,
            head_cover_gain: 0.70,
            airbox_gain: 0.64,
            engine_cover_gain: 0.42,
            rear_exhaust_gain: 0.48,
            output_gain: 2.60,
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
    pub output: f32,
}

pub struct GearboxHousing {
    casing: ModalBank,
    gear_mesh: ModalBank,
    transmission_delay: Vec<f32>,
    delay_cursor: usize,
    dc: DcBlocker,
}

pub struct CylinderHeadCovers {
    bank_a: ModalBank,
    bank_b: ModalBank,
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
    bank_a: ModalBank,
    bank_b: ModalBank,
    delay_a: Vec<f32>,
    delay_b: Vec<f32>,
    cursor_a: usize,
    cursor_b: usize,
    dc_a: DcBlocker,
    dc_b: DcBlocker,
    lowpass_a: OnePoleLowPass,
    lowpass_b: OnePoleLowPass,
}

impl RearExhaustCapture {
    pub fn new(sample_rate: f32) -> Self {
        let delay = |milliseconds: f32| {
            vec![0.0; (milliseconds * 0.001 * sample_rate).round().max(1.0) as usize + 1]
        };
        Self {
            bank_a: ModalBank::new(
                &[
                    (584.0, 0.0100, 0.24),
                    (773.0, 0.0085, -0.22),
                    (1_013.0, 0.0070, 0.20),
                    (1_307.0, 0.0058, -0.18),
                    (1_674.0, 0.0047, 0.16),
                    (2_129.0, 0.0038, -0.135),
                    (2_697.0, 0.0030, 0.105),
                    (3_421.0, 0.0023, -0.078),
                    (4_317.0, 0.0017, 0.052),
                ],
                sample_rate,
            ),
            bank_b: ModalBank::new(
                &[
                    (607.0, 0.0096, -0.23),
                    (806.0, 0.0081, 0.215),
                    (1_049.0, 0.0067, -0.195),
                    (1_352.0, 0.0055, 0.175),
                    (1_729.0, 0.0045, -0.15),
                    (2_196.0, 0.0036, 0.128),
                    (2_781.0, 0.0029, -0.10),
                    (3_519.0, 0.0022, 0.074),
                    (4_449.0, 0.0016, -0.048),
                ],
                sample_rate,
            ),
            delay_a: delay(1.85),
            delay_b: delay(2.20),
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
    pub fn process(&mut self, frame: &EngineFrame) -> (f32, f32) {
        let load_radiation = 0.32 + 0.68 * frame.load * (0.35 + 0.65 * frame.throttle);
        let excite_a = (frame.collector_a * 1.18 + frame.headers_a * 0.20) * load_radiation;
        let excite_b = (frame.collector_b * 1.18 + frame.headers_b * 0.20) * load_radiation;
        let arrived_a = Self::delayed(&mut self.delay_a, &mut self.cursor_a, excite_a);
        let arrived_b = Self::delayed(&mut self.delay_b, &mut self.cursor_b, excite_b);
        let resonant_a = self.bank_a.process(arrived_a);
        let resonant_b = self.bank_b.process(arrived_b);
        let radiated_a = self
            .lowpass_a
            .process(self.dc_a.process(arrived_a * 0.62 + resonant_a * 1.25));
        let radiated_b = self
            .lowpass_b
            .process(self.dc_b.process(arrived_b * 0.62 + resonant_b * 1.25));
        (
            (radiated_a * 12.0).tanh() / 3.1,
            (radiated_b * 12.0).tanh() / 3.1,
        )
    }
}

impl EngineCover {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = (0.00135 * sample_rate).round().max(1.0) as usize;
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
    pub fn process(&mut self, frame: &EngineFrame, airbox: f32) -> f32 {
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

        let broad = self.broad_panels.process(arrived);
        let skin = self
            .upper_skin
            .process(arrived + frame.pressure_derivative * 0.08);
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
    pub fn process(&mut self, frame: &EngineFrame) -> f32 {
        // Intake roar is driven mostly by the changing cylinder demand and
        // turbulent flow. Direct combustion pressure is deliberately absent.
        let demand = frame.pressure_derivative * 0.46
            + frame.turbulence * 0.82
            + (frame.headers_a + frame.headers_b) * 0.055;
        self.chamber_pressure += demand * 0.18 - self.chamber_leak * self.chamber_pressure;
        let chamber = self.chamber_pressure / (1.0 + self.chamber_pressure.abs() * 0.8);
        let body = self.helmholtz.process(chamber + demand * 0.20);
        let runner = self.runners.process(demand + chamber * 0.12);
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
            bank_a: ModalBank::new(
                &[
                    (1_184.0, 0.0072, 0.22),
                    (1_427.0, 0.0061, -0.20),
                    (1_731.0, 0.0051, 0.18),
                    (2_086.0, 0.0043, -0.16),
                    (2_511.0, 0.0036, 0.14),
                    (3_018.0, 0.0030, -0.12),
                    (3_647.0, 0.0024, 0.095),
                    (4_382.0, 0.0019, -0.070),
                    (5_126.0, 0.0015, 0.048),
                ],
                sample_rate,
            ),
            bank_b: ModalBank::new(
                &[
                    (1_219.0, 0.0069, -0.21),
                    (1_471.0, 0.0059, 0.195),
                    (1_784.0, 0.0049, -0.175),
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
    pub fn process(&mut self, frame: &EngineFrame) -> (f32, f32) {
        let excite_a =
            frame.pressure_derivative_a * 1.15 + frame.headers_a * 0.20 + frame.turbulence * 0.045;
        let excite_b =
            frame.pressure_derivative_b * 1.15 + frame.headers_b * 0.20 - frame.turbulence * 0.045;
        let arrived_a = Self::delayed(&mut self.delay_a, &mut self.cursor_a, excite_a);
        let arrived_b = Self::delayed(&mut self.delay_b, &mut self.cursor_b, excite_b);
        let cover_a = self.dc_a.process(self.bank_a.process(arrived_a));
        let cover_b = self.dc_b.process(self.bank_b.process(arrived_b));
        ((cover_a * 32.0).tanh() / 3.2, (cover_b * 32.0).tanh() / 3.2)
    }
}

impl GearboxHousing {
    pub fn new(sample_rate: f32) -> Self {
        let delay_samples = (0.00038 * sample_rate).round().max(1.0) as usize;
        Self {
            casing: ModalBank::new(
                &[
                    (246.0, 0.034, 0.38),
                    (329.0, 0.029, -0.34),
                    (443.0, 0.024, 0.32),
                    (574.0, 0.020, -0.29),
                    (731.0, 0.016, 0.25),
                    (914.0, 0.013, -0.22),
                    (1_147.0, 0.010, 0.18),
                ],
                sample_rate,
            ),
            gear_mesh: ModalBank::new(
                &[
                    (1_423.0, 0.0075, 0.16),
                    (1_769.0, 0.0060, -0.14),
                    (2_183.0, 0.0048, 0.12),
                    (2_677.0, 0.0038, -0.095),
                    (3_241.0, 0.0030, 0.068),
                ],
                sample_rate,
            ),
            transmission_delay: vec![0.0; delay_samples + 1],
            delay_cursor: 0,
            dc: DcBlocker::new(32.0, sample_rate),
        }
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame) -> f32 {
        // The bellhousing is driven through the rear face of the block and by
        // collector pressure, not by a copy of the final engine microphone.
        let excitation = frame.block_head * 0.62
            + frame.block * 0.48
            + (frame.collector_pressure_a + frame.collector_pressure_b) * 0.34
            + frame.exhaust * 0.08;
        let arrived = self.transmission_delay[self.delay_cursor];
        self.transmission_delay[self.delay_cursor] = excitation;
        self.delay_cursor += 1;
        if self.delay_cursor == self.transmission_delay.len() {
            self.delay_cursor = 0;
        }

        let casing = self.casing.process(arrived);
        let mesh = self
            .gear_mesh
            .process(arrived + frame.pressure_derivative * 0.12);
        let housing = self.dc.process(casing + mesh * 0.72);
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
        let taps = [(tap(1.6), 0.56), (tap(3.4), 0.28), (tap(5.8), 0.13)];
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
    heavy_castings: ModalBank,
    bellhousing: ModalBank,
    thin_panels: ModalBank,
    panel_lowpass: OnePoleLowPass,
    panel_highpass: DcBlocker,
    dc: DcBlocker,
}

impl MetallicStructure {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            heavy_castings: ModalBank::new(
                &[
                    (214.0, 0.024, 0.34),
                    (287.0, 0.019, -0.28),
                    (373.0, 0.016, 0.31),
                    (468.0, 0.013, -0.25),
                    (535.0, 0.0090, 0.060),
                    (592.0, 0.0085, 0.200),
                    (665.0, 0.0075, -0.060),
                    (735.0, 0.0065, 0.050),
                    (810.0, 0.0055, -0.040),
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
        }
    }

    #[inline]
    pub fn process(&mut self, frame: &EngineFrame) -> f32 {
        // Mounts mostly conduct the block; the bellhousing also sees collector
        // pressure; thin covers respond to fast head/pressure changes.
        let mount_excitation = frame.block * 1.15
            + frame.block_head * 0.48
            + (frame.collector_pressure_a + frame.collector_pressure_b) * 0.12;
        let bell_excitation =
            frame.head * 1.05 + frame.pressure_derivative * 0.38 + frame.exhaust * 0.10;
        let panel_excitation =
            frame.head * 0.72 + frame.pressure_derivative * 0.55 + frame.turbulence * 0.08;

        let heavy = self.heavy_castings.process(mount_excitation);
        let bell = self.bellhousing.process(bell_excitation);
        let panel_modes = self
            .panel_lowpass
            .process(self.thin_panels.process(panel_excitation));
        // A small high-passed strain component preserves the initial metallic
        // tick that modal ringing alone tends to smear away.
        let panel_edge = self.panel_highpass.process(panel_excitation);
        let metal = self
            .dc
            .process(heavy * 1.00 + bell * 1.20 + panel_modes * 2.35 + panel_edge * 0.16);

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
    dry_lowpass: OnePoleLowPass,
    dry_midpass: OnePoleLowPass,
    air_path: AirPath,
    metal_envelope: f32,
    envelope_attack: f32,
    envelope_release: f32,
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
            dry_lowpass: OnePoleLowPass::new(360.0, sample_rate),
            dry_midpass: OnePoleLowPass::new(2_650.0, sample_rate),
            air_path: AirPath::new(sample_rate),
            metal_envelope: 0.0,
            envelope_attack: 1.0 - (-1.0 / (0.0012 * sample_rate)).exp(),
            envelope_release: 1.0 - (-1.0 / (0.026 * sample_rate)).exp(),
        })
    }

    #[inline]
    pub fn process(&mut self, engine: &EngineFrame) -> AcousticFrame {
        let metallic_structure = self.metal.process(engine);
        let gearbox_housing = self.gearbox.process(engine);
        let (head_cover_a, head_cover_b) = self.head_covers.process(engine);
        let cylinder_head_covers = head_cover_a + head_cover_b;
        let airbox_plenum = self.airbox.process(engine);
        let engine_cover = self.engine_cover.process(engine, airbox_plenum);
        let (rear_exhaust_a, rear_exhaust_b) = self.rear_exhaust.process(engine);
        let rear_exhaust = rear_exhaust_a + rear_exhaust_b;
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
        // and high-frequency air is already attenuated by the remote position.
        let duck = 1.0 - 0.52 * (self.metal_envelope * 10.0).clamp(0.0, 1.0);
        let filtered_dry = dry_low * self.config.dry_low_gain
            + dry_mid * self.config.dry_mid_gain * duck
            + dry_high * self.config.dry_high_gain;
        let engine_air = self.air_path.process(filtered_dry);
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
            output: (engine_air
                + metallic_structure * self.config.metal_gain
                + gearbox_housing * self.config.gearbox_gain
                + cylinder_head_covers * self.config.head_cover_gain
                + airbox_plenum * self.config.airbox_gain
                + engine_cover * self.config.engine_cover_gain
                + rear_exhaust * self.config.rear_exhaust_gain)
                * self.config.output_gain,
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
