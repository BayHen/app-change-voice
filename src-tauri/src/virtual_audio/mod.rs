//! VoiceMorph virtual capture endpoint contracts and user-mode PCM bridge.

pub mod audio_bridge;
pub mod device_detection;
pub mod driver_manager;
pub mod errors;
pub mod installation;
pub mod manager;
#[cfg(target_os = "windows")]
pub mod windows_bridge;

pub use audio_bridge::{AudioBridge, BridgeAudioFormat, BridgeStatus, BridgeWriteOutcome};
pub use device_detection::{VirtualAudioDeviceInfo, VirtualAudioDeviceState};
pub use driver_manager::{DriverManager, DriverPackageInfo, DriverPackageState};
pub use errors::VirtualAudioError;
pub use installation::{
    DriverInstallAction, DriverInstallationPlan, DriverInstallationService, DriverReleasePackage,
};
