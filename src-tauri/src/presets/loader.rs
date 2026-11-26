use super::models::VoicePreset;

pub const BUILTIN_PRESETS_JSON: &str = include_str!("../../presets/builtin.json");

pub fn load_builtin() -> Result<Vec<VoicePreset>, String> {
    let mut presets = load_json(BUILTIN_PRESETS_JSON)?;
    presets.extend(load_json(include_str!("../../presets/female.json"))?);
    Ok(presets)
}

pub fn load_json(json: &str) -> Result<Vec<VoicePreset>, String> {
    serde_json::from_str(json).map_err(|error| format!("Preset JSON is invalid: {error}"))
}
