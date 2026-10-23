pub mod hotkeys;
pub mod mixer;
pub mod repository;

use serde::{Deserialize, Serialize};

pub const SOUNDBOARD_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SoundRoute {
    Headphones,
    VirtualMicrophone,
    Both,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoundEntry {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub category: String,
    pub favorite: bool,
    pub volume_percent: f32,
    pub route: SoundRoute,
    pub hotkey: Option<String>,
    pub duration_seconds: f32,
    pub missing: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoundboardLibrary {
    pub schema_version: u32,
    pub sounds: Vec<SoundEntry>,
}
impl Default for SoundboardLibrary {
    fn default() -> Self {
        Self {
            schema_version: SOUNDBOARD_SCHEMA_VERSION,
            sounds: vec![],
        }
    }
}
