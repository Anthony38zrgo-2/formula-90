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

const HYBRID_HEADROOM_GAIN: f32 = 0.67;

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

struct ComplementaryMidDucker {
    low: OnePoleLowPass,
    high: OnePoleLowPass,
    envelope: f32,
    attack: f32,
    release: f32,
}

impl ComplementaryMidDucker {
    fn new(sample_rate: f32) -> Self {
        Self {
            low: OnePoleLowPass::new(350.0, sample_rate),
            high: OnePoleLowPass::new(2_000.0, sample_rate),
            envelope: 0.0,
            attack: 1.0 - (-1.0 / (0.020 * sample_rate)).exp(),
            release: 1.0 - (-1.0 / (0.140 * sample_rate)).exp(),
        }
    }

    #[inline]
    fn process(&mut self, simulation: f32, sample_mid: f32) -> (f32, f32) {
        let detector = sample_mid.abs();
        let rate = if detector > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope += rate * (detector - self.envelope);
        let depth = (self.envelope / 0.080).clamp(0.0, 1.0);
        let gain = 1.0 - (1.0 - 10.0f32.powf(-3.0 / 20.0)) * depth;
        let below_high = self.high.process(simulation);
        let below_low = self.low.process(simulation);
        let mid = below_high - below_low;
        (simulation + mid * (gain - 1.0), gain)
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
    accel_seconds: f32,
    coast_end_rpm: f32,
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
        accel_seconds: 7.0,
        coast_end_rpm: 6_500.0,
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
            "--accel-seconds" => {
                parsed.accel_seconds = parse_value(&raw, &mut i, "--accel-seconds")?
            }
            "--coast-end-rpm" => {
                parsed.coast_end_rpm = parse_value(&raw, &mut i, "--coast-end-rpm")?
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

const SOURCE_PATHS: [&str; 8] = [
    "game/crates/v10-engine-synth/src/acoustics.rs",
    "game/crates/v10-engine-synth/src/config.rs",
    "game/crates/v10-engine-synth/src/crank.rs",
    "game/crates/v10-engine-synth/src/cylinder.rs",
    "game/crates/v10-engine-synth/src/engine.rs",
    "game/crates/v10-engine-synth/src/sample_layer.rs",
    "game/crates/v10-engine-synth/src/scene.rs",
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
    let mut scene = AcousticScene::new(args.sample_rate as f32, AcousticSceneConfig::default())?;
    let mut sample_layer = args
        .sample_layer_dir
        .as_ref()
        .map(|directory| {
            ThreeZoneSampleLayer::load_directory(
                args.sample_rate,
                directory,
                ThreeZoneSampleLayerConfig::default(),
            )
        })
        .transpose()?;
    let mut mid_ducker = ComplementaryMidDucker::new(args.sample_rate as f32);
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
        "gearbox_housing",
        "head_cover_a",
        "head_cover_b",
        "cylinder_head_covers",
        "airbox_plenum",
        "engine_cover",
        "rear_exhaust_a",
        "rear_exhaust_b",
        "rear_exhaust",
        "engine_mounts",
        "monocoque_seat",
        "mount_monocoque",
        "under_seat_vibration",
        "cockpit_cavity",
        "low_mid_parallel",
        "load_saturation",
        "event_residual",
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
        "sample_mid",
        "sample_max_rasp",
        "sample_layer",
        "scene_mid_ducked",
        "hybrid_mix",
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
    let mut telemetry = BufWriter::new(File::create(&telemetry_path).map_err(|e| e.to_string())?);
    writeln!(telemetry, "sample,time_s,rpm,crank_phase_deg,cylinder,bank")
        .map_err(|e| e.to_string())?;

    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut worst_reduction = 0.0f32;
    let mut event_count = 0u64;
    for sample in 0..total {
        let time_s = sample as f32 / args.sample_rate as f32;
        let current_input = render_input(&args, time_s);
        engine.set_input(current_input)?;
        let frame: EngineFrame = engine.render_sample();
        let acoustic = scene.process(&frame);
        let sampled = sample_layer
            .as_mut()
            .map(|layer| {
                layer.process(SampleLayerInput {
                    rpm: current_input.rpm,
                    throttle: current_input.throttle,
                    load: current_input.load,
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
            .get_mut("gearbox_housing")
            .unwrap()
            .push(acoustic.gearbox_housing);
        stems
            .get_mut("head_cover_a")
            .unwrap()
            .push(acoustic.head_cover_a);
        stems
            .get_mut("head_cover_b")
            .unwrap()
            .push(acoustic.head_cover_b);
        stems
            .get_mut("cylinder_head_covers")
            .unwrap()
            .push(acoustic.cylinder_head_covers);
        stems
            .get_mut("airbox_plenum")
            .unwrap()
            .push(acoustic.airbox_plenum);
        stems
            .get_mut("engine_cover")
            .unwrap()
            .push(acoustic.engine_cover);
        stems
            .get_mut("rear_exhaust_a")
            .unwrap()
            .push(acoustic.rear_exhaust_a);
        stems
            .get_mut("rear_exhaust_b")
            .unwrap()
            .push(acoustic.rear_exhaust_b);
        stems
            .get_mut("rear_exhaust")
            .unwrap()
            .push(acoustic.rear_exhaust);
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
            .get_mut("cockpit_cavity")
            .unwrap()
            .push(acoustic.cockpit_cavity);
        stems
            .get_mut("low_mid_parallel")
            .unwrap()
            .push(acoustic.low_mid_parallel);
        stems
            .get_mut("load_saturation")
            .unwrap()
            .push(acoustic.load_saturation);
        stems
            .get_mut("event_residual")
            .unwrap()
            .push(acoustic.event_residual);
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
        let sample_tonal = sampled.map_or(0.0, |frame| frame.tonal);
        let sample_residual = sampled.map_or(0.0, |frame| frame.residual);
        let sample_mid = sampled.map_or(0.0, |frame| frame.mid_bus);
        let sample_max_rasp = sampled.map_or(0.0, |frame| frame.max_rasp);
        let sample_output = sampled.map_or(0.0, |frame| frame.output);
        let (scene_mid_ducked, _) = mid_ducker.process(acoustic.output, sample_mid);
        // Hybrid headroom is static and transparent. A limiter here would hide
        // gain errors and make the sample layer part of the sound design.
        let hybrid = (scene_mid_ducked + sample_output) * HYBRID_HEADROOM_GAIN;
        stems.get_mut("sample_tonal").unwrap().push(sample_tonal);
        stems
            .get_mut("sample_residual")
            .unwrap()
            .push(sample_residual);
        stems.get_mut("sample_mid").unwrap().push(sample_mid);
        stems
            .get_mut("sample_max_rasp")
            .unwrap()
            .push(sample_max_rasp);
        stems.get_mut("sample_layer").unwrap().push(sample_output);
        stems
            .get_mut("scene_mid_ducked")
            .unwrap()
            .push(scene_mid_ducked);
        stems.get_mut("hybrid_mix").unwrap().push(hybrid);
        let rendered = if sample_layer.is_some() {
            hybrid
        } else if args.acoustic_scene {
            acoustic.output
        } else {
            frame.master
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
    let metadata_path = args.out.with_extension("metadata.json");
    let profile = if args.sweep_end_rpm.is_some() {
        "sweep_lift_coast"
    } else {
        "steady"
    };
    let peak_rpm = args.sweep_end_rpm.unwrap_or(args.rpm);
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
            "  \"hybrid_headroom_gain\": {:.6},\n",
            "  \"sample_mid_architecture\": \"zone EQ + residual parallel compression/saturation + max-zone order-5 control + complementary 350-2000 Hz duck + max-only 1800-6500 Hz rasp at +3.5 dB + max tonal/residual gains at +1.5/+2.0 dB\"\n",
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
        args.coast_end_rpm,
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
        HYBRID_HEADROOM_GAIN,
    );
    fs::write(&metadata_path, metadata).map_err(|e| e.to_string())?;
    println!("wav={}", args.out.display());
    println!("stems={}", args.stems_dir.display());
    println!("events={event_count} peak={peak:.6} rms={rms:.6} limiter_db={worst_reduction:.3}");
    Ok(())
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
