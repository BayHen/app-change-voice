use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QualityProfile {
    LowLatency,
    #[default]
    Balanced,
    HighQuality,
}
impl QualityProfile {
    /// The worker/RNNoise batching and two independent WASAPI clocks need several native periods.
    /// Do not promise a 10 ms prefill when the endpoint itself only supports 10 ms callbacks.
    pub fn negotiated_startup_ms(self, rate: u32, buffer: cpal::BufferSize) -> u32 {
        let frames = match buffer {
            cpal::BufferSize::Fixed(frames) => frames,
            _ => 512,
        };
        self.startup_ms().max((frames * 4000).div_ceil(rate.max(1)))
    }
    pub fn startup_ms(self) -> u32 {
        match self {
            Self::LowLatency => 10,
            Self::Balanced => 20,
            Self::HighQuality => 40,
        }
    }
    pub fn taps(self) -> usize {
        match self {
            Self::LowLatency => 64,
            Self::Balanced => 128,
            Self::HighQuality => 256,
        }
    }
    pub fn requested_frames(self, rate: u32) -> u32 {
        let base = match self {
            Self::LowLatency => 128,
            Self::Balanced => 256,
            Self::HighQuality => 512,
        };
        (base * rate / 48000).max(64)
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct QualitySettings {
    pub profile: QualityProfile,
}
pub fn load(path: &Path) -> QualitySettings {
    std::fs::read(path)
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_default()
}
pub fn save(path: &Path, value: QualitySettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.new");
    std::fs::write(
        &temp,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    crate::ai::runtime::replace_file(&temp, path)
}
