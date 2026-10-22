use super::{config::StoredDeviceSelection, errors::AudioDeviceError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BufferCapability {
    pub minimum_frames: Option<u32>,
    pub maximum_frames: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub is_default: bool,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub sample_format: Option<String>,
    pub buffer_capability: Option<BufferCapability>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSelection {
    pub selected_id: Option<String>,
    pub fell_back: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceSnapshot {
    pub input_devices: Vec<AudioDeviceInfo>,
    pub output_devices: Vec<AudioDeviceInfo>,
    pub input_selection: DeviceSelection,
    pub output_selection: DeviceSelection,
}

pub trait AudioDeviceProvider: Send + Sync {
    fn enumerate(&self) -> Result<(Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>), AudioDeviceError>;
}

pub fn resolve_selection(
    devices: &[AudioDeviceInfo],
    stored: Option<&StoredDeviceSelection>,
) -> DeviceSelection {
    if let Some(stored) = stored {
        if let Some(device) = devices.iter().find(|device| device.id == stored.id) {
            return DeviceSelection {
                selected_id: Some(device.id.clone()),
                fell_back: false,
                message: None,
            };
        }
        if let Some(device) = devices
            .iter()
            .find(|device| device.fingerprint == stored.fingerprint)
        {
            return DeviceSelection {
                selected_id: Some(device.id.clone()),
                fell_back: false,
                message: Some(
                    "Device ID changed; restored the matching device configuration.".into(),
                ),
            };
        }
    }

    let fallback = devices
        .iter()
        .find(|device| device.is_default)
        .or_else(|| devices.first());
    DeviceSelection {
        selected_id: fallback.map(|device| device.id.clone()),
        fell_back: stored.is_some(),
        message: stored
            .map(|_| "Saved device is unavailable; using the current default device.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: &str, fingerprint: &str, is_default: bool) -> AudioDeviceInfo {
        AudioDeviceInfo {
            id: id.into(),
            name: id.into(),
            kind: DeviceKind::Input,
            is_default,
            sample_rate: Some(48_000),
            channels: Some(2),
            sample_format: Some("f32".into()),
            buffer_capability: None,
            fingerprint: fingerprint.into(),
        }
    }

    #[test]
    fn selects_one_of_multiple_inputs_by_stable_id() {
        let devices = vec![device("one", "a", true), device("two", "b", false)];
        let stored = StoredDeviceSelection {
            id: "two".into(),
            fingerprint: "b".into(),
        };
        assert_eq!(
            resolve_selection(&devices, Some(&stored))
                .selected_id
                .as_deref(),
            Some("two")
        );
    }

    #[test]
    fn restores_changed_id_using_fingerprint() {
        let devices = vec![device("new-id", "same-hardware", false)];
        let stored = StoredDeviceSelection {
            id: "old-id".into(),
            fingerprint: "same-hardware".into(),
        };
        let result = resolve_selection(&devices, Some(&stored));
        assert_eq!(result.selected_id.as_deref(), Some("new-id"));
        assert!(!result.fell_back);
        assert!(result.message.is_some());
    }

    #[test]
    fn falls_back_to_new_default_when_saved_device_disappears() {
        let devices = vec![
            device("new-default", "new", true),
            device("other", "other", false),
        ];
        let stored = StoredDeviceSelection {
            id: "missing".into(),
            fingerprint: "missing".into(),
        };
        let result = resolve_selection(&devices, Some(&stored));
        assert_eq!(result.selected_id.as_deref(), Some("new-default"));
        assert!(result.fell_back);
    }

    #[test]
    fn handles_no_microphone() {
        let result = resolve_selection(&[], None);
        assert_eq!(result.selected_id, None);
        assert!(!result.fell_back);
    }

    #[test]
    fn follows_a_changed_default_when_no_selection_is_saved() {
        let first = vec![
            device("old-default", "old", false),
            device("new-default", "new", true),
        ];
        assert_eq!(
            resolve_selection(&first, None).selected_id.as_deref(),
            Some("new-default")
        );
    }
}
