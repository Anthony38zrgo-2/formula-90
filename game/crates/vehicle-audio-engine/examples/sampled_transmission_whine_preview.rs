use std::fs;
use std::path::Path;
use std::time::Instant;
use vehicle_audio_engine::{GrandPrixSampleBank, GrandPrixSampler, GrandPrixTelemetry};

fn write_recording(path: &Path, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if !peak.is_finite() || peak >= 1.0 {
        return Err(format!("Recording clips: {} peak={peak}", path.display()).into());
    }
    let data_bytes = samples.len() as u32 * 2;
    let mut recording = Vec::with_capacity(44 + data_bytes as usize);
    recording.extend_from_slice(b"RIFF");
    recording.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    recording.extend_from_slice(b"WAVEfmt ");
    recording.extend_from_slice(&16_u32.to_le_bytes());
    recording.extend_from_slice(&1_u16.to_le_bytes());
    recording.extend_from_slice(&1_u16.to_le_bytes());
    recording.extend_from_slice(&44_100_u32.to_le_bytes());
    recording.extend_from_slice(&88_200_u32.to_le_bytes());
    recording.extend_from_slice(&2_u16.to_le_bytes());
    recording.extend_from_slice(&16_u16.to_le_bytes());
    recording.extend_from_slice(b"data");
    recording.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        recording.extend_from_slice(&((sample * 32767.0).round() as i16).to_le_bytes());
    }
    fs::write(path, recording)?;
    println!("{} peak={peak}", path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let bank_directory = repository.join("game/sounds/banks/v10-v2-bank");
    let output_directory = repository.join("scratch/audio/sampled_transmission_whine");
    let sampled_bank = GrandPrixSampleBank::load(&bank_directory)?;
    if sampled_bank.transmission_whine.is_none() {
        return Err("Preview requires the sampled transmission recording".into());
    }
    let mut synthetic_bank = GrandPrixSampleBank::load(&bank_directory)?;
    synthetic_bank.transmission_whine = None;
    let mut sampled_sampler =
        GrandPrixSampler::new(sampled_bank, 44_100, Some(4500.0), Some(18_000.0))?;
    let mut synthetic_sampler =
        GrandPrixSampler::new(synthetic_bank, 44_100, Some(4500.0), Some(18_000.0))?;
    let gear_ratios = [3.45, 2.85, 2.40, 2.05, 1.80, 1.64, 1.50];
    for sampler in [&mut sampled_sampler, &mut synthetic_sampler] {
        sampler.set_group_gains(1.0, 0.561329504, 1.0, 0.3);
        sampler.set_gearbox_whine_gain(0.137826425);
        sampler.set_gearbox_whine_transmission(&gear_ratios, 4.25, 3.0, 20.0, 40.0);
    }
    let mut sampled_whine = Vec::with_capacity(882_000);
    let mut synthetic_whine = Vec::with_capacity(882_000);
    let mut sampled_mix = Vec::with_capacity(882_000);
    let mut synthetic_mix = Vec::with_capacity(882_000);
    let mut measurements = String::from("time_seconds,scenario,gear,engine_revolutions_per_minute,output_shaft_speed_hertz,mesh_frequency_hertz,transmitted_load_gain,sampled_root_mean_square,synthetic_root_mean_square\n");
    let mut sampled_render_seconds = 0.0;
    let mut synthetic_render_seconds = 0.0;
    for tick in 0..2000 {
        let time_seconds = tick as f64 * 0.01;
        let (scenario, gear, output_shaft_speed, load, torque_sign, shift_phase) =
            if time_seconds < 6.0 {
                let gear = 1 + (time_seconds / 1.5) as i32;
                let shift_phase = if gear > 1 && time_seconds % 1.5 < 0.05 {
                    1
                } else {
                    0
                };
                (
                    "acceleration_upshifts",
                    gear,
                    30.0 + time_seconds * 18.0,
                    if shift_phase == 1 { 0.0 } else { 1.0 },
                    1,
                    shift_phase,
                )
            } else if time_seconds < 9.0 {
                ("coast", 4, 138.0 - (time_seconds - 6.0) * 12.0, 0.35, -1, 0)
            } else if time_seconds < 12.0 {
                let gear = 3 - ((time_seconds - 9.0) / 1.5) as i32;
                let shift_phase = if (time_seconds - 9.0) % 1.5 < 0.05 {
                    3
                } else {
                    0
                };
                (
                    "downshifts",
                    gear,
                    102.0 - (time_seconds - 9.0) * 20.0,
                    if shift_phase == 3 { 0.0 } else { 0.35 },
                    -1,
                    shift_phase,
                )
            } else if time_seconds < 14.0 {
                (
                    "moving_neutral",
                    0,
                    42.0 - (time_seconds - 12.0) * 21.0,
                    0.0,
                    0,
                    0,
                )
            } else if time_seconds < 16.0 {
                ("stationary_neutral", 0, 0.0, 0.0, 0, 0)
            } else if time_seconds < 18.0 {
                (
                    "reverse",
                    -1,
                    20.0 + (time_seconds - 16.0) * 10.0,
                    0.6,
                    1,
                    0,
                )
            } else {
                ("stopped", 0, 0.0, 0.0, 0, 0)
            };
        let ratio = if gear > 0 {
            gear_ratios[gear as usize - 1]
        } else {
            3.0
        };
        let engine_speed = if time_seconds >= 18.0 {
            0.0
        } else if gear == 0 {
            4500.0
        } else {
            (output_shaft_speed * 60.0 * ratio as f64).clamp(4500.0, 18_000.0)
        };
        let telemetry = GrandPrixTelemetry {
            rpm: engine_speed,
            throttle: if torque_sign > 0 { load } else { 0.0 },
            normalized_transmitted_load: load,
            transmitted_torque_sign: torque_sign,
            gear,
            shift_phase,
            dt_seconds: 0.01,
            ..GrandPrixTelemetry::default()
        };
        for sampler in [&mut sampled_sampler, &mut synthetic_sampler] {
            sampler.set_output_shaft_speed_hertz(Some(output_shaft_speed));
            sampler.set_transmitted_torque_sign(Some(torque_sign));
            sampler.ingest(&telemetry);
        }
        let mut sampled_energy = 0.0;
        let mut synthetic_energy = 0.0;
        let started = Instant::now();
        for _ in 0..441 {
            let output = sampled_sampler.render_sample_components();
            sampled_energy += output.gearbox_whine * output.gearbox_whine;
            sampled_whine.push(output.gearbox_whine);
            sampled_mix.push(output.sum() * 0.65);
        }
        sampled_render_seconds += started.elapsed().as_secs_f64();
        let started = Instant::now();
        for _ in 0..441 {
            let output = synthetic_sampler.render_sample_components();
            synthetic_energy += output.gearbox_whine * output.gearbox_whine;
            synthetic_whine.push(output.gearbox_whine);
            synthetic_mix.push(output.sum() * 0.65);
        }
        synthetic_render_seconds += started.elapsed().as_secs_f64();
        let diagnostics = sampled_sampler.diagnostics();
        measurements.push_str(&format!(
            "{time_seconds},{scenario},{gear},{engine_speed},{output_shaft_speed},{},{},{},{}\n",
            diagnostics.gear_mesh_frequency_hertz,
            diagnostics.transmitted_load_gain,
            (sampled_energy / 441.0_f32).sqrt(),
            (synthetic_energy / 441.0_f32).sqrt()
        ));
    }
    fs::create_dir_all(&output_directory)?;
    write_recording(
        &output_directory.join("sampled_transmission_whine.wav"),
        &sampled_whine,
    )?;
    write_recording(
        &output_directory.join("previous_synthetic_whine.wav"),
        &synthetic_whine,
    )?;
    write_recording(
        &output_directory.join("sampled_transmission_mix.wav"),
        &sampled_mix,
    )?;
    write_recording(
        &output_directory.join("previous_synthetic_mix.wav"),
        &synthetic_mix,
    )?;
    fs::write(
        output_directory.join("transmission_whine_measurements.csv"),
        measurements,
    )?;
    fs::write(output_directory.join("render_performance.json"), format!("{{\"audio_seconds\":20,\"sampled_render_seconds\":{sampled_render_seconds},\"synthetic_render_seconds\":{synthetic_render_seconds}}}\n"))?;
    println!("Twenty second fixture: sampled render={sampled_render_seconds}s, synthetic render={synthetic_render_seconds}s");
    Ok(())
}
