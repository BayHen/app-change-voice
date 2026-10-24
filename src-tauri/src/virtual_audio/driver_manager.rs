use super::errors::VirtualAudioError;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverPackageState {
    NotInstalled,
    Installed,
    UpdateAvailable,
    RebootRequired,
    Problem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverPackageInfo {
    pub published_inf_name: Option<String>,
    pub provider: String,
    pub version: String,
    pub state: DriverPackageState,
    pub signed: bool,
}

/// Driver lifecycle control-plane contract.
///
/// Implementations belong to Phase 11.2.2 or later and must require an explicit
/// user action plus elevation for install/remove. Audio callbacks must never call it.
pub trait DriverManager: Send + Sync {
    fn query(&self) -> Result<DriverPackageInfo, VirtualAudioError>;
    fn install(&self, package_directory: &Path) -> Result<DriverPackageInfo, VirtualAudioError>;
    fn uninstall(&self) -> Result<DriverPackageInfo, VirtualAudioError>;
}
