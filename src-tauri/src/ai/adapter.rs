use super::inference::{AI_FRAME_SAMPLES, AIFrame, AIInferenceService, AIWorkerMetrics};

/// Non-blocking boundary for a future native ONNX backend. If output is not ready,
/// callers keep the existing DSP result instead of waiting on the audio thread.
pub struct AIProcessingAdapter {
    worker: AIInferenceService,
    sequence: u64,
}

impl AIProcessingAdapter {
    pub fn new(worker: AIInferenceService) -> Self {
        Self {
            worker,
            sequence: 0,
        }
    }

    pub fn submit(&mut self, samples: &[f32; AI_FRAME_SAMPLES]) {
        self.worker.try_submit(AIFrame {
            sequence: self.sequence,
            samples: *samples,
        });
        self.sequence = self.sequence.wrapping_add(1);
    }

    pub fn take_ready(&mut self, output: &mut [f32; AI_FRAME_SAMPLES]) -> bool {
        let Some(frame) = self.worker.try_receive() else {
            return false;
        };
        output.copy_from_slice(&frame.samples);
        true
    }

    pub fn metrics(&self) -> AIWorkerMetrics {
        self.worker.metrics()
    }
}
