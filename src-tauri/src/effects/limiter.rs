#[derive(Debug)]
pub struct OutputLimiter {
    gain: f32,
    sample_rate: f32,
}
impl OutputLimiter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            gain: 1.0,
            sample_rate: sample_rate as f32,
        }
    }
    #[inline]
    pub fn process(&mut self, sample: f32, enabled: bool, ceiling_dbfs: f32) -> (f32, bool) {
        let clean = if sample.is_finite() { sample } else { 0.0 };
        if !enabled {
            return (clean.clamp(-1.0, 1.0), false);
        }
        let ceiling = 10.0_f32.powf(ceiling_dbfs / 20.0);
        let peak = clean.abs();
        let target = if peak > ceiling { ceiling / peak } else { 1.0 };
        let coefficient = if target < self.gain {
            1.0
        } else {
            1.0 - (-1.0 / (0.050 * self.sample_rate)).exp()
        };
        self.gain += (target - self.gain) * coefficient;
        ((clean * self.gain).clamp(-ceiling, ceiling), peak > ceiling)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protects_clipping_and_invalid() {
        let mut l = OutputLimiter::new(48_000);
        let (a, p) = l.process(2.0, true, -1.0);
        assert!(p && a.abs() < 1.0);
        assert_eq!(l.process(f32::NAN, true, -1.0).0, 0.0);
    }
    #[test]
    fn bypass_is_bounded() {
        let mut l = OutputLimiter::new(48_000);
        assert_eq!(l.process(2.0, false, -1.0).0, 1.0);
    }
}
