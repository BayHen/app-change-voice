/// Bộ xử lý chạy trực tiếp trên audio thread. Implementation không được block hoặc allocate.
pub trait AudioProcessor: Send + 'static {
    fn process_sample(&mut self, sample: f32) -> f32;
}

#[derive(Debug, Default)]
pub struct PassthroughProcessor;

impl AudioProcessor for PassthroughProcessor {
    #[inline]
    fn process_sample(&mut self, sample: f32) -> f32 {
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_preserves_samples() {
        let mut processor = PassthroughProcessor;
        for sample in [-1.0, -0.25, 0.0, 0.5, 1.0] {
            assert_eq!(processor.process_sample(sample), sample);
        }
    }
}
