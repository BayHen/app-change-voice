import { invoke } from "@tauri-apps/api/core";
import type { AppStatus } from "../types/app";
import type { VoicePresetSummary } from "../types/preset";
import type {
  AudioDevice,
  AudioDeviceSnapshot,
  AudioRuntimeStatus,
  DefaultDevices,
  EffectParameters,
  VolumeSettings,
  NoiseSettings,
  RecordingSource,
  RecordingStatus,
} from "../types/audio";

export function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>("get_app_status");
}

export function getVolumeSettings(): Promise<VolumeSettings> {
  return invoke<VolumeSettings>("get_volume_settings");
}

export function previewVolumeSettings(settings: VolumeSettings): Promise<VolumeSettings> {
  return invoke<VolumeSettings>("preview_volume_settings", { settings });
}

export function setVolumeSettings(settings: VolumeSettings): Promise<VolumeSettings> {
  return invoke<VolumeSettings>("set_volume_settings", { settings });
}
export function getNoiseSettings(): Promise<NoiseSettings> {
  return invoke("get_noise_settings");
}
export function previewNoiseSettings(settings: NoiseSettings): Promise<NoiseSettings> {
  return invoke("preview_noise_settings", { settings });
}
export function setNoiseSettings(settings: NoiseSettings): Promise<NoiseSettings> {
  return invoke("set_noise_settings", { settings });
}

export function getInputDevices(): Promise<AudioDevice[]> {
  return invoke<AudioDevice[]>("get_input_devices");
}

export function getOutputDevices(): Promise<AudioDevice[]> {
  return invoke<AudioDevice[]>("get_output_devices");
}

export function getDefaultDevices(): Promise<DefaultDevices> {
  return invoke<DefaultDevices>("get_default_devices");
}

export function refreshAudioDevices(): Promise<AudioDeviceSnapshot> {
  return invoke<AudioDeviceSnapshot>("refresh_audio_devices");
}

export function selectInputDevice(id: string): Promise<AudioDeviceSnapshot> {
  return invoke<AudioDeviceSnapshot>("select_input_device", { id });
}

export function selectOutputDevice(id: string): Promise<AudioDeviceSnapshot> {
  return invoke<AudioDeviceSnapshot>("select_output_device", { id });
}

export function startMonitoring(
  inputDeviceId: string,
  outputDeviceId: string,
): Promise<AudioRuntimeStatus> {
  return invoke<AudioRuntimeStatus>("start_monitoring", { inputDeviceId, outputDeviceId });
}

export function pauseMonitoring(): Promise<AudioRuntimeStatus> {
  return invoke<AudioRuntimeStatus>("pause_monitoring");
}

export function resumeMonitoring(): Promise<AudioRuntimeStatus> {
  return invoke<AudioRuntimeStatus>("resume_monitoring");
}

export function stopMonitoring(): Promise<AudioRuntimeStatus> {
  return invoke<AudioRuntimeStatus>("stop_monitoring");
}

export function getAudioRuntimeStatus(): Promise<AudioRuntimeStatus> {
  return invoke<AudioRuntimeStatus>("get_audio_runtime_status");
}

export function getEffectParameters(): Promise<EffectParameters> {
  return invoke<EffectParameters>("get_effect_parameters");
}

export function setEffectParameters(parameters: EffectParameters): Promise<EffectParameters> {
  return invoke<EffectParameters>("set_effect_parameters", { parameters });
}

export function resetEffectParameters(): Promise<EffectParameters> {
  return invoke<EffectParameters>("reset_effect_parameters");
}

export interface AppliedPreset {
  preset: VoicePresetSummary;
  parameters: EffectParameters;
}

export function getBuiltinPresets(): Promise<VoicePresetSummary[]> {
  return invoke<VoicePresetSummary[]>("get_builtin_presets");
}

export function getActivePresetId(): Promise<string | null> {
  return invoke<string | null>("get_active_preset_id");
}

export function applyBuiltinPreset(id: string): Promise<AppliedPreset> {
  return invoke<AppliedPreset>("apply_builtin_preset", { id });
}

export function resetToNaturalPreset(): Promise<AppliedPreset> {
  return invoke<AppliedPreset>("reset_to_natural_preset");
}

export const getRecordingStatus = () => invoke<RecordingStatus>("get_recording_status");
export const startRecording = () => invoke<RecordingStatus>("start_recording");
export const stopRecording = () => invoke<RecordingStatus>("stop_recording");
export const playRecording = (source: RecordingSource) =>
  invoke<RecordingStatus>("play_recording", { source });
export const stopPlayback = () => invoke<RecordingStatus>("stop_playback");
export const setHearMyself = (enabled: boolean) =>
  invoke<RecordingStatus>("set_hear_myself", { enabled });
export const exportRecording = (source: RecordingSource, path: string) =>
  invoke<RecordingStatus>("export_recording", { source, path });
export const deleteRecording = () => invoke<RecordingStatus>("delete_recording");
