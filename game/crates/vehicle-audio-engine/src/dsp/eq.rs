use crate::{
    config::{EqConfig, EQ_BAND_FREQS},
    dsp::biquad::Biquad,
};

#[derive(Debug, Clone)]
pub struct GraphicEq {
    enabled: bool,
    filters: Vec<Biquad>,
    output: f32,
}

impl GraphicEq {
    pub fn new(config: &EqConfig, sample_rate: u32) -> Self {
        let filters = EQ_BAND_FREQS
            .iter()
            .map(|frequency| {
                Biquad::peaking(
                    sample_rate as f32,
                    *frequency as f32,
                    config.q,
                    config
                        .bands_db
                        .get(&frequency.to_string())
                        .copied()
                        .unwrap_or(0.0),
                )
            })
            .collect();
        Self {
            enabled: config.enabled,
            filters,
            output: 10.0_f32.powf(config.output_db / 20.0),
        }
    }
    #[inline]
    pub fn process(&mut self, mut sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }
        for filter in &mut self.filters {
            sample = filter.process(sample);
        }
        sample * self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_is_exact_bypass() {
        let mut eq = GraphicEq::new(&EqConfig::default(), 44100);
        for x in [-0.7, 0.0, 0.9] {
            assert_eq!(eq.process(x), x);
        }
    }
}
