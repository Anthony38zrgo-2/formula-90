use crate::config::{AdsrConfig, AdsrCurve};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

#[derive(Debug, Clone)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    attack: u32,
    decay: u32,
    release: u32,
    sustain: f32,
    curve: AdsrCurve,
    elapsed: u32,
    release_start: f32,
}

impl Adsr {
    pub fn new(config: AdsrConfig, sample_rate: u32) -> Self {
        let samples = |ms: f32| (ms.max(0.0) * sample_rate as f32 / 1000.0).round() as u32;
        Self {
            stage: Stage::Idle,
            level: 0.0,
            attack: samples(config.attack_ms),
            decay: samples(config.decay_ms),
            release: samples(config.release_ms),
            sustain: config.sustain.clamp(0.0, 1.0),
            curve: config.curve,
            elapsed: 0,
            release_start: 0.0,
        }
    }
    pub fn note_on(&mut self) {
        self.stage = Stage::Attack;
        self.level = 0.0;
        self.elapsed = 0;
    }
    pub fn note_off(&mut self) {
        if !matches!(self.stage, Stage::Idle | Stage::Done) {
            self.release_start = self.level;
            self.stage = Stage::Release;
            self.elapsed = 0;
        }
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        loop {
            match self.stage {
                Stage::Idle | Stage::Done => return 0.0,
                Stage::Attack if self.attack == 0 => {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                    self.elapsed = 0;
                }
                Stage::Decay if self.decay == 0 => {
                    self.level = self.sustain;
                    self.stage = Stage::Sustain;
                }
                Stage::Release if self.release == 0 => {
                    self.level = 0.0;
                    self.stage = Stage::Done;
                }
                Stage::Attack => {
                    self.elapsed += 1;
                    self.level = shape(self.elapsed as f32 / self.attack as f32, self.curve);
                    if self.elapsed >= self.attack {
                        self.stage = Stage::Decay;
                        self.elapsed = 0;
                        self.level = 1.0;
                    }
                    return self.level;
                }
                Stage::Decay => {
                    self.elapsed += 1;
                    let t = shape(self.elapsed as f32 / self.decay as f32, self.curve);
                    self.level = 1.0 + (self.sustain - 1.0) * t;
                    if self.elapsed >= self.decay {
                        self.stage = Stage::Sustain;
                        self.level = self.sustain;
                    }
                    return self.level;
                }
                Stage::Sustain => return self.level,
                Stage::Release => {
                    self.elapsed += 1;
                    let t = shape(self.elapsed as f32 / self.release as f32, self.curve);
                    self.level = self.release_start * (1.0 - t);
                    if self.elapsed >= self.release {
                        self.stage = Stage::Done;
                        self.level = 0.0;
                    }
                    return self.level;
                }
            }
        }
    }
}

fn shape(t: f32, curve: AdsrCurve) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match curve {
        AdsrCurve::Linear => t,
        AdsrCurve::Exponential => t * t,
        AdsrCurve::Logarithmic => t.sqrt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curves_are_finite_monotonic_and_release() {
        for curve in [
            AdsrCurve::Linear,
            AdsrCurve::Exponential,
            AdsrCurve::Logarithmic,
        ] {
            let mut env = Adsr::new(
                AdsrConfig {
                    enabled: true,
                    attack_ms: 10.0,
                    decay_ms: 0.0,
                    sustain: 1.0,
                    release_ms: 10.0,
                    curve,
                },
                1000,
            );
            env.note_on();
            let mut last = 0.0;
            for _ in 0..10 {
                let value = env.next_sample();
                assert!(value.is_finite() && value >= last);
                last = value;
            }
            env.note_off();
            last = 1.0;
            for _ in 0..10 {
                let value = env.next_sample();
                assert!(value.is_finite() && value <= last);
                last = value;
            }
            assert_eq!(env.stage(), Stage::Done);
        }
    }
    #[test]
    fn zero_times_are_safe() {
        let mut env = Adsr::new(
            AdsrConfig {
                enabled: true,
                attack_ms: 0.0,
                decay_ms: 0.0,
                sustain: 0.5,
                release_ms: 0.0,
                curve: AdsrCurve::Linear,
            },
            44100,
        );
        env.note_on();
        assert_eq!(env.next_sample(), 0.5);
        env.note_off();
        assert_eq!(env.next_sample(), 0.0);
    }
}
