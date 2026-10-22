pub mod audio;
pub mod effects;
pub mod platform;
pub mod presets;

use audio::{AudioEngine, NullAudioEngine};
use platform::PlatformInfo;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    app_name: &'static str,
    version: &'static str,
    platform: String,
    audio_engine_ready: bool,
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    let platform = platform::current_platform();
    let engine = NullAudioEngine;

    AppStatus {
        app_name: "VoiceMorph Studio",
        version: env!("CARGO_PKG_VERSION"),
        platform: format!("{}-{}", platform.os_name(), platform.architecture()),
        audio_engine_ready: engine.is_ready(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_exposes_product_metadata() {
        let status = get_app_status();
        assert_eq!(status.app_name, "VoiceMorph Studio");
        assert_eq!(status.version, "0.1.0");
        assert!(!status.audio_engine_ready);
    }
}
