use super::errors::VirtualAudioError;
use rtrb::{Consumer, Producer, RingBuffer};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub const BRIDGE_PROTOCOL_VERSION: u32 = 1;
pub const BRIDGE_SAMPLE_RATE: u32 = 48_000;
pub const BRIDGE_PACKET_FRAMES: usize = 480;
const USER_QUEUE_FRAMES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeAudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}
impl BridgeAudioFormat {
    pub const PCM_S32_MONO_48K: Self = Self {
        sample_rate: BRIDGE_SAMPLE_RATE,
        channels: 1,
        bits_per_sample: 32,
    };
    pub fn validate(self) -> Result<(), VirtualAudioError> {
        if self != Self::PCM_S32_MONO_48K {
            return Err(VirtualAudioError::InvalidFormat(format!(
                "expected 48 kHz mono signed PCM32, got {} Hz/{} ch/{} bit",
                self.sample_rate, self.channels, self.bits_per_sample
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeWriteOutcome {
    Written { frames: usize },
    Dropped { frames: usize },
    Disconnected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeStatus {
    pub connected: bool,
    pub active: bool,
    pub queued_frames: u32,
    pub capacity_frames: u32,
    pub written_frames: u64,
    pub dropped_frames: u64,
    pub underrun_frames: u64,
    pub transport_errors: u64,
}

/// This transport runs only on the bridge worker, never in an audio callback.
pub trait AudioBridge: Send {
    fn open(&mut self, format: BridgeAudioFormat) -> Result<(), VirtualAudioError>;
    fn write_packet(&mut self, samples: &[i32]) -> BridgeWriteOutcome;
    fn status(&mut self) -> Result<BridgeStatus, VirtualAudioError>;
    fn reset(&mut self) -> Result<(), VirtualAudioError>;
    fn reconnect(&mut self) -> Result<(), VirtualAudioError>;
    fn close(&mut self);
}

#[derive(Default)]
struct Shared {
    active: AtomicBool,
    connected: AtomicBool,
    queued: AtomicU32,
    capacity: AtomicU32,
    written: AtomicU64,
    dropped: AtomicU64,
    underruns: AtomicU64,
    errors: AtomicU64,
    stop: AtomicBool,
}

pub(crate) struct DriverBridgeProducer {
    producer: Producer<f32>,
    shared: Arc<Shared>,
}
impl DriverBridgeProducer {
    #[inline]
    pub fn push(&mut self, sample: f32) {
        if self.shared.active.load(Ordering::Relaxed) && self.producer.push(sample).is_err() {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub struct DriverBridgeRuntime {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl DriverBridgeRuntime {
    #[cfg(target_os = "windows")]
    pub(crate) fn start() -> Result<(Self, DriverBridgeProducer), VirtualAudioError> {
        let mut bridge = super::windows_bridge::WindowsDriverBridge::connect()?;
        bridge.open(BridgeAudioFormat::PCM_S32_MONO_48K)?;
        let (producer, consumer) = RingBuffer::new(USER_QUEUE_FRAMES);
        let shared = Arc::new(Shared::default());
        shared.connected.store(true, Ordering::Release);
        let worker_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("voicemorph-driver-bridge".into())
            .spawn(move || worker_loop(&mut bridge, consumer, &worker_shared))
            .map_err(|error| VirtualAudioError::Transport(error.to_string()))?;
        Ok((
            Self {
                shared: shared.clone(),
                worker: Some(worker),
            },
            DriverBridgeProducer { producer, shared },
        ))
    }
    pub fn set_active(&self, active: bool) {
        self.shared.active.store(active, Ordering::Release);
    }
    pub fn snapshot(&self) -> BridgeStatus {
        BridgeStatus {
            connected: self.shared.connected.load(Ordering::Acquire),
            active: self.shared.active.load(Ordering::Acquire),
            queued_frames: self.shared.queued.load(Ordering::Relaxed),
            capacity_frames: self.shared.capacity.load(Ordering::Relaxed),
            written_frames: self.shared.written.load(Ordering::Relaxed),
            dropped_frames: self.shared.dropped.load(Ordering::Relaxed),
            underrun_frames: self.shared.underruns.load(Ordering::Relaxed),
            transport_errors: self.shared.errors.load(Ordering::Relaxed),
        }
    }
}
impl Drop for DriverBridgeRuntime {
    fn drop(&mut self) {
        self.shared.active.store(false, Ordering::Release);
        self.shared.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn worker_loop(bridge: &mut dyn AudioBridge, mut consumer: Consumer<f32>, shared: &Shared) {
    let mut packet = [0_i32; BRIDGE_PACKET_FRAMES];
    let mut count = 0;
    let mut was_active = false;
    while !shared.stop.load(Ordering::Acquire) {
        if !shared.active.load(Ordering::Acquire) {
            if was_active {
                let _ = bridge.reset();
                while consumer.pop().is_ok() {}
                was_active = false;
            }
            count = 0;
            thread::park_timeout(Duration::from_millis(2));
            continue;
        }
        was_active = true;
        while count < packet.len() {
            let Ok(sample) = consumer.pop() else { break };
            packet[count] = float_to_pcm32(sample);
            count += 1;
        }
        if count == packet.len() {
            match bridge.write_packet(&packet) {
                BridgeWriteOutcome::Written { frames } => {
                    shared.written.fetch_add(frames as u64, Ordering::Relaxed);
                }
                BridgeWriteOutcome::Dropped { frames } => {
                    shared.dropped.fetch_add(frames as u64, Ordering::Relaxed);
                }
                BridgeWriteOutcome::Disconnected => {
                    shared.connected.store(false, Ordering::Release);
                    shared.errors.fetch_add(1, Ordering::Relaxed);
                    while shared.active.load(Ordering::Acquire)
                        && !shared.stop.load(Ordering::Acquire)
                    {
                        thread::sleep(Duration::from_millis(250));
                        if bridge.reconnect().is_ok() {
                            shared.connected.store(true, Ordering::Release);
                            count = 0;
                            break;
                        }
                    }
                    if !shared.connected.load(Ordering::Acquire) {
                        continue;
                    }
                }
            }
            count = 0;
            if let Ok(status) = bridge.status() {
                shared.queued.store(status.queued_frames, Ordering::Relaxed);
                shared
                    .capacity
                    .store(status.capacity_frames, Ordering::Relaxed);
                shared
                    .underruns
                    .store(status.underrun_frames, Ordering::Relaxed);
            }
        } else {
            thread::park_timeout(Duration::from_millis(1));
        }
    }
    bridge.close();
    shared.connected.store(false, Ordering::Release);
}

#[inline]
fn float_to_pcm32(sample: f32) -> i32 {
    let clean = if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    (clean * i32::MAX as f32).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contract_accepts_only_canonical_format() {
        assert!(BridgeAudioFormat::PCM_S32_MONO_48K.validate().is_ok());
        assert!(
            BridgeAudioFormat {
                sample_rate: 44_100,
                ..BridgeAudioFormat::PCM_S32_MONO_48K
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn non_finite_samples_convert_to_silence() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(float_to_pcm32(value), 0);
        }
    }

    #[test]
    fn known_float_signal_converts_to_pcm32_and_clamps() {
        assert_eq!(float_to_pcm32(0.0), 0);
        assert_eq!(float_to_pcm32(1.5), i32::MAX);
        assert_eq!(float_to_pcm32(-1.5), i32::MIN);
        assert!((float_to_pcm32(0.5) - 1_073_741_824).abs() <= 64);
    }
}
