use serde::{Deserialize, Serialize};

pub const AI_MODEL_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AIModelMetadata {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub architecture: String,
    pub backend: String,
    pub license: String,
    pub source_url: String,
    pub model_file: String,
    pub sha256: String,
    pub input_sample_rate: u32,
    pub frame_samples: usize,
    #[serde(default)]
    pub supported_profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AIModelStatus {
    #[serde(flatten)]
    pub metadata: AIModelMetadata,
    pub enabled: bool,
    pub integrity_verified: bool,
    pub runtime_available: bool,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderStatus {
    pub offline_only: bool,
    pub inference_backend: String,
    pub gpu_verified: bool,
    pub active_model_id: Option<String>,
    pub fallback_active: bool,
    pub message: String,
}
