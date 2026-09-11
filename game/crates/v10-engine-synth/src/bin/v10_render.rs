use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use v10_engine_synth::wav::write_mono_pcm16;
use v10_engine_synth::{
    AcousticScene, AcousticSceneConfig, EngineConfig, EngineFrame, EngineInput, SampleLayerInput,
    ThreeZoneSampleLayer, ThreeZoneSampleLayerConfig, V10Engine,
};

const HYBRID_HEADROOM_GAIN: f32 = 0.61;

struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            alpha: 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

/// 3rd-order (-18 dB/octave) all-pole lowpass filter cascade for natural, smooth, non-resonant rolloff.
struct ThreePoleLowPass {
    p1: OnePoleLowPass,
    p2: OnePoleLowPass,
    p3: OnePoleLowPass,
}

impl ThreePoleLowPass {
    fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            p1: OnePoleLowPass::new(cutoff_hz, sample_rate),
            p2: OnePoleLowPass::new(cutoff_hz, sample_rate),
            p3: OnePoleLowPass::new(cutoff_hz, sample_rate),
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.p3.process(self.p2.process(self.p1.process(input)))
    }
}

struct Args {
    rpm: f32,
    seconds: f32,
    warmup: f32,
    throttle: f32,
    load: f32,
    sample_rate: u32,
    seed: u64,
    out: PathBuf,
    stems_dir: PathBuf,
    acoustic_scene: bool,
    sample_layer_dir: Option<PathBuf>,
    sweep_end_rpm: Option<f32>,
    hold_before_lift: bool,
    accel_seconds: f32,
    coast_end_rpm: f32,
    physical_telemetry_csv: Option<PathBuf>,
    physical_master_lowpass_hz: Option<f32>,
    no_sample_rasp: bool,
    sample_residual_scale: f32,
    sample_residual_gain: Option<f32>,
    scene_gains: Vec<(String, f32)>,
}

fn parse_value<T: std::str::FromStr>(
    args: &[String],
    index: &mut usize,
    name: &str,
) -> Result<T, String> {
    *index += 1;
    args.get(*index)
        .ok_or_else(|| format!("missing value for {name}"))?
        .parse::<T>()
        .map_err(|_| format!("invalid value for {name}"))
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut parsed = Args {
        rpm: 5_000.0,
        seconds: 6.0,
        warmup: 1.0,
        throttle: 0.72,
        load: 0.78,
        sample_rate: 48_000,
        seed: 0xF090_0010,
        out: PathBuf::from("reports/audio/rust-greenfield/gf310_5000rpm.wav"),
        stems_dir: PathBuf::from("reports/audio/rust-greenfield/gf310_5000rpm_stems"),
        acoustic_scene: false,
        sample_layer_dir: None,
        sweep_end_rpm: None,
        hold_before_lift: false,
        accel_seconds: 7.0,
        coast_end_rpm: 6_500.0,
        physical_telemetry_csv: None,
        physical_master_lowpass_hz: Some(2_500.0),
        no_sample_rasp: false,
        sample_residual_scale: 1.0,
        sample_residual_gain: None,
        scene_gains: Vec::new(),
    };
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--rpm" => parsed.rpm = parse_value(&raw, &mut i, "--rpm")?,
            "--seconds" => parsed.seconds = parse_value(&raw, &mut i, "--seconds")?,
            "--warmup" => parsed.warmup = parse_value(&raw, &mut i, "--warmup")?,
            "--throttle" => parsed.throttle = parse_value(&raw, &mut i, "--throttle")?,
            "--load" => parsed.load = parse_value(&raw, &mut i, "--load")?,
            "--sample-rate" => parsed.sample_rate = parse_value(&raw, &mut i, "--sample-rate")?,
            "--seed" => parsed.seed = parse_value(&raw, &mut i, "--seed")?,
            "--out" => parsed.out = PathBuf::from(parse_value::<String>(&raw, &mut i, "--out")?),
            "--stems-dir" => {
                parsed.stems_dir =
                    PathBuf::from(parse_value::<String>(&raw, &mut i, "--stems-dir")?)
            }
            "--acoustic-scene" => parsed.acoustic_scene = true,
            "--sample-layer-dir" => {
                parsed.sample_layer_dir = Some(PathBuf::from(parse_value::<String>(
                    &raw,
                    &mut i,
                    "--sample-layer-dir",
                )?))
            }
            "--sweep-end-rpm" => {
                parsed.sweep_end_rpm = Some(parse_value(&raw, &mut i, "--sweep-end-rpm")?)
            }
            "--hold-before-lift" => parsed.hold_before_lift = true,
            "--accel-seconds" => {
                parsed.accel_seconds = parse_value(&raw, &mut i, "--accel-seconds")?
            }
            "--coast-end-rpm" => {
                parsed.coast_end_rpm = parse_value(&raw, &mut i, "--coast-end-rpm")?
            }
            "--physical-telemetry-csv" => {
                parsed.physical_telemetry_csv = Some(PathBuf::from(parse_value::<String>(
                    &raw,
                    &mut i,
                    "--physical-telemetry-csv",
                )?))
            }
            "--physical-master-lowpass-hz" => {
                parsed.physical_master_lowpass_hz =
                    Some(parse_value(&raw, &mut i, "--physical-master-lowpass-hz")?)
            }
            "--no-physical-master-lowpass" => {
                parsed.physical_master_lowpass_hz = None;
            }
            "--no-sample-rasp" => parsed.no_sample_rasp = true,
            "--sample-residual-scale" => {
                parsed.sample_residual_scale =
                    parse_value(&raw, &mut i, "--sample-residual-scale")?
            }
            "--sample-residual-gain" => {
                parsed.sample_residual_gain =
                    Some(parse_value(&raw, &mut i, "--sample-residual-gain")?)
            }
            "--scene-gain" => {
                let spec = parse_value::<String>(&raw, &mut i, "--scene-gain")?;
                let (name, value) = spec.split_once('=').ok_or_else(|| {
                    "expected --scene-gain <branch>=<value>, e.g. --scene-gain metal=0.9".to_string()
                })?;
                let gain: f32 = value
                    .parse()
                    .map_err(|_| format!("invalid scene gain value: {value}"))?;
                parsed.scene_gains.push((name.to_string(), gain));
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        i += 1;
    }
    if !parsed.seconds.is_finite() || !(0.1..=120.0).contains(&parsed.seconds) {
        return Err("--seconds must be in 0.1..120".into());
    }
    if !parsed.warmup.is_finite() || !(0.0..=10.0).contains(&parsed.warmup) {
        return Err("--warmup must be in 0..10".into());
    }
    if !parsed.accel_seconds.is_finite()
        || parsed.accel_seconds <= 0.0
        || parsed.accel_seconds >= parsed.seconds
    {
        return Err("--accel-seconds must be inside the render duration".into());
    }
    EngineInput {
        rpm: parsed.rpm,
        throttle: parsed.throttle,
        load: parsed.load,
    }
    .validate()?;
    if let Some(end_rpm) = parsed.sweep_end_rpm {
        EngineInput {
            rpm: end_rpm,
            throttle: parsed.throttle,
            load: parsed.load,
        }
        .validate()?;
        EngineInput {
            rpm: parsed.coast_end_rpm,
            throttle: 0.0,
            load: 0.0,
        }
        .validate()?;
    }
    if parsed.hold_before_lift && parsed.sweep_end_rpm.is_none() {
        return Err("--hold-before-lift requires --sweep-end-rpm".into());
    }
    Ok(parsed)
}

fn render_input(args: &Args, time_s: f32) -> EngineInput {
    let Some(end_rpm) = args.sweep_end_rpm else {
        return EngineInput {
            rpm: args.rpm,
            throttle: args.throttle,
            load: args.load,
        };
    };
    if args.hold_before_lift {
        let coast_end_rpm = end_rpm;
        if time_s < args.accel_seconds {
            return EngineInput {
                rpm: args.rpm,
                throttle: args.throttle,
                load: args.load,
            };
        }
        let coast_duration = (args.seconds - args.accel_seconds).max(1.0e-6);
        let t = ((time_s - args.accel_seconds) / coast_duration).clamp(0.0, 1.0);
        let rpm_decay = 1.0 - (1.0 - t).powf(1.55);
        let lift = (-t / 0.035).exp();
        return EngineInput {
            rpm: args.rpm + (coast_end_rpm - args.rpm) * rpm_decay,
            throttle: 0.035 + (args.throttle - 0.035) * lift,
            load: 0.10 + (args.load - 0.10) * (-t / 0.12).exp(),
        };
    }
    if time_s < args.accel_seconds {
        let t = (time_s / args.accel_seconds).clamp(0.0, 1.0);
        // Smoothstep avoids an acceleration discontinuity at either endpoint.
        let shaped = t * t * (3.0 - 2.0 * t);
        EngineInput {
            rpm: args.rpm + (end_rpm - args.rpm) * shaped,
            throttle: args.throttle,
            load: args.load,
        }
    } else {
        let coast_duration = (args.seconds - args.accel_seconds).max(1.0e-6);
        let t = ((time_s - args.accel_seconds) / coast_duration).clamp(0.0, 1.0);
        let rpm_decay = 1.0 - (1.0 - t).powf(1.55);
        let lift = (-t / 0.035).exp();
        EngineInput {
            rpm: end_rpm + (args.coast_end_rpm - end_rpm) * rpm_decay,
            throttle: 0.035 + (args.throttle - 0.035) * lift,
            load: 0.10 + (args.load - 0.10) * (-t / 0.12).exp(),
        }
    }
}

fn git_head() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".into())
}

const SOURCE_PATHS: [&str; 15] = [
    "game/crates/v10-engine-synth/src/acoustics.rs",
    "game/crates/v10-engine-synth/src/config.rs",
    "game/crates/v10-engine-synth/src/crank.rs",
    "game/crates/v10-engine-synth/src/cylinder.rs",
    "game/crates/v10-engine-synth/src/engine.rs",
    "game/crates/v10-engine-synth/src/geometry.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/combustion.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/gas.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/polytrope.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/exhaust_runner.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/exhaust_valve.rs",
    "game/crates/v10-engine-synth/src/sample_layer.rs",
    "game/crates/v10-engine-synth/src/scene.rs",
    "game/crates/v10-engine-synth/src/runtime.rs",
    "game/crates/v10-engine-synth/src/bin/v10_render.rs",
];

fn source_dirty() -> bool {
    let dirty = |cached: bool| {
        let mut command = std::process::Command::new("git");
        command.args(["diff", "--quiet"]);
        if cached {
            command.arg("--cached");
        }
        command.arg("--").args(SOURCE_PATHS);
        command
            .status()
            .map(|status| !status.success())
            .unwrap_or(true)
    };
    dirty(false) || dirty(true)
}

fn source_fingerprint() -> String {
    // Stable FNV-1a over path names and working-tree bytes. This is provenance,
    // not a cryptographic signature.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for path in SOURCE_PATHS {
        for byte in path.bytes().chain([0]) {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        match fs::read(path) {
            Ok(bytes) => {
                for byte in bytes {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            Err(_) => return "unavailable".into(),
        }
    }
    format!("fnv1a64:{hash:016x}")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("v10_render: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let config = EngineConfig {
        sample_rate: args.sample_rate,
        seed: args.seed,
        ..EngineConfig::default()
    };
    let mut engine = V10Engine::new(config.clone())?;
    let mut scene_config = AcousticSceneConfig::default();
    for (name, gain) in &args.scene_gains {
        let slot = match name.as_str() {
            "dry_low" => &mut scene_config.dry_low_gain,
            "dry_mid" => &mut scene_config.dry_mid_gain,
            "dry_high" => &mut scene_config.dry_high_gain,
            "metal" => &mut scene_config.metal_gain,
            "airbox" => &mut scene_config.airbox_gain,
            "engine_cover" => &mut scene_config.engine_cover_gain,
            "mount_monocoque" => &mut scene_config.mount_monocoque_gain,
            "under_seat" => &mut scene_config.under_seat_gain,
            _ => return Err(format!("unknown scene branch: {name}")),
        };
        *slot = *gain;
    }
    let scene_gain_overrides = args
        .scene_gains
        .iter()
        .map(|(name, gain)| format!("{name}={gain}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut scene = AcousticScene::new(args.sample_rate as f32, scene_config)?;
    let mut sample_layer = args
        .sample_layer_dir
        .as_ref()
        .map(|directory| {
            ThreeZoneSampleLayer::load_directory(
                args.sample_rate,
                directory,
                ThreeZoneSampleLayerConfig {
                    disable_sample_rasp: args.no_sample_rasp,
                    residual_gain_scale: args.sample_residual_scale,
                    residual_gain_closed: args.sample_residual_gain.unwrap_or(
                        ThreeZoneSampleLayerConfig::default().residual_gain_closed,
                    ),
                    residual_gain_loaded: args.sample_residual_gain.unwrap_or(
                        ThreeZoneSampleLayerConfig::default().residual_gain_loaded,
                    ),
                    ..ThreeZoneSampleLayerConfig::default()
                },
            )
        })
        .transpose()?;
    engine.set_input(EngineInput {
        rpm: args.rpm,
        throttle: args.throttle,
        load: args.load,
    })?;

    let warmup_samples = (args.warmup * args.sample_rate as f32).round() as usize;
    for _ in 0..warmup_samples {
        let frame = engine.render_sample();
        scene.process(&frame);
    }

    let total = (args.seconds * args.sample_rate as f32).round() as usize;
    let names = [
        "combustion_source",
        "pressure_derivative",
        "pressure_derivative_a",
        "pressure_derivative_b",
        "pressure_direct",
        "crankcase",
        "block",
        "head",
        "block_head",
        "headers_a",
        "headers_b",
        "collector_a",
        "collector_b",
        "collector_pressure_a",
        "collector_pressure_b",
        "exhaust",
        "turbulence",
        "master_pre_limiter",
        "master",
        "engine_dry",
        "engine_air",
        "metallic_structure",
        "airbox_plenum",
        "engine_cover",
        "engine_mounts",
        "monocoque_seat",
        "mount_monocoque",
        "under_seat_vibration",
        "cylinder_mechanical_sum",
        "cylinder_mechanical_0",
        "cylinder_mechanical_1",
        "cylinder_mechanical_2",
        "cylinder_mechanical_3",
        "cylinder_mechanical_4",
        "cylinder_mechanical_5",
        "cylinder_mechanical_6",
        "cylinder_mechanical_7",
        "cylinder_mechanical_8",
        "cylinder_mechanical_9",
        "scene_mix",
        "sample_tonal",
        "sample_residual",
        "sample_max_rasp",
        "sample_off_throttle",
        "sample_layer",
        "hybrid_mix",
        // PHY-140 physical diagnostic stems
        "cylinder_pressure",
        "chamber_temperature",
        "chamber_heat_release",
        "chamber_phase",
        "combustion_heat_release",
        "valve_area",
        "mass_flow",
        "runner_pressure",
        "runner_acoustic_pressure",
        "physical_master",
        "sample_master",
        "hybrid_master",
    ];
    let mut stems: BTreeMap<&str, Vec<f32>> = names
        .iter()
        .map(|&name| (name, Vec::with_capacity(total)))
        .collect();
    fs::create_dir_all(&args.stems_dir).map_err(|e| e.to_string())?;
    let telemetry_path = args.out.with_extension("events.csv");
    if let Some(parent) = telemetry_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if telemetry_path.exists() {
        let _ = fs::remove_file(&telemetry_path);
    }
    let mut telemetry = BufWriter::new(File::create(&telemetry_path).map_err(|e| e.to_string())?);
    writeln!(telemetry, "sample,time_s,rpm,crank_phase_deg,cylinder,bank")
        .map_err(|e| e.to_string())?;

    let mut phys_telemetry = args
        .physical_telemetry_csv
        .as_ref()
        .map(|path| {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if path.exists() {
                let _ = fs::remove_file(path);
            }
            let file = File::create(path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            writeln!(
                writer,
                "sample,time_s,rpm,crank_phase_deg,cylinder_id,chamber_pressure_pa,chamber_temperature_k,heat_release_rate,phase,valve_lift,valve_area_m2,mass_flow_kg_s,runner_pressure_pa,runner_temperature_k,runner_acoustic_pa,collector_pressure_a,collector_pressure_b"
            ).map_err(|e| e.to_string())?;
            Ok::<_, String>(writer)
        })
        .transpose()?;

    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    // F2K evidence: per-source accumulated weights plus sampled-layer
    // component energy, all from this same render.
    let source_labels: Vec<String> = sample_layer
        .as_ref()
        .map(|layer| layer.source_labels().to_vec())
        .unwrap_or_default();
    let mut source_weight_sums = vec![0.0f64; source_labels.len()];
    let mut sample_tonal_sq = 0.0f64;
    let mut sample_residual_sq = 0.0f64;
    let mut sample_off_sq = 0.0f64;
    let mut worst_reduction = 0.0f32;
    let mut event_count = 0u64;
    let mut physical_master_lp = args.physical_master_lowpass_hz.map(|cutoff| {
        ThreePoleLowPass::new(
            cutoff.clamp(100.0, args.sample_rate as f32 * 0.48),
            args.sample_rate as f32,
        )
    });
    for sample in 0..total {
        let time_s = sample as f32 / args.sample_rate as f32;
        let current_input = render_input(&args, time_s);
        engine.set_input(current_input)?;
        let frame: EngineFrame = engine.render_sample();
        let acoustic = scene.process(&frame);
        let sampled = sample_layer
            .as_mut()
            .map(|layer| {
                // Imposed evaluation trajectory (not physics braking): the
                // sweep prescribes RPM/throttle/load offline, so retention
                // signals are derived from the same prescribed controls.
                // Accel (throttle open) carries positive torque equal to the
                // prescribed load; coast (throttle closing) carries negative
                // torque proportional to the lift depth. Clutch stays engaged
                // through the lift-and-coast. No lift read-position jump.
                let retention_torque = if current_input.throttle > 0.30 {
                    current_input.load
                } else {
                    -((0.30 - current_input.throttle) / 0.30) * 0.6
                };
                layer.process(SampleLayerInput {
                    rpm: current_input.rpm,
                    throttle: current_input.throttle,
                    load: current_input.load,
                    normalized_engine_torque: retention_torque,
                    clutch_engagement: 1.0,
                    crank_phase_deg: frame.crank_phase_deg,
                })
            })
            .transpose()?;
        stems
            .get_mut("combustion_source")
            .unwrap()
            .push(frame.combustion_source);
        stems
            .get_mut("pressure_derivative")
            .unwrap()
            .push(frame.pressure_derivative);
        stems
            .get_mut("pressure_derivative_a")
            .unwrap()
            .push(frame.pressure_derivative_a);
        stems
            .get_mut("pressure_derivative_b")
            .unwrap()
            .push(frame.pressure_derivative_b);
        stems
            .get_mut("pressure_direct")
            .unwrap()
            .push(frame.pressure_direct);
        stems.get_mut("crankcase").unwrap().push(frame.crankcase);
        stems.get_mut("block").unwrap().push(frame.block);
        stems.get_mut("head").unwrap().push(frame.head);
        stems.get_mut("block_head").unwrap().push(frame.block_head);
        stems.get_mut("headers_a").unwrap().push(frame.headers_a);
        stems.get_mut("headers_b").unwrap().push(frame.headers_b);
        stems
            .get_mut("collector_a")
            .unwrap()
            .push(frame.collector_a);
        stems
            .get_mut("collector_b")
            .unwrap()
            .push(frame.collector_b);
        stems
            .get_mut("collector_pressure_a")
            .unwrap()
            .push(frame.collector_pressure_a);
        stems
            .get_mut("collector_pressure_b")
            .unwrap()
            .push(frame.collector_pressure_b);
        stems.get_mut("exhaust").unwrap().push(frame.exhaust);
        stems.get_mut("turbulence").unwrap().push(frame.turbulence);
        stems
            .get_mut("master_pre_limiter")
            .unwrap()
            .push(frame.master_pre_limiter);
        stems.get_mut("master").unwrap().push(frame.master);
        stems
            .get_mut("engine_dry")
            .unwrap()
            .push(acoustic.engine_dry);
        stems
            .get_mut("engine_air")
            .unwrap()
            .push(acoustic.engine_air);
        stems
            .get_mut("metallic_structure")
            .unwrap()
            .push(acoustic.metallic_structure);
        stems
            .get_mut("airbox_plenum")
            .unwrap()
            .push(acoustic.airbox_plenum);
        stems
            .get_mut("engine_cover")
            .unwrap()
            .push(acoustic.engine_cover);
        stems
            .get_mut("engine_mounts")
            .unwrap()
            .push(acoustic.engine_mounts);
        stems
            .get_mut("monocoque_seat")
            .unwrap()
            .push(acoustic.monocoque_seat);
        stems
            .get_mut("mount_monocoque")
            .unwrap()
            .push(acoustic.mount_monocoque);
        stems
            .get_mut("under_seat_vibration")
            .unwrap()
            .push(acoustic.under_seat_vibration);
        stems
            .get_mut("cylinder_mechanical_sum")
            .unwrap()
            .push(acoustic.cylinder_mechanical_sum);
        const CYLINDER_STEMS: [&str; 10] = [
            "cylinder_mechanical_0",
            "cylinder_mechanical_1",
            "cylinder_mechanical_2",
            "cylinder_mechanical_3",
            "cylinder_mechanical_4",
            "cylinder_mechanical_5",
            "cylinder_mechanical_6",
            "cylinder_mechanical_7",
            "cylinder_mechanical_8",
            "cylinder_mechanical_9",
        ];
        for (index, name) in CYLINDER_STEMS.iter().enumerate() {
            stems
                .get_mut(name)
                .unwrap()
                .push(acoustic.cylinder_mechanical[index]);
        }
        stems.get_mut("scene_mix").unwrap().push(acoustic.output);
        let sample_tonal = sampled.as_ref().map_or(0.0, |frame| frame.tonal);
        let sample_residual = sampled.as_ref().map_or(0.0, |frame| frame.residual);
        let sample_max_rasp = sampled.as_ref().map_or(0.0, |frame| frame.max_rasp);
        let sample_off_throttle = sampled.as_ref().map_or(0.0, |frame| frame.off_throttle);
        let sample_output = sampled.as_ref().map_or(0.0, |frame| frame.output);
        if let Some(frame) = sampled {
            for (slot, weight) in source_weight_sums.iter_mut().zip(frame.zone_weights.iter()) {
                *slot += *weight as f64;
            }
            sample_tonal_sq += (frame.tonal * frame.tonal) as f64;
            sample_residual_sq += (frame.residual * frame.residual) as f64;
            sample_off_sq += (frame.off_throttle * frame.off_throttle) as f64;
        }
        // Hybrid headroom is static and transparent. A limiter here would hide
        // gain errors and make the sample layer part of the sound design.
        let hybrid = (acoustic.output + sample_output) * HYBRID_HEADROOM_GAIN;
        stems.get_mut("sample_tonal").unwrap().push(sample_tonal);
        stems
            .get_mut("sample_residual")
            .unwrap()
            .push(sample_residual);
        stems
            .get_mut("sample_max_rasp")
            .unwrap()
            .push(sample_max_rasp);
        stems
            .get_mut("sample_off_throttle")
            .unwrap()
            .push(sample_off_throttle);
        stems.get_mut("sample_layer").unwrap().push(sample_output);
        stems.get_mut("hybrid_mix").unwrap().push(hybrid);
        stems
            .get_mut("cylinder_pressure")
            .unwrap()
            .push(frame.cylinder_chamber_pressure_pa.iter().sum::<f32>() / 10.0);
        stems
            .get_mut("chamber_temperature")
            .unwrap()
            .push(frame.cylinder_chamber_temperature_k.iter().sum::<f32>() / 10.0);
        stems
            .get_mut("chamber_heat_release")
            .unwrap()
            .push(frame.cylinder_chamber_heat_release_rate.iter().sum::<f32>() / 10.0);
        stems
            .get_mut("chamber_phase")
            .unwrap()
            .push(frame.cylinder_chamber_phase[0] as f32);
        stems
            .get_mut("combustion_heat_release")
            .unwrap()
            .push(frame.pressure_derivative);
        stems
            .get_mut("valve_area")
            .unwrap()
            .push(frame.cylinder_valve_area.iter().sum::<f32>());
        stems
            .get_mut("mass_flow")
            .unwrap()
            .push(frame.cylinder_mass_flow.iter().sum::<f32>());
        stems
            .get_mut("runner_pressure")
            .unwrap()
            .push(frame.cylinder_runner_pressure.iter().sum::<f32>() * 0.1);
        stems
            .get_mut("runner_acoustic_pressure")
            .unwrap()
            .push(frame.cylinder_runner_acoustic_pressure.iter().sum::<f32>());
        let filtered_physical_master = if let Some(lp) = physical_master_lp.as_mut() {
            lp.process(frame.master)
        } else {
            frame.master
        };
        stems
            .get_mut("physical_master")
            .unwrap()
            .push(filtered_physical_master);
        stems.get_mut("sample_master").unwrap().push(sample_output);
        stems.get_mut("hybrid_master").unwrap().push(hybrid);

        if let Some(writer) = phys_telemetry.as_mut() {
            writeln!(
                writer,
                "{sample},{time_s:.6},{rpm:.2},{crank_phase_deg:.2},{cyl},{chamber_p:.4e},{chamber_t:.2},{heat:.6e},{phase},{lift:.6},{area:.6e},{flow:.6e},{runner_p:.4e},{runner_t:.2},{acoustic_p:.6e},{col_a:.4e},{col_b:.4e}",
                sample = sample,
                time_s = time_s,
                rpm = current_input.rpm,
                crank_phase_deg = frame.crank_phase_deg,
                cyl = 0,
                chamber_p = frame.cylinder_chamber_pressure_pa[0],
                chamber_t = frame.cylinder_chamber_temperature_k[0],
                heat = frame.cylinder_chamber_heat_release_rate[0],
                phase = frame.cylinder_chamber_phase[0],
                lift = frame.cylinder_valve_lift[0],
                area = frame.cylinder_valve_area[0],
                flow = frame.cylinder_mass_flow[0],
                runner_p = frame.cylinder_runner_pressure[0],
                runner_t = frame.cylinder_runner_temperature_k[0],
                acoustic_p = frame.cylinder_runner_acoustic_pressure[0],
                col_a = frame.collector_pressure_a,
                col_b = frame.collector_pressure_b,
            ).map_err(|e| e.to_string())?;
        }

        let rendered = if sample_layer.is_some() {
            hybrid
        } else if args.acoustic_scene {
            acoustic.output
        } else {
            filtered_physical_master
        };
        peak = peak.max(rendered.abs());
        sum_sq += (rendered * rendered) as f64;
        worst_reduction = worst_reduction.min(frame.limiter_reduction_db);
        if frame.fired_mask != 0 {
            for cylinder in 0..10 {
                if frame.fired_mask & (1 << cylinder) != 0 {
                    event_count += 1;
                    let bank = if cylinder < 5 { "A" } else { "B" };
                    writeln!(
                        telemetry,
                        "{sample},{:.9},{:.3},{:.6},{cylinder},{bank}",
                        sample as f64 / args.sample_rate as f64,
                        current_input.rpm,
                        frame.crank_phase_deg,
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    telemetry.flush().map_err(|e| e.to_string())?;

    let output_stem = if sample_layer.is_some() {
        "hybrid_mix"
    } else if args.acoustic_scene {
        "scene_mix"
    } else {
        "master"
    };
    write_mono_pcm16(&args.out, args.sample_rate, stems.get(output_stem).unwrap())?;
    for (name, samples) in &stems {
        write_mono_pcm16(
            &args.stems_dir.join(format!("{name}.wav")),
            args.sample_rate,
            samples,
        )?;
    }

    let rms = (sum_sq / total as f64).sqrt();
    let sample_layer_bank = args
        .sample_layer_dir
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("none")
        .to_string();
    let sample_layer_source_weights = source_labels
        .iter()
        .zip(source_weight_sums.iter())
        .map(|(label, sum)| format!("{label}={:.6}", sum / total as f64))
        .collect::<Vec<_>>()
        .join(",");
    let sample_tonal_rms = (sample_tonal_sq / total as f64).sqrt();
    let sample_residual_rms = (sample_residual_sq / total as f64).sqrt();
    let sample_off_rms = (sample_off_sq / total as f64).sqrt();
    let metadata_path = args.out.with_extension("metadata.json");
    let profile = if args.hold_before_lift {
        "lift_coast_hold"
    } else if args.sweep_end_rpm.is_some() {
        "sweep_lift_coast"
    } else {
        "steady"
    };
    let peak_rpm = if args.hold_before_lift {
        args.rpm.max(args.sweep_end_rpm.unwrap_or(args.rpm))
    } else {
        args.sweep_end_rpm.unwrap_or(args.rpm)
    };
    let metadata_coast_end_rpm = if args.hold_before_lift {
        args.sweep_end_rpm.unwrap_or(args.coast_end_rpm)
    } else {
        args.coast_end_rpm
    };
    let metadata = format!(
        concat!(
            "{{\n",
            "  \"architecture\": \"rust-greenfield-v10\",\n",
            "  \"git_head\": \"{}\",\n",
            "  \"source_dirty\": {},\n",
            "  \"source_fingerprint\": \"{}\",\n",
            "  \"sample_rate\": {},\n",
            "  \"seed\": {},\n",
            "  \"profile\": \"{}\",\n",
            "  \"rpm\": {:.3},\n",
            "  \"peak_rpm\": {:.3},\n",
            "  \"coast_end_rpm\": {:.3},\n",
            "  \"hold_before_lift\": {},\n",
            "  \"accel_seconds\": {:.6},\n",
            "  \"throttle\": {:.6},\n",
            "  \"load\": {:.6},\n",
            "  \"duration_s\": {:.6},\n",
            "  \"warmup_s\": {:.6},\n",
            "  \"event_count\": {},\n",
            "  \"peak\": {:.9},\n",
            "  \"rms\": {:.9},\n",
            "  \"worst_limiter_reduction_db\": {:.6},\n",
            "  \"legacy_synth_used\": false,\n",
            "  \"cpp_used\": false,\n",
            "  \"faust_used\": false,\n",
            "  \"acoustic_scene_used\": {},\n",
            "  \"sample_layer_used\": {},\n",
            "  \"sample_rasp_disabled\": {},\n",
            "  \"sample_residual_scale\": {},\n",
            "  \"sample_residual_gain\": \"{}\",\n",
            "  \"scene_gain_overrides\": \"{}\",\n",
            "  \"sample_layer_bank\": \"{}\",\n",
            "  \"sample_layer_source_weights\": \"{}\",\n",
            "  \"sample_tonal_rms\": {:.9},\n",
            "  \"sample_residual_rms\": {:.9},\n",
            "  \"sample_off_rms\": {:.9},\n",
            "  \"hybrid_headroom_gain\": {:.6},\n",
            "  \"chamber_cycle_model\": \"bounded_720deg_four_stroke_fresh_charge\",\n",
            "  \"chamber_phase_contract\": \"expansion_0_180,exhaust_180_360,intake_360_540,compression_540_720\",\n",
            "  \"firing_order\": [0,5,1,6,2,7,3,8,4,9],\n",
            "  \"sample_mid_architecture\": \"zone-specific post gains + deeper max residual compression/saturation + max rasp at +2.5 dB + reduced engine/scene drift + perceptual max crossfade 10000-13750 RPM\"\n",
            "}}\n"
        ),
        git_head(),
        source_dirty(),
        source_fingerprint(),
        args.sample_rate,
        args.seed,
        profile,
        args.rpm,
        peak_rpm,
        metadata_coast_end_rpm,
        args.hold_before_lift,
        args.accel_seconds,
        args.throttle,
        args.load,
        args.seconds,
        args.warmup,
        event_count,
        peak,
        rms,
        worst_reduction,
        args.acoustic_scene,
        sample_layer.is_some(),
        args.no_sample_rasp,
        args.sample_residual_scale,
        args.sample_residual_gain
            .map(|gain| format!("{gain}"))
            .unwrap_or_else(|| "load-blended".to_string()),
        scene_gain_overrides,
        sample_layer_bank,
        sample_layer_source_weights,
        sample_tonal_rms,
        sample_residual_rms,
        sample_off_rms,
        HYBRID_HEADROOM_GAIN,
    );
    if metadata_path.exists() {
        let _ = fs::remove_file(&metadata_path);
    }
    fs::write(&metadata_path, metadata).map_err(|e| e.to_string())?;
    println!("wav={}", args.out.display());
    println!("stems={}", args.stems_dir.display());
    println!("events={event_count} peak={peak:.6} rms={rms:.6} limiter_db={worst_reduction:.3}");
    Ok(())
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
