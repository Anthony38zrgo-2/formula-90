use crate::dsp::biquad::Biquad;

#[derive(Debug)]
pub struct SampledTransmissionWhine {
    pub reference_mesh_frequency_hertz: f64,
    pub calibrated_gain: f32,
    pub minimum_playback_rate: f64,
    pub maximum_playback_rate: f64,
    filtered_recordings: Vec<Vec<f32>>,
}

impl SampledTransmissionWhine {
    pub(crate) fn new(
        samples: Vec<i16>,
        reference_mesh_frequency_hertz: f64,
        calibrated_gain: f32,
        minimum_playback_rate: f64,
        maximum_playback_rate: f64,
    ) -> Self {
        let recording: Vec<f32> = samples
            .iter()
            .map(|sample| *sample as f32 / 32768.0)
            .collect();
        let level_count = (maximum_playback_rate.max(1.0).log2() * 4.0).ceil() as usize + 2;
        let filtered_recordings = (0..level_count)
            .map(|level| {
                let cutoff_hertz = 17_640.0 / 2.0_f32.powf(level as f32 / 4.0);
                let mut filtered = recording.clone();
                let mut filters = [Biquad::lowpass(44_100.0, cutoff_hertz); 4];
                for _ in 0..2 {
                    for sample in &recording {
                        let mut value = *sample;
                        for filter in &mut filters {
                            value = filter.process(value);
                        }
                    }
                }
                for sample in &mut filtered {
                    for filter in &mut filters {
                        *sample = filter.process(*sample);
                    }
                }
                let forward_recording = filtered.clone();
                let mut filters = [Biquad::lowpass(44_100.0, cutoff_hertz); 4];
                for _ in 0..2 {
                    for sample in forward_recording.iter().rev() {
                        let mut value = *sample;
                        for filter in &mut filters {
                            value = filter.process(value);
                        }
                    }
                }
                for sample in filtered.iter_mut().rev() {
                    for filter in &mut filters {
                        *sample = filter.process(*sample);
                    }
                }
                filtered
            })
            .collect();
        Self {
            reference_mesh_frequency_hertz,
            calibrated_gain,
            minimum_playback_rate,
            maximum_playback_rate,
            filtered_recordings,
        }
    }

    pub fn render(
        &self,
        cursor: &mut f64,
        mesh_frequency_hertz: f32,
        output_sample_rate: u32,
    ) -> f32 {
        if !mesh_frequency_hertz.is_finite() || mesh_frequency_hertz <= 0.0 {
            return 0.0;
        }
        let requested_rate = mesh_frequency_hertz as f64 / self.reference_mesh_frequency_hertz;
        let playback_rate =
            requested_rate.clamp(self.minimum_playback_rate, self.maximum_playback_rate);
        let source_step = playback_rate * 44_100.0 / output_sample_rate as f64;
        let filtering_level = source_step.max(1.0).log2() * 4.0;
        let lower_level = filtering_level.floor() as usize;
        let lower_level = lower_level.min(self.filtered_recordings.len() - 2);
        let level_fraction = (filtering_level - lower_level as f64).clamp(0.0, 1.0) as f32;
        let lower_sample =
            interpolate_periodic_recording(&self.filtered_recordings[lower_level], *cursor);
        let upper_sample =
            interpolate_periodic_recording(&self.filtered_recordings[lower_level + 1], *cursor);
        let length = self.filtered_recordings[0].len() as f64;
        *cursor = (*cursor + source_step).rem_euclid(length);
        let low_speed_gain = (requested_rate / self.minimum_playback_rate).clamp(0.0, 1.0) as f32;
        (lower_sample + (upper_sample - lower_sample) * level_fraction)
            * self.calibrated_gain
            * low_speed_gain
    }
}

fn interpolate_periodic_recording(recording: &[f32], position: f64) -> f32 {
    let length = recording.len();
    let position = position.rem_euclid(length as f64);
    let index = position.floor() as usize;
    let fraction = (position - index as f64) as f32;
    let previous = recording[(index + length - 1) % length];
    let current = recording[index];
    let next = recording[(index + 1) % length];
    let following = recording[(index + 2) % length];
    current
        + 0.5
            * fraction
            * (next - previous
                + fraction
                    * (2.0 * previous - 5.0 * current + 4.0 * next - following
                        + fraction * (3.0 * (current - next) + following - previous)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerated_playback_filters_source_energy_before_resampling() {
        let samples = (0..4410)
            .map(|frame| {
                (16_000.0 * (std::f64::consts::TAU * 12_000.0 * frame as f64 / 44_100.0).sin())
                    as i16
            })
            .collect();
        let recording = SampledTransmissionWhine::new(samples, 2000.0, 1.0, 0.1, 4.0);
        let mut cursor = 0.0;
        let mut energy = 0.0;
        for _ in 0..4410 {
            let sample = recording.render(&mut cursor, 8000.0, 44_100);
            energy += sample * sample;
        }
        assert!((energy / 4410.0_f32).sqrt() < 0.001);
    }

    #[test]
    fn stopped_shaft_is_silent_and_does_not_advance_the_recording() {
        let recording = SampledTransmissionWhine::new(vec![1000; 64], 2000.0, 1.0, 0.1, 4.0);
        let mut cursor = 12.0;
        assert_eq!(recording.render(&mut cursor, 0.0, 44_100), 0.0);
        assert_eq!(cursor, 12.0);
    }
}
