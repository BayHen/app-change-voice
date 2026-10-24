#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualAudioDeviceState {
    NotInstalled,
    Installed,
    Disabled,
    Problem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualAudioDeviceInfo {
    pub instance_id: String,
    pub endpoint_id: Option<String>,
    pub friendly_name: String,
    pub driver_version: String,
    pub state: VirtualAudioDeviceState,
}

/// Detection must use the VoiceMorph hardware/device interface identity first.
/// Friendly names are display metadata, never persistent identity.
pub trait VirtualAudioDeviceDetector: Send + Sync {
    fn detect(&self) -> Result<Option<VirtualAudioDeviceInfo>, String>;
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsVirtualDeviceDetector;

#[cfg(target_os = "windows")]
impl VirtualAudioDeviceDetector for WindowsVirtualDeviceDetector {
    fn detect(&self) -> Result<Option<VirtualAudioDeviceInfo>, String> {
        use crate::virtual_audio::windows_bridge::WindowsDriverBridge;
        use crate::virtual_audio::{AudioBridge, VirtualAudioError};
        match WindowsDriverBridge::connect() {
            Ok(mut bridge) => {
                let version = bridge.driver_version();
                bridge.close();
                Ok(Some(VirtualAudioDeviceInfo {
                    instance_id: "{B566F7B7-91D8-4F31-9712-94C55A10E52A}".into(),
                    endpoint_id: None,
                    friendly_name: "VoiceMorph Virtual Microphone".into(),
                    driver_version: version,
                    state: VirtualAudioDeviceState::Installed,
                }))
            }
            Err(VirtualAudioError::DriverNotInstalled) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
}
