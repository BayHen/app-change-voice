use crate::audio::errors::AudioDeviceError;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSettings {
    pub input_percent: u8,
    pub output_percent: u8,
    pub input_muted: bool,
    pub output_muted: bool,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            input_percent: 100,
            output_percent: 100,
            input_muted: false,
            output_muted: false,
        }
    }
}

impl VolumeSettings {
    pub fn validated(self) -> Self {
        Self {
            input_percent: self.input_percent.min(100),
            output_percent: self.output_percent.min(100),
            ..self
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedGainControls {
    input_gain_bits: Arc<AtomicU32>,
    output_gain_bits: Arc<AtomicU32>,
    input_muted: Arc<AtomicBool>,
    output_muted: Arc<AtomicBool>,
}

impl SharedGainControls {
    pub fn new(settings: VolumeSettings) -> Self {
        let settings = settings.validated();
        Self {
            input_gain_bits: Arc::new(AtomicU32::new(
                percent_to_gain(settings.input_percent).to_bits(),
            )),
            output_gain_bits: Arc::new(AtomicU32::new(
                percent_to_gain(settings.output_percent).to_bits(),
            )),
            input_muted: Arc::new(AtomicBool::new(settings.input_muted)),
            output_muted: Arc::new(AtomicBool::new(settings.output_muted)),
        }
    }

    #[inline]
    pub fn input_target(&self) -> f32 {
        if self.input_muted.load(Ordering::Relaxed) {
            0.0
        } else {
            f32::from_bits(self.input_gain_bits.load(Ordering::Relaxed))
        }
    }
    #[inline]
    pub fn output_target(&self) -> f32 {
        if self.output_muted.load(Ordering::Relaxed) {
            0.0
        } else {
            f32::from_bits(self.output_gain_bits.load(Ordering::Relaxed))
        }
    }
    pub fn apply(&self, settings: VolumeSettings) {
        let settings = settings.validated();
        self.input_gain_bits.store(
            percent_to_gain(settings.input_percent).to_bits(),
            Ordering::Relaxed,
        );
        self.output_gain_bits.store(
            percent_to_gain(settings.output_percent).to_bits(),
            Ordering::Relaxed,
        );
        self.input_muted
            .store(settings.input_muted, Ordering::Relaxed);
        self.output_muted
            .store(settings.output_muted, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SmoothedGain {
    current: f32,
    coefficient: f32,
}

impl SmoothedGain {
    pub fn new(initial: f32, sample_rate: u32) -> Self {
        let smoothing_samples = (sample_rate as f32 * 0.010).max(1.0);
        Self {
            current: initial,
            coefficient: 1.0 - (-1.0 / smoothing_samples).exp(),
        }
    }

    #[inline]
    pub fn process(&mut self, sample: f32, target: f32) -> f32 {
        let safe_sample = if sample.is_finite() { sample } else { 0.0 };
        let safe_target = if target.is_finite() {
            target.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.current += (safe_target - self.current) * self.coefficient;
        (safe_sample * self.current).clamp(-1.0, 1.0)
    }
}

#[inline]
pub fn percent_to_gain(percent: u8) -> f32 {
    f32::from(percent.min(100)) / 100.0
}

pub fn load(path: &Path) -> Result<VolumeSettings, AudioDeviceError> {
    if !path.exists() {
        return Ok(VolumeSettings::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    let settings: VolumeSettings = serde_json::from_str(&text)
        .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    Ok(settings.validated())
}

pub fn save(path: &Path, settings: VolumeSettings) -> Result<(), AudioDeviceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    }
    let bytes = serde_json::to_vec_pretty(&settings.validated())
        .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    fs::write(path, bytes).map_err(|error| AudioDeviceError::persistence(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_preserves_signal_after_settling() {
        let mut gain = SmoothedGain::new(1.0, 48_000);
        assert_eq!(gain.process(0.5, 1.0), 0.5);
    }

    #[test]
    fn zero_gain_reaches_silence_without_non_finite_values() {
        let mut gain = SmoothedGain::new(0.0, 48_000);
        for sample in [f32::NAN, f32::INFINITY, 0.5] {
            assert!(gain.process(sample, 0.0).is_finite());
        }
        assert_eq!(gain.process(0.5, 0.0), 0.0);
    }

    #[test]
    fn mute_target_is_zero() {
        let controls = SharedGainControls::new(VolumeSettings {
            input_muted: true,
            output_muted: true,
            ..VolumeSettings::default()
        });
        assert_eq!(controls.input_target(), 0.0);
        assert_eq!(controls.output_target(), 0.0);
    }

    #[test]
    fn smoothing_changes_gain_gradually() {
        let mut gain = SmoothedGain::new(1.0, 48_000);
        let first = gain.process(1.0, 0.0);
        assert!(first > 0.0 && first < 1.0);
        let mut previous = first;
        for _ in 0..100 {
            let next = gain.process(1.0, 0.0);
            assert!(next <= previous);
            previous = next;
        }
    }

    #[test]
    fn settings_round_trip_and_clamp_invalid_percentages() {
        let path = std::env::temp_dir().join(format!(
            "voicemorph-volume-settings-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        ));
        let settings = VolumeSettings {
            input_percent: 120,
            output_percent: 75,
            input_muted: true,
            output_muted: false,
        };
        save(&path, settings).expect("settings should save");
        let restored = load(&path).expect("settings should load");
        assert_eq!(restored.input_percent, 100);
        assert_eq!(restored.output_percent, 75);
        assert!(restored.input_muted);
        let _ = std::fs::remove_file(path);
    }
}
