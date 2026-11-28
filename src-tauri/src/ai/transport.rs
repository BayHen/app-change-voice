//! The CPAL boundary contains only bounded SPSC operations and atomics.
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Default)]
pub struct SharedTransport {
    pub hybrid_dsp: AtomicBool,
    pub process_id: AtomicU64,
    pub ai_selected: AtomicBool,
    pub ready: AtomicBool,
    pub faulted: AtomicBool,
    pub classic_fallback: AtomicBool,
    pub generation: AtomicU64,
    pub dropped_input_samples: AtomicU64,
    pub missing_output_samples: AtomicU64,
    pub dropped_output_samples: AtomicU64,
    pub queued_input_samples: AtomicU64,
    pub queued_output_samples: AtomicU64,
    pub completed_chunks: AtomicU64,
    pub round_trip_micros: AtomicU64,
    pub conversion_micros: AtomicU64,
    pub output_prefill_frames: AtomicU64,
    pub startup_wait_samples: AtomicU64,
}
#[derive(Clone, Copy)]
pub struct TaggedSample {
    pub generation: u64,
    pub value: f32,
}
pub struct RealtimePort {
    input: Producer<TaggedSample>,
    output: Consumer<TaggedSample>,
    shared: Arc<SharedTransport>,
    generation: u64,
    primed: bool,
    ever_primed: bool,
    last: f32,
    fade: f32,
    fade_step: f32,
}
pub struct WorkerPort {
    pub diagnostics: Option<crate::audio::capture::Tap>,
    pub input: Consumer<TaggedSample>,
    pub output: Producer<TaggedSample>,
    pub sample_rate: u32,
}
pub fn ports(sample_rate: u32, shared: Arc<SharedTransport>) -> (RealtimePort, WorkerPort) {
    let capacity = (sample_rate as usize / 2)
        .clamp(8192, 65536)
        .next_power_of_two();
    let (input, input_rx) = RingBuffer::new(capacity);
    let (output_tx, output) = RingBuffer::new(capacity);
    (
        RealtimePort {
            input,
            output,
            shared,
            generation: 0,
            primed: false,
            ever_primed: false,
            last: 0.0,
            fade: 0.0,
            fade_step: 1.0 / (sample_rate as f32 * 0.005).max(1.0),
        },
        WorkerPort {
            diagnostics: None,
            input: input_rx,
            output: output_tx,
            sample_rate,
        },
    )
}
impl RealtimePort {
    pub fn post_processing(&self) -> (bool, u64) {
        (
            self.shared.hybrid_dsp.load(Ordering::Relaxed)
                && self.shared.ready.load(Ordering::Acquire)
                && !self.shared.faulted.load(Ordering::Acquire)
                && self.shared.ai_selected.load(Ordering::Acquire),
            self.shared.generation.load(Ordering::Acquire),
        )
    }

    /// None selects the existing Classic DSP path. AI warmup/faults return silence,
    /// except for a failure policy explicitly opted into by the user.
    #[inline]
    pub fn process(&mut self, sample: f32) -> Option<f32> {
        if !self.shared.ai_selected.load(Ordering::Acquire)
            || (self.shared.faulted.load(Ordering::Acquire)
                && self.shared.classic_fallback.load(Ordering::Acquire))
        {
            return None;
        }
        let generation = self.shared.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.generation = generation;
            self.primed = false;
            self.ever_primed = false;
            self.last = 0.0;
            self.fade = 0.0;
        }
        let ready = self.shared.ready.load(Ordering::Acquire);
        if ready
            && self
                .input
                .push(TaggedSample {
                    generation,
                    value: if sample.is_finite() {
                        sample.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    },
                })
                .is_err()
        {
            self.shared
                .dropped_input_samples
                .fetch_add(1, Ordering::Relaxed);
        }
        if !ready {
            self.primed = false;
            self.ever_primed = false;
            self.last = 0.0;
            self.fade = 0.0;
            return Some(0.0);
        }
        let threshold = (self.shared.output_prefill_frames.load(Ordering::Relaxed) as usize)
            .min(self.output.buffer().capacity() * 3 / 4);
        if ready && !self.primed && self.output.slots() < threshold {
            if self.ever_primed {
                self.shared
                    .missing_output_samples
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                self.shared
                    .startup_wait_samples
                    .fetch_add(1, Ordering::Relaxed);
            }
            self.last *= 1.0 - self.fade_step;
            return Some(self.last);
        }
        self.primed = true;
        let sample = self
            .output
            .pop()
            .ok()
            .filter(|v| v.generation == generation && ready)
            .map(|v| v.value);
        if ready && sample.is_none() {
            self.shared
                .missing_output_samples
                .fetch_add(1, Ordering::Relaxed);
            self.primed = false;
            self.fade = 0.0;
            self.last *= 1.0 - self.fade_step;
            return Some(self.last);
        }
        self.ever_primed = true;
        self.fade = (self.fade + self.fade_step).min(1.0);
        self.last = sample.unwrap_or(0.0) * self.fade;
        Some(self.last)
    }
}

/// Streaming linear interpolation is used only at the non-realtime IPC boundary.
/// 48 kHz is exact passthrough. Other rates need listening/aliasing acceptance tests.
pub struct Resampler(crate::audio::resample::StreamResampler);
impl Resampler {
    pub fn new(source: u32, target: u32) -> Result<Self, String> {
        if !(8000..=96000).contains(&source) || !(8000..=96000).contains(&target) {
            return Err("AI transport supports 8–96 kHz mono input/output".into());
        }
        crate::audio::resample::StreamResampler::new(source, target).map(Self)
    }
    pub fn push(&mut self, sample: f32, output: &mut Vec<f32>) {
        self.0.push(sample, output);
    }
}
pub fn to_pcm16(value: f32) -> i16 {
    if !value.is_finite() {
        0
    } else {
        (value.clamp(-1.0, 1.0) * 32768.0)
            .round()
            .clamp(-32768.0, 32767.0) as i16
    }
}

/// Also guards downstream queues: samples queued before an AI switch must not
/// leak the old Classic voice while the sidecar is starting or has failed.
pub struct OutputPrivacyGate {
    shared: Arc<SharedTransport>,
    generation: u64,
}
impl OutputPrivacyGate {
    pub fn new(shared: Arc<SharedTransport>) -> Self {
        let generation = shared.generation.load(Ordering::Acquire);
        Self { shared, generation }
    }
    pub fn discard_if_changed(&mut self, consumer: &mut Consumer<f32>) -> bool {
        let generation = self.shared.generation.load(Ordering::Acquire);
        if generation == self.generation {
            return false;
        }
        self.generation = generation;
        for _ in 0..consumer.slots() {
            let _ = consumer.pop();
        }
        true
    }
    #[inline]
    pub fn filter(&self, sample: f32) -> f32 {
        if !self.shared.ai_selected.load(Ordering::Acquire) {
            return sample;
        }
        if self.shared.faulted.load(Ordering::Acquire)
            && self.shared.classic_fallback.load(Ordering::Acquire)
        {
            return sample;
        }
        if self.shared.ready.load(Ordering::Acquire) {
            sample
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn downstream_gate_discards_queued_classic_audio_on_ai_selection() {
        let shared = Arc::new(SharedTransport::default());
        let mut gate = OutputPrivacyGate::new(shared.clone());
        let (mut tx, mut rx) = RingBuffer::new(16);
        tx.push(0.9).unwrap();
        assert_eq!(gate.filter(0.9), 0.9);
        shared.ai_selected.store(true, Ordering::Release);
        shared.generation.fetch_add(1, Ordering::AcqRel);
        assert!(gate.discard_if_changed(&mut rx));
        assert!(rx.pop().is_err());
        assert_eq!(gate.filter(0.9), 0.0);
        shared.ready.store(true, Ordering::Release);
        assert_eq!(gate.filter(0.4), 0.4);
        shared.ready.store(false, Ordering::Release);
        assert_eq!(gate.filter(0.4), 0.0);
    }
    #[test]
    fn ai_faults_never_implicitly_expose_dry_input() {
        let shared = Arc::new(SharedTransport::default());
        let (mut rt, _) = ports(48000, shared.clone());
        assert!(rt.process(0.7).is_none());
        shared.ai_selected.store(true, Ordering::Release);
        assert_eq!(rt.process(0.7), Some(0.0));
        shared.faulted.store(true, Ordering::Release);
        assert_eq!(rt.process(0.7), Some(0.0));
        shared.classic_fallback.store(true, Ordering::Release);
        assert!(rt.process(0.7).is_none());
    }
    #[test]
    fn bounded_backpressure_and_stale_generation_protection() {
        let shared = Arc::new(SharedTransport::default());
        shared.ai_selected.store(true, Ordering::Release);
        shared.ready.store(true, Ordering::Release);
        shared.generation.store(2, Ordering::Release);
        let (mut rt, mut worker) = ports(48000, shared.clone());
        worker
            .output
            .push(TaggedSample {
                generation: 1,
                value: 0.8,
            })
            .unwrap();
        assert_eq!(rt.process(0.5), Some(0.0));
        for _ in 0..50000 {
            rt.process(0.5);
        }
        assert!(shared.dropped_input_samples.load(Ordering::Relaxed) > 0);
        assert!(shared.missing_output_samples.load(Ordering::Relaxed) > 0);
    }
    #[test]
    fn resampling_preserves_duration_across_chunk_boundaries() {
        for (source, target) in [
            (44100, 48000),
            (48000, 44100),
            (48000, 48000),
            (16000, 48000),
        ] {
            let mut r = Resampler::new(source, target).unwrap();
            let mut out = Vec::new();
            for _ in 0..source {
                r.push(0.5, &mut out);
            }
            assert!(
                out.len().abs_diff(target as usize) < 1024,
                "Streaming tail is retained, not reset/flushed per chunk"
            );
            assert!(out.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        }
    }
    #[test]
    fn pcm_is_clamped_and_non_finite_safe() {
        assert_eq!(to_pcm16(-1.0), i16::MIN);
        assert_eq!(to_pcm16(1.0), i16::MAX);
        assert_eq!(to_pcm16(f32::NAN), 0);
    }
}
