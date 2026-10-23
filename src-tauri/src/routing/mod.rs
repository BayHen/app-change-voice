//! Cross-platform routing contracts. Platform-specific stream creation stays in `windows`.

pub mod errors;
#[cfg(target_os = "windows")]
pub mod windows;

use rtrb::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::{fs, path::Path};

pub const ROUTING_QUEUE_MILLISECONDS: u32 = 80;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoutingConfiguration {
    pub output_device_id: Option<String>,
    pub output_device_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RoutingState {
    NotConfigured,
    Ready,
    Active,
    Faulted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutingStatus {
    pub state: RoutingState,
    pub output_device_id: Option<String>,
    pub output_device_name: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub underrun_count: u64,
    pub overflow_count: u64,
    pub output_peak_dbfs: f32,
    pub endpoint_verified: bool,
    pub message: Option<String>,
}

pub trait AudioRoutingService {
    fn routing_status(&self) -> RoutingStatus;
    fn stop_routing(&mut self) -> Result<RoutingStatus, errors::RoutingError>;
}

pub trait VirtualDeviceManager {
    fn is_likely_virtual_output(name: &str) -> bool;
}

pub trait VirtualOutputSink {
    fn play(&self) -> Result<(), errors::RoutingError>;
    fn pause(&self) -> Result<(), errors::RoutingError>;
}

#[derive(Default)]
pub(crate) struct SharedRoutingMetrics {
    pub active: AtomicBool,
    pub faulted: AtomicBool,
    pub underruns: AtomicU64,
    pub overflows: AtomicU64,
    peak_bits: AtomicU64,
}

impl SharedRoutingMetrics {
    pub fn set_peak(&self, peak: f32) {
        self.peak_bits
            .store(peak.to_bits() as u64, Ordering::Relaxed);
    }
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.load(Ordering::Relaxed) as u32)
    }
}

pub(crate) struct RoutingProducer {
    producer: Producer<f32>,
    shared: Arc<SharedRoutingMetrics>,
}

impl RoutingProducer {
    #[inline]
    pub fn push(&mut self, sample: f32) {
        if self.shared.active.load(Ordering::Relaxed) && self.producer.push(sample).is_err() {
            self.shared.overflows.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(crate) fn routing_queue(
    sample_rate: u32,
) -> (RoutingProducer, Consumer<f32>, Arc<SharedRoutingMetrics>) {
    let capacity = ((sample_rate * ROUTING_QUEUE_MILLISECONDS / 1_000) as usize)
        .max(1024)
        .next_power_of_two();
    let (producer, consumer) = RingBuffer::new(capacity);
    let shared = Arc::new(SharedRoutingMetrics::default());
    (
        RoutingProducer {
            producer,
            shared: shared.clone(),
        },
        consumer,
        shared,
    )
}

pub fn likely_virtual_device(name: &str) -> bool {
    let value = name.to_ascii_lowercase();
    [
        "virtual",
        "cable input",
        "vb-audio",
        "voicemeeter",
        "vac ",
        "line 1",
        "line 2",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

pub fn amplitude_to_dbfs(value: f32) -> f32 {
    if value <= 0.000_015_849 {
        -96.0
    } else {
        20.0 * value.min(1.0).log10()
    }
}

pub fn load_configuration(path: &Path) -> Result<RoutingConfiguration, errors::RoutingError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| errors::RoutingError::new("invalidConfiguration", error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(RoutingConfiguration::default())
        }
        Err(error) => Err(errors::RoutingError::new(
            "configurationRead",
            error.to_string(),
        )),
    }
}

pub fn save_configuration(
    path: &Path,
    configuration: &RoutingConfiguration,
) -> Result<(), errors::RoutingError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| errors::RoutingError::new("configurationWrite", e.to_string()))?;
    }
    let temporary = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(configuration)
        .map_err(|e| errors::RoutingError::new("configurationWrite", e.to_string()))?;
    fs::write(&temporary, data)
        .map_err(|e| errors::RoutingError::new("configurationWrite", e.to_string()))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| errors::RoutingError::new("configurationWrite", e.to_string()))?;
    }
    fs::rename(temporary, path)
        .map_err(|e| errors::RoutingError::new("configurationWrite", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_common_virtual_endpoint_names_without_requiring_one_vendor() {
        assert!(likely_virtual_device(
            "CABLE Input (VB-Audio Virtual Cable)"
        ));
        assert!(likely_virtual_device("Virtual Audio Cable Line 1"));
        assert!(!likely_virtual_device("Speakers (Realtek Audio)"));
    }
    #[test]
    fn inactive_tap_does_not_accumulate_stale_audio() {
        let (mut tap, mut consumer, shared) = routing_queue(48_000);
        tap.push(0.5);
        assert!(consumer.pop().is_err());
        shared.active.store(true, Ordering::Release);
        tap.push(0.5);
        assert_eq!(consumer.pop().unwrap(), 0.5);
    }

    #[test]
    fn routing_configuration_survives_restart_load() {
        let directory = std::env::temp_dir().join(format!(
            "voicemorph-routing-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("audio-routing.json");
        let expected = RoutingConfiguration {
            output_device_id: Some("wasapi:test-cable".into()),
            output_device_fingerprint: Some("test-fingerprint".into()),
        };
        save_configuration(&path, &expected).unwrap();
        assert_eq!(load_configuration(&path).unwrap(), expected);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
