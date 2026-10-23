use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct SoundboardRepository {
    path: PathBuf,
    library: SoundboardLibrary,
}
impl SoundboardRepository {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let mut library = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid soundboard library: {e}"))?
        } else {
            SoundboardLibrary::default()
        };
        if library.schema_version != SOUNDBOARD_SCHEMA_VERSION {
            return Err("Unsupported soundboard schema version".into());
        }
        for sound in &mut library.sounds {
            sound.missing = !Path::new(&sound.file_path).is_file();
        }
        Ok(Self { path, library })
    }
    pub fn list(&self) -> Vec<SoundEntry> {
        self.library.sounds.clone()
    }
    pub fn get(&self, id: &str) -> Option<&SoundEntry> {
        self.library.sounds.iter().find(|s| s.id == id)
    }
    pub fn add(&mut self, file_path: String, name: String) -> Result<SoundEntry, String> {
        let path = Path::new(&file_path);
        if !path.is_file() {
            return Err("Audio file does not exist".into());
        }
        if path
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.eq_ignore_ascii_case("wav"))
            != Some(true)
        {
            return Err("Only WAV files are supported in this version".into());
        }
        let reader = hound::WavReader::open(path).map_err(|e| format!("Invalid WAV: {e}"))?;
        let spec = reader.spec();
        let duration = reader.duration() as f32 / spec.sample_rate as f32;
        let id = format!(
            "sound-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let entry = SoundEntry {
            id,
            name: name.trim().to_owned(),
            file_path,
            category: "General".into(),
            favorite: false,
            volume_percent: 100.0,
            route: SoundRoute::Headphones,
            hotkey: None,
            duration_seconds: duration,
            missing: false,
        };
        validate(&entry)?;
        self.library.sounds.push(entry.clone());
        self.save()?;
        Ok(entry)
    }
    pub fn update(&mut self, entry: SoundEntry) -> Result<SoundEntry, String> {
        validate(&entry)?;
        let target = self
            .library
            .sounds
            .iter_mut()
            .find(|s| s.id == entry.id)
            .ok_or("Sound not found")?;
        *target = entry.clone();
        self.save()?;
        Ok(entry)
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let before = self.library.sounds.len();
        self.library.sounds.retain(|s| s.id != id);
        if before == self.library.sounds.len() {
            return Err("Sound not found".into());
        }
        self.save()
    }
    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        let tmp = self.path.with_extension("tmp");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&self.library).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(tmp, &self.path).map_err(|e| e.to_string())
    }
}
fn validate(s: &SoundEntry) -> Result<(), String> {
    if s.name.trim().is_empty() {
        return Err("Sound name is required".into());
    }
    if !s.volume_percent.is_finite() || !(0.0..=100.0).contains(&s.volume_percent) {
        return Err("Volume must be between 0 and 100".into());
    }
    Ok(())
}
