//! Fixed-storage vocal dynamics. Zero settings preserve the input exactly.
pub struct VocalDynamics {
    low: f32,
    hf_envelope: f32,
    power: f32,
    strength: f32,
    ratio: f32,
    split: f32,
    attack: f32,
    release: f32,
    rms: f32,
}
impl VocalDynamics {
    pub fn new(rate: u32) -> Self {
        let rate = rate as f32;
        Self {
            low: 0.0,
            hf_envelope: 0.0,
            power: 0.0,
            strength: 0.0,
            ratio: 1.0,
            split: 1.0 - (-std::f32::consts::TAU * 5500.0 / rate).exp(),
            attack: 1.0 - (-1.0 / (0.001 * rate)).exp(),
            release: 1.0 - (-1.0 / (0.060 * rate)).exp(),
            rms: 1.0 - (-1.0 / (0.010 * rate)).exp(),
        }
    }
    pub fn process(&mut self, input: f32, strength: f32, ratio: f32, smoothing: f32) -> f32 {
        self.strength += (strength - self.strength) * smoothing;
        self.ratio += (ratio - self.ratio) * smoothing;
        if self.strength < 0.000001 && (self.ratio - 1.0).abs() < 0.000001 {
            return input;
        }
        self.low += self.split * (input - self.low);
        let high = input - self.low;
        let coefficient = if high.abs() > self.hf_envelope {
            self.attack
        } else {
            self.release
        };
        self.hf_envelope += (high.abs() - self.hf_envelope) * coefficient;
        let reduction = (1.0 - 0.04 / self.hf_envelope.max(0.04)) * self.strength * 0.75;
        let deessed = input - high * reduction;
        self.power += (deessed * deessed - self.power) * self.rms;
        let level = self.power.max(0.0).sqrt();
        let gain = if level > 0.125 {
            (level / 0.125).powf(1.0 / self.ratio - 1.0)
        } else {
            1.0
        };
        deessed * gain
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neutral_is_exact_and_compression_reduces_sustained_level() {
        let mut neutral = VocalDynamics::new(48000);
        assert_eq!(neutral.process(1.2, 0.0, 1.0, 0.01), 1.2);
        let mut compressor = VocalDynamics::new(48000);
        let mut power = 0.0_f64;
        for i in 0..96000 {
            let sample = (i as f32 * std::f32::consts::TAU * 997.0 / 48000.0).sin() * 0.6;
            let out = compressor.process(sample, 0.0, 3.0, 0.002);
            assert!(out.is_finite());
            if i >= 48000 {
                power += f64::from(out).powi(2);
            }
        }
        assert!((power / 48000.0).sqrt() < 0.25);
    }
    #[test]
    fn deessing_attenuates_high_frequency_without_non_finite_output() {
        let mut processor = VocalDynamics::new(48000);
        let mut power = 0.0;
        for i in 0..48000 {
            let input = (i as f32 * std::f32::consts::TAU * 9000.0 / 48000.0).sin() * 0.6;
            let output = processor.process(input, 1.0, 1.0, 0.002);
            assert!(output.is_finite());
            if i > 24000 {
                power += output * output;
            }
        }
        assert!((power / 23999.0).sqrt() < 0.42);
    }
}
