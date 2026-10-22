use signalsmith_stretch::Stretch;

/// Realtime pitch/formant engine. Construction and warm-up happen before CPAL
/// starts, avoiding first-use FFT/buffer setup in the audio callback.
pub struct PitchShiftProcessor {
    stretch: Stretch,
    latency_frames: usize,
}

impl PitchShiftProcessor {
    pub fn new(sample_rate: u32, warmup_frames: usize) -> Self {
        // A 2048-frame analysis window with a 512-frame interval keeps speech
        // monitoring latency substantially below the library's offline-oriented preset.
        let block_length = if sample_rate >= 88_200 { 4_096 } else { 2_048 };
        let mut stretch = Stretch::new(1, block_length, block_length / 4);
        stretch.set_transpose_factor_semitones(0.0, None);
        stretch.set_formant_factor_semitones(0.0, false);
        let latency_frames = stretch.input_latency() + stretch.output_latency();
        let warmup_frames = latency_frames + warmup_frames * 2;
        let input: Vec<f32> = (0..warmup_frames)
            .map(|index| {
                (std::f32::consts::TAU * 220.0 * index as f32 / sample_rate as f32).sin() * 0.01
            })
            .collect();
        let mut output = vec![0.0; warmup_frames];
        stretch.process(&input, &mut output);
        stretch.reset();
        Self {
            stretch,
            latency_frames,
        }
    }

    pub fn latency_frames(&self) -> usize {
        self.latency_frames
    }

    pub fn set_parameters(&mut self, pitch_semitones: f32, formant_semitones: f32) {
        self.stretch
            .set_transpose_factor_semitones(pitch_semitones, None);
        self.stretch
            .set_formant_factor_semitones(formant_semitones, false);
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        debug_assert_eq!(input.len(), output.len());
        self.stretch.process(input, output);
    }
}
