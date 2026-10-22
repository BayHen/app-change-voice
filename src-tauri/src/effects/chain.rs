use super::parameters::EffectParameters;

const TAU: f32 = std::f32::consts::TAU;

/// Allocation-free processors used by creative presets after pitch/formant processing.
pub struct AdditionalEffectChain {
    sample_rate: f32,
    ring_phase: f32,
    tremolo_phase: f32,
    high_pass: HighPass,
    low_pass: LowPass,
    current_ring_mix: f32,
    current_radio_mix: f32,
    current_tremolo_depth: f32,
    current_makeup_gain: f32,
    target_makeup_gain: f32,
    bass_filter: LowPass,
    treble_filter: LowPass,
    current_bass_gain: f32,
    current_treble_gain: f32,
}

impl AdditionalEffectChain {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            ring_phase: 0.0,
            tremolo_phase: 0.0,
            high_pass: HighPass::default(),
            low_pass: LowPass::default(),
            current_ring_mix: 0.0,
            current_radio_mix: 0.0,
            current_tremolo_depth: 0.0,
            current_makeup_gain: 1.0,
            target_makeup_gain: 1.0,
            bass_filter: LowPass::default(),
            treble_filter: LowPass::default(),
            current_bass_gain: 1.0,
            current_treble_gain: 1.0,
        }
    }

    pub fn prepare_block(&mut self, parameters: EffectParameters) {
        self.high_pass
            .set_cutoff(parameters.radio_high_pass_hz, self.sample_rate);
        self.low_pass
            .set_cutoff(parameters.radio_low_pass_hz, self.sample_rate);
        self.target_makeup_gain = 10.0_f32.powf(parameters.makeup_gain_db / 20.0);
        self.bass_filter.set_cutoff(200.0, self.sample_rate);
        self.treble_filter.set_cutoff(4_000.0, self.sample_rate);
    }

    #[inline]
    pub fn process_sample(&mut self, sample: f32, target: EffectParameters, smoothing: f32) -> f32 {
        self.current_ring_mix += (target.ring_mod_mix - self.current_ring_mix) * smoothing;
        self.current_radio_mix += (target.radio_mix - self.current_radio_mix) * smoothing;
        self.current_tremolo_depth +=
            (target.tremolo_depth - self.current_tremolo_depth) * smoothing;
        self.current_makeup_gain +=
            (self.target_makeup_gain - self.current_makeup_gain) * smoothing;
        self.current_bass_gain +=
            (10.0_f32.powf(target.bass_db / 20.0) - self.current_bass_gain) * smoothing;
        self.current_treble_gain +=
            (10.0_f32.powf(target.treble_db / 20.0) - self.current_treble_gain) * smoothing;

        self.ring_phase =
            (self.ring_phase + TAU * target.ring_mod_frequency_hz / self.sample_rate) % TAU;
        let ring = sample * self.ring_phase.sin();
        let mut output = sample + (ring - sample) * self.current_ring_mix;

        let filtered = self.low_pass.process(self.high_pass.process(output));
        let driven = (filtered * target.radio_drive).tanh();
        output += (driven - output) * self.current_radio_mix;

        self.tremolo_phase =
            (self.tremolo_phase + TAU * target.tremolo_rate_hz / self.sample_rate) % TAU;
        let tremolo_gain =
            1.0 - self.current_tremolo_depth * (0.5 + 0.5 * self.tremolo_phase.sin());
        let low = self.bass_filter.process(output);
        let high = output - self.treble_filter.process(output);
        output += low * (self.current_bass_gain - 1.0) + high * (self.current_treble_gain - 1.0);
        (output * tremolo_gain * self.current_makeup_gain).clamp(-1.0, 1.0)
    }
}

#[derive(Default)]
struct HighPass {
    previous_input: f32,
    previous_output: f32,
    alpha: f32,
}

impl HighPass {
    fn set_cutoff(&mut self, cutoff: f32, sample_rate: f32) {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (TAU * cutoff);
        self.alpha = rc / (rc + dt);
    }
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.alpha * (self.previous_output + input - self.previous_input);
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

#[derive(Default)]
struct LowPass {
    output: f32,
    alpha: f32,
}

impl LowPass {
    fn set_cutoff(&mut self, cutoff: f32, sample_rate: f32) {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (TAU * cutoff);
        self.alpha = dt / (rc + dt);
    }
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.output += self.alpha * (input - self.output);
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_processors_remain_finite_and_bounded() {
        let mut chain = AdditionalEffectChain::new(48_000);
        let parameters = EffectParameters {
            ring_mod_mix: 1.0,
            radio_mix: 1.0,
            radio_drive: 4.0,
            tremolo_depth: 0.8,
            makeup_gain_db: 6.0,
            ..EffectParameters::default()
        };
        chain.prepare_block(parameters);
        for index in 0..96_000 {
            let sample = (index as f32 * 0.03).sin();
            let output = chain.process_sample(sample, parameters, 0.01);
            assert!(output.is_finite() && output.abs() <= 1.0);
        }
    }

    #[test]
    fn robot_radio_and_cartoon_each_change_the_signal() {
        for parameters in [
            EffectParameters {
                ring_mod_mix: 0.8,
                ..EffectParameters::default()
            },
            EffectParameters {
                radio_mix: 1.0,
                radio_drive: 1.8,
                ..EffectParameters::default()
            },
            EffectParameters {
                tremolo_depth: 0.2,
                ..EffectParameters::default()
            },
        ] {
            let mut chain = AdditionalEffectChain::new(48_000);
            chain.prepare_block(parameters);
            let difference = (0..48_000)
                .map(|index| {
                    let input = (index as f32 * 0.04).sin() * 0.2;
                    (chain.process_sample(input, parameters, 0.01) - input).abs()
                })
                .sum::<f32>()
                / 48_000.0;
            assert!(difference > 0.001);
        }
    }

    #[test]
    fn bass_and_treble_eq_change_output_without_non_finite_samples() {
        let mut chain = AdditionalEffectChain::new(48_000);
        let parameters = EffectParameters {
            bass_db: 6.0,
            treble_db: -6.0,
            ..EffectParameters::default()
        };
        chain.prepare_block(parameters);
        let mut difference = 0.0;
        for index in 0..48_000 {
            let input = (index as f32 * 0.07).sin() * 0.2;
            let output = chain.process_sample(input, parameters, 0.01);
            difference += (input - output).abs();
            assert!(output.is_finite());
        }
        assert!(difference > 1.0);
    }
}
