use super::PlatformInfo;

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
