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
    noise::{self, NoiseSettings},
    recording::{RecordingSource, RecordingStatus},
    stream::{AudioRuntimeStatus, AudioStreamManager},
};
use effects::{
    gain::{self, VolumeSettings},
    parameters::EffectParameters,
};
use platform::PlatformInfo;
use presets::{
    PresetRegistry, VoicePreset, repository::JsonPresetRepository, service::UserPresetService,
    validation::validate_preset,
};
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
type UserPresetState = Mutex<UserPresetService>;
struct VolumeConfigPath(PathBuf);
struct NoiseConfigPath(PathBuf);

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
fn get_recording_status(
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.recording_status()))
}
#[tauri::command]
fn start_recording(
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.start_recording())
}
#[tauri::command]
fn stop_recording(
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.stop_recording())
}
#[tauri::command]
fn play_recording(
    source: RecordingSource,
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.play_recording(source))
}
#[tauri::command]
fn stop_playback(state: State<'_, AudioRuntimeState>) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.stop_playback())
}
#[tauri::command]
fn set_hear_myself(
    enabled: bool,
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.set_hear_myself(enabled))
}
#[tauri::command]
fn export_recording(
    source: RecordingSource,
    path: PathBuf,
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.export_recording(source, path))
}
#[tauri::command]
fn delete_recording(
    state: State<'_, AudioRuntimeState>,
) -> Result<RecordingStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| runtime.delete_recording())
}

#[tauri::command]
fn get_audio_runtime_status(
    state: State<'_, AudioRuntimeState>,
) -> Result<AudioRuntimeStatus, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.status()))
}

#[tauri::command]
fn get_volume_settings(
    state: State<'_, AudioRuntimeState>,
) -> Result<VolumeSettings, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.volume_settings()))
}

#[tauri::command]
fn set_volume_settings(
    settings: VolumeSettings,
    state: State<'_, AudioRuntimeState>,
    path: State<'_, VolumeConfigPath>,
) -> Result<VolumeSettings, AudioDeviceError> {
    let settings = settings.validated();
    gain::save(&path.0, settings)?;
    with_audio_runtime(state, |runtime| Ok(runtime.set_volume_settings(settings)))
}

#[tauri::command]
fn preview_volume_settings(
    settings: VolumeSettings,
    state: State<'_, AudioRuntimeState>,
) -> Result<VolumeSettings, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.set_volume_settings(settings)))
}

#[tauri::command]
fn get_noise_settings(
    state: State<'_, AudioRuntimeState>,
) -> Result<NoiseSettings, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.noise_settings()))
}

#[tauri::command]
fn preview_noise_settings(
    settings: NoiseSettings,
    state: State<'_, AudioRuntimeState>,
) -> Result<NoiseSettings, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.set_noise_settings(settings)))
}

#[tauri::command]
fn set_noise_settings(
    settings: NoiseSettings,
    state: State<'_, AudioRuntimeState>,
    path: State<'_, NoiseConfigPath>,
) -> Result<NoiseSettings, AudioDeviceError> {
    let settings = settings.validated();
    noise::save(&path.0, settings)?;
    with_audio_runtime(state, |runtime| Ok(runtime.set_noise_settings(settings)))
}

#[tauri::command]
fn get_effect_parameters(
    state: State<'_, AudioRuntimeState>,
) -> Result<EffectParameters, AudioDeviceError> {
    with_audio_runtime(state, |runtime| Ok(runtime.effect_parameters()))
}

#[tauri::command]
fn set_effect_parameters(
    parameters: EffectParameters,
    state: State<'_, AudioRuntimeState>,
) -> Result<EffectParameters, AudioDeviceError> {
    with_audio_runtime(state, |runtime| {
        Ok(runtime.set_effect_parameters(parameters))
    })
}

#[tauri::command]
fn reset_effect_parameters(
    state: State<'_, AudioRuntimeState>,
) -> Result<EffectParameters, AudioDeviceError> {
    with_audio_runtime(state, |runtime| {
        Ok(runtime.set_effect_parameters(EffectParameters::default()))
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppliedPreset {
    preset: VoicePreset,
    parameters: EffectParameters,
}

#[tauri::command]
fn get_builtin_presets(registry: State<'_, PresetRegistry>) -> Vec<VoicePreset> {
    registry.all().to_vec()
}

#[tauri::command]
fn get_all_presets(
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<Vec<VoicePreset>, AudioDeviceError> {
    let mut result = registry.all().to_vec();
    let users = users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?;
    result.extend_from_slice(users.all());
    Ok(result)
}
#[tauri::command]
fn get_preset_warnings(users: State<'_, UserPresetState>) -> Result<Vec<String>, AudioDeviceError> {
    Ok(users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .warnings()
        .to_vec())
}
fn builtin_ids(registry: &PresetRegistry) -> Vec<&str> {
    registry.all().iter().map(|p| p.id.as_str()).collect()
}
#[tauri::command]
fn create_user_preset(
    preset: VoicePreset,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<VoicePreset, AudioDeviceError> {
    let ids = builtin_ids(&registry);
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .create(preset, &ids)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn update_user_preset(
    preset: VoicePreset,
    users: State<'_, UserPresetState>,
) -> Result<VoicePreset, AudioDeviceError> {
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .update(preset)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn duplicate_preset(
    id: String,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<VoicePreset, AudioDeviceError> {
    let mut users = users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?;
    let source = registry
        .get(&id)
        .ok()
        .cloned()
        .or_else(|| users.get(&id).ok().cloned())
        .ok_or_else(|| AudioDeviceError::backend(format!("Preset not found: {id}")))?;
    let ids = builtin_ids(&registry);
    users
        .duplicate(&source, &ids)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn delete_user_preset(
    id: String,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<(), AudioDeviceError> {
    if registry.get(&id).is_ok() {
        return Err(AudioDeviceError::backend(
            "Built-in presets cannot be deleted",
        ));
    }
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .delete(&id)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn set_preset_favorite(
    id: String,
    favorite: bool,
    users: State<'_, UserPresetState>,
) -> Result<VoicePreset, AudioDeviceError> {
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .favorite(&id, favorite)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn import_user_preset(
    path: PathBuf,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<VoicePreset, AudioDeviceError> {
    let ids = builtin_ids(&registry);
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .import(&path, &ids)
        .map_err(AudioDeviceError::backend)
}
#[tauri::command]
fn export_user_preset(
    id: String,
    path: PathBuf,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
) -> Result<(), AudioDeviceError> {
    if let Ok(preset) = registry.get(&id) {
        return presets::import_export::export(&path, preset).map_err(AudioDeviceError::backend);
    }
    users
        .lock()
        .map_err(|_| AudioDeviceError::backend("User preset state lock was poisoned"))?
        .export(&id, &path)
        .map_err(AudioDeviceError::backend)
}

#[tauri::command]
fn apply_preset(
    id: String,
    registry: State<'_, PresetRegistry>,
    users: State<'_, UserPresetState>,
    state: State<'_, AudioRuntimeState>,
) -> Result<AppliedPreset, AudioDeviceError> {
    let preset = registry
        .get(&id)
        .ok()
        .cloned()
        .or_else(|| users.lock().ok()?.get(&id).ok().cloned())
        .ok_or_else(|| AudioDeviceError::backend(format!("Preset not found: {id}")))?;
    validate_preset(&preset).map_err(AudioDeviceError::backend)?;
    let parameters = with_audio_runtime(state, |runtime| {
        let mut volume = runtime.volume_settings();
        volume.input_percent = preset.parameters.input_gain_percent.round() as u8;
        volume.output_percent = preset.parameters.output_gain_percent.round() as u8;
        runtime.set_volume_settings(volume);
        let mut noise = runtime.noise_settings();
        noise.enabled = preset.parameters.noise_suppression;
        noise.suppression_strength = preset.parameters.suppression_strength;
        noise.gate_enabled = preset.parameters.noise_gate;
        noise.gate_threshold_dbfs = preset.parameters.gate_threshold_dbfs;
        noise.limiter_enabled = preset.parameters.limiter_enabled;
        noise.limiter_ceiling_dbfs = preset.parameters.limiter_ceiling_dbfs;
        runtime.set_noise_settings(noise);
        Ok(runtime.apply_preset(&preset.id, preset.effect_parameters()))
    })?;
    Ok(AppliedPreset { preset, parameters })
}

#[tauri::command]
fn preview_preset(
    preset: VoicePreset,
    state: State<'_, AudioRuntimeState>,
) -> Result<EffectParameters, AudioDeviceError> {
    validate_preset(&preset).map_err(AudioDeviceError::backend)?;
    with_audio_runtime(state, |runtime| {
        let mut volume = runtime.volume_settings();
        volume.input_percent = preset.parameters.input_gain_percent.round() as u8;
        volume.output_percent = preset.parameters.output_gain_percent.round() as u8;
        runtime.set_volume_settings(volume);
        let mut noise = runtime.noise_settings();
        noise.enabled = preset.parameters.noise_suppression;
        noise.suppression_strength = preset.parameters.suppression_strength;
        noise.gate_enabled = preset.parameters.noise_gate;
        noise.gate_threshold_dbfs = preset.parameters.gate_threshold_dbfs;
        noise.limiter_enabled = preset.parameters.limiter_enabled;
        noise.limiter_ceiling_dbfs = preset.parameters.limiter_ceiling_dbfs;
        runtime.set_noise_settings(noise);
        Ok(runtime.set_effect_parameters(preset.effect_parameters()))
    })
}

#[tauri::command]
fn get_active_preset_id(
    state: State<'_, AudioRuntimeState>,
) -> Result<Option<String>, AudioDeviceError> {
    with_audio_runtime(state, |runtime| {
        Ok(runtime.active_preset_id().map(str::to_owned))
    })
}

#[tauri::command]
fn apply_builtin_preset(
    id: String,
    registry: State<'_, PresetRegistry>,
    state: State<'_, AudioRuntimeState>,
) -> Result<AppliedPreset, AudioDeviceError> {
    let preset = registry
        .get(&id)
        .map_err(AudioDeviceError::backend)?
        .clone();
    validate_preset(&preset).map_err(AudioDeviceError::backend)?;
    let requested = preset.effect_parameters();
    let parameters = with_audio_runtime(state, |runtime| {
        Ok(runtime.apply_preset(&preset.id, requested))
    })?;
    Ok(AppliedPreset { preset, parameters })
}

#[tauri::command]
fn reset_to_natural_preset(
    registry: State<'_, PresetRegistry>,
    state: State<'_, AudioRuntimeState>,
) -> Result<AppliedPreset, AudioDeviceError> {
    let preset = registry
        .get("natural")
        .map_err(AudioDeviceError::backend)?
        .clone();
    let parameters = with_audio_runtime(state, |runtime| {
        Ok(runtime.apply_preset(&preset.id, preset.effect_parameters()))
    })?;
    Ok(AppliedPreset { preset, parameters })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let preferences_path = app.path().app_config_dir()?.join("audio-devices.json");
            let volume_path = app.path().app_config_dir()?.join("volume-settings.json");
            let volume_settings = gain::load(&volume_path).unwrap_or_default();
            let noise_path = app.path().app_config_dir()?.join("noise-settings.json");
            let noise_settings = noise::load(&noise_path).unwrap_or_default();
            let recordings_dir = app.path().app_cache_dir()?.join("recordings");
            let user_presets_dir = app.path().app_data_dir()?.join("presets");
            app.manage(Mutex::new(AudioDeviceService::new(
                platform::WindowsAudioDeviceProvider,
                preferences_path,
            )?));
            app.manage(Mutex::new(AudioStreamManager::with_settings(
                volume_settings,
                noise_settings,
                recordings_dir,
            )));
            app.manage(VolumeConfigPath(volume_path));
            app.manage(NoiseConfigPath(noise_path));
            app.manage(PresetRegistry::built_in().map_err(std::io::Error::other)?);
            app.manage(Mutex::new(
                UserPresetService::new(JsonPresetRepository::new(user_presets_dir))
                    .map_err(std::io::Error::other)?,
            ));
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
            get_recording_status,
            start_recording,
            stop_recording,
            play_recording,
            stop_playback,
            set_hear_myself,
            export_recording,
            delete_recording,
            get_audio_runtime_status,
            get_volume_settings,
            set_volume_settings,
            preview_volume_settings,
            get_noise_settings,
            preview_noise_settings,
            set_noise_settings,
            get_effect_parameters,
            set_effect_parameters,
            reset_effect_parameters,
            get_builtin_presets,
            get_all_presets,
            get_preset_warnings,
            create_user_preset,
            update_user_preset,
            duplicate_preset,
            delete_user_preset,
            set_preset_favorite,
            import_user_preset,
            export_user_preset,
            apply_preset,
            preview_preset,
            get_active_preset_id,
            apply_builtin_preset,
            reset_to_natural_preset
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
