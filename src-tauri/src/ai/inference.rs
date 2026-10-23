use std::{
    sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
    thread,
};

pub const AI_FRAME_SAMPLES: usize = 2_048;

#[derive(Clone)]
pub struct AIFrame {
    pub sequence: u64,
    pub samples: [f32; AI_FRAME_SAMPLES],
}

pub trait AIInferenceBackend: Send + 'static {
    fn infer(&mut self, input: &AIFrame) -> Result<AIFrame, String>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AIWorkerMetrics {
    pub submitted: u64,
    pub completed: u64,
    pub dropped_input: u64,
    pub dropped_output: u64,
    pub failures: u64,
}

pub struct AIInferenceService {
    input: SyncSender<AIFrame>,
    output: Receiver<Result<AIFrame, String>>,
    metrics: AIWorkerMetrics,
}

impl AIInferenceService {
    pub fn spawn(mut backend: impl AIInferenceBackend) -> Self {
        let (input_tx, input_rx) = sync_channel::<AIFrame>(2);
        let (output_tx, output_rx) = sync_channel::<Result<AIFrame, String>>(2);
        thread::Builder::new()
            .name("ai-inference".into())
            .spawn(move || {
                while let Ok(frame) = input_rx.recv() {
                    let result = backend.infer(&frame);
                    let _ = output_tx.try_send(result);
                }
            })
            .expect("could not start AI inference worker");
        Self {
            input: input_tx,
            output: output_rx,
            metrics: AIWorkerMetrics::default(),
        }
    }

    pub fn try_submit(&mut self, frame: AIFrame) {
        match self.input.try_send(frame) {
            Ok(()) => self.metrics.submitted += 1,
            Err(TrySendError::Full(_)) => self.metrics.dropped_input += 1,
            Err(TrySendError::Disconnected(_)) => self.metrics.failures += 1,
        }
    }

    pub fn try_receive(&mut self) -> Option<AIFrame> {
        match self.output.try_recv() {
            Ok(Ok(frame)) => {
                self.metrics.completed += 1;
                Some(frame)
            }
            Ok(Err(_)) => {
                self.metrics.failures += 1;
                None
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.metrics.failures += 1;
                None
            }
        }
    }

    pub fn metrics(&self) -> AIWorkerMetrics {
        self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    struct GainBackend;
    impl AIInferenceBackend for GainBackend {
        fn infer(&mut self, input: &AIFrame) -> Result<AIFrame, String> {
            let mut output = input.clone();
            output.samples.iter_mut().for_each(|sample| *sample *= 0.5);
            Ok(output)
        }
    }

    #[test]
    fn worker_processes_a_fixed_size_frame() {
        let mut worker = AIInferenceService::spawn(GainBackend);
        worker.try_submit(AIFrame {
            sequence: 7,
            samples: [0.4; AI_FRAME_SAMPLES],
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Some(output) = worker.try_receive() {
                assert_eq!(output.sequence, 7);
                assert!((output.samples[0] - 0.2).abs() < f32::EPSILON);
                break;
            }
            assert!(Instant::now() < deadline, "worker timed out");
            std::thread::yield_now();
        }
    }

    #[test]
    fn bounded_queue_drops_instead_of_blocking() {
        struct SlowBackend;
        impl AIInferenceBackend for SlowBackend {
            fn infer(&mut self, input: &AIFrame) -> Result<AIFrame, String> {
                std::thread::sleep(Duration::from_millis(30));
                Ok(input.clone())
            }
        }
        let mut worker = AIInferenceService::spawn(SlowBackend);
        for sequence in 0..20 {
            worker.try_submit(AIFrame {
                sequence,
                samples: [0.0; AI_FRAME_SAMPLES],
            });
        }
        assert!(worker.metrics().dropped_input > 0);
    }
}
