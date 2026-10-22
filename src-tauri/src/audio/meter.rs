pub const DBFS_FLOOR: f32 = -96.0;
pub const CLIPPING_THRESHOLD: f32 = 0.999;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MeterReading {
    pub peak: f32,
    pub rms: f32,
    pub clipping: bool,
}

#[derive(Debug, Default)]
pub struct SignalMeter {
    sum_squares: f64,
    sample_count: u64,
    peak: f32,
}

impl SignalMeter {
    #[inline]
    pub fn push(&mut self, sample: f32) {
        let safe = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.peak = self.peak.max(safe.abs());
        self.sum_squares += f64::from(safe) * f64::from(safe);
        self.sample_count += 1;
    }

    pub fn finish(self) -> MeterReading {
        let rms = if self.sample_count == 0 {
            0.0
        } else {
            (self.sum_squares / self.sample_count as f64).sqrt() as f32
        };
        MeterReading {
            peak: self.peak,
            rms,
            clipping: self.peak >= CLIPPING_THRESHOLD,
        }
    }
}

#[inline]
pub fn amplitude_to_dbfs(amplitude: f32) -> f32 {
    if !amplitude.is_finite() || amplitude <= 0.0 {
        DBFS_FLOOR
    } else {
        (20.0 * amplitude.min(1.0).log10()).max(DBFS_FLOOR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_and_peak_match_known_signal() {
        let mut meter = SignalMeter::default();
        for sample in [1.0, -1.0, 1.0, -1.0] {
            meter.push(sample);
        }
        let reading = meter.finish();
        assert_eq!(reading.rms, 1.0);
        assert_eq!(reading.peak, 1.0);
        assert!(reading.clipping);
    }

    #[test]
    fn silence_has_floor_dbfs_and_no_clipping() {
        let mut meter = SignalMeter::default();
        for _ in 0..64 {
            meter.push(0.0);
        }
        let reading = meter.finish();
        assert_eq!(amplitude_to_dbfs(reading.rms), DBFS_FLOOR);
        assert!(!reading.clipping);
    }

    #[test]
    fn non_finite_samples_are_sanitized() {
        let mut meter = SignalMeter::default();
        meter.push(f32::NAN);
        meter.push(f32::INFINITY);
        let reading = meter.finish();
        assert!(reading.rms.is_finite());
        assert!(reading.peak.is_finite());
    }
}
