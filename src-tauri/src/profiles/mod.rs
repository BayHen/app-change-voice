use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf};

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ApplicationKind {
    Valorant,
    Discord,
    Obs,
    Zoom,
    Teams,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationProfile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub application: ApplicationKind,
    pub preset_id: String,
    pub noise_suppression: bool,
    pub limiter_enabled: bool,
    pub soundboard_enabled: bool,
    pub soundboard_volume_percent: f32,
    pub clarity_priority: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileFile {
    schema_version: u32,
    profiles: Vec<ApplicationProfile>,
}

pub struct ProfileRepository {
    path: PathBuf,
    profiles: Vec<ApplicationProfile>,
}
impl ProfileRepository {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let profiles = if path.exists() {
            let data: ProfileFile =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Invalid profile file: {e}"))?;
            if data.schema_version != PROFILE_SCHEMA_VERSION {
                return Err("Unsupported application profile schema".into());
            }
            data.profiles
        } else {
            defaults()
        };
        validate_all(&profiles)?;
        Ok(Self { path, profiles })
    }
    pub fn all(&self) -> Vec<ApplicationProfile> {
        self.profiles.clone()
    }
    pub fn save(&mut self, mut profile: ApplicationProfile) -> Result<ApplicationProfile, String> {
        profile.schema_version = PROFILE_SCHEMA_VERSION;
        validate(&profile)?;
        if let Some(existing) = self.profiles.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile.clone()
        } else {
            self.profiles.push(profile.clone())
        }
        self.persist()?;
        Ok(profile)
    }
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if before == self.profiles.len() {
            return Err("Profile not found".into());
        }
        self.persist()
    }
    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        let temp = self.path.with_extension("tmp");
        let data = ProfileFile {
            schema_version: PROFILE_SCHEMA_VERSION,
            profiles: self.profiles.clone(),
        };
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|e| e.to_string())?
        }
        fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }
}
fn validate(p: &ApplicationProfile) -> Result<(), String> {
    if p.id.is_empty()
        || !p
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Profile ID must contain lowercase ASCII, digits or hyphens".into());
    }
    if p.name.trim().is_empty() || p.preset_id.trim().is_empty() {
        return Err("Profile name and preset are required".into());
    }
    if !p.soundboard_volume_percent.is_finite()
        || !(0.0..=100.0).contains(&p.soundboard_volume_percent)
    {
        return Err("Soundboard volume must be between 0 and 100".into());
    }
    Ok(())
}
fn validate_all(items: &[ApplicationProfile]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for p in items {
        validate(p)?;
        if !ids.insert(&p.id) {
            return Err(format!("Duplicate profile ID: {}", p.id));
        }
    }
    Ok(())
}
fn defaults() -> Vec<ApplicationProfile> {
    vec![
        profile(
            "valorant",
            "VALORANT Voice Chat",
            ApplicationKind::Valorant,
            "natural",
            true,
            true,
            false,
            25.0,
            true,
        ),
        profile(
            "discord",
            "Discord",
            ApplicationKind::Discord,
            "natural",
            true,
            true,
            true,
            60.0,
            false,
        ),
        profile(
            "obs",
            "OBS Broadcast",
            ApplicationKind::Obs,
            "radio",
            false,
            true,
            true,
            70.0,
            true,
        ),
    ]
}
#[allow(clippy::too_many_arguments)]
fn profile(
    id: &str,
    name: &str,
    application: ApplicationKind,
    preset: &str,
    noise: bool,
    limiter: bool,
    soundboard: bool,
    volume: f32,
    clarity: bool,
) -> ApplicationProfile {
    ApplicationProfile {
        schema_version: PROFILE_SCHEMA_VERSION,
        id: id.into(),
        name: name.into(),
        application,
        preset_id: preset.into(),
        noise_suppression: noise,
        limiter_enabled: limiter,
        soundboard_enabled: soundboard,
        soundboard_volume_percent: volume,
        clarity_priority: clarity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_valid() {
        validate_all(&defaults()).unwrap()
    }
    #[test]
    fn persistence_survives_restart() {
        let dir = std::env::temp_dir().join(format!("vm-profiles-{}", std::process::id()));
        let path = dir.join("profiles.json");
        let mut r = ProfileRepository::load(path.clone()).unwrap();
        let mut p = r.all()[0].clone();
        p.name = "Gaming".into();
        r.save(p).unwrap();
        assert_eq!(
            ProfileRepository::load(path).unwrap().all()[0].name,
            "Gaming"
        );
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn invalid_volume_is_rejected() {
        let mut p = defaults()[0].clone();
        p.soundboard_volume_percent = 101.0;
        assert!(validate(&p).is_err())
    }
}
