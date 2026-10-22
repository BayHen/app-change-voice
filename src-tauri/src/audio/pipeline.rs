use super::processor::AudioProcessor;

pub struct AudioPipeline<P: AudioProcessor> {
    processor: P,
}

impl<P: AudioProcessor> AudioPipeline<P> {
    pub fn new(processor: P) -> Self {
        Self { processor }
    }

    #[inline]
    pub fn process(&mut self, sample: f32) -> f32 {
        self.processor.process_sample(sample).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::processor::PassthroughProcessor;

    #[test]
    fn pipeline_is_float32_passthrough() {
        let mut pipeline = AudioPipeline::new(PassthroughProcessor);
        assert_eq!(pipeline.process(0.375), 0.375);
    }
}
