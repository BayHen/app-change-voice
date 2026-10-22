use super::{loader, models::VoicePreset, validation};

#[derive(Debug, Clone)]
pub struct PresetRegistry {
    presets: Vec<VoicePreset>,
}

impl PresetRegistry {
    pub fn built_in() -> Result<Self, String> {
        Self::new(loader::load_builtin()?)
    }
    pub fn new(presets: Vec<VoicePreset>) -> Result<Self, String> {
        validation::validate_registry(&presets)?;
        Ok(Self { presets })
    }
    pub fn all(&self) -> &[VoicePreset] {
        &self.presets
    }
    pub fn get(&self, id: &str) -> Result<&VoicePreset, String> {
        self.presets
            .iter()
            .find(|preset| preset.id == id)
            .ok_or_else(|| format!("Preset not found: {id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{loader::load_json, validation};

    #[test]
    fn all_eight_builtins_load_and_validate() {
        let registry = PresetRegistry::built_in().unwrap();
        assert_eq!(registry.all().len(), 8);
        assert_eq!(registry.get("natural").unwrap().name, "Natural Voice");
        assert!(registry.all().iter().all(|preset| preset.built_in));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let preset = PresetRegistry::built_in().unwrap().all()[0].clone();
        assert!(
            PresetRegistry::new(vec![preset.clone(), preset])
                .unwrap_err()
                .contains("Duplicate")
        );
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        let mut preset = PresetRegistry::built_in().unwrap().all()[0].clone();
        preset.pitch_semitones = 100.0;
        assert!(validation::validate_preset(&preset).is_err());
    }

    #[test]
    fn unknown_effect_type_is_rejected_during_schema_load() {
        let json = r#"[{"schemaVersion":1,"id":"bad","name":"Bad","description":"Bad effect","category":"Creative","pitchSemitones":0,"formantSemitones":0,"gain":{"makeupDb":0},"effects":[{"type":"missingProcessor"}],"builtIn":false}]"#;
        assert!(load_json(json).unwrap_err().contains("invalid"));
    }
}
