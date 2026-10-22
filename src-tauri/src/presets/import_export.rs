use super::{
    models::VoicePreset,
    repository::{atomic_write, load_file},
};
use std::path::Path;
pub fn import(path: &Path) -> Result<VoicePreset, String> {
    let mut preset = load_file(path)?;
    preset.built_in = false;
    Ok(preset)
}
pub fn export(path: &Path, preset: &VoicePreset) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(preset).map_err(|e| e.to_string())?;
    atomic_write(path, &bytes)
}
