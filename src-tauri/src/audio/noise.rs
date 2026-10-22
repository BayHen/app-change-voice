use nnnoiseless::DenoiseState;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use super::errors::AudioDeviceError;

const RNNOISE_RATE: u32 = 48_000;
const FRAME: usize = DenoiseState::FRAME_SIZE;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NoiseSettings {
    pub enabled: bool,
    pub bypassed: bool,
    pub suppression_strength: f32,
    pub gate_enabled: bool,
    pub gate_threshold_dbfs: f32,
    pub limiter_enabled: bool,
    pub limiter_ceiling_dbfs: f32,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bypassed: false,
            suppression_strength: 0.8,
            gate_enabled: true,
            gate_threshold_dbfs: -52.0,
            limiter_enabled: true,
            limiter_ceiling_dbfs: -1.0,
        }
    }
}

impl NoiseSettings {
    pub fn validated(self) -> Self {
        Self {
            suppression_strength: finite(self.suppression_strength, 0.8).clamp(0.0, 1.0),
            gate_threshold_dbfs: finite(self.gate_threshold_dbfs, -52.0).clamp(-80.0, -20.0),
            limiter_ceiling_dbfs: finite(self.limiter_ceiling_dbfs, -1.0).clamp(-12.0, -0.1),
            ..self
        }
    }
}
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[derive(Debug, Clone)]
pub struct SharedNoiseSettings(Arc<NoiseAtoms>);
#[derive(Debug)]
struct NoiseAtoms {
    enabled: AtomicBool,
    bypassed: AtomicBool,
    strength: AtomicU32,
    gate_enabled: AtomicBool,
    gate_threshold: AtomicU32,
    limiter_enabled: AtomicBool,
    limiter_ceiling: AtomicU32,
}
impl SharedNoiseSettings {
    pub fn new(value: NoiseSettings) -> Self {
        let v = value.validated();
        Self(Arc::new(NoiseAtoms {
            enabled: AtomicBool::new(v.enabled),
            bypassed: AtomicBool::new(v.bypassed),
            strength: AtomicU32::new(v.suppression_strength.to_bits()),
            gate_enabled: AtomicBool::new(v.gate_enabled),
            gate_threshold: AtomicU32::new(v.gate_threshold_dbfs.to_bits()),
            limiter_enabled: AtomicBool::new(v.limiter_enabled),
            limiter_ceiling: AtomicU32::new(v.limiter_ceiling_dbfs.to_bits()),
        }))
    }
    pub fn store(&self, value: NoiseSettings) -> NoiseSettings {
        let v = value.validated();
        self.0.enabled.store(v.enabled, Ordering::Release);
        self.0.bypassed.store(v.bypassed, Ordering::Release);
        self.0
            .strength
            .store(v.suppression_strength.to_bits(), Ordering::Relaxed);
        self.0.gate_enabled.store(v.gate_enabled, Ordering::Relaxed);
        self.0
            .gate_threshold
            .store(v.gate_threshold_dbfs.to_bits(), Ordering::Relaxed);
        self.0
            .limiter_enabled
            .store(v.limiter_enabled, Ordering::Relaxed);
        self.0
            .limiter_ceiling
            .store(v.limiter_ceiling_dbfs.to_bits(), Ordering::Relaxed);
        v
    }
    pub fn load(&self) -> NoiseSettings {
        NoiseSettings {
            enabled: self.0.enabled.load(Ordering::Acquire),
            bypassed: self.0.bypassed.load(Ordering::Acquire),
            suppression_strength: f32::from_bits(self.0.strength.load(Ordering::Relaxed)),
            gate_enabled: self.0.gate_enabled.load(Ordering::Relaxed),
            gate_threshold_dbfs: f32::from_bits(self.0.gate_threshold.load(Ordering::Relaxed)),
            limiter_enabled: self.0.limiter_enabled.load(Ordering::Relaxed),
            limiter_ceiling_dbfs: f32::from_bits(self.0.limiter_ceiling.load(Ordering::Relaxed)),
        }
    }
}

pub fn load(path: &Path) -> Result<NoiseSettings, AudioDeviceError> {
    if !path.exists() {
        return Ok(NoiseSettings::default());
    }
    let text = fs::read_to_string(path).map_err(|e| AudioDeviceError::backend(e.to_string()))?;
    serde_json::from_str::<NoiseSettings>(&text)
        .map(NoiseSettings::validated)
        .map_err(|e| AudioDeviceError::backend(e.to_string()))
}
pub fn save(path: &Path, settings: NoiseSettings) -> Result<(), AudioDeviceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AudioDeviceError::backend(e.to_string()))?
    }
    let text = serde_json::to_string_pretty(&settings.validated())
        .map_err(|e| AudioDeviceError::backend(e.to_string()))?;
    fs::write(path, text).map_err(|e| AudioDeviceError::backend(e.to_string()))
}

/// Fixed-storage 48 kHz RNNoise adapter. For uncommon device rates, a bounded
/// streaming rate adapter preserves the device clock without changing CPAL's rate.
pub struct NoiseProcessor {
    state: Box<DenoiseState<'static>>,
    controls: SharedNoiseSettings,
    sample_rate: u32,
    input: [f32; FRAME],
    dry: [f32; FRAME],
    wet: [f32; FRAME],
    len: usize,
    input_phase: u64,
    output_phase: u64,
    gate_gain: f32,
}
impl NoiseProcessor {
    pub fn new(sample_rate: u32, controls: SharedNoiseSettings) -> Self {
        Self {
            state: DenoiseState::new(),
            controls,
            sample_rate,
            input: [0.0; FRAME],
            dry: [0.0; FRAME],
            wet: [0.0; FRAME],
            len: 0,
            input_phase: 0,
            output_phase: 0,
            gate_gain: 1.0,
        }
    }
    pub fn latency_frames(&self) -> usize {
        ((FRAME as u64 * self.sample_rate as u64) / RNNOISE_RATE as u64) as usize
    }
    #[inline]
    pub fn push(&mut self, sample: f32, mut emit: impl FnMut(f32)) {
        let clean = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.input_phase += RNNOISE_RATE as u64;
        while self.input_phase >= self.sample_rate as u64 {
            self.input_phase -= self.sample_rate as u64;
            self.input[self.len] = clean * 32768.0;
            self.dry[self.len] = clean;
            self.len += 1;
            if self.len == FRAME {
                self.process_frame(&mut emit);
                self.len = 0;
            }
        }
    }
    fn process_frame(&mut self, emit: &mut impl FnMut(f32)) {
        let settings = self.controls.load();
        let _speech_probability = self.state.process_frame(&mut self.wet, &self.input);
        let target_mix = if settings.enabled && !settings.bypassed {
            settings.suppression_strength
        } else {
            0.0
        };
        let threshold = 10.0_f32.powf(settings.gate_threshold_dbfs / 20.0);
        for index in 0..FRAME {
            let denoised = (self.wet[index] / 32768.0).clamp(-1.0, 1.0);
            let mut value = self.dry[index] + (denoised - self.dry[index]) * target_mix;
            let gate_target = if !settings.gate_enabled || value.abs() >= threshold {
                1.0
            } else {
                0.0
            };
            self.gate_gain += (gate_target - self.gate_gain)
                * if gate_target > self.gate_gain {
                    0.08
                } else {
                    0.003
                };
            value *= self.gate_gain;
            self.output_phase += self.sample_rate as u64;
            while self.output_phase >= RNNOISE_RATE as u64 {
                self.output_phase -= RNNOISE_RATE as u64;
                emit(if value.is_finite() { value } else { 0.0 });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_invalid_values() {
        let v = NoiseSettings {
            suppression_strength: f32::NAN,
            gate_threshold_dbfs: 3.0,
            limiter_ceiling_dbfs: f32::INFINITY,
            ..Default::default()
        }
        .validated();
        assert_eq!(v.suppression_strength, 0.8);
        assert_eq!(v.gate_threshold_dbfs, -20.0);
        assert_eq!(v.limiter_ceiling_dbfs, -1.0);
    }
    #[test]
    fn silence_is_finite() {
        let c = SharedNoiseSettings::new(NoiseSettings {
            enabled: true,
            ..Default::default()
        });
        let mut p = NoiseProcessor::new(48_000, c);
        let mut out = Vec::new();
        for _ in 0..960 {
            p.push(0.0, |x| out.push(x));
        }
        assert_eq!(out.len(), 960);
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1e-6));
    }
    #[test]
    fn adapts_non_48k_without_changing_duration() {
        let c = SharedNoiseSettings::new(Default::default());
        let mut p = NoiseProcessor::new(44_100, c);
        let mut count = 0;
        for _ in 0..44_100 {
            p.push(0.1, |_| count += 1);
        }
        assert!((count as isize - 43_659).abs() < 500);
    }
}
