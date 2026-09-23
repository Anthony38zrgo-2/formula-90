use std::fs;
use std::path::PathBuf;
use vehicle_audio_engine::grand_prix_sample_bank::GrandPrixSampleBank;
use vehicle_audio_engine::grand_prix_sampler::{GrandPrixSampler, GrandPrixTelemetry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let bank_directory = PathBuf::from(arguments.get(1).ok_or("Expected bank directory")?);
    let output_path = PathBuf::from(arguments.get(2).ok_or("Expected output WAV path")?);
    let sample_rate: u32 = 44_100;
    let output_frame_count = sample_rate * 30;
    let mut sampler = GrandPrixSampler::new(GrandPrixSampleBank::load(&bank_directory)?, sample_rate, None, None)?;
    let mut output_samples = Vec::with_capacity(output_frame_count as usize);
    let mut peak_amplitude = 0.0_f32;
    for frame_index in 0..output_frame_count {
        if frame_index % 147 == 0 {
            let elapsed_seconds = frame_index as f64 / sample_rate as f64;
            let sweep_section_index = (elapsed_seconds / 6.0).floor() as usize;
            let sweep_progress_fraction = (elapsed_seconds % 6.0) / 6.0;
            let revolutions_per_minute = match sweep_section_index {
                0 | 2 => 4_500.0 + 13_500.0 * sweep_progress_fraction,
                1 | 3 => 18_000.0 - 13_500.0 * sweep_progress_fraction,
                _ => 12_000.0,
            };
            let throttle = match sweep_section_index {
                0 | 1 => 1.0,
                2 | 3 => 0.0,
                _ => (0.5 - 0.5 * (std::f64::consts::TAU * sweep_progress_fraction * 3.0).cos()) as f32,
            };
            sampler.ingest(&GrandPrixTelemetry {
                rpm: revolutions_per_minute,
                throttle,
                gear: 3,
                shift_phase: 0,
                rev_limiter_active: false,
                dt_seconds: 147.0 / sample_rate as f64,
            });
        }
        let sample_value = sampler.render_sample_components().engine;
        peak_amplitude = peak_amplitude.max(sample_value.abs());
        output_samples.push((sample_value.clamp(-1.0, 1.0) * 32767.0).round() as i16);
    }
    if peak_amplitude >= 1.0 {
        return Err(format!("Engine preview exceeds full scale: {peak_amplitude}").into());
    }
    let data_bytes = output_frame_count * 2;
    let mut output = Vec::with_capacity(data_bytes as usize + 44);
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    output.extend_from_slice(b"WAVEfmt ");
    output.extend_from_slice(&16_u32.to_le_bytes());
    output.extend_from_slice(&1_u16.to_le_bytes());
    output.extend_from_slice(&1_u16.to_le_bytes());
    output.extend_from_slice(&sample_rate.to_le_bytes());
    output.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    output.extend_from_slice(&2_u16.to_le_bytes());
    output.extend_from_slice(&16_u16.to_le_bytes());
    output.extend_from_slice(b"data");
    output.extend_from_slice(&data_bytes.to_le_bytes());
    for sample_value in output_samples {
        output.extend_from_slice(&sample_value.to_le_bytes());
    }
    if let Some(directory) = output_path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&output_path, output)?;
    println!("{}: 30 seconds, peak={peak_amplitude:.6}, rate_clamps={}", output_path.display(), sampler.diagnostics().clamped_rate_samples);
    Ok(())
}
