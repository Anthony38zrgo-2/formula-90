//! Reproducible V10-001..003 probes.
//!
//! This is an evidence harness, not a production toggle. It records the
//! effective saturation value, deterministic paired GF509 output, and the
//! settled Near/Far behavior of the active mixer route without changing
//! any runtime configuration.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use sha2::{Digest, Sha256};
use v10_engine_synth::{
    Gf509Runtime, Gf509RuntimeConfig, RuntimeTelemetry, ShiftPhase, TorqueSign,
};
use vehicle_audio_engine::ffi::{VehicleAudioTelemetryV3, VEHICLE_AUDIO_ABI_VERSION};
use vehicle_audio_engine::{ContinuousSourceKind, DiagnosticMode, VehicleAudioEngine};

const SAMPLE_RATE: u32 = 44_100;
const BLOCK: usize = 256;

fn telemetry() -> RuntimeTelemetry {
    RuntimeTelemetry {
        rpm: 12_000.0,
        throttle: 1.0,
        normalized_engine_load: 1.0,
        normalized_engine_torque: 0.8,
        torque_sign: TorqueSign::Positive,
        rpm_derivative: 0.0,
        throttle_derivative: 0.0,
        gear: 5,
        shift_phase: ShiftPhase::None,
        clutch_engagement: 1.0,
        tc_cut_ratio: 0.0,
        rev_limiter_active: false,
        dt_seconds: BLOCK as f32 / SAMPLE_RATE as f32,
    }
}

fn packet(t: RuntimeTelemetry) -> VehicleAudioTelemetryV3 {
    VehicleAudioTelemetryV3 {
        schema_version: VEHICLE_AUDIO_ABI_VERSION,
        struct_size: std::mem::size_of::<VehicleAudioTelemetryV3>() as u32,
        rpm: t.rpm as f64,
        idle_rpm: 4_500.0,
        max_rpm: 17_000.0,
        throttle: t.throttle,
        normalized_engine_load: t.normalized_engine_load,
        normalized_engine_torque: t.normalized_engine_torque,
        rpm_derivative: t.rpm_derivative,
        throttle_derivative: t.throttle_derivative,
        speed_kph: 180.0,
        slip: 0.0,
        gear: t.gear as i32,
        torque_sign: t.torque_sign as i32,
        shift_phase: t.shift_phase as i32,
        clutch_engagement: t.clutch_engagement,
        tc_cut_ratio: t.tc_cut_ratio,
        rev_limiter_active: u32::from(t.rev_limiter_active),
    }
}

fn max_abs_diff(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

fn rms(values: &[f32]) -> f32 {
    (values.iter().map(|v| v * v).sum::<f32>() / values.len() as f32).sqrt()
}

fn sha256_f32(left: &[f32], right: &[f32]) -> String {
    let mut hash = Sha256::new();
    for value in left.iter().chain(right) {
        hash.update(value.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn runtime_pair_probe() -> Result<serde_json::Value, String> {
    let mut config = Gf509RuntimeConfig::default();
    config.engine.sample_rate = SAMPLE_RATE;
    config.max_block_frames = BLOCK;
    config.sample_layer_directory = Some(PathBuf::from("game/audio/v10_gf509"));
    let mut left_runtime = Gf509Runtime::new(config.clone())?;
    let mut right_runtime = Gf509Runtime::new(config)?;
    let mut left = [0.0f32; BLOCK];
    let mut right = [0.0f32; BLOCK];
    let mut paired_max_abs_diff = 0.0f32;
    let input = telemetry();
    for _ in 0..24 {
        left_runtime.update_telemetry(input)?;
        right_runtime.update_telemetry(input)?;
        left_runtime.render_block(&mut left, &mut right)?;
        let mut right_left = [0.0f32; BLOCK];
        let mut right_right = [0.0f32; BLOCK];
        right_runtime.render_block(&mut right_left, &mut right_right)?;
        paired_max_abs_diff = paired_max_abs_diff.max(max_abs_diff(&left, &right_left));
        paired_max_abs_diff = paired_max_abs_diff.max(max_abs_diff(&right, &right_right));
    }
    Ok(json!({
        "sample_rate_hz": SAMPLE_RATE,
        "block_frames": BLOCK,
        "input": {"rpm": 12000.0, "throttle": 1.0, "load": 1.0, "torque": 0.8},
        "paired_max_abs_diff": paired_max_abs_diff,
        "paired_bit_exact": paired_max_abs_diff == 0.0,
        "last_pair_sha256": sha256_f32(&left, &right),
    }))
}

fn mixer_probe(
    bank: &Path,
    assets: &Path,
    config_path: &Path,
) -> Result<serde_json::Value, String> {
    let config_text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let requested_saturation = serde_json::from_str::<serde_json::Value>(&config_text)
        .ok()
        .and_then(|value| value.pointer("/master/saturation").and_then(|v| v.as_f64()))
        .unwrap_or(f64::NAN);

    let mut configured = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    let saturation_before = configured.config().saturation;
    let reload = configured.apply_config_json(&config_text);
    let saturation_after_reload = configured.config().saturation;

    let mut near = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    near.enable_v10_gf509(assets)?;
    near.set_diagnostic_mode(DiagnosticMode::V10Only);
    near.set_stage_diagnostics_enabled(true);
    near.set_listener_distance(5.0);
    let mut far = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    far.enable_v10_gf509(assets)?;
    far.set_diagnostic_mode(DiagnosticMode::V10Only);
    far.set_listener_distance(250.0);
    let input = packet(telemetry());
    let mut near_left = vec![0.0f32; BLOCK];
    let mut near_right = vec![0.0f32; BLOCK];
    let mut far_left = vec![0.0f32; BLOCK];
    let mut far_right = vec![0.0f32; BLOCK];
    for _ in 0..16 {
        near.set_telemetry(&input, "asphalt");
        far.set_telemetry(&input, "asphalt");
        near.render(&mut near_left, &mut near_right, BLOCK);
        far.render(&mut far_left, &mut far_right, BLOCK);
    }
    let near_far_diff =
        max_abs_diff(&near_left, &far_left).max(max_abs_diff(&near_right, &far_right));
    let near_stage = near.continuous_diagnostics();

    let mut overload = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    overload.enable_v10_gf509(assets)?;
    overload.set_diagnostic_mode(DiagnosticMode::V10Only);
    overload.set_stage_diagnostics_enabled(true);
    overload.set_synth_volume(2.0);
    let overload_config = overload.apply_config_json(
        r#"{
            "schema_version": 2,
            "master": {"output_db": 12.0, "saturation": 0.0, "limiter_threshold": 0.90}
        }"#,
    );
    overload.set_telemetry(&input, "asphalt");
    for _ in 0..16 {
        overload.render(&mut near_left, &mut near_right, BLOCK);
    }
    let overload_stage = overload.continuous_diagnostics();

    let mut output_only_mute = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    output_only_mute.enable_v10_gf509(assets)?;
    output_only_mute.set_diagnostic_mode(DiagnosticMode::V10Only);
    output_only_mute.set_telemetry(&input, "asphalt");
    output_only_mute.render(&mut near_left, &mut near_right, BLOCK);
    output_only_mute.set_diagnostic_mode(DiagnosticMode::EventsOnly);
    let mute_before = output_only_mute.continuous_diagnostics();
    output_only_mute.render(&mut near_left, &mut near_right, BLOCK);
    let mute_after = output_only_mute.continuous_diagnostics();
    let muted_rms = rms(&near_left).max(rms(&near_right));

    let mut graph_excision = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    graph_excision.enable_v10_gf509(assets)?;
    graph_excision.set_diagnostic_mode(DiagnosticMode::V10Only);
    graph_excision.set_telemetry(&input, "asphalt");
    graph_excision.render(&mut near_left, &mut near_right, BLOCK);
    graph_excision.use_legacy_continuous_source();
    let excision_before = graph_excision.continuous_diagnostics();
    graph_excision.render(&mut near_left, &mut near_right, BLOCK);
    let excision_after = graph_excision.continuous_diagnostics();

    let mut processing_bypass = VehicleAudioEngine::new(bank).map_err(|e| e.to_string())?;
    processing_bypass.enable_v10_gf509(assets)?;
    processing_bypass.set_diagnostic_mode(DiagnosticMode::V10Only);
    processing_bypass.set_telemetry(&input, "asphalt");
    processing_bypass.render(&mut near_left, &mut near_right, BLOCK);
    processing_bypass.set_gf509_processing_bypass(true);
    let bypass_before = processing_bypass.continuous_diagnostics();
    processing_bypass.render(&mut near_left, &mut near_right, BLOCK);
    let bypass_after = processing_bypass.continuous_diagnostics();
    processing_bypass.set_gf509_processing_bypass(false);
    let resume_before = processing_bypass.continuous_diagnostics();
    processing_bypass.render(&mut near_left, &mut near_right, BLOCK);
    let resume_after = processing_bypass.continuous_diagnostics();

    Ok(json!({
        "config_reload_valid": reload.is_valid(),
        "requested_master_saturation": requested_saturation,
        "constructed_saturation_before_reload": saturation_before,
        "constructed_saturation_after_reload": saturation_after_reload,
        "effective_limiter_threshold_after_reload": configured.config().limiter_threshold,
        "continuous_source_near": format!("{:?}", near.continuous_source()),
        "continuous_source_far": format!("{:?}", far.continuous_source()),
        "near_lod": format!("{:?}", near.synth_lod()),
        "far_lod": format!("{:?}", far.synth_lod()),
        "settled_near_rms": rms(&near_left),
        "settled_far_rms": rms(&far_left),
        "settled_near_far_max_abs_diff": near_far_diff,
        "stage_diagnostics": {
            "enabled": near.stage_diagnostics_enabled(),
            "steady": {
                "master_input_peak": near_stage.master_input_peak,
                "master_input_rms": near_stage.master_input_rms,
                "master_color_peak": near_stage.master_color_peak,
                "master_color_rms": near_stage.master_color_rms,
                "final_output_peak": near_stage.final_output_peak,
                "final_output_rms": near_stage.final_output_rms,
                "coloration_delta_peak": near_stage.coloration_delta_peak,
                "linked_limiter_gain_reduction_db": near_stage.linked_limiter_gain_reduction_db,
                "linked_limiter_active_samples": near_stage.linked_limiter_active_samples,
                "effective_saturation": near_stage.effective_saturation,
                "effective_limiter_threshold": near_stage.effective_limiter_threshold,
            },
            "overload": {
                "config_reload_valid": overload_config.is_valid(),
                "master_input_peak": overload_stage.master_input_peak,
                "master_color_peak": overload_stage.master_color_peak,
                "final_output_peak": overload_stage.final_output_peak,
                "coloration_delta_peak": overload_stage.coloration_delta_peak,
                "linked_limiter_gain_reduction_db": overload_stage.linked_limiter_gain_reduction_db,
                "linked_limiter_active_samples": overload_stage.linked_limiter_active_samples,
                "effective_saturation": overload_stage.effective_saturation,
                "effective_limiter_threshold": overload_stage.effective_limiter_threshold,
            },
            "contract": "master input -> zero-transparent coloration -> linked safety limiter -> final output",
        },
        "gf509_render_failed_near": near.gf509_render_failed(),
        "gf509_render_failed_far": far.gf509_render_failed(),
        "experiment_modes": {
            "output_only_mute": {
                "mode": "EventsOnly",
                "gf509_render_calls_delta": mute_after.gf509_render_calls - mute_before.gf509_render_calls,
                "gf509_bypass_blocks_delta": mute_after.gf509_bypass_blocks - mute_before.gf509_bypass_blocks,
                "output_rms": muted_rms,
                "contract": "output is muted at the V10 mix point while GF509 processing continues"
            },
            "graph_excision": {
                "source_after_switch": format!("{:?}", graph_excision.continuous_source()),
                "gf509_render_calls_delta": excision_after.gf509_render_calls - excision_before.gf509_render_calls,
                "contract": "continuous GF509 consumer is removed by selecting the legacy source"
            },
            "processing_bypass": {
                "bypass_enabled_during_probe": true,
                "bypass_cleared_for_resume": !processing_bypass.gf509_processing_bypassed(),
                "gf509_render_calls_delta": bypass_after.gf509_render_calls - bypass_before.gf509_render_calls,
                "gf509_bypass_blocks_delta": bypass_after.gf509_bypass_blocks - bypass_before.gf509_bypass_blocks,
                "resume_gf509_render_calls_delta": resume_after.gf509_render_calls - resume_before.gf509_render_calls,
                "contract": "GF509 state is frozen and its buffer is silenced while bypassed; clearing resumes from that state"
            }
        },
        "interpretation": "GF509 remains active at both distances in this baseline; identical settled output is evidence that LOD labels do not reduce GF509 work.",
    }))
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        return Err("usage: v10_provenance_probe <bank-dir> <gf509-assets> <output.json>".into());
    }
    let bank = PathBuf::from(&args[1]);
    let assets = PathBuf::from(&args[2]);
    let output = PathBuf::from(&args[3]);
    let mixer_config = Path::new("game/sounds/sound_mixer_config.json");
    let payload = json!({
        "schema": "v10-001-003-probes-v2",
        "git_head": std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()),
        "runtime_source": format!("{:?}", ContinuousSourceKind::V10Gf509),
        "runtime_pair": runtime_pair_probe()?,
        "mixer": mixer_probe(&bank, &assets, mixer_config)?,
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_string_pretty(&payload).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    Ok(())
}
