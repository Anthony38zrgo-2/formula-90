//! Offline listening-package renderer for the Grand Prix sampler backend.
//!
//! Replays one deterministic telemetry sequence through the production
//! `AudioModule`/mixer for both the Grand Prix sampler and the configured
//! GF509 reference, writing full mixes, pre-master stems, steady holds and
//! both-direction sweeps as WAV files plus a provenance manifest.

use formula90_core::audio::AudioModule;
use std::fs;
use std::path::{Path, PathBuf};
use vehicle_audio_engine::ffi::{VehicleAudioTelemetryV3, VEHICLE_AUDIO_ABI_VERSION};
use vehicle_audio_engine::{ContinuousSourceKind, Trigger};

const SAMPLE_RATE: u32 = 44_100;
const TICK_SECONDS: f64 = 0.01;
const TICK_FRAMES: usize = 441;
const TOTAL_SECONDS: f64 = 28.0;

struct TelemetryTick {
    rpm: f64,
    throttle: f32,
    gear: i32,
    shift_phase: i32,
    rev_limiter_active: u32,
    surface: &'static str,
    scrub: f32,
    trigger: Option<i32>,
}

fn telemetry_sequence() -> Vec<TelemetryTick> {
    let mut ticks = Vec::new();
    let total_ticks = (TOTAL_SECONDS / TICK_SECONDS) as usize;
    let mut previous = TelemetryTick {
        rpm: 4_500.0,
        throttle: 0.05,
        gear: 1,
        shift_phase: 0,
        rev_limiter_active: 0,
        surface: "asphalt",
        scrub: 0.0,
        trigger: None,
    };
    for index in 0..total_ticks {
        let time = index as f64 * TICK_SECONDS;
        let mut tick = TelemetryTick {
            rpm: previous.rpm,
            throttle: previous.throttle,
            gear: previous.gear,
            shift_phase: 0,
            rev_limiter_active: 0,
            surface: "asphalt",
            scrub: 0.0,
            trigger: None,
        };
        if time < 2.0 {
            tick.rpm = 4_500.0;
            tick.throttle = 0.05;
            tick.gear = 1;
        } else if time < 12.5 {
            let phase_start = 2.0;
            let progress = ((time - phase_start) / 10.5).clamp(0.0, 1.0);
            tick.rpm = 4_500.0 + 13_400.0 * progress;
            tick.throttle = 1.0;
            tick.gear = if tick.rpm < 9_500.0 {
                1
            } else if tick.rpm < 11_500.0 {
                2
            } else if tick.rpm < 13_000.0 {
                3
            } else if tick.rpm < 14_500.0 {
                4
            } else if tick.rpm < 16_000.0 {
                5
            } else {
                6
            };
            let shift_marks = [3.0, 4.6, 6.4, 8.4, 10.5];
            for mark in shift_marks {
                if time >= mark && time < mark + 0.06 {
                    tick.shift_phase = 1;
                    let dip = (time - mark) / 0.06;
                    tick.rpm -= 1_800.0 * (1.0 - dip);
                }
            }
            if time >= 12.2 && time < 12.8 {
                tick.rev_limiter_active = 1;
                tick.rpm = 17_900.0 + 60.0 * ((time * 40.0).sin());
            } else if time >= 12.5 {
                tick.rpm = 17_900.0;
            }
        } else if time < 13.5 {
            tick.rpm = 17_900.0;
            tick.throttle = 1.0;
            tick.gear = 6;
        } else if time < 15.0 {
            let progress = ((time - 13.5) / 1.5).clamp(0.0, 1.0);
            tick.rpm = 17_900.0 - 2_900.0 * progress;
            tick.throttle = 0.0;
            tick.gear = 6;
        } else if time < 19.0 {
            let mark = if time < 16.5 {
                15.0
            } else if time < 18.0 {
                16.5
            } else {
                18.0
            };
            let segment = ((time - mark) / 1.5).clamp(0.0, 1.0);
            let base = match mark {
                15.0 => 15_000.0,
                16.5 => 13_800.0,
                _ => 12_500.0,
            };
            tick.rpm = base - 2_500.0 * segment;
            tick.throttle = 0.0;
            tick.gear = match mark {
                15.0 => 6,
                16.5 => 5,
                _ => 4,
            };
            if time >= mark && time < mark + 0.07 {
                tick.shift_phase = 3;
                let blip = (time - mark) / 0.07;
                tick.rpm += 1_100.0 * (1.0 - blip);
            }
        } else if time < 23.0 {
            let progress = ((time - 19.0) / 4.0).clamp(0.0, 1.0);
            tick.rpm = 11_000.0 - 5_000.0 * progress;
            tick.throttle = 0.0;
            tick.gear = 3;
        } else if time < 25.0 {
            tick.rpm = 6_000.0;
            tick.throttle = 0.0;
            tick.gear = 2;
            tick.scrub = 0.55;
            if time < 23.6 {
                tick.surface = "curb";
            } else if time < 24.2 {
                tick.surface = "sand";
            } else {
                tick.surface = "grass";
            }
            if (time - 24.0).abs() < TICK_SECONDS {
                tick.trigger = Some(3);
            }
        } else {
            let progress = ((time - 25.0) / 3.0).clamp(0.0, 1.0);
            tick.rpm = 6_000.0 - 1_500.0 * progress;
            tick.throttle = 0.05;
            tick.gear = 1;
        }
        ticks.push(TelemetryTick { ..tick });
        previous = tick;
    }
    ticks
}

fn packet(tick: &TelemetryTick) -> VehicleAudioTelemetryV3 {
    VehicleAudioTelemetryV3 {
        schema_version: VEHICLE_AUDIO_ABI_VERSION,
        struct_size: std::mem::size_of::<VehicleAudioTelemetryV3>() as u32,
        rpm: tick.rpm,
        idle_rpm: 4_500.0,
        max_rpm: 18_000.0,
        throttle: tick.throttle,
        normalized_engine_load: tick.throttle,
        normalized_engine_torque: (tick.throttle - 0.3).clamp(-1.0, 1.0),
        rpm_derivative: 0.0,
        throttle_derivative: 0.0,
        speed_kph: 0.0,
        slip: 0.0,
        gear: tick.gear,
        torque_sign: if tick.throttle > 0.2 { 1 } else { -1 },
        shift_phase: tick.shift_phase,
        clutch_engagement: 1.0,
        tc_cut_ratio: 0.0,
        rev_limiter_active: tick.rev_limiter_active,
    }
}

fn build_module(
    bank_directory: &Path,
    profile_path: &Path,
    source_override: &str,
) -> Result<AudioModule, String> {
    let mut module = AudioModule::new(Some(bank_directory), true);
    if !module.healthy() {
        return Err("audio bank failed to load".to_string());
    }
    let text = fs::read_to_string(profile_path).map_err(|error| error.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut audio = json.get("audio").cloned().unwrap_or(serde_json::Value::Null);
    if let Some(object) = audio.as_object_mut() {
        object.insert(
            "continuous_source".to_string(),
            serde_json::Value::String(source_override.to_string()),
        );
        object.remove("cpp_dsp");
    }
    let powertrain = vehicle_physics_engine::VehicleConfig::from_json_path(profile_path).ok();
    module.enable_synth_from_profile(Some(&audio), powertrain.as_ref());
    Ok(module)
}

struct StereoBuffer {
    left: Vec<f32>,
    right: Vec<f32>,
}

impl StereoBuffer {
    fn new() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    fn push_block(&mut self, left: &[f32], right: &[f32]) {
        self.left.extend_from_slice(left);
        self.right.extend_from_slice(right);
    }

    fn rms(&self) -> f64 {
        let samples = self.left.len().max(1) as f64 * 2.0;
        let sum: f64 = self
            .left
            .iter()
            .chain(self.right.iter())
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum();
        (sum / samples).sqrt()
    }

    fn peak(&self) -> f32 {
        self.left
            .iter()
            .chain(self.right.iter())
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
    }

    fn scaled(&self, gain: f32) -> Self {
        Self {
            left: self.left.iter().map(|sample| sample * gain).collect(),
            right: self.right.iter().map(|sample| sample * gain).collect(),
        }
    }
}

fn write_wav(path: &Path, buffer: &StereoBuffer) -> Result<(), String> {
    let frames = buffer.left.len().min(buffer.right.len());
    let mut bytes = Vec::with_capacity(44 + frames * 4);
    let data_bytes = (frames * 4) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for index in 0..frames {
        for channel in [buffer.left[index], buffer.right[index]] {
            let clamped = channel.clamp(-1.0, 32_767.0 / 32_768.0);
            let value = (clamped * 32_768.0).round() as i32;
            bytes.extend_from_slice(&(value.clamp(-32_768, 32_767) as i16).to_le_bytes());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn render_sequence(
    module: &mut AudioModule,
    ticks: &[TelemetryTick],
    capture_stems: bool,
) -> (StereoBuffer, Option<Stems>) {
    let mut full = StereoBuffer::new();
    let mut stems = capture_stems.then(Stems::new);
    if let Some(engine) = module.mut_engine() {
        engine.set_stem_capture_enabled(capture_stems);
    }
    let mut left = vec![0.0f32; TICK_FRAMES];
    let mut right = vec![0.0f32; TICK_FRAMES];
    for tick in ticks {
        if let Some(engine) = module.mut_engine() {
            engine.set_telemetry_timed(&packet(tick), tick.surface, TICK_SECONDS as f32);
            if tick.scrub > 0.0 {
                engine.set_tire_scrub_state(
                    [0.35, 0.4, 0.45, 0.5],
                    [0.08, -0.08, 0.1, -0.1],
                    [1.0, 1.0, 0.95, 0.95],
                    [3_500.0, 3_600.0, 4_100.0, 4_200.0],
                    90.0,
                    tick.surface,
                );
            } else {
                engine.set_tire_scrub_state([0.0; 4], [0.0; 4], [1.0; 4], [3_500.0; 4], 90.0, tick.surface);
            }
            if tick.trigger.is_some() {
                engine.trigger(Trigger::Hit1);
            }
        }
        module.render(&mut left, &mut right, TICK_FRAMES);
        full.push_block(&left, &right);
        if let (Some(stems), Some(engine)) = (stems.as_mut(), module.mut_engine()) {
            if let Some(captured) = engine.captured_render_stems(TICK_FRAMES) {
                stems.push(&captured);
            }
        }
    }
    (full, stems)
}

struct Stems {
    engine: StereoBuffer,
    gearbox: StereoBuffer,
    backfire: StereoBuffer,
    limiter: StereoBuffer,
    preserved: StereoBuffer,
    pre_master: StereoBuffer,
}

impl Stems {
    fn new() -> Self {
        Self {
            engine: StereoBuffer::new(),
            gearbox: StereoBuffer::new(),
            backfire: StereoBuffer::new(),
            limiter: StereoBuffer::new(),
            preserved: StereoBuffer::new(),
            pre_master: StereoBuffer::new(),
        }
    }

    fn push(&mut self, captured: &vehicle_audio_engine::GrandPrixRenderStems) {
        self.engine.push_block(&captured.engine_left, &captured.engine_right);
        self.gearbox.push_block(&captured.gearbox_left, &captured.gearbox_right);
        self.backfire.push_block(&captured.backfire_left, &captured.backfire_right);
        self.limiter.push_block(&captured.limiter_left, &captured.limiter_right);
        self.preserved.push_block(&captured.preserved_left, &captured.preserved_right);
        let mut left = captured.engine_left.clone();
        let mut right = captured.engine_right.clone();
        for index in 0..left.len() {
            left[index] += captured.gearbox_left[index]
                + captured.backfire_left[index]
                + captured.limiter_left[index]
                + captured.preserved_left[index];
            right[index] += captured.gearbox_right[index]
                + captured.backfire_right[index]
                + captured.limiter_right[index]
                + captured.preserved_right[index];
        }
        self.pre_master.push_block(&left, &right);
    }
}

fn render_holds(module: &mut AudioModule) -> StereoBuffer {
    let mut buffer = StereoBuffer::new();
    let mut left = vec![0.0f32; TICK_FRAMES];
    let mut right = vec![0.0f32; TICK_FRAMES];
    for rpm in [4_500.0, 6_500.0, 9_000.0, 11_500.0, 15_000.0, 18_000.0] {
        let tick = TelemetryTick {
            rpm,
            throttle: 0.85,
            gear: 4,
            shift_phase: 0,
            rev_limiter_active: 0,
            surface: "asphalt",
            scrub: 0.0,
            trigger: None,
        };
        for _ in 0..60 {
            if let Some(engine) = module.mut_engine() {
                engine.set_telemetry_timed(&packet(&tick), "asphalt", TICK_SECONDS as f32);
            }
            module.render(&mut left, &mut right, TICK_FRAMES);
            buffer.push_block(&left, &right);
        }
    }
    buffer
}

fn render_sweep(module: &mut AudioModule, ascending: bool) -> StereoBuffer {
    let mut buffer = StereoBuffer::new();
    let mut left = vec![0.0f32; TICK_FRAMES];
    let mut right = vec![0.0f32; TICK_FRAMES];
    let ticks = 800;
    for index in 0..ticks {
        let progress = index as f64 / (ticks - 1) as f64;
        let rpm = if ascending {
            4_500.0 + 13_500.0 * progress
        } else {
            18_000.0 - 13_500.0 * progress
        };
        let tick = TelemetryTick {
            rpm,
            throttle: 0.85,
            gear: 4,
            shift_phase: 0,
            rev_limiter_active: 0,
            surface: "asphalt",
            scrub: 0.0,
            trigger: None,
        };
        if let Some(engine) = module.mut_engine() {
            engine.set_telemetry_timed(&packet(&tick), "asphalt", TICK_SECONDS as f32);
        }
        module.render(&mut left, &mut right, TICK_FRAMES);
        buffer.push_block(&left, &right);
    }
    buffer
}

fn render_event_auditions(
    module: &mut AudioModule,
) -> (StereoBuffer, StereoBuffer, StereoBuffer, StereoBuffer) {
    let mut downshifts = StereoBuffer::new();
    let mut downshift_events = StereoBuffer::new();
    let mut backfires = StereoBuffer::new();
    let mut backfire_events = StereoBuffer::new();
    let mut left = vec![0.0f32; TICK_FRAMES];
    let mut right = vec![0.0f32; TICK_FRAMES];
    for _ in 0..3 {
        let tick = TelemetryTick {
            rpm: 10_000.0,
            throttle: 0.0,
            gear: 4,
            shift_phase: 0,
            rev_limiter_active: 0,
            surface: "asphalt",
            scrub: 0.0,
            trigger: None,
        };
        for frame in 0..200 {
            if let Some(engine) = module.mut_engine() {
                engine.set_telemetry_timed(&packet(&tick), "asphalt", TICK_SECONDS as f32);
                if frame == 5 {
                    engine.trigger(Trigger::ShiftDown);
                }
            }
            module.render(&mut left, &mut right, TICK_FRAMES);
            downshifts.push_block(&left, &right);
            if let Some(captured) = module
                .mut_engine()
                .and_then(|engine| engine.captured_render_stems(TICK_FRAMES))
            {
                downshift_events.push_block(&captured.gearbox_left, &captured.gearbox_right);
            }
        }
    }
    for _ in 0..3 {
        let base = TelemetryTick {
            rpm: 14_000.0,
            throttle: 0.95,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: 0,
            surface: "asphalt",
            scrub: 0.0,
            trigger: None,
        };
        for frame in 0..420 {
            if let Some(engine) = module.mut_engine() {
                let tick = TelemetryTick {
                    throttle: if frame == 5 { 0.0 } else { base.throttle },
                    ..base
                };
                engine.set_telemetry_timed(&packet(&tick), "asphalt", TICK_SECONDS as f32);
            }
            module.render(&mut left, &mut right, TICK_FRAMES);
            backfires.push_block(&left, &right);
            if let Some(captured) = module
                .mut_engine()
                .and_then(|engine| engine.captured_render_stems(TICK_FRAMES))
            {
                backfire_events.push_block(&captured.backfire_left, &captured.backfire_right);
            }
        }
    }
    (downshifts, downshift_events, backfires, backfire_events)
}

fn sha256(path: &Path) -> String {
    vehicle_audio_engine::bank::sha256_hex(&fs::read(path).unwrap_or_default())
}

fn main() {
    let worker = std::thread::Builder::new()
        .name("grand-prix-sampler-render".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .expect("render thread");
    if worker.join().is_err() {
        eprintln!("render thread panicked");
        std::process::exit(1);
    }
}

fn run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let arguments: Vec<String> = std::env::args().collect();
    let mut profile_path = root.join("data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    let mut bank_directory = root.join("sounds/banks/commons");
    let mut output_directory = root.join("../reports/audio-v10/grand-prix-sampler/renders");
    let mut index = 1;
    while index + 1 < arguments.len() {
        match arguments[index].as_str() {
            "--profile" => profile_path = PathBuf::from(&arguments[index + 1]),
            "--bank" => bank_directory = PathBuf::from(&arguments[index + 1]),
            "--out" => output_directory = PathBuf::from(&arguments[index + 1]),
            _ => {}
        }
        index += 2;
    }
    let ticks = telemetry_sequence();

    let mut sampler = match build_module(&bank_directory, &profile_path, "grand_prix_sampler") {
        Ok(module) => module,
        Err(error) => {
            eprintln!("sampler module failed: {error}");
            std::process::exit(1);
        }
    };
    let sampler_active = sampler.grand_prix_sampler_enabled();
    if !sampler_active {
        eprintln!("sampler backend did not initialize; aborting listening package");
        std::process::exit(1);
    }
    let bank_sha256 = sampler
        .mut_engine()
        .and_then(|engine| engine.grand_prix_bank_sha256().map(str::to_string))
        .unwrap_or_default();
    let sequence_started = std::time::Instant::now();
    let (sampler_full, stems) = render_sequence(&mut sampler, &ticks, true);
    let sequence_render_seconds = sequence_started.elapsed().as_secs_f64();
    let sampler_diagnostics = sampler
        .mut_engine()
        .and_then(|engine| engine.grand_prix_diagnostics());
    let sampler_holds = render_holds(&mut sampler);
    let sampler_sweep_up = render_sweep(&mut sampler, true);
    let sampler_sweep_down = render_sweep(&mut sampler, false);
    let (downshift_auditions, downshift_event_stem, backfire_auditions, backfire_event_stem) =
        render_event_auditions(&mut sampler);

    let mut reference = match build_module(&bank_directory, &profile_path, "v10_gf509") {
        Ok(module) => module,
        Err(error) => {
            eprintln!("reference module failed: {error}");
            std::process::exit(1);
        }
    };
    let reference_source = reference
        .mut_engine()
        .map(|engine| engine.continuous_source());
    let (reference_full, _) = render_sequence(&mut reference, &ticks, false);

    let reference_rms = reference_full.rms().max(1e-9);
    let sampler_rms = sampler_full.rms().max(1e-9);
    let loudness_match_target = reference_rms.min(sampler_rms);
    let reference_match_gain = (loudness_match_target / reference_rms) as f32;
    let sampler_match_gain = (loudness_match_target / sampler_rms) as f32;

    let outputs: Vec<(&str, &StereoBuffer)> = vec![
        ("full_grand_prix_sampler.wav", &sampler_full),
        ("full_gf509_reference.wav", &reference_full),
        ("steady_holds.wav", &sampler_holds),
        ("sweep_ascending.wav", &sampler_sweep_up),
        ("sweep_descending.wav", &sampler_sweep_down),
        ("audition_downshift_variants.wav", &downshift_auditions),
        ("audition_downshift_gearbox_stem.wav", &downshift_event_stem),
        ("audition_backfire_variants.wav", &backfire_auditions),
        ("audition_backfire_stem.wav", &backfire_event_stem),
    ];
    let mut written: Vec<(String, f32)> = Vec::new();
    for (name, buffer) in &outputs {
        let path = output_directory.join(name);
        if let Err(error) = write_wav(&path, buffer) {
            eprintln!("write {name} failed: {error}");
            std::process::exit(1);
        }
        written.push((name.to_string(), buffer.peak()));
    }
    let loudness_matched: Vec<(&str, StereoBuffer)> = vec![
        (
            "full_grand_prix_sampler_loudness_matched.wav",
            sampler_full.scaled(sampler_match_gain),
        ),
        (
            "full_gf509_reference_loudness_matched.wav",
            reference_full.scaled(reference_match_gain),
        ),
    ];
    for (name, buffer) in &loudness_matched {
        let path = output_directory.join(name);
        if let Err(error) = write_wav(&path, buffer) {
            eprintln!("write {name} failed: {error}");
            std::process::exit(1);
        }
        written.push((name.to_string(), buffer.peak()));
    }
    if let Some(stems) = &stems {
        let stem_outputs: Vec<(&str, &StereoBuffer)> = vec![
            ("stem_engine.wav", &stems.engine),
            ("stem_gearbox.wav", &stems.gearbox),
            ("stem_backfire.wav", &stems.backfire),
            ("stem_limiter.wav", &stems.limiter),
            ("stem_preserved_effects.wav", &stems.preserved),
            ("stem_pre_master_sum.wav", &stems.pre_master),
        ];
        for (name, buffer) in &stem_outputs {
            let path = output_directory.join(name);
            if let Err(error) = write_wav(&path, buffer) {
                eprintln!("write {name} failed: {error}");
                std::process::exit(1);
            }
            written.push((name.to_string(), buffer.peak()));
        }
    }

    let manifest = serde_json::json!({
        "renderer": "formula90-core/src/bin/grand_prix_sampler_render.rs",
        "profile": profile_path.to_string_lossy().replace('\\', "/"),
        "profile_sha256": sha256(&profile_path),
        "bank_directory": bank_directory.to_string_lossy().replace('\\', "/"),
        "grand_prix_bank_sha256": bank_sha256,
        "sampler_backend_active": sampler_active,
        "reference_source": format!("{reference_source:?}"),
        "sequence_render_seconds": sequence_render_seconds,
        "sequence_realtime_factor": TOTAL_SECONDS / sequence_render_seconds.max(1e-9),
        "sample_rate_hz": SAMPLE_RATE,
        "tick_seconds": TICK_SECONDS,
        "total_seconds": TOTAL_SECONDS,
        "sequence": "idle -> progressive full-load acceleration with five upshifts -> limiter entry/exit -> lift backfire -> three downshifts -> coast -> curb/sand/grass with tire scrub and one impact -> return to idle",
        "loudness_match_gain_sampler_to_reference": sampler_match_gain,
        "loudness_match_gain_reference_to_sampler": reference_match_gain,
        "sampler_diagnostics": sampler_diagnostics.map(|diagnostics| serde_json::json!({
            "accepted_events": diagnostics.accepted_events,
            "accepted_upshift_events": diagnostics.accepted_upshift_events,
            "accepted_downshift_events": diagnostics.accepted_downshift_events,
            "accepted_backfire_events": diagnostics.accepted_backfire_events,
            "accepted_limiter_events": diagnostics.accepted_limiter_events,
            "suppressed_events": diagnostics.suppressed_events,
            "late_events": diagnostics.late_events,
            "overflowed_events": diagnostics.overflowed_events,
            "clamped_rate_samples": diagnostics.clamped_rate_samples,
            "stale_input_holds": diagnostics.stale_input_holds,
            "output_peak": diagnostics.output_peak,
        })),
        "files": written.iter().map(|(name, peak)| serde_json::json!({
            "file": name,
            "sha256": sha256(&output_directory.join(name)),
            "peak": peak,
        })).collect::<Vec<_>>(),
    });
    let manifest_path = output_directory.join("render_manifest.json");
    if let Some(parent) = manifest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(error) = fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    ) {
        eprintln!("manifest write failed: {error}");
        std::process::exit(1);
    }
    println!(
        "renders written to {} (sampler active: {sampler_active}, reference: {reference_source:?})",
        output_directory.display()
    );
    if reference_source != Some(ContinuousSourceKind::V10Gf509) {
        eprintln!("warning: GF509 reference did not initialize");
    }
}
