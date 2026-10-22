//! Adapter hệ điều hành. Audio engine và DSP không import module này.

pub trait PlatformInfo {
    fn os_name(&self) -> &'static str;
    fn architecture(&self) -> &'static str;
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{WindowsAudioDeviceProvider, WindowsPlatform};

#[cfg(target_os = "windows")]
pub fn current_platform() -> WindowsPlatform {
    WindowsPlatform
}

// Điểm mở rộng có chủ đích cho adapter macOS trong tương lai.
#[cfg(not(target_os = "windows"))]
pub struct UnsupportedPlatform;

#[cfg(not(target_os = "windows"))]
impl PlatformInfo for UnsupportedPlatform {
    fn os_name(&self) -> &'static str {
        std::env::consts::OS
    }

    fn architecture(&self) -> &'static str {
        std::env::consts::ARCH
    }
}

#[cfg(not(target_os = "windows"))]
pub fn current_platform() -> UnsupportedPlatform {
    UnsupportedPlatform
}
