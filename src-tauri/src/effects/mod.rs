//! Các thuật toán DSP độc lập hệ điều hành.

pub mod gain;

pub trait VoiceEffect {
    fn process(&mut self, samples: &mut [f32]);
}

#[derive(Debug, Default)]
pub struct PassthroughEffect;

impl VoiceEffect for PassthroughEffect {
    fn process(&mut self, _samples: &mut [f32]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_keeps_samples_unchanged() {
        let mut samples = [0.25, -0.5, 1.0];
        let expected = samples;
        PassthroughEffect.process(&mut samples);
        assert_eq!(samples, expected);
    }
}
