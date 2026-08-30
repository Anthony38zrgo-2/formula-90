//! Offline diagnostic renderer for the procedural V10.
//!
//! This is not a unit test: it runs the same `VehicleAudioEngine` mixer used by
//! the runtime, enables the profile's half-block synth, reconstructs the second
//! bank acoustically and writes the final stereo mix to a 16-bit WAV.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};
use vehicle_audio_engine::powertrain;
use vehicle_audio_engine::synth::{HalfBlock, PowertrainConfig};
use vehicle_audio_engine::VehicleAudioEngine;

const SAMPLE_RATE: u32 = 44_100;
const DURATION_S: f64 = 10.0;
const BLOCK_SIZE: usize = 512;

fn write_wav_header<W: Write>(writer: &mut W, frames: u32) -> std::io::Result<()> {
    let channels = 2u16;
    let bits = 16u16;
    let bytes_per_sample = (bits / 8) as u32;
    let data_bytes = frames * channels as u32 * bytes_per_sample;
    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_bytes).to_le_bytes())?;
    writer.write_all(b"WAVEfmt ")?;
    writer.write_all(&16u32.to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&SAMPLE_RATE.to_le_bytes())?;
    writer.write_all(&(SAMPLE_RATE * channels as u32 * bytes_per_sample).to_le_bytes())?;
    writer.write_all(&(channels * bits / 8).to_le_bytes())?;
    writer.write_all(&bits.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_bytes.to_le_bytes())
}

fn write_mono_header<W: Write>(writer: &mut W, frames: u32) -> std::io::Result<()> {
    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + frames * 2).to_le_bytes())?;
    writer.write_all(b"WAVEfmt ")?;
    writer.write_all(&16u32.to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&SAMPLE_RATE.to_le_bytes())?;
    writer.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?;
    writer.write_all(&2u16.to_le_bytes())?;
    writer.write_all(&16u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&(frames * 2).to_le_bytes())
}

fn pcm16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

fn sweep_state(time_s: f64, idle_rpm: f64, max_rpm: f64, duration_s: f64) -> (f64, f32) {
    if time_s < 1.5 {
        return (idle_rpm, 0.0);
    }
    let x = ((time_s - 1.5) / (duration_s - 1.5).max(0.01)).clamp(0.0, 1.0);
    let smooth = x * x * (3.0 - 2.0 * x);
    let rpm = idle_rpm + (max_rpm - idle_rpm) * smooth;
    let throttle = (0.12 + 0.88 * x.sqrt()) as f32;
    (rpm, throttle)
}

/// Phased road schedule: bottom hold (off throttle) -> up ramp -> top hold ->
/// exponential coast-down (off throttle) -> bottom hold again.
///
/// The down-leg decays like a free-wheeling engine: fast immediately after the
/// throttle closes, slow as it approaches the idle range. Both endpoints are
/// exact: `state(0) == (min_rpm, 0.0)` and `state(&spec.decay_shape)` reaches
/// `min_rpm` at the end of the down phase.
struct RoadSpec {
    min_rpm: f64,
    bottom_hold_s: f64,
    up_s: f64,
    top_hold_s: f64,
    down_s: f64,
    tail_s: f64,
    decay_k: f64,
}

impl RoadSpec {
    fn total_s(&self) -> f64 {
        self.bottom_hold_s + self.up_s + self.top_hold_s + self.down_s + self.tail_s
    }

    fn state(&self, time_s: f64, max_rpm: f64) -> (f64, f32) {
        let t = time_s - self.bottom_hold_s;
        if t <= 0.0 {
            return (self.min_rpm, 0.0);
        }
        if t < self.up_s {
            let x = (t / self.up_s).clamp(0.0, 1.0);
            let smooth = x * x * (3.0 - 2.0 * x);
            let rpm = self.min_rpm + (max_rpm - self.min_rpm) * smooth;
            return (rpm, (0.12 + 0.88 * x.sqrt()) as f32);
        }
        let t = t - self.up_s;
        if t < self.top_hold_s {
            return (max_rpm, 1.0);
        }
        let t = t - self.top_hold_s;
        if t < self.down_s {
            let u = (t / self.down_s).clamp(0.0, 1.0);
            let end = (-self.decay_k).exp();
            let frac = ((-self.decay_k * u).exp() - end) / (1.0 - end);
            let rpm = self.min_rpm + (max_rpm - self.min_rpm) * frac;
            return (rpm, 0.0);
        }
        (self.min_rpm, 0.0)
    }
}

fn profile_rpm(profile: &Value, key: &str) -> Result<f64, String> {
    profile
        .get("powertrain")
        .and_then(|v| v.get(key))
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing numeric powertrain.{key}"))
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn render_diagnostic(
    dir: &Path,
    contract: &vehicle_audio_engine::powertrain::AudioPowertrainSynthesis,
    idle_rpm: f64,
    max_rpm: f64,
    fixed_rpm: Option<f64>,
    road: Option<&RoadSpec>,
    raw_engine: bool,
    duration_s: f64,
    warmup_s: f64,
    wav_path: &Path,
    csv_path: &Path,
    metadata_path: &Path,
    profile_text: &str,
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut synth = HalfBlock::new(&PowertrainConfig::from(contract), SAMPLE_RATE);
    let mut master: Vec<(f32, f32)> =
        Vec::with_capacity((duration_s * SAMPLE_RATE as f64) as usize);
    let mut step =
        |count: usize,
         time_s: f64,
         discard: bool,
         buffers: &mut std::collections::HashMap<&'static str, Vec<f32>>| {
            let (rpm, throttle) = if let Some(min) = fixed_rpm {
                (min, 0.85)
            } else if let Some(spec) = road {
                spec.state(time_s, max_rpm)
            } else {
                sweep_state(time_s, idle_rpm, max_rpm, duration_s)
            };
            synth.update_controls(rpm, idle_rpm, max_rpm, throttle);
            for _ in 0..count {
                let frame = synth.render_diagnostic_frame();
                if !discard {
                    // --raw-engine: bypass every filter, limiter and DC guard.
                    // The master is the clean combustion impulse train
                    // (bank_a_raw + bank_b_raw), still dual-mono.
                    let sample = if raw_engine {
                        frame.bank_a_raw + frame.bank_b_raw
                    } else {
                        frame.master.0
                    };
                    master.push((sample, sample));
                    let mut all_values: Vec<(&str, f32)> = vec![
                        ("pre_body_a", frame.pre_body_a),
                        ("pre_body_b", frame.pre_body_b),
                        ("combustion_body_a", frame.body_a),
                        ("combustion_body_b", frame.body_b),
                        ("combustion_body", frame.combustion_body),
                        ("combustion_edge_a", frame.combustion_edge_a),
                        ("combustion_edge_b", frame.combustion_edge_b),
                        ("combustion_edge", frame.combustion_edge),
                        ("mono_engine", frame.mono_engine),
                        // Disabled voices: must remain exact silence.
                        ("intake", frame.intake),
                        ("exhaust", frame.exhaust),
                        ("rasp", frame.rasp),
                        ("master_l", frame.master.0),
                        ("master_r", frame.master.1),
                    ];
                    if raw_engine {
                        all_values.push(("bank_a_raw", frame.bank_a_raw));
                        all_values.push(("bank_b_raw", frame.bank_b_raw));
                        all_values.push(("raw_engine", frame.bank_a_raw + frame.bank_b_raw));
                    }
                    for (name, value) in all_values {
                        buffers.entry(name).or_default().push(value);
                    }
                }
            }
        };
    let warm = (warmup_s * SAMPLE_RATE as f64).round() as usize;
    step(warm, 0.0, true, &mut std::collections::HashMap::new());
    let total = (duration_s * SAMPLE_RATE as f64).round() as usize;
    let mut buffers = std::collections::HashMap::new();
    let mut rendered = 0;
    while rendered < total {
        let count = (total - rendered).min(BLOCK_SIZE);
        step(
            count,
            rendered as f64 / SAMPLE_RATE as f64,
            false,
            &mut buffers,
        );
        rendered += count;
    }
    // Dual-mono contract: the continuous procedural engine must carry the same
    // sample on both channels, bit for bit.
    let mut dual_mono_identical = true;
    let mut first_mismatch = 0usize;
    for (index, (left, right)) in master.iter().enumerate() {
        if left.to_bits() != right.to_bits() {
            dual_mono_identical = false;
            first_mismatch = index;
            break;
        }
    }
    if !dual_mono_identical {
        return Err(format!(
            "dual-mono contract violated: L != R at sample {first_mismatch}"
        ));
    }
    // Combustion-only provenance: the two voices must reconstruct pre_body
    // exactly (residual is the complementary high-pass, so body + edge ==
    // pre_body up to f32 rounding), and the disabled voices must be silent.
    let mut max_reconstruction_error = 0.0f32;
    if let (Some(pre_a), Some(body_a), Some(edge_a), Some(pre_b), Some(body_b), Some(edge_b)) = (
        buffers.get("pre_body_a"),
        buffers.get("combustion_body_a"),
        buffers.get("combustion_edge_a"),
        buffers.get("pre_body_b"),
        buffers.get("combustion_body_b"),
        buffers.get("combustion_edge_b"),
    ) {
        for index in 0..pre_a.len().min(pre_b.len()) {
            let residual_a = pre_a[index] - (body_a[index] + edge_a[index]);
            let residual_b = pre_b[index] - (body_b[index] + edge_b[index]);
            max_reconstruction_error = max_reconstruction_error
                .max(residual_a.abs())
                .max(residual_b.abs());
        }
    }
    if max_reconstruction_error > 1e-5 {
        return Err(format!(
            "combustion_edge is not the complementary residual: reconstruction error {max_reconstruction_error}"
        ));
    }
    for name in ["intake", "exhaust"] {
        if let Some(values) = buffers.get(name) {
            if values.iter().any(|v| *v != 0.0) {
                return Err(format!(
                    "disabled voice {name} produced non-zero samples; it must be exact silence"
                ));
            }
        }
    }
    if !contract.rasp.enabled {
        if let Some(values) = buffers.get("rasp") {
            if values.iter().any(|v| *v != 0.0) {
                return Err(
                    "disabled voice rasp produced non-zero samples; it must be exact silence"
                        .to_string(),
                );
            }
        }
    }
    for (name, values) in &buffers {
        let path = dir.join(format!("{name}.wav"));
        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        write_mono_header(&mut writer, values.len() as u32).map_err(|e| e.to_string())?;
        for value in values {
            writer
                .write_all(&pcm16(*value).to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
    }
    ensure_parent(wav_path).map_err(|e| e.to_string())?;
    let mut wav = BufWriter::new(File::create(wav_path).map_err(|e| e.to_string())?);
    write_wav_header(&mut wav, master.len() as u32).map_err(|e| e.to_string())?;
    for (left, right) in &master {
        wav.write_all(&pcm16(*left).to_le_bytes())
            .map_err(|e| e.to_string())?;
        wav.write_all(&pcm16(*right).to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    wav.flush().map_err(|e| e.to_string())?;
    ensure_parent(csv_path).map_err(|e| e.to_string())?;
    let mut csv = BufWriter::new(File::create(csv_path).map_err(|e| e.to_string())?);
    writeln!(csv, "time_s,rpm,throttle,rms_l,rms_r,peak").map_err(|e| e.to_string())?;
    for (block_index, block) in master.chunks(BLOCK_SIZE).enumerate() {
        let index = block_index * BLOCK_SIZE;
        let time_s = index as f64 / SAMPLE_RATE as f64;
        let (rpm, throttle) = if let Some(min) = fixed_rpm {
            (min, 0.85)
        } else if let Some(spec) = road {
            spec.state(time_s, max_rpm)
        } else {
            sweep_state(time_s, idle_rpm, max_rpm, duration_s)
        };
        let frames = block.len().max(1) as f64;
        let rms_l = (block
            .iter()
            .map(|(left, _)| (*left as f64) * (*left as f64))
            .sum::<f64>()
            / frames)
            .sqrt();
        let rms_r = (block
            .iter()
            .map(|(_, right)| (*right as f64) * (*right as f64))
            .sum::<f64>()
            / frames)
            .sqrt();
        let peak = block
            .iter()
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0f32, f32::max);
        writeln!(
            csv,
            "{:.6},{:.3},{:.6},{:.9},{:.9},{:.9}",
            time_s, rpm, throttle, rms_l, rms_r, peak
        )
        .map_err(|e| e.to_string())?;
    }
    csv.flush().map_err(|e| e.to_string())?;
    let mut hashes = serde_json::Map::new();
    for (name, _) in &buffers {
        let path = dir.join(format!("{name}.wav"));
        if let Ok(bytes) = fs::read(&path) {
            hashes.insert(
                (*name).to_string(),
                serde_json::Value::String(format!("{:x}", Sha256::digest(bytes))),
            );
        }
    }
    if let Ok(bytes) = fs::read(wav_path) {
        hashes.insert(
            "master_wav".to_string(),
            serde_json::Value::String(format!("{:x}", Sha256::digest(bytes))),
        );
    }
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let metadata = serde_json::json!({"schema":"engine-sweep-render-v2", "diagnostic_stems":true,
        "dual_mono_identical":dual_mono_identical, "master_l_sha256":hashes.get("master_l").cloned(),
        "master_r_sha256":hashes.get("master_r").cloned(),
        "git_sha":git_sha, "synthesis_version":env!("CARGO_PKG_VERSION"), "hashes":hashes,
        "sample_rate_hz":SAMPLE_RATE, "duration_s":duration_s, "warmup_s":warmup_s, "fixed_rpm":fixed_rpm,
        "raw_engine":raw_engine,
        "road":road.as_ref().map(|spec| serde_json::json!({"min_rpm":spec.min_rpm,
            "bottom_hold_s":spec.bottom_hold_s, "up_s":spec.up_s, "top_hold_s":spec.top_hold_s,
            "down_s":spec.down_s, "tail_s":spec.tail_s, "decay_k":spec.decay_k,
            "total_s":spec.total_s()})),
        "config_sha256":format!("{:x}", Sha256::digest(profile_text.as_bytes())),
        "wav":wav_path.to_string_lossy(), "telemetry":csv_path.to_string_lossy(), "stem_dir":dir.to_string_lossy(), "stem_count":buffers.len()});
    ensure_parent(metadata_path).map_err(|e| e.to_string())?;
    fs::write(
        metadata_path,
        serde_json::to_string_pretty(&metadata).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        return Err("usage: engine_sweep_render <bank_dir> <physics_profile.json> <output.wav> <telemetry.csv> [--fixed-rpm RPM] [--road-sweep MIN_RPM] [--road-up-s SEC] [--road-down-s SEC] [--road-top-s SEC] [--road-bottom-s SEC] [--road-tail-s SEC] [--road-decay-k K] [--raw-engine] [--duration SEC] [--warmup SEC] [--metadata PATH] [--diagnostic-stems DIR]".to_string());
    }
    let bank_dir = PathBuf::from(&args[1]);
    let profile_path = PathBuf::from(&args[2]);
    let wav_path = PathBuf::from(&args[3]);
    let csv_path = PathBuf::from(&args[4]);
    let mut fixed_rpm: Option<f64> = None;
    let mut road_min_rpm: Option<f64> = None;
    let mut road_up_s = 11.0;
    let mut road_down_s = 11.0;
    let mut road_top_s = 2.0;
    let mut road_bottom_s = 1.5;
    let mut road_tail_s = 2.0;
    let mut road_decay_k = 5.0;
    let mut duration_s = DURATION_S;
    let mut duration_explicit = false;
    let mut warmup_s = 0.0;
    let mut metadata_path = wav_path.with_extension("metadata.json");
    let mut diagnostic_dir: Option<PathBuf> = None;
    let mut raw_engine = false;
    let mut i = 5;
    while i < args.len() {
        let flag = &args[i];
        if flag == "--raw-engine" {
            raw_engine = true;
            i += 1;
            continue;
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--fixed-rpm" => {
                fixed_rpm = Some(
                    value
                        .parse()
                        .map_err(|_| "invalid --fixed-rpm".to_string())?,
                )
            }
            "--road-sweep" => {
                road_min_rpm = Some(
                    value
                        .parse()
                        .map_err(|_| "invalid --road-sweep".to_string())?,
                )
            }
            "--road-up-s" => {
                road_up_s = value.parse().map_err(|_| "invalid --road-up-s".to_string())?
            }
            "--road-down-s" => {
                road_down_s = value.parse().map_err(|_| "invalid --road-down-s".to_string())?
            }
            "--road-top-s" => {
                road_top_s = value.parse().map_err(|_| "invalid --road-top-s".to_string())?
            }
            "--road-bottom-s" => {
                road_bottom_s = value.parse().map_err(|_| "invalid --road-bottom-s".to_string())?
            }
            "--road-tail-s" => {
                road_tail_s = value.parse().map_err(|_| "invalid --road-tail-s".to_string())?
            }
            "--road-decay-k" => {
                road_decay_k = value.parse().map_err(|_| "invalid --road-decay-k".to_string())?
            }
            "--duration" => {
                duration_s = value
                    .parse()
                    .map_err(|_| "invalid --duration".to_string())?;
                duration_explicit = true;
            }
            "--warmup" => warmup_s = value.parse().map_err(|_| "invalid --warmup".to_string())?,
            "--metadata" => metadata_path = PathBuf::from(value),
            "--diagnostic-stems" => diagnostic_dir = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown option {flag}")),
        }
        i += 2;
    }
    if duration_s <= 0.0 || warmup_s < 0.0 {
        return Err("duration must be positive and warmup non-negative".to_string());
    }
    let profile_text = fs::read_to_string(&profile_path)
        .map_err(|e| format!("read {}: {e}", profile_path.display()))?;
    let profile: Value = serde_json::from_str(&profile_text).map_err(|e| e.to_string())?;
    let idle_rpm = profile_rpm(&profile, "idle_rpm")?;
    let max_rpm = profile_rpm(&profile, "max_rpm")?;
    if let Some(rpm) = fixed_rpm {
        if rpm <= 0.0 || rpm < idle_rpm || rpm > max_rpm {
            return Err(format!(
                "--fixed-rpm {rpm} must be within profile range {idle_rpm}..{max_rpm}"
            ));
        }
    }
    let contract = powertrain::from_profile_json(&profile_text)
        .map_err(|e| format!("powertrain contract: {e}"))?
        .ok_or_else(|| "profile has no audio.powertrain_synthesis".to_string())?;
    if !contract.enabled {
        return Err("audio.powertrain_synthesis is disabled".to_string());
    }
    let road = match road_min_rpm {
        Some(min) => {
            if fixed_rpm.is_some() {
                return Err("--road-sweep and --fixed-rpm are mutually exclusive".to_string());
            }
            if min <= 0.0 || min >= max_rpm {
                return Err(format!(
                    "--road-sweep {min} must be 0 < min_rpm < max_rpm {max_rpm}"
                ));
            }
            if road_up_s <= 0.0
                || road_down_s <= 0.0
                || road_top_s < 0.0
                || road_bottom_s < 0.0
                || road_tail_s < 0.0
                || road_decay_k <= 0.0
            {
                return Err(
                    "road phases must be non-negative and up/down/decay_k strictly positive"
                        .to_string(),
                );
            }
            Some(RoadSpec {
                min_rpm: min,
                bottom_hold_s: road_bottom_s,
                up_s: road_up_s,
                top_hold_s: road_top_s,
                down_s: road_down_s,
                tail_s: road_tail_s,
                decay_k: road_decay_k,
            })
        }
        None => None,
    };
    let road_total_s = road.as_ref().map(|spec| spec.total_s());
    if let Some(total) = road_total_s {
        if duration_explicit && (duration_s - total).abs() > 0.1 {
            return Err(format!(
                "--duration {duration_s} conflicts with road schedule total {total}s"
            ));
        }
        if !duration_explicit {
            duration_s = total;
        }
    }
    if let Some(dir) = diagnostic_dir {
        return render_diagnostic(
            &dir,
            &contract,
            idle_rpm,
            max_rpm,
            fixed_rpm,
            road.as_ref(),
            raw_engine,
            duration_s,
            warmup_s,
            &wav_path,
            &csv_path,
            &metadata_path,
            &profile_text,
        );
    }
    if raw_engine {
        return Err("--raw-engine requires --diagnostic-stems".to_string());
    }

    ensure_parent(&wav_path).map_err(|e| e.to_string())?;
    ensure_parent(&csv_path).map_err(|e| e.to_string())?;
    ensure_parent(&metadata_path).map_err(|e| e.to_string())?;
    let mut engine = VehicleAudioEngine::new(&bank_dir)
        .map_err(|e| format!("load bank {}: {e}", bank_dir.display()))?;
    engine.enable_synth(&contract);
    engine.set_listener_distance(3.0);

    // Warm-up is rendered and discarded so fixed-RPM output starts after the
    // same filter/envelope settling period used by the runtime.
    if warmup_s > 0.0 {
        let warm_frames = (warmup_s * SAMPLE_RATE as f64).round() as usize;
        let mut wl = vec![0.0f32; BLOCK_SIZE];
        let mut wr = vec![0.0f32; BLOCK_SIZE];
        let mut done = 0;
        while done < warm_frames {
            let count = (warm_frames - done).min(BLOCK_SIZE);
            let rpm = fixed_rpm.unwrap_or_else(|| {
                road.as_ref().map_or(idle_rpm, |spec| spec.min_rpm)
            });
            engine.set_state(rpm, idle_rpm, max_rpm, 0.8, 0.0, 3, 0.0, "asphalt");
            engine.render(&mut wl[..count], &mut wr[..count], count);
            done += count;
        }
    }

    let total_frames = (duration_s * SAMPLE_RATE as f64).round() as usize;
    let wav_file = File::create(&wav_path).map_err(|e| e.to_string())?;
    let mut wav = BufWriter::new(wav_file);
    write_wav_header(&mut wav, total_frames as u32).map_err(|e| e.to_string())?;
    let csv_file = File::create(&csv_path).map_err(|e| e.to_string())?;
    let mut csv = BufWriter::new(csv_file);
    writeln!(csv, "time_s,rpm,throttle,rms_l,rms_r,peak").map_err(|e| e.to_string())?;

    let mut rendered = 0usize;
    let mut left = vec![0.0f32; BLOCK_SIZE];
    let mut right = vec![0.0f32; BLOCK_SIZE];
    while rendered < total_frames {
        let count = (total_frames - rendered).min(BLOCK_SIZE);
        let time_s = rendered as f64 / SAMPLE_RATE as f64;
        let (rpm, throttle) = if let Some(min) = fixed_rpm {
            (min, 0.85)
        } else if let Some(spec) = road.as_ref() {
            spec.state(time_s, max_rpm)
        } else {
            sweep_state(time_s, idle_rpm, max_rpm, duration_s)
        };
        let ramp = ((rpm - idle_rpm) / (max_rpm - idle_rpm)).clamp(0.0, 1.0);
        engine.set_state(
            rpm,
            idle_rpm,
            max_rpm,
            throttle,
            ramp * 300.0,
            3,
            0.0,
            "asphalt",
        );
        engine.render(&mut left[..count], &mut right[..count], count);
        let mut sum_l = 0.0f64;
        let mut sum_r = 0.0f64;
        let mut peak = 0.0f32;
        for (&l, &r) in left[..count].iter().zip(&right[..count]) {
            sum_l += (l * l) as f64;
            sum_r += (r * r) as f64;
            peak = peak.max(l.abs()).max(r.abs());
            wav.write_all(&pcm16(l).to_le_bytes())
                .map_err(|e| e.to_string())?;
            wav.write_all(&pcm16(r).to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        let rms_l = (sum_l / count as f64).sqrt();
        let rms_r = (sum_r / count as f64).sqrt();
        writeln!(
            csv,
            "{time_s:.6},{rpm:.3},{throttle:.6},{rms_l:.9},{rms_r:.9},{peak:.9}"
        )
        .map_err(|e| e.to_string())?;
        rendered += count;
    }
    wav.flush().map_err(|e| e.to_string())?;
    csv.flush().map_err(|e| e.to_string())?;
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mode = if let Some(rpm) = fixed_rpm {
        format!("fixed-rpm:{rpm}")
    } else if let Some(spec) = road.as_ref() {
        format!("road-sweep:{:.0}-{:.0}", spec.min_rpm, max_rpm)
    } else {
        "sweep".to_string()
    };
    let config_sha256 = format!("{:x}", Sha256::digest(profile_text.as_bytes()));
    let metadata = serde_json::json!({"schema":"engine-sweep-render-v2", "git_sha":git_sha,
        "sample_rate_hz":SAMPLE_RATE, "duration_s":duration_s, "warmup_s":warmup_s,
        "mode":mode, "fixed_rpm":fixed_rpm, "road_min_rpm":road.as_ref().map(|s| s.min_rpm),
        "road_total_s":road_total_s, "synthesis_version":env!("CARGO_PKG_VERSION"),
        "profile_path":profile_path.to_string_lossy(), "config_sha256":config_sha256});
    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!(
        "rendered {:.1}s / {} frames / {:.0}-{:.0} RPM -> {}",
        duration_s,
        total_frames,
        idle_rpm,
        max_rpm,
        wav_path.display()
    );
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("engine_sweep_render: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
