use super::models::{PRESET_SCHEMA_VERSION, VoicePreset};

pub fn migrate(mut preset: VoicePreset) -> Result<VoicePreset, String> {
    match preset.schema_version {
        1 => {
            preset.schema_version = PRESET_SCHEMA_VERSION;
            Ok(preset)
        }
        PRESET_SCHEMA_VERSION => Ok(preset),
        version => Err(format!("Unsupported preset schema version: {version}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::PresetRegistry;
    #[test]
    fn migrates_v1_and_rejects_future_schema() {
        let mut p = PresetRegistry::built_in().unwrap().all()[0].clone();
        assert_eq!(migrate(p.clone()).unwrap().schema_version, 2);
        p.schema_version = 999;
        assert!(migrate(p).unwrap_err().contains("Unsupported"));
    }
}
