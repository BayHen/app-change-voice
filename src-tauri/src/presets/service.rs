use super::{
    import_export,
    models::{PRESET_SCHEMA_VERSION, VoicePreset},
    repository::PresetRepository,
    validation::{validate_preset, validate_registry},
};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct UserPresetService {
    repository: Box<dyn PresetRepository>,
    presets: Vec<VoicePreset>,
    warnings: Vec<String>,
}
impl UserPresetService {
    pub fn new(repository: impl PresetRepository + 'static) -> Result<Self, String> {
        let report = repository.load_all()?;
        validate_registry(&report.presets)?;
        Ok(Self {
            repository: Box::new(repository),
            presets: report.presets,
            warnings: report.warnings,
        })
    }
    pub fn all(&self) -> &[VoicePreset] {
        &self.presets
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    pub fn get(&self, id: &str) -> Result<&VoicePreset, String> {
        self.presets
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("User preset not found: {id}"))
    }
    pub fn replace_all(&mut self, presets: Vec<VoicePreset>) -> Result<(), String> {
        validate_registry(&presets)?;
        if presets.iter().any(|preset| preset.built_in) {
            return Err("Built-in presets cannot be restored as user presets".into());
        }
        let previous = self.presets.clone();
        for preset in &presets {
            if let Err(error) = self.repository.save(preset) {
                for old in &previous {
                    let _ = self.repository.save(old);
                }
                return Err(error);
            }
        }
        for old in &previous {
            if !presets.iter().any(|preset| preset.id == old.id)
                && let Err(error) = self.repository.delete(&old.id)
            {
                for value in &previous {
                    let _ = self.repository.save(value);
                }
                return Err(error);
            }
        }
        self.presets = presets;
        Ok(())
    }
    pub fn create(
        &mut self,
        mut preset: VoicePreset,
        builtin_ids: &[&str],
    ) -> Result<VoicePreset, String> {
        preset.id = unique_id(&preset.name, &self.presets, builtin_ids);
        preset.schema_version = PRESET_SCHEMA_VERSION;
        preset.built_in = false;
        let now = timestamp();
        preset.created_at = now.clone();
        preset.updated_at = now;
        validate_preset(&preset)?;
        self.repository.save(&preset)?;
        self.presets.push(preset.clone());
        Ok(preset)
    }
    pub fn update(&mut self, mut preset: VoicePreset) -> Result<VoicePreset, String> {
        if preset.built_in {
            return Err("Built-in presets are read-only; duplicate one before editing".into());
        }
        let index = self
            .presets
            .iter()
            .position(|p| p.id == preset.id)
            .ok_or_else(|| format!("User preset not found: {}", preset.id))?;
        preset.schema_version = PRESET_SCHEMA_VERSION;
        preset.created_at = self.presets[index].created_at.clone();
        preset.updated_at = timestamp();
        validate_preset(&preset)?;
        self.repository.save(&preset)?;
        self.presets[index] = preset.clone();
        Ok(preset)
    }
    pub fn duplicate(
        &mut self,
        source: &VoicePreset,
        builtin_ids: &[&str],
    ) -> Result<VoicePreset, String> {
        let mut copy = source.clone();
        copy.name = format!("{} Copy", source.name);
        copy.built_in = false;
        copy.favorite = false;
        self.create(copy, builtin_ids)
    }
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .presets
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| format!("User preset not found: {id}"))?;
        self.repository.delete(id)?;
        self.presets.remove(index);
        Ok(())
    }
    pub fn favorite(&mut self, id: &str, favorite: bool) -> Result<VoicePreset, String> {
        let mut preset = self.get(id)?.clone();
        preset.favorite = favorite;
        self.update(preset)
    }
    pub fn import(&mut self, path: &Path, builtin_ids: &[&str]) -> Result<VoicePreset, String> {
        let preset = import_export::import(path)?;
        if self.presets.iter().any(|p| p.id == preset.id)
            || builtin_ids.contains(&preset.id.as_str())
        {
            return Err(format!("Duplicate preset ID: {}", preset.id));
        }
        self.repository.save(&preset)?;
        self.presets.push(preset.clone());
        Ok(preset)
    }
    pub fn export(&self, id: &str, path: &Path) -> Result<(), String> {
        import_export::export(path, self.get(id)?)
    }
}
fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
fn unique_id(name: &str, users: &[VoicePreset], builtins: &[&str]) -> String {
    let base: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "custom" } else { &base };
    let mut id = base.to_owned();
    let mut suffix = 2;
    while users.iter().any(|p| p.id == id) || builtins.contains(&id.as_str()) {
        id = format!("{base}-{suffix}");
        suffix += 1;
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{PresetRegistry, repository::JsonPresetRepository};
    use std::fs;
    fn setup(name: &str) -> (UserPresetService, Vec<VoicePreset>, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("voicemorph-service-{}-{name}", std::process::id()));
        let built = PresetRegistry::built_in().unwrap().all().to_vec();
        (
            UserPresetService::new(JsonPresetRepository::new(root.clone())).unwrap(),
            built,
            root,
        )
    }
    #[test]
    fn create_update_duplicate_delete_persist() {
        let (mut service, built, root) = setup("crud");
        let ids = built.iter().map(|p| p.id.as_str()).collect::<Vec<_>>();
        let p = service.create(built[0].clone(), &ids).unwrap();
        let mut edited = p.clone();
        edited.name = "Edited".into();
        service.update(edited).unwrap();
        let copy = service.duplicate(&p, &ids).unwrap();
        service.delete(&p.id).unwrap();
        assert_eq!(service.all(), &[copy]);
        let reloaded = UserPresetService::new(JsonPresetRepository::new(root.clone())).unwrap();
        assert_eq!(reloaded.all().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn built_in_update_is_rejected() {
        let (mut service, built, root) = setup("builtin");
        assert!(service.update(built[0].clone()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn export_import_and_duplicate_id_are_validated() {
        let (mut service, built, root) = setup("transfer");
        let ids = built.iter().map(|p| p.id.as_str()).collect::<Vec<_>>();
        let preset = service.create(built[1].clone(), &ids).unwrap();
        let exported = root.join("exported.json");
        service.export(&preset.id, &exported).unwrap();
        service.delete(&preset.id).unwrap();
        let imported = service.import(&exported, &ids).unwrap();
        assert_eq!(imported.id, preset.id);
        assert!(
            service
                .import(&exported, &ids)
                .unwrap_err()
                .contains("Duplicate")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
