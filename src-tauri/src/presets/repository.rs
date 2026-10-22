use super::{migration, models::VoicePreset, validation::validate_preset};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const MAX_PRESET_BYTES: u64 = 256 * 1024;

#[derive(Debug, Default)]
pub struct LoadReport {
    pub presets: Vec<VoicePreset>,
    pub warnings: Vec<String>,
}
pub trait PresetRepository: Send {
    fn load_all(&self) -> Result<LoadReport, String>;
    fn save(&self, preset: &VoicePreset) -> Result<(), String>;
    fn delete(&self, id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct JsonPresetRepository {
    root: PathBuf,
}
impl JsonPresetRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        Ok(self.root.join(format!("{id}.json")))
    }
}

impl PresetRepository for JsonPresetRepository {
    fn load_all(&self) -> Result<LoadReport, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let mut report = LoadReport::default();
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let path = match entry {
                Ok(e) => e.path(),
                Err(e) => {
                    report.warnings.push(e.to_string());
                    continue;
                }
            };
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            match load_file(&path) {
                Ok(p) => report.presets.push(p),
                Err(e) => report.warnings.push(format!("{}: {e}", path.display())),
            }
        }
        Ok(report)
    }
    fn save(&self, preset: &VoicePreset) -> Result<(), String> {
        validate_preset(preset)?;
        if preset.built_in {
            return Err("Built-in presets cannot be stored as user presets".into());
        }
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = self.path(&preset.id)?;
        let bytes = serde_json::to_vec_pretty(preset).map_err(|e| e.to_string())?;
        atomic_write(&path, &bytes)
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.path(id)?;
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?
        }
        Ok(())
    }
}

pub fn load_file(path: &Path) -> Result<VoicePreset, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_PRESET_BYTES {
        return Err(format!(
            "Preset exceeds {} KiB limit",
            MAX_PRESET_BYTES / 1024
        ));
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let preset: VoicePreset =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid preset JSON: {e}"))?;
    let preset = migration::migrate(preset)?;
    validate_preset(&preset)?;
    Ok(preset)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid destination path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup).map_err(|e| e.to_string())?;
    }
    match fs::rename(&temp, path) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = fs::rename(&backup, path);
            }
            let _ = fs::remove_file(temp);
            Err(error.to_string())
        }
    }
}
fn validate_id(id: &str) -> Result<(), String> {
    if !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        Ok(())
    } else {
        Err("Invalid preset ID; path traversal is not allowed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::PresetRegistry;
    fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("voicemorph-presets-{}-{name}", std::process::id()))
    }
    #[test]
    fn save_load_delete_are_isolated() {
        let root = dir("crud");
        let repo = JsonPresetRepository::new(root.clone());
        let mut a = PresetRegistry::built_in().unwrap().all()[0].clone();
        a.id = "user-a".into();
        a.built_in = false;
        a.schema_version = 2;
        repo.save(&a).unwrap();
        let mut b = a.clone();
        b.id = "user-b".into();
        repo.save(&b).unwrap();
        repo.delete("user-a").unwrap();
        let report = repo.load_all().unwrap();
        assert_eq!(report.presets.len(), 1);
        assert_eq!(report.presets[0].id, "user-b");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupted_file_does_not_hide_valid_presets() {
        let root = dir("corrupt");
        let repo = JsonPresetRepository::new(root.clone());
        let mut p = PresetRegistry::built_in().unwrap().all()[0].clone();
        p.id = "valid".into();
        p.built_in = false;
        p.schema_version = 2;
        repo.save(&p).unwrap();
        fs::write(root.join("broken.json"), "{").unwrap();
        let report = repo.load_all().unwrap();
        assert_eq!(report.presets.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn path_traversal_is_rejected() {
        assert!(
            JsonPresetRepository::new(dir("path"))
                .delete("../escape")
                .is_err()
        );
    }
}
