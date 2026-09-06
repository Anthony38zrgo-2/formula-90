//! Deterministic AUD-02 capture through the two real GF509 runtime routes.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use v10_engine_synth::{
    Gf509Runtime, Gf509RuntimeConfig, RuntimeTelemetry, ShiftPhase, TorqueSign,
};
use vehicle_audio_engine::ffi::{VehicleAudioTelemetryV3, VEHICLE_AUDIO_ABI_VERSION};
use vehicle_audio_engine::{ContinuousSourceKind, VehicleAudioEngine};

const SAMPLE_RATE: u32 = 44_100;
const BLOCK_FRAMES: usize = 256;
const BLOCKS: usize = 1_034;
const DT: f32 = BLOCK_FRAMES as f32 / SAMPLE_RATE as f32;

fn trace_state(block: usize, blocks: usize, sweep_10s: bool) -> (f32, f32, f32, f32) {
    let t = block as f32 / (blocks - 1) as f32;
    if sweep_10s {
        if t < 0.70 {
            let u = t / 0.70;
            return (5_000.0 + 13_000.0 * u, 0.95, 0.90, 0.85);
        }
        let u = (t - 0.70) / 0.30;
        return (18_000.0 - 10_000.0 * u, 0.0, 0.45, -0.50);
    }
    if t < 0.72 {
        let u = t / 0.72;
        (4_500.0 + 10_500.0 * u, 0.92, 0.88, 0.82)
    } else {
        let u = (t - 0.72) / 0.28;
        (15_000.0 - 8_500.0 * u, 0.035, 0.10, -0.45)
    }
}

fn trace(block: usize, blocks: usize, sweep_10s: bool) -> RuntimeTelemetry {
    let t = block as f32 / (blocks - 1) as f32;
    let (rpm, throttle, load, torque) = trace_state(block, blocks, sweep_10s);
    let (previous_rpm, previous_throttle, _, _) =
        trace_state(block.saturating_sub(1), blocks, sweep_10s);
    RuntimeTelemetry {
        rpm,
        throttle,
        normalized_engine_load: load,
        normalized_engine_torque: torque,
        torque_sign: if torque < 0.0 {
            TorqueSign::Negative
        } else {
            TorqueSign::Positive
        },
        rpm_derivative: (rpm - previous_rpm) / DT,
        throttle_derivative: (throttle - previous_throttle) / DT,
        gear: if t < 0.48 { 3 } else { 4 },
        shift_phase: if (0.48..0.50).contains(&t) {
            ShiftPhase::UpshiftCut
        } else {
            ShiftPhase::None
        },
        clutch_engagement: 1.0,
        tc_cut_ratio: 0.0,
        rev_limiter_active: false,
        dt_seconds: DT,
    }
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn directory_hashes(
    directory: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let mut paths: Vec<_> = fs::read_dir(directory)
        .map_err(|e| format!("read directory {}: {e}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|v| v.to_str()) != Some("import")
        })
        .collect();
    paths.sort();
    let mut hashes = serde_json::Map::new();
    for path in paths {
        hashes.insert(
            path.file_name().unwrap().to_string_lossy().into_owned(),
            serde_json::Value::String(sha256(&path)?),
        );
    }
    Ok(hashes)
}

fn source_hashes() -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let vehicle_audio = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = vehicle_audio.parent().ok_or("crate root has no parent")?;
    let v10 = crates.join("v10-engine-synth/src");
    let mut paths = vec![
        vehicle_audio.join("src/mixer.rs"),
        vehicle_audio.join("src/config.rs"),
        vehicle_audio.join("src/powertrain.rs"),
    ];
    let mut pending = vec![v10];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let mut hashes = serde_json::Map::new();
    for path in paths {
        let relative = path.strip_prefix(crates).map_err(|e| e.to_string())?;
        hashes.insert(
            relative.to_string_lossy().replace('\\', "/"),
            serde_json::Value::String(sha256(&path)?),
        );
    }
    Ok(hashes)
}

fn validate_audio(name: &str, samples: &[f32]) -> Result<f32, String> {
    if let Some((index, value)) = samples
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(format!(
            "{name} contains non-finite sample {value} at frame {index}"
        ));
    }
    let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
    if peak > 1.0 {
        return Err(format!("{name} peak {peak} exceeds PCM range"));
    }
    Ok(peak)
}

fn pcm16(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

fn write_stereo_pcm16(path: &Path, left: &[f32], right: &[f32]) -> Result<(), String> {
    if left.len() != right.len() {
        return Err("stereo channel length mismatch".into());
    }
    let data_bytes = (left.len() * 4) as u32;
    let mut writer = BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
    writer.write_all(b"RIFF").map_err(|e| e.to_string())?;
    writer
        .write_all(&(36 + data_bytes).to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(b"WAVEfmt \x10\0\0\0\x01\0\x02\0")
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&SAMPLE_RATE.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&(SAMPLE_RATE * 4).to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&4u16.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&16u16.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"data").map_err(|e| e.to_string())?;
    writer
        .write_all(&data_bytes.to_le_bytes())
        .map_err(|e| e.to_string())?;
    for (&l, &r) in left.iter().zip(right) {
        writer
            .write_all(&pcm16(l).to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&pcm16(r).to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        return Err("usage: aud_runtime_baseline <bank-dir> <gf509-assets> <output-dir>".into());
    }
    let bank_dir = PathBuf::from(&args[1]);
    let asset_dir = PathBuf::from(&args[2]);
    let output_dir = PathBuf::from(&args[3]);
    let sweep_10s = std::env::var_os("F90_AUDIO_SWEEP_10S").is_some();
    let blocks = if sweep_10s {
        (10.0 * SAMPLE_RATE as f32 / BLOCK_FRAMES as f32).ceil() as usize
    } else {
        BLOCKS
    };
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let mut config = Gf509RuntimeConfig::default();
    config.engine.sample_rate = SAMPLE_RATE;
    config.max_block_frames = BLOCK_FRAMES;
    config.sample_layer_directory = Some(asset_dir.clone());
    let mut gf509 = Gf509Runtime::new(config)?;
    let mut mixer = VehicleAudioEngine::new(&bank_dir).map_err(|e| e.to_string())?;
    mixer.enable_v10_gf509(&asset_dir)?;
    if mixer.continuous_source() != ContinuousSourceKind::V10Gf509 {
        return Err("VehicleAudioEngine did not activate GF509".into());
    }

    let mut direct_left = Vec::with_capacity(blocks * BLOCK_FRAMES);
    let mut direct_right = Vec::with_capacity(blocks * BLOCK_FRAMES);
    let mut mixed_left = Vec::with_capacity(blocks * BLOCK_FRAMES);
    let mut mixed_right = Vec::with_capacity(blocks * BLOCK_FRAMES);
    let mut trace_csv = String::from("block,time_s,rpm,throttle,normalized_engine_load,normalized_engine_torque,torque_sign,rpm_derivative,throttle_derivative,gear,shift_phase,dt_seconds,clutch_engagement,tc_cut_ratio,rev_limiter_active\n");
    let mut left = [0.0f32; BLOCK_FRAMES];
    let mut right = [0.0f32; BLOCK_FRAMES];
    for block in 0..blocks {
        let telemetry = trace(block, blocks, sweep_10s);
        gf509.update_telemetry(telemetry)?;
        gf509.render_block(&mut left, &mut right)?;
        direct_left.extend_from_slice(&left);
        direct_right.extend_from_slice(&right);

        let packet = VehicleAudioTelemetryV3 {
            schema_version: VEHICLE_AUDIO_ABI_VERSION,
            struct_size: std::mem::size_of::<VehicleAudioTelemetryV3>() as u32,
            rpm: telemetry.rpm as f64,
            idle_rpm: 4_500.0,
            max_rpm: if sweep_10s { 18_000.0 } else { 17_000.0 },
            throttle: telemetry.throttle,
            normalized_engine_load: telemetry.normalized_engine_load,
            normalized_engine_torque: telemetry.normalized_engine_torque,
            rpm_derivative: telemetry.rpm_derivative,
            throttle_derivative: telemetry.throttle_derivative,
            speed_kph: 180.0,
            slip: 0.0,
            gear: telemetry.gear as i32,
            torque_sign: telemetry.torque_sign as i32,
            shift_phase: telemetry.shift_phase as i32,
            clutch_engagement: if telemetry.shift_phase == ShiftPhase::None {
                1.0
            } else {
                0.0
            },
            tc_cut_ratio: 0.0,
            rev_limiter_active: 0,
        };
        writeln!(
            trace_csv,
            "{block},{:.9},{:.6},{:.6},{:.6},{:.6},{},{:.6},{:.6},{},{},{:.9},{:.6},{:.6},{}",
            block as f32 * DT,
            packet.rpm,
            packet.throttle,
            packet.normalized_engine_load,
            packet.normalized_engine_torque,
            packet.torque_sign,
            packet.rpm_derivative,
            packet.throttle_derivative,
            packet.gear,
            packet.shift_phase,
            DT,
            packet.clutch_engagement,
            packet.tc_cut_ratio,
            packet.rev_limiter_active
        )
        .unwrap();
        mixer.set_telemetry(&packet, "asphalt");
        mixer.render(&mut left, &mut right, BLOCK_FRAMES);
        if mixer.gf509_render_failed() {
            return Err(format!("GF509 render failed at block {block}"));
        }
        mixed_left.extend_from_slice(&left);
        mixed_right.extend_from_slice(&right);
    }
    if sweep_10s {
        let exact_frames = 10 * SAMPLE_RATE as usize;
        direct_left.truncate(exact_frames);
        direct_right.truncate(exact_frames);
        mixed_left.truncate(exact_frames);
        mixed_right.truncate(exact_frames);
    }

    let direct_path = output_dir.join("gf509_runtime.wav");
    let mixer_path = output_dir.join("vehicle_audio_engine.wav");
    let direct_peak = validate_audio("Gf509Runtime left", &direct_left)?
        .max(validate_audio("Gf509Runtime right", &direct_right)?);
    let mixer_peak = validate_audio("VehicleAudioEngine left", &mixed_left)?
        .max(validate_audio("VehicleAudioEngine right", &mixed_right)?);
    write_stereo_pcm16(&direct_path, &direct_left, &direct_right)?;
    write_stereo_pcm16(&mixer_path, &mixed_left, &mixed_right)?;
    let trace_path = output_dir.join("trace.csv");
    fs::write(&trace_path, trace_csv).map_err(|e| e.to_string())?;
    let git_head = command_output("git", &["rev-parse", "HEAD"]);
    let source_status = command_output("git", &["status", "--short"]);
    let rustc = command_output("rustc", &["--version", "--verbose"]);
    let harness_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/aud_runtime_baseline.rs");
    let metadata = serde_json::json!({
        "schema": "aud02-runtime-baseline-v1",
        "sample_rate_hz": SAMPLE_RATE,
        "block_frames": BLOCK_FRAMES,
        "blocks": blocks,
        "frames": direct_left.len(),
        "capture_mode": if sweep_10s { "10s_5000_to_18000_then_lift_coast_7s_3s" } else { "aud02_baseline" },
        "trace_sha256": sha256(&trace_path)?,
        "trace_file": "trace.csv",
        "git_head": git_head,
        "source_dirty": !source_status.is_empty(),
        "source_status": source_status,
        "rustc": rustc,
        "harness_sha256": sha256(&harness_path)?,
        "audio_source_hashes": source_hashes()?,
        "runtime_config": {"sample_rate_hz": SAMPLE_RATE, "max_block_frames": BLOCK_FRAMES, "gf509_config": "Gf509RuntimeConfig::default except listed overrides"},
        "mixer_dt_behavior": "VehicleAudioEngine::set_telemetry forwards dt_seconds=0.0 to GF509 at this baseline; direct Gf509Runtime receives trace dt_seconds",
        "gf509_activated": true,
        "gf509_render_failed": false,
        "gf509_runtime_peak": direct_peak,
        "vehicle_audio_engine_peak": mixer_peak,
        "gf509_runtime_sha256": sha256(&direct_path)?,
        "vehicle_audio_engine_sha256": sha256(&mixer_path)?,
        "gf509_asset_hashes": directory_hashes(&asset_dir)?,
        "bank_asset_hashes": directory_hashes(&bank_dir)?,
    });
    fs::write(
        output_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&metadata).unwrap());
    Ok(())
}
