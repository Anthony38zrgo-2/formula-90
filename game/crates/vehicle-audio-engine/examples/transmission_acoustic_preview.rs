use std::fs;
use std::path::PathBuf;
use vehicle_audio_engine::grand_prix_sample_bank::GrandPrixSampleBank;
use vehicle_audio_engine::grand_prix_sampler::{GrandPrixSampler, GrandPrixTelemetry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output_directory = workspace.join("../scratch/audio/transmission_acoustics");
    let bank = GrandPrixSampleBank::load(&workspace.join("sounds/banks/v10-v2-bank"))?;
    let mut sampler = GrandPrixSampler::new(bank, 44_100, Some(4500.0), Some(18_000.0))?;
    sampler.set_group_gains(1.0, 0.561329504, 1.0, 1.0);
    sampler.set_gearbox_whine_gain(0.137826425);
    sampler.set_gearbox_whine_transmission(
        &[3.45, 2.85, 2.40, 2.05, 1.80, 1.64, 1.50],
        4.25,
        3.0,
        20.0,
        40.0,
    );
    let mut samples = Vec::new();
    let mut measurements = String::from("time_seconds,physical_engine_speed,acoustic_engine_speed,pitch_modulation,gear_mesh_frequency_hertz,transmitted_load_gain\n");
    let mut peak = 0.0_f32;
    for tick in 0..600 {
        let time_seconds = tick as f64 * 0.01;
        let shifting = (2.0..2.06).contains(&time_seconds);
        let gear = if time_seconds < 2.06 { 3 } else { 4 };
        let engine_speed = if time_seconds < 2.0 {
            16_000.0 + time_seconds * 1000.0
        } else if shifting {
            18_000.0 - (time_seconds - 2.0) * 8000.0
        } else {
            (15_375.0 + (time_seconds - 2.06) * 500.0).min(17_000.0)
        };
        let load = if shifting {
            0.0
        } else if time_seconds >= 4.5 {
            0.35
        } else {
            1.0
        };
        let output_shaft_speed = if time_seconds < 2.0 {
            engine_speed / 60.0 / 2.40
        } else if shifting {
            125.0
        } else {
            engine_speed / 60.0 / 2.05
        };
        sampler.set_output_shaft_speed_hertz(Some(output_shaft_speed));
        sampler.set_transmitted_torque_sign(Some(if time_seconds >= 4.5 { -1 } else { 1 }));
        sampler.ingest(&GrandPrixTelemetry {
            rpm: engine_speed,
            throttle: if time_seconds >= 4.5 { 0.0 } else { 1.0 },
            gear,
            normalized_transmitted_load: load,
            transmitted_torque_sign: 1,
            shift_phase: if shifting { 1 } else { 0 },
            dt_seconds: 0.01,
            ..GrandPrixTelemetry::default()
        });
        for _ in 0..441 {
            let value = sampler.render_sample_components().sum() * 0.65;
            peak = peak.max(value.abs());
            samples.push((value.clamp(-1.0, 1.0) * 32767.0).round() as i16);
        }
        let diagnostics = sampler.diagnostics();
        measurements.push_str(&format!(
            "{time_seconds},{engine_speed},{},{},{},{}\n",
            diagnostics.rendered_revolutions_per_minute,
            diagnostics.upshift_acoustic_pitch_modulation,
            diagnostics.gear_mesh_frequency_hertz,
            diagnostics.transmitted_load_gain
        ));
    }
    if peak >= 1.0 {
        return Err(format!("Preview clipping: {peak}").into());
    }
    let data_bytes = samples.len() as u32 * 2;
    let mut recording = Vec::new();
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
        recording.extend_from_slice(&sample.to_le_bytes());
    }
    fs::create_dir_all(&output_directory)?;
    fs::write(
        output_directory.join("transmission_acoustic_preview.wav"),
        recording,
    )?;
    fs::write(
        output_directory.join("transmission_acoustic_measurements.csv"),
        measurements,
    )?;
    println!("Controlled audio fixture: 6 seconds, peak={peak}, upshift at 2 seconds, coast at 4.5 seconds");
    Ok(())
}
