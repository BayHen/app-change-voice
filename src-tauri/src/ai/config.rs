use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const UPSTREAM_COMMIT: &str = "7e2ce5c4800010bbc7604968bd29b802534fac05";
pub const PROTOCOL_VERSION: u32 = 1;
pub const AI_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailurePolicy {
    #[default]
    Mute,
    ClassicDsp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineConfig {
    #[serde(default)]
    pub hybrid_dsp: bool,
    pub provider: String,
    pub allow_cpu_fallback: bool,
    pub failure_policy: FailurePolicy,
    pub pitch_semitones: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_rate: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<f32>,
    pub f0_method: String,
    pub chunk_frames: usize,
    pub crossfade_frames: usize,
    pub extra_context_frames: usize,
    pub noise_threshold: f32,
    pub cpu_threads: usize,
}
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            hybrid_dsp: false,
            provider: "auto".into(),
            allow_cpu_fallback: false,
            failure_policy: FailurePolicy::Mute,
            pitch_semitones: 0,
            index_rate: None,
            protect: None,
            f0_method: "dio".into(),
            chunk_frames: 4096,
            crossfade_frames: 1024,
            extra_context_frames: 4096,
            noise_threshold: 0.00001,
            cpu_threads: 4,
        }
    }
}
impl EngineConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !["auto", "cpu", "cuda"].contains(&self.provider.as_str()) {
            return Err("Provider must be auto, cpu or cuda; DirectML is not validated".into());
        }
        if !(-12..=12).contains(&self.pitch_semitones) {
            return Err("Pitch must be between -12 and 12 semitones".into());
        }
        if self
            .index_rate
            .is_some_and(|value| !(0.0..=1.0).contains(&value))
            || self
                .protect
                .is_some_and(|value| !(0.0..=0.5).contains(&value))
        {
            return Err("Index rate must be 0..1 and Protect must be 0..0.5".into());
        }
        if !["dio", "harvest", "rmvpe_onnx"].contains(&self.f0_method.as_str()) {
            return Err("Unsupported F0 method".into());
        }
        if ![2048, 4096, 8192, 16384].contains(&self.chunk_frames) {
            return Err("Chunk must be 2048, 4096, 8192 or 16384 frames".into());
        }
        if !(128..=4096).contains(&self.crossfade_frames)
            || self.crossfade_frames > self.chunk_frames
        {
            return Err("Crossfade must be 128..4096 frames and no larger than the chunk".into());
        }
        if self.extra_context_frames > 32768 || !(1..=8).contains(&self.cpu_threads) {
            return Err("Context/CPU thread limit exceeded".into());
        }
        if !self.noise_threshold.is_finite() || !(0.0..=0.1).contains(&self.noise_threshold) {
            return Err("Noise threshold must be finite and 0..0.1".into());
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() > 8192 {
            return Err("AI configuration exceeds size limit".into());
        }
        let value: Self = serde_json::from_reader(file).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        fs::create_dir_all(path.parent().ok_or("Configuration has no parent")?)
            .map_err(|e| e.to_string())?;
        let temporary = path.with_extension("json.new");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        // Windows MoveFileEx provides replacement without deleting the old valid configuration.
        super::runtime::replace_file(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsupported_providers_and_unsafe_parameters() {
        let mut c = EngineConfig::default();
        assert!(c.validate().is_ok());
        c.provider = "directml".into();
        assert!(c.validate().is_err());
        c.provider = "cpu".into();
        c.noise_threshold = f32::NAN;
        assert!(c.validate().is_err());
        c.noise_threshold = 0.0;
        c.crossfade_frames = 16384;
        assert!(c.validate().is_err());
    }
    #[test]
    fn persists_only_valid_configuration() {
        let p = std::env::temp_dir().join(format!(
            "vm-ai-config-{}-{}.json",
            std::process::id(),
            super::super::process::now_ms()
        ));
        let c = EngineConfig::default();
        c.save(&p).unwrap();
        let mut next = c.clone();
        next.pitch_semitones = 2;
        next.save(&p).unwrap();
        assert_eq!(EngineConfig::load(&p).unwrap(), next);
        let _ = fs::remove_file(p);
    }
}
