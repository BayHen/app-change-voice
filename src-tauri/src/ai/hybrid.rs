//! Optional effects run after neural identity conversion, on the DSP worker.
use crate::effects::{
    chain::AdditionalEffectChain,
    parameters::{EffectParameters, SharedEffectParameters},
};
pub struct NeuralPostProcessor {
    chain: AdditionalEffectChain,
    controls: SharedEffectParameters,
    target: EffectParameters,
    rate: u32,
    count: usize,
    generation: u64,
    wet_mix: f32,
}
impl NeuralPostProcessor {
    pub fn new(rate: u32, controls: SharedEffectParameters) -> Self {
        Self {
            chain: AdditionalEffectChain::new(rate),
            target: EffectParameters::default(),
            controls,
            rate,
            count: 0,
            generation: 0,
            wet_mix: 0.0,
        }
    }
    pub fn process(&mut self, sample: f32, generation: u64) -> f32 {
        if generation != self.generation {
            self.chain = AdditionalEffectChain::new(self.rate);
            self.generation = generation;
            self.count = 0;
            self.wet_mix = 0.0;
        }
        if self.count == 0 {
            self.target = self.controls.load();
            self.chain.prepare_block(self.target);
        }
        self.count = (self.count + 1) % 256;
        let target_mix = if self.target.enabled && !self.target.bypassed {
            1.0
        } else {
            0.0
        };
        self.wet_mix +=
            (target_mix - self.wet_mix) * (1.0 - (-1.0 / (self.rate as f32 * 0.010)).exp());
        let wet = self.chain.process_sample(
            sample,
            self.target,
            1.0 - (-1.0 / (self.rate as f32 * 0.05)).exp(),
        );
        if self.wet_mix < 0.000001 {
            sample
        } else {
            sample + (wet - sample) * self.wet_mix
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_effects_change_neural_pcm_and_bypass_is_exact() {
        let controls = SharedEffectParameters::new(EffectParameters {
            ring_mod_mix: 1.0,
            ..Default::default()
        });
        let mut fx = NeuralPostProcessor::new(48000, controls.clone());
        let mut changed = 0;
        for _ in 0..10000 {
            if (fx.process(0.25, 1) - 0.25).abs() > 0.01 {
                changed += 1;
            }
        }
        assert!(changed > 100);
        let mut bypass = controls.load();
        bypass.bypassed = true;
        controls.store(bypass);
        for _ in 0..15000 {
            fx.process(0.25, 1);
        }
        assert_eq!(fx.process(0.25, 1), 0.25);
        assert_eq!(fx.process(0.0, 2), 0.0);
    }
}
