use crate::config::TubeColorConfig;

#[derive(Debug, Clone, Copy)]
pub struct Tube {
    config: TubeColorConfig,
    drive: f32,
    dc: f32,
}

impl Tube {
    pub fn new(config: TubeColorConfig, positive_boost_db: f32) -> Self {
        let drive = (config.amount + config.boost_link * (positive_boost_db.max(0.0) / 180.0))
            .clamp(0.0, 1.0);
        Self {
            config,
            drive,
            dc: 0.0,
        }
    }
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        if !self.config.enabled || self.config.mix == 0.0 || self.drive == 0.0 {
            return input;
        }
        let gain = 1.0 + self.drive * 8.0;
        let shaped = ((input + self.config.bias) * gain).tanh();
        self.dc += 0.001 * (shaped - self.dc);
        let wet = shaped - self.dc;
        let wet = if self.config.auto_gain {
            wet / gain.tanh().max(0.1)
        } else {
            wet
        };
        input + (wet - input) * self.config.mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bypass_contracts_are_transparent() {
        for config in [
            TubeColorConfig::default(),
            TubeColorConfig {
                enabled: true,
                amount: 0.8,
                mix: 0.0,
                ..TubeColorConfig::default()
            },
            TubeColorConfig {
                enabled: true,
                amount: 0.0,
                boost_link: 0.0,
                mix: 1.0,
                ..TubeColorConfig::default()
            },
        ] {
            let mut tube = Tube::new(config, 0.0);
            for input in [-0.9, -0.1, 0.0, 0.4, 0.9] {
                assert_eq!(tube.process(input), input);
            }
        }
    }
    #[test]
    fn enabled_changes_signal_and_stays_finite() {
        let mut tube = Tube::new(
            TubeColorConfig {
                enabled: true,
                amount: 0.8,
                mix: 1.0,
                ..TubeColorConfig::default()
            },
            6.0,
        );
        let output = tube.process(0.8);
        assert!(output.is_finite() && (output - 0.8).abs() > 1e-3);
    }
}
