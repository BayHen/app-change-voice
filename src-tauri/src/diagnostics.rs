use crate::{
    audio::{devices::AudioDeviceSnapshot, stream::AudioRuntimeStatus},
    routing::RoutingStatus,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDiagnosticsReport {
    pub report_schema_version: u32,
    pub generated_at_unix_seconds: u64,
    pub application_version: String,
    pub platform: String,
    pub active_input_device: Option<String>,
    pub active_output_device: Option<String>,
    pub runtime: AudioRuntimeStatus,
    pub routing: RoutingStatus,
    pub processing_cpu_estimate_percent: Option<f64>,
    pub privacy_notice: String,
}

impl AudioDiagnosticsReport {
    pub fn new(
        devices: &AudioDeviceSnapshot,
        runtime: AudioRuntimeStatus,
        routing: RoutingStatus,
    ) -> Self {
        let name_for = |id: &Option<String>, input: bool| {
            let candidates = if input {
                &devices.input_devices
            } else {
                &devices.output_devices
            };
            id.as_ref()
                .and_then(|id| candidates.iter().find(|device| &device.id == id))
                .map(|device| device.name.clone())
        };
        let processing_cpu_estimate_percent = runtime.metrics.as_ref().map(|metrics| {
            // averageProcessingMicros is measured DSP time per frame. This ratio is an
            // estimate for one processing core, not whole-process CPU utilization.
            metrics.average_processing_micros * f64::from(metrics.sample_rate) / 10_000.0
        });
        Self {
            report_schema_version: 1,
            generated_at_unix_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or_default(),
            application_version: env!("CARGO_PKG_VERSION").into(),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            active_input_device: name_for(&runtime.input_device_id, true),
            active_output_device: name_for(&runtime.output_device_id, false),
            runtime,
            routing,
            processing_cpu_estimate_percent,
            privacy_notice: "Metadata and counters only; no audio samples, recordings, secrets, or tokens are included.".into(),
        }
    }
}

pub fn write_atomic(path: &Path, report: &AudioDiagnosticsReport) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Diagnostics path has no parent directory")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("Diagnostics filename is invalid")?;
    let temporary = parent.join(format!(".{file_name}.tmp"));
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audio::{devices::DeviceSelection, stream::MonitoringState},
        routing::RoutingState,
    };

    fn report() -> AudioDiagnosticsReport {
        AudioDiagnosticsReport::new(
            &AudioDeviceSnapshot {
                input_devices: vec![],
                output_devices: vec![],
                input_selection: DeviceSelection {
                    selected_id: None,
                    fell_back: false,
                    message: None,
                },
                output_selection: DeviceSelection {
                    selected_id: None,
                    fell_back: false,
                    message: None,
                },
            },
            AudioRuntimeStatus {
                state: MonitoringState::Stopped,
                input_device_id: None,
                output_device_id: None,
                sample_rate: None,
                input_channels: None,
                output_channels: None,
                dsp_latency_frames: None,
                metrics: None,
            },
            RoutingStatus {
                state: RoutingState::NotConfigured,
                output_device_id: None,
                output_device_name: None,
                sample_rate: None,
                channels: None,
                underrun_count: 0,
                overflow_count: 0,
                output_peak_dbfs: -96.0,
                endpoint_verified: false,
                message: None,
            },
        )
    }

    #[test]
    fn serialized_report_contains_no_audio_or_secret_payload_fields() {
        let json = serde_json::to_string(&report()).unwrap();
        for forbidden in ["audioSamples", "recordingPath", "\"token\":", "\"secret\":"] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("privacyNotice"));
    }

    #[test]
    fn writes_valid_json_through_temporary_file() {
        let directory =
            std::env::temp_dir().join(format!("voicemorph-diagnostics-{}", std::process::id()));
        let path = directory.join("report.json");
        write_atomic(&path, &report()).unwrap();
        let decoded: AudioDiagnosticsReport =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(decoded.report_schema_version, 1);
        let _ = fs::remove_dir_all(directory);
    }
}
