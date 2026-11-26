#[derive(Debug)]
pub struct OutputLimiter {
    gain: f32,
    attack: f32,
    release: f32,
    ceiling: f32,
    ceiling_db: f32,
    delay: Box<[f32]>,
    cursor: usize,
    held_peak: f32,
    hold: usize,
}
impl OutputLimiter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            gain: 1.0,
            attack: 1.0 - (-1.0 / (0.00015 * sample_rate as f32)).exp(),
            release: 1.0 - (-1.0 / (0.050 * sample_rate as f32)).exp(),
            ceiling: 10.0_f32.powf(-1.0 / 20.0),
            ceiling_db: -1.0,
            delay: vec![0.0; (sample_rate as usize / 1000).max(1)].into_boxed_slice(),
            cursor: 0,
            held_peak: 0.0,
            hold: 0,
        }
    }
    #[inline]
    pub fn process(&mut self, sample: f32, enabled: bool, ceiling_dbfs: f32) -> (f32, bool) {
        let clean = if sample.is_finite() { sample } else { 0.0 };
        let delayed = self.delay[self.cursor];
        self.delay[self.cursor] = clean;
        self.cursor = (self.cursor + 1) % self.delay.len();
        if !enabled {
            self.gain = 1.0;
            return (delayed.clamp(-1.0, 1.0), clean.abs() > 1.0);
        }
        if ceiling_dbfs != self.ceiling_db {
            self.ceiling_db = ceiling_dbfs;
            self.ceiling = 10.0_f32.powf(ceiling_dbfs / 20.0);
        }
        let ceiling = self.ceiling;
        let peak = clean.abs();
        if peak >= self.held_peak || self.hold == 0 {
            self.held_peak = peak;
            self.hold = self.delay.len();
        } else {
            self.hold -= 1;
        }
        let target = if self.held_peak > ceiling {
            ceiling / self.held_peak
        } else {
            1.0
        };
        let coefficient = if target < self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain += (target - self.gain) * coefficient;
        let safe_gain = self.gain.min(ceiling / delayed.abs().max(ceiling));
        (
            (delayed * safe_gain).clamp(-ceiling, ceiling),
            peak > ceiling,
        )
    }
    pub fn latency_frames(&self) -> usize {
        self.delay.len()
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
        l.process(2.0, false, -1.0);
        for _ in 1..l.latency_frames() {
            l.process(0.0, false, -1.0);
        }
        assert_eq!(l.process(0.0, false, -1.0).0, 1.0);
    }
}
