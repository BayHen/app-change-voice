use crate::effects::{
    chain::AdditionalEffectChain, formant::FormantProcessor, parameters::SharedEffectParameters,
    pitch_shift::PitchShiftProcessor,
};

pub const DSP_BLOCK_FRAMES: usize = 256;
const PARAMETER_SMOOTH_SECONDS: f32 = 0.050;
const BYPASS_SMOOTH_SECONDS: f32 = 0.010;

/// Fixed-size, allocation-free realtime chain after construction.
pub struct ProcessorChain {
    pitch: PitchShiftProcessor,
    formant: FormantProcessor,
    controls: SharedEffectParameters,
    input: Box<[f32; DSP_BLOCK_FRAMES]>,
    wet: Box<[f32; DSP_BLOCK_FRAMES]>,
    dry_delay: Box<[f32]>,
    delay_cursor: usize,
    current_pitch: f32,
    wet_mix: f32,
    sample_rate: u32,
    additional: AdditionalEffectChain,
}

impl ProcessorChain {
    pub fn new(sample_rate: u32, controls: SharedEffectParameters) -> Self {
        let initial = controls.load();
        let pitch = PitchShiftProcessor::new(sample_rate, DSP_BLOCK_FRAMES);
        let latency = pitch.latency_frames().max(1);
        Self {
            pitch,
            formant: FormantProcessor::new(initial.formant_semitones),
            controls,
            input: Box::new([0.0; DSP_BLOCK_FRAMES]),
            wet: Box::new([0.0; DSP_BLOCK_FRAMES]),
            dry_delay: vec![0.0; latency].into_boxed_slice(),
            delay_cursor: 0,
            current_pitch: initial.pitch_semitones,
            wet_mix: if initial.enabled && !initial.bypassed {
                1.0
            } else {
                0.0
            },
            sample_rate,
            additional: AdditionalEffectChain::new(sample_rate),
        }
    }

    pub fn latency_frames(&self) -> usize {
        self.dry_delay.len()
    }

    pub fn process_block(
        &mut self,
        input: &[f32; DSP_BLOCK_FRAMES],
        output: &mut [f32; DSP_BLOCK_FRAMES],
    ) {
        self.input.copy_from_slice(input);
        let target = self.controls.load();
        let block_seconds = DSP_BLOCK_FRAMES as f32 / self.sample_rate as f32;
        let coefficient = 1.0 - (-block_seconds / PARAMETER_SMOOTH_SECONDS).exp();
        self.current_pitch += (target.pitch_semitones - self.current_pitch) * coefficient;
        let formant = self
            .formant
            .smooth_towards(target.formant_semitones, coefficient);
        self.pitch.set_parameters(self.current_pitch, formant);
        self.pitch.process(&self.input[..], &mut self.wet[..]);
        self.additional.prepare_block(target);

        let target_mix = f32::from(target.enabled && !target.bypassed);
        let mix_step = 1.0 / (BYPASS_SMOOTH_SECONDS * self.sample_rate as f32).max(1.0);
        let effect_smoothing =
            1.0 - (-1.0 / (PARAMETER_SMOOTH_SECONDS * self.sample_rate as f32)).exp();
        for (index, sample) in output.iter_mut().enumerate() {
            let dry = self.dry_delay[self.delay_cursor];
            self.dry_delay[self.delay_cursor] = self.input[index];
            self.delay_cursor = (self.delay_cursor + 1) % self.dry_delay.len();
            self.wet_mix += (target_mix - self.wet_mix).clamp(-mix_step, mix_step);
            let wet = self
                .additional
                .process_sample(self.wet[index], target, effect_smoothing);
            *sample = (dry + (wet - dry) * self.wet_mix).clamp(-1.0, 1.0);
        }
    }
}

pub struct AudioPipeline {
    chain: ProcessorChain,
    pending: Box<[f32; DSP_BLOCK_FRAMES]>,
    output: Box<[f32; DSP_BLOCK_FRAMES]>,
    pending_len: usize,
}

impl AudioPipeline {
    pub fn new(sample_rate: u32, controls: SharedEffectParameters) -> Self {
        Self {
            chain: ProcessorChain::new(sample_rate, controls),
            pending: Box::new([0.0; DSP_BLOCK_FRAMES]),
            output: Box::new([0.0; DSP_BLOCK_FRAMES]),
            pending_len: 0,
        }
    }

    pub fn latency_frames(&self) -> usize {
        self.chain.latency_frames() + DSP_BLOCK_FRAMES
    }

    #[inline]
    pub fn push(&mut self, sample: f32, mut emit: impl FnMut(f32)) {
        self.pending[self.pending_len] = sample;
        self.pending_len += 1;
        if self.pending_len == DSP_BLOCK_FRAMES {
            self.chain.process_block(&self.pending, &mut self.output);
            for sample in self.output.iter().copied() {
                emit(sample);
            }
            self.pending_len = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::parameters::EffectParameters;

    fn process_signal(parameters: EffectParameters, input: &[f32]) -> (Vec<f32>, usize) {
        let controls = SharedEffectParameters::new(parameters);
        let mut pipeline = AudioPipeline::new(48_000, controls);
        let latency = pipeline.latency_frames();
        let mut output = Vec::with_capacity(input.len());
        for &sample in input {
            pipeline.push(sample, |sample| output.push(sample));
        }
        (output, latency)
    }

    #[test]
    fn silence_stays_finite_and_silent() {
        let (output, _) = process_signal(EffectParameters::default(), &[0.0; 48_128]);
        assert!(
            output
                .iter()
                .all(|sample| sample.is_finite() && *sample == 0.0)
        );
    }

    #[test]
    fn bypass_uses_latency_aligned_dry_signal() {
        let parameters = EffectParameters {
            enabled: false,
            ..EffectParameters::default()
        };
        let mut input = vec![0.0; 48_128];
        input[0] = 0.5;
        let (output, latency) = process_signal(parameters, &input);
        assert_eq!(output.len(), input.len());
        assert!((output[latency - DSP_BLOCK_FRAMES] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn rapid_parameter_updates_never_produce_non_finite_samples() {
        let controls = SharedEffectParameters::new(EffectParameters::default());
        let mut pipeline = AudioPipeline::new(48_000, controls.clone());
        let mut finite = true;
        for index in 0..48_128 {
            if index % 257 == 0 {
                controls.store(EffectParameters {
                    pitch_semitones: if index % 2 == 0 { 12.0 } else { -12.0 },
                    formant_semitones: if index % 2 == 0 { -6.0 } else { 6.0 },
                    enabled: true,
                    bypassed: false,
                    ..EffectParameters::default()
                });
            }
            pipeline.push((index as f32 * 0.01).sin() * 0.2, |sample| {
                finite &= sample.is_finite()
            });
        }
        assert!(finite);
    }

    fn sine(frequency: f32, seconds: usize) -> Vec<f32> {
        (0..48_000 * seconds)
            .map(|index| (std::f32::consts::TAU * frequency * index as f32 / 48_000.0).sin() * 0.2)
            .collect()
    }

    fn estimate_frequency(samples: &[f32]) -> f32 {
        let crossings = samples
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        crossings as f32 * 48_000.0 / samples.len() as f32
    }

    #[test]
    fn pitch_shifts_preserve_duration_and_expected_frequency() {
        for semitones in [-6.0_f32, -3.0, 0.0, 3.0, 6.0] {
            let parameters = EffectParameters {
                pitch_semitones: semitones,
                ..EffectParameters::default()
            };
            let mut input = sine(220.0, 3);
            input.resize(input.len().next_multiple_of(DSP_BLOCK_FRAMES), 0.0);
            let (output, latency) = process_signal(parameters, &input);
            assert_eq!(output.len(), input.len());
            let start = latency + 48_000;
            let measured = estimate_frequency(&output[start..start + 48_000]);
            let expected = 220.0 * 2.0_f32.powf(semitones / 12.0);
            assert!(
                (measured - expected).abs() < 6.0,
                "{semitones} st: measured {measured}, expected {expected}"
            );
            assert!(
                output
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
            );
        }
    }

    #[test]
    fn zero_pitch_is_near_neutral_in_frequency_and_energy() {
        let mut input = sine(220.0, 3);
        input.resize(input.len().next_multiple_of(DSP_BLOCK_FRAMES), 0.0);
        let (output, latency) = process_signal(EffectParameters::default(), &input);
        let start = latency + 48_000;
        let section = &output[start..start + 48_000];
        let frequency = estimate_frequency(section);
        let rms = (section.iter().map(|sample| sample * sample).sum::<f32>()
            / section.len() as f32)
            .sqrt();
        let expected_rms = 0.2 / 2.0_f32.sqrt();
        assert!((frequency - 220.0).abs() < 3.0);
        assert!((rms - expected_rms).abs() < 0.03);
    }

    #[test]
    fn formant_is_independent_from_pitch_parameter() {
        let input: Vec<f32> = (0..48_000 * 3)
            .map(|index| {
                let time = index as f32 / 48_000.0;
                (std::f32::consts::TAU * 120.0 * time).sin() * 0.15
                    + (std::f32::consts::TAU * 720.0 * time).sin() * 0.08
                    + (std::f32::consts::TAU * 1_440.0 * time).sin() * 0.04
            })
            .collect();
        let (neutral, latency) = process_signal(EffectParameters::default(), &input);
        let (shifted, _) = process_signal(
            EffectParameters {
                formant_semitones: 4.0,
                ..EffectParameters::default()
            },
            &input,
        );
        let start = latency + 48_000;
        let difference = neutral[start..]
            .iter()
            .zip(&shifted[start..])
            .map(|(left, right)| (left - right).abs())
            .sum::<f32>()
            / (neutral.len() - start) as f32;
        assert!(
            difference > 0.001,
            "formant output did not change: {difference}"
        );
    }

    #[test]
    fn switches_all_builtin_presets_without_restart_or_invalid_output() {
        let registry = crate::presets::PresetRegistry::built_in().unwrap();
        let controls = SharedEffectParameters::new(EffectParameters::default());
        let mut pipeline = AudioPipeline::new(48_000, controls.clone());
        let mut output = Vec::new();
        for preset in registry.all() {
            controls.store(preset.effect_parameters());
            for index in 0..24_064 {
                let input = (std::f32::consts::TAU * 180.0 * index as f32 / 48_000.0).sin() * 0.2;
                pipeline.push(input, |sample| output.push(sample));
            }
        }
        assert_eq!(output.len(), 24_064 * registry.all().len());
        assert!(
            output
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
        );
        let tail = &output[pipeline.latency_frames() + 48_000..];
        let rms =
            (tail.iter().map(|sample| sample * sample).sum::<f32>() / tail.len() as f32).sqrt();
        assert!(rms > 0.01, "preset switching unexpectedly lost audio");
    }

    #[test]
    #[ignore = "manual performance measurement; run in release mode"]
    fn measures_algorithm_latency_and_cpu_time() {
        let input = sine(220.0, 10);
        let started = std::time::Instant::now();
        let (_, latency) = process_signal(
            EffectParameters {
                pitch_semitones: 6.0,
                formant_semitones: 3.0,
                ..EffectParameters::default()
            },
            &input,
        );
        let elapsed = started.elapsed();
        println!(
            "DSP latency: {:.2} ms; processed 10 s in {:.2} ms; realtime CPU ratio: {:.3}",
            latency as f64 * 1_000.0 / 48_000.0,
            elapsed.as_secs_f64() * 1_000.0,
            elapsed.as_secs_f64() / 10.0,
        );
    }
}
