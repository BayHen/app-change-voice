use crate::audio::processor::AudioProcessor;
use std::collections::BTreeMap;

pub trait EffectFactory: Send + Sync {
    fn effect_id(&self) -> &'static str;
    fn create(
        &self,
        sample_rate: u32,
        parameters: &BTreeMap<String, f32>,
    ) -> Result<Box<dyn AudioProcessor>, String>;
}

pub struct TrimFactory;
struct TrimProcessor {
    gain: f32,
}
impl AudioProcessor for TrimProcessor {
    fn process_sample(&mut self, sample: f32) -> f32 {
        sample * self.gain
    }
}
impl EffectFactory for TrimFactory {
    fn effect_id(&self) -> &'static str {
        "builtin.trim"
    }
    fn create(
        &self,
        _: u32,
        parameters: &BTreeMap<String, f32>,
    ) -> Result<Box<dyn AudioProcessor>, String> {
        let gain = *parameters.get("gain").unwrap_or(&1.0);
        if !gain.is_finite() || !(0.0..=2.0).contains(&gain) {
            return Err("gain must be between 0 and 2".into());
        }
        Ok(Box::new(TrimProcessor { gain }))
    }
}
