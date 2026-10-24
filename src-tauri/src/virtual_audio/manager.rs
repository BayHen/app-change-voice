#[cfg(target_os = "windows")]
use super::windows_bridge::WindowsDriverBridge;
use super::{AudioBridge, BridgeStatus, VirtualAudioError};
use serde::Serialize;

pub const DRIVER_NAME: &str = "VoiceMorph Virtual Microphone";
pub const EXPECTED_DRIVER_VERSION: &str = "0.1.0.0";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VirtualMicrophoneState {
    NotInstalled,
    Installed,
    DriverUnavailable,
    Ready,
    BridgeConnected,
    Streaming,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMicrophoneStatus {
    pub state: VirtualMicrophoneState,
    pub driver_name: &'static str,
    pub driver_version: Option<String>,
    pub expected_driver_version: &'static str,
    pub version_compatible: bool,
    pub protocol_version: u32,
    pub capture_endpoint_available: bool,
    pub capture_endpoint_name: Option<String>,
    pub bridge_connected: bool,
    pub routing_active: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMicrophoneMetrics {
    pub connected: bool,
    pub streaming: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub packet_frames: u32,
    pub queued_frames: u32,
    pub capacity_frames: u32,
    pub written_frames: u64,
    pub dropped_frames: u64,
    pub underrun_frames: u64,
    pub transport_errors: u64,
    /// Queue-only estimate. This is not measured end-to-end latency.
    pub bridge_buffer_estimate_ms: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMicrophoneVersion {
    pub driver_name: &'static str,
    pub installed_version: Option<String>,
    pub expected_version: &'static str,
    pub protocol_version: u32,
    pub compatible: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMicrophoneCommandError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

impl VirtualMicrophoneCommandError {
    pub fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }
}

pub struct VirtualMicrophoneManager {
    #[cfg(target_os = "windows")]
    bridge: Option<WindowsDriverBridge>,
    installed_version: Option<String>,
    base_state: VirtualMicrophoneState,
    last_error: Option<String>,
}

pub trait DriverStatusService {
    fn refresh_driver_status(&mut self);
    fn driver_version_status(&self) -> VirtualMicrophoneVersion;
}

pub trait AudioBridgeManager {
    fn connect_bridge(&mut self) -> Result<(), VirtualMicrophoneCommandError>;
    fn disconnect_bridge(&mut self);
    fn bridge_connected(&self) -> bool;
}

pub trait VirtualRoutingController {
    fn validate_stream_start(&self) -> Result<(), VirtualMicrophoneCommandError>;
}

impl Default for VirtualMicrophoneManager {
    fn default() -> Self {
        let mut manager = Self {
            #[cfg(target_os = "windows")]
            bridge: None,
            installed_version: None,
            base_state: VirtualMicrophoneState::NotInstalled,
            last_error: None,
        };
        manager.check_driver();
        manager
    }
}

impl VirtualMicrophoneManager {
    pub fn check_driver(&mut self) {
        #[cfg(target_os = "windows")]
        match WindowsDriverBridge::connect() {
            Ok(mut bridge) => {
                let version = bridge.driver_version();
                bridge.close();
                self.installed_version = Some(version.clone());
                self.base_state = if version == EXPECTED_DRIVER_VERSION {
                    VirtualMicrophoneState::Installed
                } else {
                    VirtualMicrophoneState::Error
                };
                self.last_error = (version != EXPECTED_DRIVER_VERSION).then(|| format!("Driver version {version} is incompatible with expected version {EXPECTED_DRIVER_VERSION}"));
            }
            Err(VirtualAudioError::DriverNotInstalled) => {
                self.installed_version = None;
                self.base_state = VirtualMicrophoneState::NotInstalled;
                self.last_error = None;
            }
            Err(error) => {
                self.base_state = VirtualMicrophoneState::DriverUnavailable;
                self.last_error = Some(error.to_string());
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.base_state = VirtualMicrophoneState::DriverUnavailable;
            self.last_error =
                Some("VoiceMorph virtual microphone is available only on Windows".into());
        }
    }

    pub fn connect(&mut self) -> Result<(), VirtualMicrophoneCommandError> {
        #[cfg(target_os = "windows")]
        {
            if self.bridge.is_some() {
                return Ok(());
            }
            let bridge = WindowsDriverBridge::connect().map_err(map_error)?;
            let version = bridge.driver_version();
            if version != EXPECTED_DRIVER_VERSION {
                return Err(VirtualMicrophoneCommandError::new(
                    "versionMismatch",
                    format!("Installed driver {version}; expected {EXPECTED_DRIVER_VERSION}"),
                    false,
                ));
            }
            self.installed_version = Some(version);
            self.bridge = Some(bridge);
            self.base_state = VirtualMicrophoneState::BridgeConnected;
            self.last_error = None;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        Err(VirtualMicrophoneCommandError::new(
            "unsupportedPlatform",
            "Virtual microphone is Windows-only",
            false,
        ))
    }

    pub fn disconnect(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(mut bridge) = self.bridge.take() {
            bridge.close();
        }
        self.base_state = if self.installed_version.is_some() {
            VirtualMicrophoneState::Installed
        } else {
            VirtualMicrophoneState::NotInstalled
        };
    }

    pub fn poll_connection(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(bridge) = self.bridge.as_mut()
            && let Err(error) = bridge.status()
        {
            bridge.close();
            self.bridge = None;
            self.base_state = VirtualMicrophoneState::DriverUnavailable;
            self.last_error = Some(error.to_string());
        }
    }

    pub fn is_connected(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.bridge.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    pub fn version(&self) -> VirtualMicrophoneVersion {
        VirtualMicrophoneVersion {
            driver_name: DRIVER_NAME,
            installed_version: self.installed_version.clone(),
            expected_version: EXPECTED_DRIVER_VERSION,
            protocol_version: super::audio_bridge::BRIDGE_PROTOCOL_VERSION,
            compatible: self.installed_version.as_deref() == Some(EXPECTED_DRIVER_VERSION),
        }
    }

    pub fn status(
        &self,
        capture_endpoint_name: Option<String>,
        runtime: Option<BridgeStatus>,
    ) -> VirtualMicrophoneStatus {
        let routing_active = runtime.is_some_and(|value| value.active && value.connected);
        let endpoint_available = capture_endpoint_name.is_some();
        let connected = self.is_connected();
        let state = if routing_active {
            VirtualMicrophoneState::Streaming
        } else if connected {
            VirtualMicrophoneState::BridgeConnected
        } else if self.base_state == VirtualMicrophoneState::Installed && endpoint_available {
            VirtualMicrophoneState::Ready
        } else {
            self.base_state
        };
        let message = self.last_error.clone().unwrap_or_else(|| match state {
            VirtualMicrophoneState::NotInstalled => "VoiceMorph driver is not installed. Install a signed package on a supported test or production system.".into(),
            VirtualMicrophoneState::Installed => "Driver interface found; the Windows capture endpoint has not been verified.".into(),
            VirtualMicrophoneState::DriverUnavailable => "Driver is installed or registered but its bridge cannot be opened.".into(),
            VirtualMicrophoneState::Ready => "Driver and virtual capture endpoint are available.".into(),
            VirtualMicrophoneState::BridgeConnected => "Control bridge connected. Audio capture remains stopped until routing is started.".into(),
            VirtualMicrophoneState::Streaming => "Processed audio is being sent to the VoiceMorph virtual microphone.".into(),
            VirtualMicrophoneState::Error => "Virtual microphone configuration is incompatible.".into(),
        });
        VirtualMicrophoneStatus {
            state,
            driver_name: DRIVER_NAME,
            driver_version: self.installed_version.clone(),
            expected_driver_version: EXPECTED_DRIVER_VERSION,
            version_compatible: self.installed_version.as_deref() == Some(EXPECTED_DRIVER_VERSION),
            protocol_version: super::audio_bridge::BRIDGE_PROTOCOL_VERSION,
            capture_endpoint_available: endpoint_available,
            capture_endpoint_name,
            bridge_connected: connected,
            routing_active,
            message,
        }
    }

    pub fn metrics(runtime: Option<BridgeStatus>) -> VirtualMicrophoneMetrics {
        let value = runtime.unwrap_or_default();
        VirtualMicrophoneMetrics {
            connected: value.connected,
            streaming: value.active && value.connected,
            sample_rate: 48_000,
            channels: 1,
            packet_frames: 480,
            queued_frames: value.queued_frames,
            capacity_frames: value.capacity_frames,
            written_frames: value.written_frames,
            dropped_frames: value.dropped_frames,
            underrun_frames: value.underrun_frames,
            transport_errors: value.transport_errors,
            bridge_buffer_estimate_ms: value.queued_frames as f32 / 48.0,
        }
    }
}

fn map_error(error: VirtualAudioError) -> VirtualMicrophoneCommandError {
    match error {
        VirtualAudioError::DriverNotInstalled => {
            VirtualMicrophoneCommandError::new("driverNotInstalled", error.to_string(), true)
        }
        VirtualAudioError::AccessDenied => {
            VirtualMicrophoneCommandError::new("accessDenied", error.to_string(), true)
        }
        VirtualAudioError::IncompatibleProtocol { .. } => {
            VirtualMicrophoneCommandError::new("protocolMismatch", error.to_string(), false)
        }
        _ => VirtualMicrophoneCommandError::new("bridgeUnavailable", error.to_string(), true),
    }
}

impl DriverStatusService for VirtualMicrophoneManager {
    fn refresh_driver_status(&mut self) {
        self.check_driver();
    }
    fn driver_version_status(&self) -> VirtualMicrophoneVersion {
        self.version()
    }
}

impl AudioBridgeManager for VirtualMicrophoneManager {
    fn connect_bridge(&mut self) -> Result<(), VirtualMicrophoneCommandError> {
        self.connect()
    }
    fn disconnect_bridge(&mut self) {
        self.disconnect();
    }
    fn bridge_connected(&self) -> bool {
        self.is_connected()
    }
}

impl VirtualRoutingController for VirtualMicrophoneManager {
    fn validate_stream_start(&self) -> Result<(), VirtualMicrophoneCommandError> {
        if self.is_connected() {
            Ok(())
        } else {
            Err(VirtualMicrophoneCommandError::new(
                "bridgeNotConnected",
                "Connect the VoiceMorph driver bridge first",
                true,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn states_keep_installed_connected_and_streaming_distinct() {
        let manager = VirtualMicrophoneManager {
            #[cfg(target_os = "windows")]
            bridge: None,
            installed_version: Some(EXPECTED_DRIVER_VERSION.into()),
            base_state: VirtualMicrophoneState::Installed,
            last_error: None,
        };
        assert_eq!(
            manager.status(None, None).state,
            VirtualMicrophoneState::Installed
        );
        assert_eq!(
            manager.status(Some(DRIVER_NAME.into()), None).state,
            VirtualMicrophoneState::Ready
        );
        assert_eq!(
            manager
                .status(
                    Some(DRIVER_NAME.into()),
                    Some(BridgeStatus {
                        connected: true,
                        active: true,
                        ..Default::default()
                    })
                )
                .state,
            VirtualMicrophoneState::Streaming
        );
    }
    #[test]
    fn bridge_latency_is_labeled_as_queue_estimate() {
        let metrics = VirtualMicrophoneManager::metrics(Some(BridgeStatus {
            connected: true,
            queued_frames: 480,
            ..Default::default()
        }));
        assert_eq!(metrics.bridge_buffer_estimate_ms, 10.0);
    }
}
