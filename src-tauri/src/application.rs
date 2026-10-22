use crate::{
    AudioRuntimeState, UserPresetState, VolumeConfigPath,
    effects::gain,
    hotkeys::config::HotkeyAction,
    presets::{PresetRegistry, VoicePreset},
    routing::{AudioRoutingService, RoutingState},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub action: HotkeyAction,
    pub message: String,
}

pub fn dispatch(app: &AppHandle, action: HotkeyAction) {
    if let Err(message) = execute(app, action) {
        let _ = app.emit(
            "voicemorph://action-error",
            ActionResult { action, message },
        );
    }
}

pub fn execute(app: &AppHandle, action: HotkeyAction) -> Result<ActionResult, String> {
    let message = match action {
        HotkeyAction::ToggleMicrophoneMute => {
            let state = app.state::<AudioRuntimeState>();
            let mut runtime = state
                .lock()
                .map_err(|_| "Audio runtime lock was poisoned")?;
            let mut settings = runtime.volume_settings();
            settings.input_muted = !settings.input_muted;
            gain::save(&app.state::<VolumeConfigPath>().0, settings)
                .map_err(|error| error.to_string())?;
            runtime.set_volume_settings(settings);
            if settings.input_muted {
                "Microphone muted"
            } else {
                "Microphone unmuted"
            }
            .into()
        }
        HotkeyAction::ToggleVoiceEffects => {
            let state = app.state::<AudioRuntimeState>();
            let mut runtime = state
                .lock()
                .map_err(|_| "Audio runtime lock was poisoned")?;
            let mut parameters = runtime.effect_parameters();
            parameters.enabled = !parameters.enabled;
            runtime.set_effect_parameters_preserving_preset(parameters);
            if parameters.enabled {
                "Voice effects enabled"
            } else {
                "Voice effects disabled"
            }
            .into()
        }
        HotkeyAction::NextVoicePreset => cycle_preset(app, 1)?,
        HotkeyAction::PreviousVoicePreset => cycle_preset(app, -1)?,
        HotkeyAction::ToggleHearMyself => {
            let state = app.state::<AudioRuntimeState>();
            let runtime = state
                .lock()
                .map_err(|_| "Audio runtime lock was poisoned")?;
            let enabled = !runtime.recording_status().hear_myself;
            runtime
                .set_hear_myself(enabled)
                .map_err(|error| error.to_string())?;
            if enabled {
                "Hear Myself enabled"
            } else {
                "Hear Myself disabled"
            }
            .into()
        }
        HotkeyAction::ToggleVirtualMicrophone => {
            let state = app.state::<AudioRuntimeState>();
            let mut runtime = state
                .lock()
                .map_err(|_| "Audio runtime lock was poisoned")?;
            if runtime.routing_status().state == RoutingState::Active {
                runtime.stop_routing().map_err(|error| error.to_string())?;
                "Virtual microphone routing stopped".into()
            } else {
                runtime.start_routing().map_err(|error| error.to_string())?;
                "Virtual microphone routing started".into()
            }
        }
        HotkeyAction::ToggleRecording => {
            let state = app.state::<AudioRuntimeState>();
            let runtime = state
                .lock()
                .map_err(|_| "Audio runtime lock was poisoned")?;
            if runtime.recording_status().recording {
                runtime
                    .stop_recording()
                    .map_err(|error| error.to_string())?;
                "Recording stopped".into()
            } else {
                runtime
                    .start_recording()
                    .map_err(|error| error.to_string())?;
                "Recording started".into()
            }
        }
    };
    let result = ActionResult { action, message };
    let _ = app.emit("voicemorph://action", &result);
    crate::tray::manager::refresh(app);
    Ok(result)
}

fn cycle_preset(app: &AppHandle, direction: isize) -> Result<String, String> {
    let registry = app.state::<PresetRegistry>();
    let user_state = app.state::<UserPresetState>();
    let users = user_state
        .lock()
        .map_err(|_| "User preset state lock was poisoned")?;
    let presets: Vec<VoicePreset> = registry.all().iter().chain(users.all()).cloned().collect();
    if presets.is_empty() {
        return Err("No voice presets are available".into());
    }
    let runtime_state = app.state::<AudioRuntimeState>();
    let mut runtime = runtime_state
        .lock()
        .map_err(|_| "Audio runtime lock was poisoned")?;
    let current = runtime
        .active_preset_id()
        .and_then(|id| presets.iter().position(|preset| preset.id == id))
        .unwrap_or(0);
    let next = (current as isize + direction).rem_euclid(presets.len() as isize) as usize;
    let preset = &presets[next];
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
    runtime.apply_preset(&preset.id, preset.effect_parameters());
    Ok(format!("Preset: {}", preset.name))
}

pub fn cleanup(app: &AppHandle) {
    if let Ok(mut runtime) = app.state::<AudioRuntimeState>().lock() {
        runtime.stop();
    }
    if let Ok(mut hotkeys) = app.state::<crate::hotkeys::manager::HotkeyState>().lock() {
        hotkeys.unregister_all(app);
    }
}
