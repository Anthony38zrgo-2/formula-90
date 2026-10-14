use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dsp::biquad::Biquad;
use crate::grand_prix_sample_bank::{GrandPrixBankError, GrandPrixDecodedLoop};
use crate::grand_prix_sampler::read_loop_sample;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordedEngineControlPoint {
    pub parameter_value: f64,
    pub amplitude: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordedEngineVoiceAsset {
    pub instrument_guid: String,
    pub filename: String,
    pub sha256: String,
    pub reference_revolutions_per_minute: f64,
    pub loop_start_frame: usize,
    pub loop_end_frame_exclusive: usize,
    pub revolutions_per_minute_curves: Vec<Vec<RecordedEngineControlPoint>>,
    pub throttle_curve: Vec<RecordedEngineControlPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordedEngineAutomationManifest {
    pub interpolation: String,
    pub voices: Vec<RecordedEngineVoiceAsset>,
}

struct RecordedEngineVoice {
    asset: RecordedEngineVoiceAsset,
    recording: GrandPrixDecodedLoop,
    cursor: f64,
    anti_alias_filter: Biquad,
    filter_cutoff_hertz: f32,
}

pub struct RecordedEngineAutomation {
    voices: Vec<RecordedEngineVoice>,
}

fn validate_curve(points: &[RecordedEngineControlPoint]) -> bool {
    points.len() >= 2
        && points.len() <= 256
        && points.iter().all(|point| {
            point.parameter_value.is_finite()
                && point.amplitude.is_finite()
                && (0.0..=1.0).contains(&point.amplitude)
        })
        && points
            .windows(2)
            .all(|pair| pair[0].parameter_value < pair[1].parameter_value)
}

pub fn control_amplitude(points: &[RecordedEngineControlPoint], parameter_value: f64) -> f64 {
    if parameter_value <= points[0].parameter_value {
        return points[0].amplitude;
    }
    for pair in points.windows(2) {
        if parameter_value <= pair[1].parameter_value {
            let position = (parameter_value - pair[0].parameter_value)
                / (pair[1].parameter_value - pair[0].parameter_value);
            return pair[0].amplitude + (pair[1].amplitude - pair[0].amplitude) * position;
        }
    }
    points[points.len() - 1].amplitude
}

impl RecordedEngineAutomation {
    pub fn load(
        directory: &Path,
        manifest: &RecordedEngineAutomationManifest,
    ) -> Result<Self, GrandPrixBankError> {
        if manifest.interpolation != "linear_reconstruction"
            || manifest.voices.is_empty()
            || manifest.voices.len() > 16
        {
            return Err(GrandPrixBankError::InvalidManifest(
                "recorded engine automation invalid".into(),
            ));
        }
        let mut voices = Vec::with_capacity(manifest.voices.len());
        for asset in &manifest.voices {
            if Path::new(&asset.filename).components().count() != 1
                || asset.filename.contains(['/', '\\', ':'])
                || !asset.filename.ends_with(".wav")
                || !asset.reference_revolutions_per_minute.is_finite()
                || asset.reference_revolutions_per_minute <= 0.0
                || asset.revolutions_per_minute_curves.is_empty()
                || asset.revolutions_per_minute_curves.len() > 2
                || !asset
                    .revolutions_per_minute_curves
                    .iter()
                    .all(|curve| validate_curve(curve))
                || !validate_curve(&asset.throttle_curve)
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    asset.instrument_guid.clone(),
                    "recorded engine voice invalid".into(),
                ));
            }
            let samples = crate::grand_prix_sample_bank::load_verified_pcm(
                directory,
                &asset.filename,
                &asset.sha256,
            )?;
            if asset.loop_start_frame >= asset.loop_end_frame_exclusive
                || asset.loop_end_frame_exclusive > samples.len()
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    asset.instrument_guid.clone(),
                    "recorded engine loop bounds invalid".into(),
                ));
            }
            voices.push(RecordedEngineVoice {
                asset: asset.clone(),
                recording: GrandPrixDecodedLoop {
                    id: asset.instrument_guid.clone(),
                    pcm: samples,
                    reference_revolutions_per_minute: asset.reference_revolutions_per_minute,
                    calibrated_gain: 1.0,
                    valid_playback_rate_min: 0.01,
                    valid_playback_rate_max: 8.0,
                    loop_start_frame: asset.loop_start_frame,
                    loop_end_frame_exclusive: asset.loop_end_frame_exclusive,
                    crossfade_frames: 1,
                    presence_curve: Vec::new(),
                },
                cursor: asset.loop_start_frame as f64,
                anti_alias_filter: Biquad::lowpass(44_100.0, 19_000.0),
                filter_cutoff_hertz: 0.0,
            });
        }
        Ok(Self { voices })
    }

    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.cursor = voice.asset.loop_start_frame as f64;
            voice.anti_alias_filter = Biquad::lowpass(44_100.0, 19_000.0);
            voice.filter_cutoff_hertz = 0.0;
        }
    }

    pub fn render(
        &mut self,
        revolutions_per_minute: f64,
        throttle: f64,
        output_sample_rate: u32,
    ) -> (f32, f32, u8) {
        let mut output = 0.0;
        let mut amplitude_sum = 0.0;
        let mut active_voices = 0;
        for voice in &mut self.voices {
            let mut amplitude = control_amplitude(&voice.asset.throttle_curve, throttle);
            for curve in &voice.asset.revolutions_per_minute_curves {
                amplitude *= control_amplitude(curve, revolutions_per_minute);
            }
            let sample = read_loop_sample(
                &voice.recording,
                &mut voice.cursor,
                &mut voice.anti_alias_filter,
                &mut voice.filter_cutoff_hertz,
                output_sample_rate,
                revolutions_per_minute / voice.asset.reference_revolutions_per_minute,
            );
            output += sample * amplitude as f32;
            amplitude_sum += amplitude as f32;
            if amplitude > 0.001 {
                active_voices += 1;
            }
        }
        (output, amplitude_sum, active_voices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_recorded_automation_does_not_allocate() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/v8-bank");
        let manifest: crate::grand_prix_sample_bank::GrandPrixManifest = serde_json::from_slice(
            &std::fs::read(directory.join("manifest.json")).expect("manifest"),
        )
        .expect("parse");
        let mut engine = RecordedEngineAutomation::load(
            &directory,
            manifest
                .recorded_engine_automation
                .as_ref()
                .expect("automation"),
        )
        .expect("load");
        crate::allocation_probe::start();
        for frame in 0..44100 {
            engine.render(4500.0 + frame as f64 * 13500.0 / 44100.0, 0.75, 44100);
        }
        let allocation_counts = crate::allocation_probe::stop();
        assert_eq!(allocation_counts, (0, 0));
    }

    #[test]
    fn invalid_control_points_and_external_paths_are_rejected() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/v8-bank");
        let manifest: crate::grand_prix_sample_bank::GrandPrixManifest = serde_json::from_slice(
            &std::fs::read(directory.join("manifest.json")).expect("manifest"),
        )
        .expect("parse");
        let original = manifest.recorded_engine_automation.expect("automation");
        for kind in 0..4 {
            let mut invalid = original.clone();
            match kind {
                0 => invalid.voices[0].filename = "../outside.wav".into(),
                1 => invalid.voices[0].revolutions_per_minute_curves[0][0].amplitude = f64::NAN,
                2 => invalid.voices[0].throttle_curve.reverse(),
                _ => invalid.voices[0].loop_end_frame_exclusive = usize::MAX,
            }
            assert!(RecordedEngineAutomation::load(&directory, &invalid).is_err());
        }
    }
}
