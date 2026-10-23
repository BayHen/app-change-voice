use crate::{
    audio::noise::NoiseSettings,
    effects::gain::VolumeSettings,
    hotkeys::config::HotkeyConfiguration,
    presets::{VoicePreset, validation::validate_preset},
    profiles::ApplicationProfile,
    soundboard::SoundRoute,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

pub const BACKUP_SCHEMA_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableSoundboardEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub favorite: bool,
    pub volume_percent: f32,
    pub route: SoundRoute,
    pub hotkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    pub backup_schema_version: u32,
    pub application_version: String,
    pub created_at: String,
    pub presets: Vec<VoicePreset>,
    pub hotkeys: HotkeyConfiguration,
    pub volume_settings: VolumeSettings,
    pub noise_settings: NoiseSettings,
    pub application_profiles: Vec<ApplicationProfile>,
    pub soundboard_metadata: Vec<PortableSoundboardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresetPack {
    pub schema_version: u32,
    pub application_version: String,
    pub presets: Vec<VoicePreset>,
}
pub fn write_presets(path: &Path, presets: &[VoicePreset]) -> Result<(), String> {
    let pack = PresetPack {
        schema_version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        presets: presets.to_vec(),
    };
    for preset in &pack.presets {
        validate_preset(preset)?;
        if preset.built_in {
            return Err("Built-in presets are already included with the application".into());
        }
    }
    let parent = path.parent().ok_or("Invalid preset pack destination")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("preset-pack.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?
    }
    fs::rename(temp, path).map_err(|e| e.to_string())
}
pub fn read_presets(path: &Path) -> Result<Vec<VoicePreset>, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_BACKUP_BYTES {
        return Err("Preset pack exceeds 4 MiB limit".into());
    }
    let pack: PresetPack = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid preset pack: {e}"))?;
    if pack.schema_version != 1 {
        return Err("Unsupported preset pack schema".into());
    }
    let mut ids = HashSet::new();
    for preset in &pack.presets {
        validate_preset(preset)?;
        if preset.built_in || !ids.insert(&preset.id) {
            return Err("Preset pack contains built-in or duplicate IDs".into());
        }
    }
    Ok(pack.presets)
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreMode {
    Merge,
    Replace,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub application_version: String,
    pub created_at: String,
    pub preset_count: usize,
    pub profile_count: usize,
    pub sound_count: usize,
    pub preset_conflicts: Vec<String>,
    pub profile_conflicts: Vec<String>,
    pub portable: bool,
    pub warnings: Vec<String>,
}

pub fn validate(value: &BackupManifest) -> Result<(), String> {
    if value.backup_schema_version != BACKUP_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported backup schema version: {}",
            value.backup_schema_version
        ));
    }
    if value.application_version.trim().is_empty() || value.created_at.trim().is_empty() {
        return Err("Backup identity metadata is required".into());
    }
    let mut ids = HashSet::new();
    for preset in &value.presets {
        validate_preset(preset)?;
        if preset.built_in {
            return Err("Backup must not contain built-in presets".into());
        }
        if !ids.insert(&preset.id) {
            return Err(format!("Duplicate preset ID: {}", preset.id));
        }
    }
    crate::hotkeys::config::validate(&value.hotkeys)?;
    if value.volume_settings != value.volume_settings.validated() {
        return Err("Invalid volume settings".into());
    }
    if value.noise_settings != value.noise_settings.validated() {
        return Err("Invalid noise settings".into());
    }
    crate::profiles::validate_all(&value.application_profiles)?;
    let mut sound_ids = HashSet::new();
    for sound in &value.soundboard_metadata {
        if sound.id.trim().is_empty() || sound.name.trim().is_empty() {
            return Err("Soundboard metadata identity is required".into());
        }
        if !sound.volume_percent.is_finite() || !(0.0..=100.0).contains(&sound.volume_percent) {
            return Err("Invalid soundboard volume".into());
        }
        if !sound_ids.insert(&sound.id) {
            return Err(format!("Duplicate sound ID: {}", sound.id));
        }
    }
    Ok(())
}

pub fn read(path: &Path) -> Result<BackupManifest, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_BACKUP_BYTES {
        return Err("Backup exceeds 4 MiB limit".into());
    }
    let value: BackupManifest = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid backup JSON: {e}"))?;
    validate(&value)?;
    Ok(value)
}
pub fn write(path: &Path, value: &BackupManifest) -> Result<(), String> {
    validate(value)?;
    let parent = path.parent().ok_or("Invalid backup destination")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("vms-backup.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?
    }
    fs::rename(temp, path).map_err(|e| e.to_string())
}

pub trait SyncProvider {
    fn status(&self) -> SyncStatus;
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub provider: String,
    pub enabled: bool,
    pub state: String,
    pub last_synced_at: Option<String>,
    pub message: String,
}
pub struct LocalOnlySyncProvider;
impl SyncProvider for LocalOnlySyncProvider {
    fn status(&self) -> SyncStatus {
        SyncStatus {
            provider: "local".into(),
            enabled: false,
            state: "deferred".into(),
            last_synced_at: None,
            message: "Cloud sync is deferred; no audio, credentials or configuration is uploaded."
                .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{presets::PresetRegistry, profiles::ProfileRepository};
    fn sample() -> BackupManifest {
        let mut preset = PresetRegistry::built_in().unwrap().all()[0].clone();
        preset.id = "portable".into();
        preset.built_in = false;
        let path =
            std::env::temp_dir().join(format!("profiles-backup-{}.json", std::process::id()));
        let profiles = ProfileRepository::load(path).unwrap().all();
        BackupManifest {
            backup_schema_version: 1,
            application_version: "0.1.0".into(),
            created_at: "1".into(),
            presets: vec![preset],
            hotkeys: HotkeyConfiguration::default(),
            volume_settings: VolumeSettings::default(),
            noise_settings: NoiseSettings::default(),
            application_profiles: profiles,
            soundboard_metadata: vec![],
        }
    }
    #[test]
    fn validates_portable_backup() {
        validate(&sample()).unwrap()
    }
    #[test]
    fn rejects_future_schema() {
        let mut b = sample();
        b.backup_schema_version = 99;
        assert!(validate(&b).is_err())
    }
    #[test]
    fn rejects_partial_backup() {
        let path =
            std::env::temp_dir().join(format!("vm-partial-backup-{}.json", std::process::id()));
        fs::write(
            &path,
            r#"{"backupSchemaVersion":1,"applicationVersion":"0.1.0"}"#,
        )
        .unwrap();
        assert!(read(&path).is_err());
        let _ = fs::remove_file(path);
    }
    #[test]
    fn round_trip_file() {
        let p = std::env::temp_dir().join(format!("vm-backup-{}.json", std::process::id()));
        write(&p, &sample()).unwrap();
        assert_eq!(read(&p).unwrap().presets.len(), 1);
        let _ = fs::remove_file(p);
    }
}
