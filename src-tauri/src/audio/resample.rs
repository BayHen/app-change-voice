//! Persistent, preallocated sinc conversion; no logging, locks or allocation while processing.
use rtrb::Consumer;
use rubato::{
    Resampler, SincFixedIn, SincFixedOut, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

fn parameters(taps: usize) -> SincInterpolationParameters {
    SincInterpolationParameters {
        sinc_len: taps,
        f_cutoff: 0.92,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    }
}

pub struct StreamResampler {
    filter: Option<SincFixedIn<f32>>,
    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,
    used: usize,
}
impl StreamResampler {
    pub fn new(source: u32, target: u32) -> Result<Self, String> {
        if !(8000..=192000).contains(&source) || !(8000..=192000).contains(&target) {
            return Err("Resampling supports 8–192 kHz".into());
        }
        let filter = if source == target {
            None
        } else {
            Some(
                SincFixedIn::new(
                    f64::from(target) / f64::from(source),
                    1.0,
                    parameters(128),
                    128,
                    1,
                )
                .map_err(|e| e.to_string())?,
            )
        };
        let input = filter
            .as_ref()
            .map(|r| r.input_buffer_allocate(true))
            .unwrap_or_default();
        let output = filter
            .as_ref()
            .map(|r| r.output_buffer_allocate(true))
            .unwrap_or_default();
        Ok(Self {
            filter,
            input,
            output,
            used: 0,
        })
    }
    pub fn push(&mut self, sample: f32, output: &mut Vec<f32>) {
        let sample = if sample.is_finite() { sample } else { 0.0 };
        if let Some(filter) = &mut self.filter {
            self.input[0][self.used] = sample;
            self.used += 1;
            if self.used == filter.input_frames_next() {
                if let Ok((_, count)) =
                    filter.process_into_buffer(&self.input, &mut self.output, None)
                {
                    output.extend_from_slice(&self.output[0][..count]);
                }
                self.used = 0;
            }
        } else {
            output.push(sample);
        }
    }
    pub fn delay_frames(&self) -> usize {
        self.filter.as_ref().map_or(0, |r| r.output_delay())
    }
}

/// One independent consumer clock. Small, smoothly ramped ratio corrections regulate occupancy.
pub struct ClockedReader {
    consumer: Consumer<f32>,
    filter: SincFixedOut<f32>,
    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,
    cursor: usize,
    valid: usize,
    target: usize,
    primed: bool,
    last: f32,
    fade: f32,
    fade_step: f32,
    correction: f64,
    integral: f64,
    pub underruns: u64,
    pub discarded: u64,
}
impl ClockedReader {
    pub fn new(
        consumer: Consumer<f32>,
        source: u32,
        target: u32,
        startup_ms: u32,
        taps: usize,
    ) -> Result<Self, String> {
        let filter = SincFixedOut::new(
            f64::from(target) / f64::from(source),
            1.01,
            parameters(taps),
            256,
            1,
        )
        .map_err(|e| e.to_string())?;
        let input = filter.input_buffer_allocate(true);
        let output = filter.output_buffer_allocate(true);
        let target_depth = (source as usize * startup_ms as usize / 1000)
            .max(filter.input_frames_max() * 2)
            .min(consumer.buffer().capacity() / 2);
        Ok(Self {
            consumer,
            filter,
            input,
            output,
            cursor: 0,
            valid: 0,
            target: target_depth,
            primed: false,
            last: 0.0,
            fade: 0.0,
            fade_step: 1.0 / (f64::from(target) * 0.005) as f32,
            correction: 0.0,
            integral: 0.0,
            underruns: 0,
            discarded: 0,
        })
    }
    pub fn consumer_mut(&mut self) -> &mut Consumer<f32> {
        &mut self.consumer
    }
    pub fn depth(&self) -> usize {
        self.consumer.slots() + self.valid.saturating_sub(self.cursor)
    }
    pub fn correction_ppm(&self) -> f64 {
        self.correction * 1_000_000.0
    }
    pub fn delay_frames(&self) -> usize {
        self.filter.output_delay()
    }
    pub fn reset(&mut self) {
        self.filter.reset();
        self.cursor = 0;
        self.valid = 0;
        self.primed = false;
        self.fade = 0.0;
        self.last = 0.0;
        self.correction = 0.0;
        self.integral = 0.0;
    }
    pub fn next_sample(&mut self) -> f32 {
        if self.cursor == self.valid {
            let depth = self.consumer.slots();
            if !self.primed && depth < self.target {
                return self.fade_out();
            }
            if depth > self.consumer.buffer().capacity() * 3 / 4 {
                let count = depth.saturating_sub(self.target);
                for _ in 0..count {
                    let _ = self.consumer.pop();
                }
                self.discarded += count as u64;
                self.filter.reset();
                self.primed = false;
                self.fade = 0.0;
            }
            if self.primed {
                let error =
                    (self.consumer.slots() as f64 - self.target as f64) / self.target as f64;
                self.integral = (self.integral + error * 0.0000004).clamp(-0.0025, 0.0025);
                self.correction +=
                    ((error * 0.004 + self.integral).clamp(-0.003, 0.003) - self.correction) * 0.02;
                let _ = self
                    .filter
                    .set_resample_ratio_relative(1.0 - self.correction, true);
            }
            let needed = self.filter.input_frames_next();
            if self.consumer.slots() < needed {
                if self.primed {
                    self.underruns += 1;
                    self.primed = false;
                    self.fade = 0.0;
                    self.filter.reset();
                }
                return self.fade_out();
            }
            for i in 0..needed {
                self.input[0][i] = self.consumer.pop().unwrap_or(0.0);
            }
            match self
                .filter
                .process_into_buffer(&self.input, &mut self.output, None)
            {
                Ok((_, count)) => {
                    self.cursor = 0;
                    self.valid = count;
                    self.primed = true;
                }
                Err(_) => {
                    self.underruns += 1;
                    self.primed = false;
                    self.fade = 0.0;
                    return self.fade_out();
                }
            }
        }
        if self.cursor < self.valid {
            let value = self.output[0][self.cursor];
            self.cursor += 1;
            self.fade = (self.fade + self.fade_step).min(1.0);
            self.last = if value.is_finite() {
                value * self.fade
            } else {
                0.0
            };
            self.last
        } else {
            self.fade_out()
        }
    }
    fn fade_out(&mut self) -> f32 {
        self.last *= 1.0 - self.fade_step.min(1.0);
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_rate_pairs_keep_dc_level_and_bounded_streaming_tail() {
        for (source, target) in [
            (44100, 48000),
            (48000, 44100),
            (96000, 16000),
            (16000, 48000),
            (48000, 96000),
        ] {
            let mut converter = StreamResampler::new(source, target).unwrap();
            let mut output = Vec::with_capacity(target as usize * 2);
            for _ in 0..source * 2 {
                converter.push(0.125, &mut output);
            }
            let expected = target as usize * 2;
            assert!(output.len().abs_diff(expected) < 512 + converter.delay_frames());
            assert!(output.iter().all(|v| v.is_finite()));
            let mean = output[4096..].iter().map(|v| f64::from(*v)).sum::<f64>()
                / (output.len() - 4096) as f64;
            assert!((mean - 0.125).abs() < 0.00001, "{source}->{target}: {mean}");
        }
    }
    #[test]
    fn follows_positive_and_negative_clock_drift_for_three_simulated_minutes() {
        for ppm in [-1000.0, 1000.0] {
            let (mut producer, consumer) = rtrb::RingBuffer::new(8192);
            let mut reader = ClockedReader::new(consumer, 48000, 48000, 20, 64).unwrap();
            for _ in 0..960 {
                producer.push(0.1).unwrap();
            }
            let mut fractional = 0.0_f64;
            let mut settled_correction = 0.0;
            for tick in 0..18000 {
                fractional += 480.0 * (1.0 + ppm / 1_000_000.0);
                let frames = fractional.floor() as usize;
                fractional -= frames as f64;
                for _ in 0..frames {
                    producer.push(0.1).expect("bounded producer queue");
                }
                for _ in 0..480 {
                    assert!(reader.next_sample().is_finite());
                }
                if tick >= 15000 {
                    settled_correction += reader.correction_ppm();
                }
            }
            assert_eq!(reader.underruns, 0, "drift {ppm}");
            assert_eq!(reader.discarded, 0, "drift {ppm}");
            assert!(
                (settled_correction / 3000.0 - ppm).abs() < 250.0,
                "{ppm}: {}",
                reader.correction_ppm()
            );
        }
    }
    #[test]
    fn downsampling_rejects_alias_and_preserves_identity_headroom() {
        let mut same = StreamResampler::new(48000, 48000).unwrap();
        let mut output = Vec::new();
        same.push(1.2, &mut output);
        assert_eq!(output, [1.2]);
        let mut filter = StreamResampler::new(48000, 16000).unwrap();
        let mut output = Vec::new();
        for i in 0..48000 {
            filter.push(
                (std::f64::consts::TAU * 18000.0 * i as f64 / 48000.0).sin() as f32 * 0.2,
                &mut output,
            );
        }
        let rms = (output[1000..]
            .iter()
            .map(|v| f64::from(*v).powi(2))
            .sum::<f64>()
            / (output.len() - 1000) as f64)
            .sqrt();
        assert!(rms < 0.0001, "Alias RMS {rms}");
    }
    #[test]
    fn bounded_startup_and_clock_drift_do_not_hard_cut_on_empty() {
        let (mut producer, consumer) = rtrb::RingBuffer::new(8192);
        let mut reader = ClockedReader::new(consumer, 48000, 48000, 20, 64).unwrap();
        assert_eq!(reader.next_sample(), 0.0);
        for _ in 0..2000 {
            producer.push(0.2).unwrap();
        }
        let mut last = 0.0;
        for _ in 0..4000 {
            let value = reader.next_sample();
            assert!(value.is_finite());
            assert!((value - last).abs() < 0.01);
            last = value;
        }
        assert_eq!(reader.underruns, 1);
    }
}
