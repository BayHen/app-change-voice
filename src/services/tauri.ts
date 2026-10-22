import { invoke } from "@tauri-apps/api/core";
import type { AppStatus } from "../types/app";
import type { AudioDevice, AudioDeviceSnapshot, DefaultDevices } from "../types/audio";

export function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>("get_app_status");
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
