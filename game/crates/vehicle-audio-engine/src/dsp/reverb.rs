use crate::config::ReverbBusConfig;

#[derive(Debug, Clone)]
struct Delay {
    data: Vec<f32>,
    index: usize,
    feedback: f32,
    damping: f32,
    filtered: f32,
}
impl Delay {
    fn new(length: usize, feedback: f32, damping: f32) -> Self {
        Self {
            data: vec![0.0; length.max(1)],
            index: 0,
            feedback,
            damping,
            filtered: 0.0,
        }
    }
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.data[self.index];
        self.filtered += (1.0 - self.damping) * (delayed - self.filtered);
        self.data[self.index] = input + self.filtered * self.feedback;
        self.index = (self.index + 1) % self.data.len();
        delayed
    }
}

#[derive(Debug, Clone)]
pub struct StereoReverb {
    enabled: bool,
    left: Vec<Delay>,
    right: Vec<Delay>,
    wet: f32,
    width: f32,
}
impl StereoReverb {
    pub fn new(config: ReverbBusConfig, sample_rate: u32) -> Self {
        let scale = sample_rate as f32 / 44100.0;
        let lengths = [1557usize, 1617, 1491, 1422];
        let feedback = 10.0_f32.powf(-3.0 * 0.035 / config.decay_s.max(0.05));
        let left = lengths
            .iter()
            .map(|n| Delay::new((*n as f32 * scale) as usize, feedback, config.damping))
            .collect();
        let right = lengths
            .iter()
            .map(|n| {
                Delay::new(
                    ((*n + 23) as f32 * scale) as usize,
                    feedback,
                    config.damping,
                )
            })
            .collect();
        Self {
            enabled: config.enabled,
            left,
            right,
            wet: 10.0_f32.powf(config.wet_db / 20.0),
            width: config.stereo_width,
        }
    }
    #[inline]
    pub fn process(&mut self, input_left: f32, input_right: f32) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        let mut left = 0.0;
        let mut right = 0.0;
        for delay in &mut self.left {
            left += delay.process(input_left);
        }
        for delay in &mut self.right {
            right += delay.process(input_right);
        }
        left *= 0.25;
        right *= 0.25;
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5 * self.width;
        ((mid + side) * self.wet, (mid - side) * self.wet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_is_silent_and_tail_persists() {
        let mut off = StereoReverb::new(
            ReverbBusConfig {
                enabled: false,
                ..ReverbBusConfig::default()
            },
            44100,
        );
        assert_eq!(off.process(1.0, 1.0), (0.0, 0.0));
        let mut reverb = StereoReverb::new(ReverbBusConfig::default(), 44100);
        reverb.process(1.0, 0.0);
        let mut energy = 0.0;
        for _ in 0..2000 {
            let (l, r) = reverb.process(0.0, 0.0);
            energy += l.abs() + r.abs();
        }
        assert!(energy > 0.0);
    }
}
