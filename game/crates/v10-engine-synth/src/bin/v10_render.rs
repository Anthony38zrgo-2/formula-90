use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use v10_engine_synth::wav::write_mono_pcm16;
use v10_engine_synth::{EngineConfig, EngineFrame, EngineInput, V10Engine};

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
    EngineInput {
        rpm: parsed.rpm,
        throttle: parsed.throttle,
        load: parsed.load,
    }
    .validate()?;
    Ok(parsed)
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
    engine.set_input(EngineInput {
        rpm: args.rpm,
        throttle: args.throttle,
        load: args.load,
    })?;

    let warmup_samples = (args.warmup * args.sample_rate as f32).round() as usize;
    for _ in 0..warmup_samples {
        engine.render_sample();
    }

    let total = (args.seconds * args.sample_rate as f32).round() as usize;
    let names = [
        "combustion_source",
        "pressure_derivative",
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
        let frame: EngineFrame = engine.render_sample();
        stems
            .get_mut("combustion_source")
            .unwrap()
            .push(frame.combustion_source);
        stems
            .get_mut("pressure_derivative")
            .unwrap()
            .push(frame.pressure_derivative);
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
        peak = peak.max(frame.master.abs());
        sum_sq += (frame.master * frame.master) as f64;
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
                        args.rpm,
                        frame.crank_phase_deg,
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    telemetry.flush().map_err(|e| e.to_string())?;

    write_mono_pcm16(&args.out, args.sample_rate, stems.get("master").unwrap())?;
    for (name, samples) in &stems {
        write_mono_pcm16(
            &args.stems_dir.join(format!("{name}.wav")),
            args.sample_rate,
            samples,
        )?;
    }

    let rms = (sum_sq / total as f64).sqrt();
    let metadata_path = args.out.with_extension("metadata.json");
    let metadata = format!(
        concat!(
            "{{\n",
            "  \"architecture\": \"rust-greenfield-v10\",\n",
            "  \"git_head\": \"{}\",\n",
            "  \"sample_rate\": {},\n",
            "  \"seed\": {},\n",
            "  \"rpm\": {:.3},\n",
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
            "  \"faust_used\": false\n",
            "}}\n"
        ),
        git_head(),
        args.sample_rate,
        args.seed,
        args.rpm,
        args.throttle,
        args.load,
        args.seconds,
        args.warmup,
        event_count,
        peak,
        rms,
        worst_reduction,
    );
    fs::write(&metadata_path, metadata).map_err(|e| e.to_string())?;
    println!("wav={}", args.out.display());
    println!("stems={}", args.stems_dir.display());
    println!("events={event_count} peak={peak:.6} rms={rms:.6} limiter_db={worst_reduction:.3}");
    Ok(())
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
