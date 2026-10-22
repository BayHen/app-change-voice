use super::PlatformInfo;
use crate::audio::{
    devices::{AudioDeviceInfo, AudioDeviceProvider, BufferCapability, DeviceKind},
    errors::AudioDeviceError,
};
use cpal::{
    Device, SupportedBufferSize,
    traits::{DeviceTrait, HostTrait},
};

#[derive(Debug, Clone, Copy)]
pub struct WindowsPlatform;

impl PlatformInfo for WindowsPlatform {
    fn os_name(&self) -> &'static str {
        "windows"
    }

    fn architecture(&self) -> &'static str {
        std::env::consts::ARCH
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsAudioDeviceProvider;

impl WindowsAudioDeviceProvider {
    fn device_info(
        device: Device,
        kind: DeviceKind,
        default_id: Option<&str>,
    ) -> Result<AudioDeviceInfo, AudioDeviceError> {
        let id = device
            .id()
            .map_err(|error| AudioDeviceError::backend(error.to_string()))?
            .to_string();
        let name = device
            .description()
            .map_err(|error| AudioDeviceError::backend(error.to_string()))?
            .name()
            .to_owned();
        let config = match kind {
            DeviceKind::Input => device.default_input_config(),
            DeviceKind::Output => device.default_output_config(),
        }
        .ok();

        let buffer_capability = config
            .as_ref()
            .and_then(|config| match config.buffer_size() {
                SupportedBufferSize::Range { min, max } => Some(BufferCapability {
                    minimum_frames: Some(*min),
                    maximum_frames: Some(*max),
                }),
                SupportedBufferSize::Unknown => None,
            });

        let (sample_rate, channels, sample_format) = config
            .map(|config| {
                (
                    Some(config.sample_rate()),
                    Some(config.channels()),
                    Some(config.sample_format().to_string()),
                )
            })
            .unwrap_or((None, None, None));
        let fingerprint = format!(
            "{}|{:?}|{}|{}|{}",
            name.trim().to_lowercase(),
            kind,
            sample_rate.unwrap_or_default(),
            channels.unwrap_or_default(),
            sample_format.as_deref().unwrap_or("unknown")
        );

        Ok(AudioDeviceInfo {
            is_default: default_id == Some(id.as_str()),
            id,
            name,
            kind,
            sample_rate,
            channels,
            sample_format,
            buffer_capability,
            fingerprint,
        })
    }

    fn device_id(device: Option<Device>) -> Option<String> {
        device
            .and_then(|device| device.id().ok())
            .map(|id| id.to_string())
    }
}

impl AudioDeviceProvider for WindowsAudioDeviceProvider {
    fn enumerate(&self) -> Result<(Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>), AudioDeviceError> {
        let host = cpal::default_host();
        let default_input = Self::device_id(host.default_input_device());
        let default_output = Self::device_id(host.default_output_device());

        let inputs = host
            .input_devices()
            .map_err(|error| AudioDeviceError::backend(error.to_string()))?
            .filter_map(|device| {
                Self::device_info(device, DeviceKind::Input, default_input.as_deref()).ok()
            })
            .collect();
        let outputs = host
            .output_devices()
            .map_err(|error| AudioDeviceError::backend(error.to_string()))?
            .filter_map(|device| {
                Self::device_info(device, DeviceKind::Output, default_output.as_deref()).ok()
            })
            .collect();
        Ok((inputs, outputs))
    }
}

#[cfg(test)]
mod manual_hardware_tests {
    use super::*;

    #[test]
    #[ignore = "manual hardware test: requires an attached Windows audio input"]
    fn windows_has_at_least_one_input_device() {
        let (inputs, _) = WindowsAudioDeviceProvider
            .enumerate()
            .expect("WASAPI enumeration failed");
        assert!(!inputs.is_empty(), "No Windows input device was reported");
    }

    #[test]
    #[ignore = "manual hardware test: depends on the attached Windows audio devices"]
    fn windows_devices_expose_unique_cpal_ids() {
        let (inputs, outputs) = WindowsAudioDeviceProvider
            .enumerate()
            .expect("WASAPI enumeration failed");
        let mut ids = std::collections::HashSet::new();
        for device in inputs.iter().chain(outputs.iter()) {
            assert!(
                ids.insert((&device.kind, &device.id)),
                "Duplicate CPAL device ID"
            );
        }
    }
}
