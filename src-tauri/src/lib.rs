pub mod audio;
pub mod effects;
pub mod platform;
pub mod presets;

use audio::{AudioEngine, NullAudioEngine};
use audio::{
    config::{self, AudioDevicePreferences, StoredDeviceSelection},
    devices::{
        AudioDeviceInfo, AudioDeviceProvider, AudioDeviceSnapshot, DeviceKind, resolve_selection,
    },
    errors::AudioDeviceError,
    stream::{AudioRuntimeStatus, AudioStreamManager},
};
use platform::PlatformInfo;
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Manager, State};

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

struct AudioDeviceService {
    provider: Box<dyn AudioDeviceProvider>,
    preferences_path: PathBuf,
    preferences: AudioDevicePreferences,
}

impl AudioDeviceService {
    fn new(
        provider: impl AudioDeviceProvider + 'static,
        preferences_path: PathBuf,
    ) -> Result<Self, AudioDeviceError> {
        let preferences = config::load(&preferences_path)?;
        Ok(Self {
            provider: Box::new(provider),
            preferences_path,
            preferences,
        })
    }

    fn refresh(&mut self) -> Result<AudioDeviceSnapshot, AudioDeviceError> {
        let (input_devices, output_devices) = self.provider.enumerate()?;
        let input_selection = resolve_selection(&input_devices, self.preferences.input.as_ref());
        let output_selection = resolve_selection(&output_devices, self.preferences.output.as_ref());
        let mut preferences_changed = false;
        for (kind, selection, devices) in [
            (DeviceKind::Input, &input_selection, &input_devices),
            (DeviceKind::Output, &output_selection, &output_devices),
        ] {
            if selection.message.is_some()
                && let Some(device) = selection
                    .selected_id
                    .as_ref()
                    .and_then(|id| devices.iter().find(|device| &device.id == id))
            {
                self.preferences.set_selection(
                    kind,
                    StoredDeviceSelection {
                        id: device.id.clone(),
                        fingerprint: device.fingerprint.clone(),
                    },
                );
                preferences_changed = true;
            }
        }
        if preferences_changed {
            config::save(&self.preferences_path, &self.preferences)?;
        }
        Ok(AudioDeviceSnapshot {
            input_devices,
            output_devices,
            input_selection,
            output_selection,
        })
    }

    fn select(
        &mut self,
        kind: DeviceKind,
        id: &str,
    ) -> Result<AudioDeviceSnapshot, AudioDeviceError> {
        let snapshot = self.refresh()?;
        let devices = match kind {
            DeviceKind::Input => &snapshot.input_devices,
            DeviceKind::Output => &snapshot.output_devices,
        };
        let device = devices
            .iter()
            .find(|device| device.id == id)
            .ok_or_else(|| AudioDeviceError::device_not_found(id))?;
        self.preferences.set_selection(
            kind,
            StoredDeviceSelection {
                id: device.id.clone(),
                fingerprint: device.fingerprint.clone(),
            },
        );
        config::save(&self.preferences_path, &self.preferences)?;
        self.refresh()
    }
}

type AudioDeviceState = Mutex<AudioDeviceService>;
type AudioRuntimeState = Mutex<AudioStreamManager>;

fn with_audio_service<T>(
    state: State<'_, AudioDeviceState>,
    operation: impl FnOnce(&mut AudioDeviceService) -> Result<T, AudioDeviceError>,
) -> Result<T, AudioDeviceError> {
    let mut service = state
        .lock()
        .map_err(|_| AudioDeviceError::backend("Audio device state lock was poisoned"))?;
    operation(&mut service)
}

#[tauri::command]
fn get_input_devices(
    state: State<'_, AudioDeviceState>,
) -> Result<Vec<AudioDeviceInfo>, AudioDeviceError> {
    with_audio_service(state, |service| Ok(service.refresh()?.input_devices))
}

#[tauri::command]
fn get_output_devices(
    state: State<'_, AudioDeviceState>,
) -> Result<Vec<AudioDeviceInfo>, AudioDeviceError> {
    with_audio_service(state, |service| Ok(service.refresh()?.output_devices))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultDevices {
    input_id: Option<String>,
    output_id: Option<String>,
}

#[tauri::command]
fn get_default_devices(
    state: State<'_, AudioDeviceState>,
) -> Result<DefaultDevices, AudioDeviceError> {
    with_audio_service(state, |service| {
        let snapshot = service.refresh()?;
        Ok(DefaultDevices {
            input_id: snapshot
                .input_devices
                .iter()
                .find(|device| device.is_default)
                .map(|device| device.id.clone()),
            output_id: snapshot
                .output_devices
                .iter()
                .find(|device| device.is_default)
                .map(|device| device.id.clone()),
        })
    })
}

#[tauri::command]
fn select_input_device(
    id: String,
    state: State<'_, AudioDeviceState>,
    runtime: State<'_, AudioRuntimeState>,
) -> Result<AudioDeviceSnapshot, AudioDeviceError> {
    runtime
        .lock()
        .map_err(|_| AudioDeviceError::backend("Audio runtime lock was poisoned"))?
        .stop();
    with_audio_service(state, |service| service.select(DeviceKind::Input, &id))
}

#[tauri::command]
fn select_output_device(
    id: String,
    state: State<'_, AudioDeviceState>,
    runtime: State<'_, AudioRuntimeState>,
) -> Result<AudioDeviceSnapshot, AudioDeviceError> {
    runtime
        .lock()
        .map_err(|_| AudioDeviceError::backend("Audio runtime lock was poisoned"))?
        .stop();
    with_audio_service(state, |service| service.select(DeviceKind::Output, &id))
}

#[tauri::command]
fn refresh_audio_devices(
    state: State<'_, AudioDeviceState>,
) -> Result<AudioDeviceSnapshot, AudioDeviceError> {
    with_audio_service(state, AudioDeviceService::refresh)
}

fn with_audio_runtime<T>(
    state: State<'_, AudioRuntimeState>,
    operation: impl FnOnce(&mut AudioStreamManager) -> Result<T, AudioDeviceError>,
) -> Result<T, AudioDeviceError> {
    let mut runtime = state
        .lock()
        .map_err(|_| AudioDeviceError::backend("Audio runtime lock was poisoned"))?;
    operation(&mut runtime)
}

#[tauri::command]
fn start_monitoring(
    input_device_id: String,
    output_device_id: String,
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| {
        runtime.start(&input_device_id, &output_device_id)
    })
}

#[tauri::command]
fn pause_monitoring(
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, AudioStreamManager::pause)
}

#[tauri::command]
fn resume_monitoring(
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, AudioStreamManager::resume)
}

#[tauri::command]
fn stop_monitoring(
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.stop()))
}

#[tauri::command]
fn get_audio_runtime_status(
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.status()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let preferences_path = app.path().app_config_dir()?.join("audio-devices.json");
            app.manage(Mutex::new(AudioDeviceService::new(
                platform::WindowsAudioDeviceProvider,
                preferences_path,
            )?));
            app.manage(Mutex::new(AudioStreamManager::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_input_devices,
            get_output_devices,
            get_default_devices,
            select_input_device,
            select_output_device,
            refresh_audio_devices,
            start_monitoring,
            pause_monitoring,
            resume_monitoring,
            stop_monitoring,
            get_audio_runtime_status
        ])
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
