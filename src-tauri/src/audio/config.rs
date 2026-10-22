use super::{devices::DeviceKind, errors::AudioDeviceError};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredDeviceSelection {
    pub id: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevicePreferences {
    pub input: Option<StoredDeviceSelection>,
    pub output: Option<StoredDeviceSelection>,
}

impl AudioDevicePreferences {
    pub fn selection(&self, kind: DeviceKind) -> Option<&StoredDeviceSelection> {
        match kind {
            DeviceKind::Input => self.input.as_ref(),
            DeviceKind::Output => self.output.as_ref(),
        }
    }

    pub fn set_selection(&mut self, kind: DeviceKind, selection: StoredDeviceSelection) {
        match kind {
            DeviceKind::Input => self.input = Some(selection),
            DeviceKind::Output => self.output = Some(selection),
        }
    }
}

pub fn load(path: &Path) -> Result<AudioDevicePreferences, AudioDeviceError> {
    if !path.exists() {
        return Ok(AudioDevicePreferences::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    serde_json::from_str(&content).map_err(|error| AudioDeviceError::persistence(error.to_string()))
}

pub fn save(path: &Path, preferences: &AudioDevicePreferences) -> Result<(), AudioDeviceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    }
    let content = serde_json::to_vec_pretty(preferences)
        .map_err(|error| AudioDeviceError::persistence(error.to_string()))?;
    fs::write(path, content).map_err(|error| AudioDeviceError::persistence(error.to_string()))
}
