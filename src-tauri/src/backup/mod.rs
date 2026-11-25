use crate::{
    audio::noise::NoiseSettings,
    effects::gain::VolumeSettings,
    hotkeys::config::HotkeyConfiguration,
    presets::{VoicePreset, validation::validate_preset},
    profiles::ApplicationProfile,
    soundboard::SoundRoute,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub const BACKUP_SCHEMA_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024;

fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(parent)?;
    let (temporary, mut file) = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".voicemorph-{}-{sequence}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_limited(path: &Path, kind: &str) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err(format!("{kind} exceeds 4 MiB limit"));
    }
    Ok(bytes)
}

fn write_limited(path: &Path, value: &impl Serialize, kind: &str) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err(format!("{kind} exceeds 4 MiB limit"));
    }
    replace_file(path, &bytes).map_err(|error| error.to_string())
}

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
    validate_pack(&pack)?;
    write_limited(path, &pack, "Preset pack")
}
pub fn read_presets(path: &Path) -> Result<Vec<VoicePreset>, String> {
    let pack: PresetPack = serde_json::from_slice(&read_limited(path, "Preset pack")?)
        .map_err(|e| format!("Invalid preset pack: {e}"))?;
    validate_pack(&pack)?;
    Ok(pack.presets)
}

fn validate_pack(pack: &PresetPack) -> Result<(), String> {
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
    Ok(())
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
    let value: BackupManifest = serde_json::from_slice(&read_limited(path, "Backup")?)
        .map_err(|e| format!("Invalid backup JSON: {e}"))?;
    validate(&value)?;
    Ok(value)
}
pub fn write(path: &Path, value: &BackupManifest) -> Result<(), String> {
    validate(value)?;
    write_limited(path, value, "Backup")
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
        let mut updated = sample();
        updated.created_at = "2".into();
        write(&p, &updated).unwrap();
        assert_eq!(read(&p).unwrap(), updated);
        let _ = fs::remove_file(p);
    }

    #[cfg(windows)]
    #[test]
    fn failed_replacement_preserves_backup_and_cleans_temporary_file() {
        use std::os::windows::fs::OpenOptionsExt;

        let dir = std::env::temp_dir().join(format!("vm-locked-backup-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("backup.json");
        write(&path, &sample()).unwrap();
        let previous = fs::read(&path).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(1) // FILE_SHARE_READ, without FILE_SHARE_DELETE.
            .open(&path)
            .unwrap();
        let mut updated = sample();
        updated.created_at = "2".into();
        let result = write(&path, &updated);
        drop(lock);
        let unchanged = fs::read(&path).unwrap() == previous;
        let files = fs::read_dir(&dir).unwrap().count();
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
        assert!(result.is_err());
        assert!(unchanged);
        assert_eq!(files, 1);
    }
}
