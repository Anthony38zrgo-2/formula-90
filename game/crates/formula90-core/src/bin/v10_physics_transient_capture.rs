use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use game_sim::DriverInput;
use formula90_core::{CoreConfig, CoreFacade};
use serde_json::json;

const HZ: f64 = 120.0;
const DT: f64 = 1.0 / HZ;
const AUDIO_BLOCK: usize = 256;
const STEPS: usize = 960;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").join(relative)
}

fn scripted_input(step: usize, last_gear: i32) -> DriverInput {
    let mut input = if step < 240 {
        DriverInput { throttle: 0.78, ..Default::default() }
    } else if step < 330 {
        DriverInput { throttle: 0.0, ..Default::default() }
    } else if step < 420 {
        DriverInput { throttle: 0.92, ..Default::default() }
    } else if step < 510 {
        DriverInput { throttle: 0.58, ..Default::default() }
    } else if step < 600 {
        DriverInput { throttle: 0.18, ..Default::default() }
    } else if step < 720 {
        DriverInput { throttle: 1.0, ..Default::default() }
    } else {
        DriverInput { throttle: 0.35, ..Default::default() }
    };
    // These are real physics requests. The facade's audio adapter derives the
    // cut/recovery phase from the resulting powertrain shift timer.
    if step == 420 && last_gear > 0 {
        input.gear_request = Some((last_gear + 1).min(8) as i8);
        input.clutch = 1.0;
    } else if (420..=430).contains(&step) {
        input.clutch = 1.0;
    }
    if step == 600 && last_gear > 1 {
        input.gear_request = Some((last_gear - 1).max(1) as i8);
        input.clutch = 1.0;
    } else if (600..=610).contains(&step) {
        input.clutch = 1.0;
    }
    // Toggle the authoritative aid once; whether it produces a physical cut is
    // recorded from the frame rather than assumed by this harness.
    if step == 720 {
        input.toggle_traction_control = true;
    }
    input
}

fn main() -> Result<(), String> {
    let output_dir = repo_path("reports/audio-v10/v10-007-011");
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let csv_path = output_dir.join("physics-transient-capture.csv");
    let json_path = output_dir.join("physics-transient-summary.json");

    let mut core = CoreFacade::new(CoreConfig {
        bank_dir: Some(repo_path("game/sounds/banks/v10_vehicle")),
        config_json_path: Some(repo_path("game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json")),
        use_canonical: false,
        enable_audio: true,
        fixed_dt: DT,
        ..CoreConfig::default()
    })
    .map_err(|e| e.to_string())?;
    let id = core.ensure_spawned().map_err(|e| e.to_string())?;
    if !core.audio_healthy() {
        return Err("physics transient capture requires a healthy mixer".into());
    }

    let mut csv = File::create(&csv_path).map_err(|e| e.to_string())?;
    writeln!(
        csv,
        "step,time_s,rpm,throttle,gear,drive_torque,tc_enabled,tc_active,tc_raw_cut_ratio,tc_cut_ratio,shift_phase,clutch_engagement,load,torque,rpm_derivative,throttle_derivative,limiter_active,audio_trigger,audio_peak,audio_rms"
    )
    .map_err(|e| e.to_string())?;

    let mut last_gear = 1;
    let mut peak_audio = 0.0f32;
    let mut sum_audio_sq = 0.0f64;
    let mut audio_samples = 0usize;
    let mut shift_cut_steps = 0usize;
    let mut shift_recovery_steps = 0usize;
    let mut tc_active_steps = 0usize;
    let mut limiter_active_steps = 0usize;
    let mut first_shift_cut_s = None;
    let mut first_shift_recovery_s = None;
    let mut first_tc_active_s = None;
    let mut first_limiter_active_s = None;
    let mut max_audio_step = 0.0f32;
    let mut previous_audio = 0.0f32;

    for step in 0..STEPS {
        let input = scripted_input(step, last_gear);
        let samples = core.flat_samples(id);
        let frame = core.step_standalone(id, &input, &samples, DT).clone();
        last_gear = frame.gear;
        let mut left = vec![0.0f32; AUDIO_BLOCK];
        let mut right = vec![0.0f32; AUDIO_BLOCK];
        let rendered = core.audio_render(&mut left, &mut right, AUDIO_BLOCK);
        let audio_peak = left.iter().chain(right.iter()).copied().map(f32::abs).fold(0.0, f32::max);
        let audio_rms = if rendered > 0 {
            (left.iter().map(|v| *v as f64 * *v as f64).sum::<f64>() / rendered as f64).sqrt() as f32
        } else {
            0.0
        };
        peak_audio = peak_audio.max(audio_peak);
        sum_audio_sq += left.iter().map(|v| *v as f64 * *v as f64).sum::<f64>();
        audio_samples += rendered;
        max_audio_step = max_audio_step.max((audio_peak - previous_audio).abs());
        previous_audio = audio_peak;

        let phase = frame.audio.shift_phase;
        if phase == 1 || phase == 3 {
            shift_cut_steps += 1;
            first_shift_cut_s.get_or_insert(step as f64 * DT);
        }
        if phase == 2 || phase == 5 {
            shift_recovery_steps += 1;
            first_shift_recovery_s.get_or_insert(step as f64 * DT);
        }
        if frame.tc_active || frame.tc_cut_ratio > 1.0e-4 {
            tc_active_steps += 1;
            first_tc_active_s.get_or_insert(step as f64 * DT);
        }
        if frame.audio.rev_limiter_active {
            limiter_active_steps += 1;
            first_limiter_active_s.get_or_insert(step as f64 * DT);
        }
        writeln!(
            csv,
            "{step},{:.6},{:.3},{:.5},{},{:.5},{},{},{:.6},{:.6},{},{:.6},{:.6},{:.6},{:.5},{:.5},{},{},{:.7},{:.7}",
            step as f64 * DT,
            frame.rpm,
            frame.throttle,
            frame.gear,
            frame.drive_torque,
            frame.tc_enabled,
            frame.tc_active,
            frame.tc_raw_cut_ratio,
            frame.tc_cut_ratio,
            phase,
            frame.audio.clutch_engagement,
            frame.audio.normalized_engine_load,
            frame.audio.normalized_engine_torque,
            frame.audio.rpm_derivative,
            frame.audio.throttle_derivative,
            frame.audio.rev_limiter_active,
            frame.audio.trigger_code,
            audio_peak,
            audio_rms,
        )
        .map_err(|e| e.to_string())?;
    }

    let summary = json!({
        "schema_version": "v10-010-physics-transient-capture-v1",
        "source": "CoreFacade::step_standalone -> CoreFrame -> AudioTelemetryAdapter -> VehicleAudioEngine::render",
        "vehicle_config": "game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json",
        "fixed_hz": HZ,
        "steps": STEPS,
        "duration_s": STEPS as f64 * DT,
        "audio_block_frames": AUDIO_BLOCK,
        "scenarios": [
            {"name": "throttle_hold", "steps": [0, 239]},
            {"name": "lift", "steps": [240, 329]},
            {"name": "reapply", "steps": [330, 419]},
            {"name": "upshift_request_and_clutch_open", "steps": [420, 510]},
            {"name": "downshift_request_and_clutch_open", "steps": [600, 610]},
            {"name": "full_load_before_tc_toggle", "steps": [611, 719]},
            {"name": "traction_control_toggle_and_recovery", "steps": [720, 959]}
        ],
        "observed": {
            "shift_cut_steps": shift_cut_steps,
            "shift_recovery_steps": shift_recovery_steps,
            "tc_active_steps": tc_active_steps,
            "limiter_active_steps": limiter_active_steps,
            "first_shift_cut_s": first_shift_cut_s,
            "first_shift_recovery_s": first_shift_recovery_s,
            "first_tc_active_s": first_tc_active_s,
            "first_limiter_active_s": first_limiter_active_s,
            "audio_peak": peak_audio,
            "audio_rms": (sum_audio_sq / audio_samples.max(1) as f64).sqrt(),
            "max_audio_block_peak_step": max_audio_step,
        },
        "interpretation": {
            "physical_trace": true,
            "synthetic_forced_flags": false,
            "missing_conditions_are_not_invented": true,
            "human_listening_gate": "PENDING_HUMAN_LISTENING"
        },
        "artifacts": {
            "csv": csv_path.to_string_lossy(),
            "summary": json_path.to_string_lossy()
        }
    });
    fs::write(&json_path, serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    println!("{}", csv_path.display());
    println!("{}", json_path.display());
    println!("shift_cut_steps={shift_cut_steps}; shift_recovery_steps={shift_recovery_steps}; tc_active_steps={tc_active_steps}; limiter_active_steps={limiter_active_steps}");
    Ok(())
}
