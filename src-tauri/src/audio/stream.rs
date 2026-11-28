use super::{
    errors::AudioDeviceError,
    meter::SignalMeter,
    metrics::{AudioMetrics, SharedAudioMetrics},
    noise::{NoiseSettings, SharedNoiseSettings},
    pipeline::AudioPipeline,
    recording::{
        RecordingEndpoints, RecordingFrame, RecordingRuntime, RecordingSource, RecordingStatus,
    },
};
use crate::effects::{
    gain::{SharedGainControls, SmoothedGain, VolumeSettings},
    limiter::OutputLimiter,
    parameters::{EffectParameters, SharedEffectParameters},
};
use crate::routing::{
    AudioRoutingService, RoutingConfiguration, RoutingProducer, RoutingState, RoutingStatus,
    VirtualOutputSink, amplitude_to_dbfs, errors::RoutingError, routing_queue_with_ai,
    windows::WindowsVirtualOutputSink,
};
use crate::soundboard::{
    SoundEntry,
    mixer::{MixerCommand, SoundboardMixer},
};
use cpal::{
    BufferSize, Device, DeviceId, SampleFormat, Stream, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig, SupportedStreamConfigRange,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::Serialize;
use std::{path::PathBuf, str::FromStr, sync::atomic::Ordering, time::Instant};

const PREFERRED_SAMPLE_RATE: u32 = 48_000;
const REQUESTED_BUFFER_FRAMES: u32 = 256;
const RING_BUFFER_MILLISECONDS: u32 = 160;

#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MonitoringState {
    Stopped,
    Running,
    Paused,
    Faulted,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioRuntimeStatus {
    pub state: MonitoringState,
    pub input_device_id: Option<String>,
    pub output_device_id: Option<String>,
    pub sample_rate: Option<u32>,
    pub input_channels: Option<u16>,
    pub output_channels: Option<u16>,
    pub dsp_latency_frames: Option<usize>,
    pub metrics: Option<AudioMetrics>,
    pub input_sample_format: Option<String>,
    pub output_sample_format: Option<String>,
    pub diagnostic_recording: bool,
}

struct ActiveStreams {
    input: Stream,
    output: Stream,
    input_device_id: String,
    output_device_id: String,
    sample_rate: u32,
    input_channels: u16,
    output_channels: u16,
    input_sample_format: String,
    output_sample_format: String,
    input_buffer_frames: u32,
    output_buffer_frames: u32,
    dsp_latency_frames: usize,
    metrics: SharedAudioMetrics,
    routing: Option<WindowsVirtualOutputSink>,
    _worker: DspWorker,
    _diagnostics: super::capture::CaptureWriter,
}

struct DspWorker {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for DspWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[derive(Clone, Copy)]
struct CaptureFrame {
    raw: f32,
    gained: f32,
}

pub struct AudioStreamManager {
    pub ai: crate::ai::engine::AIEngine,
    streams: Option<ActiveStreams>,
    state: MonitoringState,
    last_fault: Option<AudioRuntimeStatus>,
    quality: super::quality::QualitySettings,
    quality_path: Option<PathBuf>,
    diagnostic_control: std::sync::Arc<super::capture::CaptureControl>,
    controls: SharedGainControls,
    volume_settings: VolumeSettings,
    effect_controls: SharedEffectParameters,
    effect_parameters: EffectParameters,
    active_preset_id: Option<String>,
    noise_controls: SharedNoiseSettings,
    noise_settings: NoiseSettings,
    recording: Option<RecordingRuntime>,
    recordings_dir: PathBuf,
    routing_configuration: RoutingConfiguration,
    soundboard_entries: Vec<SoundEntry>,
    soundboard_commands: Option<Producer<MixerCommand>>,
    soundboard_ids: Vec<String>,
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self {
            ai: Default::default(),
            streams: None,
            state: MonitoringState::Stopped,
            last_fault: None,
            quality: Default::default(),
            quality_path: None,
            diagnostic_control: Default::default(),
            controls: SharedGainControls::new(VolumeSettings::default()),
            volume_settings: VolumeSettings::default(),
            effect_controls: SharedEffectParameters::new(EffectParameters::default()),
            effect_parameters: EffectParameters::default(),
            active_preset_id: Some("natural".into()),
            noise_controls: SharedNoiseSettings::new(NoiseSettings::default()),
            noise_settings: NoiseSettings::default(),
            recording: None,
            recordings_dir: std::env::temp_dir().join("voicemorph-studio"),
            routing_configuration: RoutingConfiguration::default(),
            soundboard_entries: Vec::new(),
            soundboard_commands: None,
            soundboard_ids: Vec::new(),
        }
    }
}

impl AudioStreamManager {
    pub fn capture_status(&self) -> super::capture::CaptureStatus {
        self.diagnostic_control.status()
    }
    pub fn start_capture(
        &self,
        stages: Vec<super::capture::Stage>,
        seconds: u64,
        consent: bool,
    ) -> Result<super::capture::CaptureStatus, String> {
        if self.status().state != MonitoringState::Running {
            return Err("Enable Monitor before an explicit diagnostic capture".into());
        }
        let folder = self
            .recordings_dir
            .parent()
            .unwrap_or(&self.recordings_dir)
            .join("audio-diagnostics")
            .join(format!(
                "capture-{}-{}",
                std::process::id(),
                crate::ai::process::now_ms()
            ));
        self.diagnostic_control
            .start(stages, seconds, consent, folder)
    }
    pub fn stop_capture(&self) -> super::capture::CaptureStatus {
        self.diagnostic_control.stop()
    }
    pub fn configure_quality(&mut self, path: PathBuf) {
        self.quality = super::quality::load(&path);
        self.quality_path = Some(path);
    }
    pub fn quality(&self) -> super::quality::QualitySettings {
        self.quality
    }
    pub fn set_quality(
        &mut self,
        settings: super::quality::QualitySettings,
    ) -> Result<super::quality::QualitySettings, String> {
        if let Some(path) = &self.quality_path {
            super::quality::save(path, settings)?;
        }
        self.stop();
        self.quality = settings;
        Ok(settings)
    }
    pub fn with_settings(
        settings: VolumeSettings,
        noise_settings: NoiseSettings,
        recordings_dir: PathBuf,
    ) -> Self {
        let settings = settings.validated();
        Self {
            ai: Default::default(),
            streams: None,
            state: MonitoringState::Stopped,
            last_fault: None,
            quality: Default::default(),
            quality_path: None,
            diagnostic_control: Default::default(),
            controls: SharedGainControls::new(settings),
            volume_settings: settings,
            effect_controls: SharedEffectParameters::new(EffectParameters::default()),
            effect_parameters: EffectParameters::default(),
            active_preset_id: Some("natural".into()),
            noise_controls: SharedNoiseSettings::new(noise_settings),
            noise_settings,
            recording: None,
            recordings_dir,
            routing_configuration: RoutingConfiguration::default(),
            soundboard_entries: Vec::new(),
            soundboard_commands: None,
            soundboard_ids: Vec::new(),
        }
    }

    pub fn set_routing_configuration(&mut self, configuration: RoutingConfiguration) {
        self.routing_configuration = configuration;
    }

    pub fn routing_configuration(&self) -> &RoutingConfiguration {
        &self.routing_configuration
    }

    pub fn set_soundboard_entries(&mut self, entries: Vec<SoundEntry>) {
        self.soundboard_entries = entries;
    }
    pub fn play_sound(&mut self, id: &str) -> Result<(), AudioDeviceError> {
        if self.soundboard_commands.is_none() {
            return Err(AudioDeviceError::backend(
                "Start audio processing before playing a sound",
            ));
        }
        let index = self.soundboard_ids.iter().position(|value| value == id).ok_or_else(|| AudioDeviceError::backend("Sound is unavailable in the active audio session; restart monitoring after library changes"))?;
        self.soundboard_commands
            .as_mut()
            .ok_or_else(|| {
                AudioDeviceError::backend("Start audio processing before playing a sound")
            })?
            .push(MixerCommand::Play(index))
            .map_err(|_| AudioDeviceError::backend("Soundboard command queue is full"))
    }
    pub fn stop_sound(&mut self, id: Option<&str>) -> Result<(), AudioDeviceError> {
        if self.soundboard_commands.is_none() {
            return Ok(());
        }
        let command = match id {
            Some(id) => MixerCommand::Stop(
                self.soundboard_ids
                    .iter()
                    .position(|value| value == id)
                    .ok_or_else(|| AudioDeviceError::backend("Sound not found"))?,
            ),
            None => MixerCommand::StopAll,
        };
        self.soundboard_commands
            .as_mut()
            .ok_or_else(|| AudioDeviceError::backend("Audio processing is stopped"))?
            .push(command)
            .map_err(|_| AudioDeviceError::backend("Soundboard command queue is full"))
    }

    pub fn recording_status(&self) -> RecordingStatus {
        self.recording
            .as_ref()
            .map(RecordingRuntime::status)
            .unwrap_or(RecordingStatus {
                recording: false,
                playing: false,
                hear_myself: false,
                source: RecordingSource::Processed,
                duration_seconds: 0.0,
                playback_seconds: 0.0,
                dropped_frames: 0,
                has_recording: false,
                incomplete: false,
                error: None,
            })
    }
    pub fn start_recording(&self) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("Start the audio session before recording"))?;
        r.start()?;
        Ok(r.status())
    }
    pub fn stop_recording(&self) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("No audio session"))?;
        r.stop()?;
        Ok(r.status())
    }
    pub fn play_recording(
        &self,
        source: RecordingSource,
    ) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("Start the audio session before playback"))?;
        r.play(source)?;
        Ok(r.status())
    }
    pub fn stop_playback(&self) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("No audio session"))?;
        r.stop_playback()?;
        Ok(r.status())
    }
    pub fn set_hear_myself(&self, value: bool) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("Start the audio session first"))?;
        r.set_hear_myself(value);
        Ok(r.status())
    }
    pub fn export_recording(
        &self,
        source: RecordingSource,
        path: PathBuf,
    ) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("No recording available"))?;
        r.export(source, path)?;
        Ok(r.status())
    }
    pub fn delete_recording(&self) -> Result<RecordingStatus, AudioDeviceError> {
        let r = self
            .recording
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("No audio session"))?;
        r.delete()?;
        Ok(r.status())
    }

    pub fn noise_settings(&self) -> NoiseSettings {
        self.noise_settings
    }
    pub fn set_noise_settings(&mut self, settings: NoiseSettings) -> NoiseSettings {
        let settings = self.noise_controls.store(settings);
        self.noise_settings = settings;
        settings
    }

    pub fn volume_settings(&self) -> VolumeSettings {
        self.volume_settings
    }

    pub fn set_volume_settings(&mut self, settings: VolumeSettings) -> VolumeSettings {
        let settings = settings.validated();
        self.volume_settings = settings;
        self.controls.apply(settings);
        settings
    }

    pub fn effect_parameters(&self) -> EffectParameters {
        self.effect_parameters
    }

    pub fn set_effect_parameters(&mut self, parameters: EffectParameters) -> EffectParameters {
        let parameters = self.effect_controls.store(parameters);
        self.effect_parameters = parameters;
        self.active_preset_id = None;
        parameters
    }

    pub fn set_effect_parameters_preserving_preset(
        &mut self,
        parameters: EffectParameters,
    ) -> EffectParameters {
        let parameters = self.effect_controls.store(parameters);
        self.effect_parameters = parameters;
        parameters
    }

    pub fn apply_preset(&mut self, id: &str, parameters: EffectParameters) -> EffectParameters {
        let parameters = self.effect_controls.store(parameters);
        self.effect_parameters = parameters;
        self.active_preset_id = Some(id.to_owned());
        self.ai.select_classic();
        parameters
    }

    pub fn preview_dsp_preset(&mut self, parameters: EffectParameters) -> EffectParameters {
        let applied = self.set_effect_parameters(parameters);
        self.ai.select_classic();
        applied
    }

    pub fn active_preset_id(&self) -> Option<&str> {
        self.active_preset_id.as_deref()
    }

    pub fn start(
        &mut self,
        input_id: &str,
        output_id: &str,
    ) -> Result<AudioRuntimeStatus, AudioDeviceError> {
        self.stop();
        let host = cpal::default_host();
        let input = find_device(&host, input_id)?;
        let output = find_device(&host, output_id)?;
        let (input_supported, output_supported) = compatible_configs(&input, &output)?;
        let sample_rate = input_supported.sample_rate();
        let input_channels = input_supported.channels();
        let output_channels = output_supported.channels();
        let input_format = input_supported.sample_format();
        let output_format = output_supported.sample_format();
        let input_config = stream_config(input_supported, self.quality.profile);
        let output_config = stream_config(output_supported, self.quality.profile);

        let ring_capacity = ((sample_rate * RING_BUFFER_MILLISECONDS / 1_000) as usize)
            .max(REQUESTED_BUFFER_FRAMES as usize * 2)
            .next_power_of_two();
        let (producer, consumer) = RingBuffer::<f32>::new(ring_capacity);
        let (diagnostic_writer, taps) =
            super::capture::CaptureWriter::new(self.diagnostic_control.clone())
                .map_err(AudioDeviceError::backend)?;
        let (sound_producer, sound_consumer) = RingBuffer::<f32>::new(ring_capacity);
        let mixer = SoundboardMixer::load(&self.soundboard_entries, sample_rate)
            .map_err(AudioDeviceError::backend)?;
        self.soundboard_ids = self
            .soundboard_entries
            .iter()
            .filter(|entry| !entry.missing)
            .map(|entry| entry.id.clone())
            .collect();
        let (soundboard_command_producer, soundboard_command_consumer) =
            RingBuffer::<MixerCommand>::new(64);
        self.soundboard_commands = Some(soundboard_command_producer);
        let metrics = SharedAudioMetrics::default();
        self.recording.take();
        let (recording, recording_endpoints) =
            RecordingRuntime::new(sample_rate, self.recordings_dir.clone())?;
        let RecordingEndpoints {
            record_producer,
            playback_consumer,
            shared: recording_shared,
        } = recording_endpoints;
        let (routing_producer, routing_sink) =
            if let Some(device_id) = self.routing_configuration.output_device_id.as_deref() {
                let (producer, consumer, shared) =
                    routing_queue_with_ai(sample_rate, self.ai.shared.clone());
                let sink = WindowsVirtualOutputSink::build_with_quality(
                    device_id,
                    sample_rate,
                    consumer,
                    shared,
                    self.noise_controls.clone(),
                    self.quality.profile,
                    Some(taps.virtual_output),
                )
                .map_err(|error| AudioDeviceError::backend(error.message))?;
                (Some(producer), Some(sink))
            } else {
                (None, None)
            };
        let (input_stream, dsp_latency_frames, worker) = build_input(
            &input,
            input_config,
            input_format,
            producer,
            metrics.clone(),
            self.controls.clone(),
            self.effect_controls.clone(),
            self.noise_controls.clone(),
            record_producer,
            recording_shared.clone(),
            routing_producer,
            mixer,
            soundboard_command_consumer,
            sound_producer,
            self.ai.attach_with_diagnostics(sample_rate, Some(taps.ai)),
            taps.input,
            taps.dsp,
        )?;
        let output_stream = build_output(
            &output,
            output_config,
            output_format,
            consumer,
            metrics.clone(),
            self.controls.clone(),
            self.noise_controls.clone(),
            playback_consumer,
            recording_shared,
            sound_consumer,
            self.ai.shared.clone(),
            self.quality.profile,
            taps.monitor,
        )?;
        let input_buffer_frames = input_stream
            .buffer_size()
            .unwrap_or(REQUESTED_BUFFER_FRAMES);
        let output_buffer_frames = output_stream
            .buffer_size()
            .unwrap_or(REQUESTED_BUFFER_FRAMES);

        output_stream.play().map_err(audio_error)?;
        if let Err(error) = input_stream.play() {
            let _ = output_stream.pause();
            return Err(audio_error(error));
        }

        self.streams = Some(ActiveStreams {
            input: input_stream,
            output: output_stream,
            input_device_id: input_id.to_owned(),
            output_device_id: output_id.to_owned(),
            sample_rate,
            input_channels,
            output_channels,
            input_sample_format: input_format.to_string(),
            output_sample_format: output_format.to_string(),
            input_buffer_frames,
            output_buffer_frames,
            dsp_latency_frames,
            metrics,
            routing: routing_sink,
            _worker: worker,
            _diagnostics: diagnostic_writer,
        });
        self.recording = Some(recording);
        self.state = MonitoringState::Running;
        Ok(self.status())
    }

    pub fn pause(&mut self) -> Result<AudioRuntimeStatus, AudioDeviceError> {
        let streams = self
            .streams
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("Monitoring is not active"))?;
        streams.input.pause().map_err(audio_error)?;
        if let Err(error) = streams.output.pause() {
            let _ = streams.input.play();
            return Err(audio_error(error));
        }
        self.state = MonitoringState::Paused;
        Ok(self.status())
    }

    pub fn resume(&mut self) -> Result<AudioRuntimeStatus, AudioDeviceError> {
        let streams = self
            .streams
            .as_ref()
            .ok_or_else(|| AudioDeviceError::backend("Monitoring has not been started"))?;
        streams.output.play().map_err(audio_error)?;
        if let Err(error) = streams.input.play() {
            let _ = streams.output.pause();
            return Err(audio_error(error));
        }
        self.state = MonitoringState::Running;
        Ok(self.status())
    }

    pub fn stop(&mut self) -> AudioRuntimeStatus {
        self.last_fault = None;
        if let Some(recording) = &self.recording {
            let _ = recording.stop();
            recording.set_hear_myself(false);
        }
        self.streams.take();
        self.soundboard_commands = None;
        self.soundboard_ids.clear();
        self.state = MonitoringState::Stopped;
        self.status()
    }

    pub fn start_routing(&mut self) -> Result<RoutingStatus, RoutingError> {
        self.refresh_status();
        if self.status().state != MonitoringState::Running {
            return Err(RoutingError::new(
                "sessionNotRunning",
                "Select the cable, enable Monitor, then start routing.",
            ));
        }
        if self.routing_configuration.output_device_id.is_none() {
            return Err(RoutingError::new(
                "notConfigured",
                "Select a virtual cable playback endpoint first",
            ));
        }
        let streams = self.streams.as_ref().ok_or_else(|| {
            RoutingError::new(
                "sessionNotRunning",
                "Start audio processing before virtual microphone routing",
            )
        })?;
        let sink = streams.routing.as_ref().ok_or_else(|| {
            RoutingError::new(
                "routingUnavailable",
                "Restart audio processing after selecting the virtual output",
            )
        })?;
        crate::routing::validate_virtual_output(&sink.device_name)?;
        sink.shared.route_generation.fetch_add(1, Ordering::AcqRel);
        sink.shared.faulted.store(false, Ordering::Release);
        sink.shared.active.store(true, Ordering::Release);
        if let Err(error) = sink.play() {
            sink.shared.active.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(self.routing_status())
    }

    /// Release failed streams on the control thread, preserving diagnostics until explicit retry/stop.
    pub fn refresh_status(&mut self) -> AudioRuntimeStatus {
        let status = self.status();
        if self.streams.is_some() && status.state == MonitoringState::Faulted {
            self.stop();
            self.last_fault = Some(status);
        }
        self.status()
    }

    pub fn status(&self) -> AudioRuntimeStatus {
        match &self.streams {
            Some(streams) => {
                let metrics = streams.metrics.snapshot(
                    streams.sample_rate,
                    streams.input_buffer_frames,
                    streams.output_buffer_frames,
                    streams.dsp_latency_frames,
                );
                AudioRuntimeStatus {
                    state: if metrics.stream_faulted {
                        MonitoringState::Faulted
                    } else {
                        self.state
                    },
                    input_device_id: Some(streams.input_device_id.clone()),
                    output_device_id: Some(streams.output_device_id.clone()),
                    sample_rate: Some(streams.sample_rate),
                    input_channels: Some(streams.input_channels),
                    output_channels: Some(streams.output_channels),
                    dsp_latency_frames: Some(streams.dsp_latency_frames),
                    metrics: Some(metrics),
                    input_sample_format: Some(streams.input_sample_format.clone()),
                    output_sample_format: Some(streams.output_sample_format.clone()),
                    diagnostic_recording: self.diagnostic_control.is_recording(),
                }
            }
            None => self
                .last_fault
                .clone()
                .map(|mut status| {
                    status.diagnostic_recording = self.diagnostic_control.is_recording();
                    status
                })
                .unwrap_or(AudioRuntimeStatus {
                    state: MonitoringState::Stopped,
                    input_device_id: None,
                    output_device_id: None,
                    sample_rate: None,
                    input_channels: None,
                    output_channels: None,
                    dsp_latency_frames: None,
                    metrics: None,
                    input_sample_format: None,
                    output_sample_format: None,
                    diagnostic_recording: self.diagnostic_control.is_recording(),
                }),
        }
    }
}

impl AudioRoutingService for AudioStreamManager {
    fn routing_status(&self) -> RoutingStatus {
        let configured_id = self.routing_configuration.output_device_id.clone();
        let Some(streams) = &self.streams else {
            return RoutingStatus {
                state: if configured_id.is_some() {
                    RoutingState::Ready
                } else {
                    RoutingState::NotConfigured
                },
                output_device_id: configured_id,
                output_device_name: None,
                sample_rate: None,
                channels: None,
                underrun_count: 0,
                overflow_count: 0,
                output_peak_dbfs: -96.0,
                endpoint_verified: false,
                message: Some("Audio processing is stopped; routing is not active.".into()),
            };
        };
        let Some(sink) = &streams.routing else {
            return RoutingStatus {
                state: RoutingState::NotConfigured,
                output_device_id: configured_id,
                output_device_name: None,
                sample_rate: None,
                channels: None,
                underrun_count: 0,
                overflow_count: 0,
                output_peak_dbfs: -96.0,
                endpoint_verified: false,
                message: Some("Restart audio processing after selecting a virtual output.".into()),
            };
        };
        let faulted = sink.shared.faulted.load(Ordering::Acquire);
        let active = sink.shared.active.load(Ordering::Acquire);
        RoutingStatus {
            state: if faulted {
                RoutingState::Faulted
            } else if active {
                RoutingState::Active
            } else {
                RoutingState::Ready
            },
            output_device_id: Some(sink.device_id.clone()),
            output_device_name: Some(sink.device_name.clone()),
            sample_rate: Some(sink.sample_rate),
            channels: Some(sink.channels),
            underrun_count: sink.shared.underruns.load(Ordering::Relaxed),
            overflow_count: sink.shared.overflows.load(Ordering::Relaxed),
            output_peak_dbfs: amplitude_to_dbfs(sink.shared.peak()),
            endpoint_verified: active && !faulted,
            message: if faulted {
                Some("The virtual output endpoint disconnected or reported a stream error.".into())
            } else if active {
                Some("Playback endpoint is open. Verify its paired capture endpoint in the target app.".into())
            } else {
                None
            },
        }
    }

    fn stop_routing(&mut self) -> Result<RoutingStatus, RoutingError> {
        if let Some(sink) = self
            .streams
            .as_ref()
            .and_then(|streams| streams.routing.as_ref())
        {
            sink.shared.active.store(false, Ordering::Release);
            sink.pause()?;
            sink.shared.set_peak(0.0);
        }
        Ok(self.routing_status())
    }
}

fn find_device(host: &cpal::Host, id: &str) -> Result<Device, AudioDeviceError> {
    let parsed =
        DeviceId::from_str(id).map_err(|error| AudioDeviceError::backend(error.to_string()))?;
    host.device_by_id(&parsed)
        .ok_or_else(|| AudioDeviceError::device_not_found(id))
}

fn compatible_configs(
    input: &Device,
    output: &Device,
) -> Result<(SupportedStreamConfig, SupportedStreamConfig), AudioDeviceError> {
    let inputs: Vec<_> = input
        .supported_input_configs()
        .map_err(audio_error)?
        .collect();
    let outputs: Vec<_> = output
        .supported_output_configs()
        .map_err(audio_error)?
        .collect();
    let sample_rate = choose_sample_rate(&inputs, &outputs).ok_or_else(|| {
        AudioDeviceError::backend("Input and output devices have no common sample rate")
    })?;
    let input_range = choose_range(&inputs, sample_rate)
        .ok_or_else(|| AudioDeviceError::backend("No compatible input format"))?;
    let output_range = choose_range(&outputs, sample_rate)
        .ok_or_else(|| AudioDeviceError::backend("No compatible output format"))?;
    Ok((
        input_range.with_sample_rate(sample_rate),
        output_range.with_sample_rate(sample_rate),
    ))
}

fn choose_sample_rate(
    inputs: &[SupportedStreamConfigRange],
    outputs: &[SupportedStreamConfigRange],
) -> Option<u32> {
    if supports_rate(inputs, PREFERRED_SAMPLE_RATE) && supports_rate(outputs, PREFERRED_SAMPLE_RATE)
    {
        return Some(PREFERRED_SAMPLE_RATE);
    }
    inputs
        .iter()
        .flat_map(|input| outputs.iter().map(move |output| (input, output)))
        .filter_map(|(input, output)| {
            let minimum = input.min_sample_rate().max(output.min_sample_rate());
            let maximum = input.max_sample_rate().min(output.max_sample_rate());
            (minimum <= maximum).then_some(maximum.min(PREFERRED_SAMPLE_RATE).max(minimum))
        })
        .max()
}

fn supports_rate(configs: &[SupportedStreamConfigRange], rate: u32) -> bool {
    configs
        .iter()
        .any(|config| config.min_sample_rate() <= rate && rate <= config.max_sample_rate())
}

fn choose_range(
    configs: &[SupportedStreamConfigRange],
    rate: u32,
) -> Option<SupportedStreamConfigRange> {
    configs
        .iter()
        .copied()
        .filter(|config| config.min_sample_rate() <= rate && rate <= config.max_sample_rate())
        .max_by_key(|config| sample_format_rank(config.sample_format()))
}

fn sample_format_rank(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 4,
        SampleFormat::I32 => 3,
        SampleFormat::I16 => 2,
        SampleFormat::U16 => 1,
        _ => 0,
    }
}

fn stream_config(
    config: SupportedStreamConfig,
    quality: super::quality::QualityProfile,
) -> StreamConfig {
    let buffer_size = match config.buffer_size() {
        SupportedBufferSize::Range { min, max } => BufferSize::Fixed(
            quality
                .requested_frames(config.sample_rate())
                .clamp(*min, *max),
        ),
        SupportedBufferSize::Unknown => BufferSize::Default,
    };
    StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size,
    }
}

trait RealtimeSample: cpal::SizedSample + Copy + Send + 'static {
    fn to_f32(self) -> f32;
    fn from_f32(sample: f32) -> Self;
}

impl RealtimeSample for f32 {
    #[inline]
    fn to_f32(self) -> f32 {
        if self.is_finite() { self } else { 0.0 }
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
}
impl RealtimeSample for i16 {
    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / 32768.0
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        crate::ai::transport::to_pcm16(sample)
    }
}
impl RealtimeSample for u16 {
    #[inline]
    fn to_f32(self) -> f32 {
        (f32::from(self) - 32768.0) / 32768.0
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        (i32::from(crate::ai::transport::to_pcm16(sample)) + 32768) as u16
    }
}
impl RealtimeSample for i32 {
    fn to_f32(self) -> f32 {
        (f64::from(self) / 2147483648.0) as f32
    }
    fn from_f32(sample: f32) -> Self {
        if sample.is_finite() {
            (f64::from(sample).clamp(-1.0, 1.0) * 2147483648.0)
                .round()
                .clamp(i32::MIN as f64, i32::MAX as f64) as i32
        } else {
            0
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_input(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    producer: Producer<f32>,
    metrics: SharedAudioMetrics,
    controls: SharedGainControls,
    effect_controls: SharedEffectParameters,
    noise_controls: SharedNoiseSettings,
    record_producer: Producer<RecordingFrame>,
    recording_shared: std::sync::Arc<crate::audio::recording::Shared>,
    routing_producer: Option<RoutingProducer>,
    mixer: SoundboardMixer,
    soundboard_commands: Consumer<MixerCommand>,
    sound_producer: Producer<f32>,
    ai_port: crate::ai::transport::RealtimePort,
    input_tap: super::capture::Tap,
    dsp_tap: super::capture::Tap,
) -> Result<(Stream, usize, DspWorker), AudioDeviceError> {
    macro_rules! build {
        ($ty:ty) => {
            build_input_typed::<$ty>(
                device,
                config,
                producer,
                metrics,
                controls,
                effect_controls,
                noise_controls,
                record_producer,
                recording_shared,
                routing_producer,
                mixer,
                soundboard_commands,
                sound_producer,
                ai_port,
                input_tap,
                dsp_tap,
            )
        };
    }
    match format {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::U16 => build!(u16),
        _ => Err(AudioDeviceError::backend(format!(
            "Unsupported input sample format: {format}"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_input_typed<T: RealtimeSample>(
    device: &Device,
    config: StreamConfig,
    mut producer: Producer<f32>,
    metrics: SharedAudioMetrics,
    controls: SharedGainControls,
    effect_controls: SharedEffectParameters,
    noise_controls: SharedNoiseSettings,
    mut record_producer: Producer<RecordingFrame>,
    recording_shared: std::sync::Arc<crate::audio::recording::Shared>,
    mut routing_producer: Option<RoutingProducer>,
    mut mixer: SoundboardMixer,
    mut soundboard_commands: Consumer<MixerCommand>,
    mut sound_producer: Producer<f32>,
    mut ai_port: crate::ai::transport::RealtimePort,
    mut input_tap: super::capture::Tap,
    mut dsp_tap: super::capture::Tap,
) -> Result<(Stream, usize, DspWorker), AudioDeviceError> {
    let channels = usize::from(config.channels);
    let sample_rate = config.sample_rate;
    let error_metrics = metrics.clone();
    let mut neural_post =
        crate::ai::hybrid::NeuralPostProcessor::new(config.sample_rate, effect_controls.clone());
    let mut pipeline = AudioPipeline::new(config.sample_rate, effect_controls, noise_controls);
    let dsp_latency_frames = pipeline.latency_frames();
    let signal_latency_frames = pipeline.signal_latency_frames();
    let (mut original_producer, mut original_consumer) =
        RingBuffer::new((dsp_latency_frames + 4096).next_power_of_two());
    for _ in 0..signal_latency_frames {
        let _ = original_producer.push(0.0);
    }
    let mut gain = SmoothedGain::new(controls.input_target(), config.sample_rate);
    let (mut raw_tx, mut raw_rx) =
        RingBuffer::<CaptureFrame>::new((config.sample_rate as usize / 5).next_power_of_two());
    let worker_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_copy = worker_stop.clone();
    let worker_metrics = metrics.clone();
    let worker_thread = std::thread::Builder::new()
        .name("voice-dsp-worker".into())
        .spawn(move || {
            while !stop_copy.load(Ordering::Acquire) && !raw_rx.is_abandoned() {
                let available = raw_rx.slots();
                if available == 0 {
                    std::thread::park_timeout(std::time::Duration::from_millis(1));
                    continue;
                }
                let started = Instant::now();
                let mut overflowed = false;
                let mut frames = 0_u64;
                for _ in 0..soundboard_commands.slots() {
                    if let Ok(command) = soundboard_commands.pop() {
                        mixer.command(command);
                    }
                }
                for _ in 0..available {
                    let Ok(frame) = raw_rx.pop() else {
                        break;
                    };
                    let _ = original_producer.push(frame.raw);
                    dsp_tap.push(
                        super::capture::Stage::BypassReference,
                        sample_rate,
                        frame.gained,
                    );
                    let converted = ai_port.process(frame.gained);
                    let classic = converted.is_none();
                    let mut emit = |processed| {
                        let (headphone_sound, virtual_sound) = mixer.next_sample();
                        let original = original_consumer.pop().unwrap_or(0.0);
                        recording_shared.push_recording(
                            &mut record_producer,
                            RecordingFrame {
                                original,
                                processed,
                            },
                        );
                        if classic {
                            dsp_tap.push(super::capture::Stage::ClassicDsp, sample_rate, processed);
                        }
                        dsp_tap.push(
                            super::capture::Stage::Mixer,
                            sample_rate,
                            processed + virtual_sound,
                        );
                        if let Some(routing) = routing_producer.as_mut() {
                            routing.push(processed + virtual_sound);
                        }
                        let _ = sound_producer.push(headphone_sound);
                        overflowed |= producer.push(processed).is_err();
                    };
                    if let Some(converted) = converted {
                        let (hybrid, generation) = ai_port.post_processing();
                        emit(if hybrid {
                            neural_post.process(converted, generation)
                        } else {
                            converted
                        });
                    } else {
                        pipeline.push(frame.gained, &mut emit);
                    }
                    frames += 1;
                }
                if overflowed {
                    worker_metrics.record_overflow();
                }
                worker_metrics.record_processing(
                    frames,
                    started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                );
            }
        })
        .map_err(|e| AudioDeviceError::backend(e.to_string()))?;
    let worker = DspWorker {
        stop: worker_stop,
        thread: Some(worker_thread),
    };
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let started = Instant::now();
                let mut overflowed = false;
                let mut meter = SignalMeter::default();
                for frame in data.chunks_exact(channels) {
                    input_tap.push_frame(
                        super::capture::Stage::RawInput,
                        sample_rate,
                        channels as u16,
                        |channel| frame[channel].to_f32(),
                    );
                    let mono =
                        frame.iter().map(|sample| sample.to_f32()).sum::<f32>() / channels as f32;
                    let gained = gain.process(mono, controls.input_target());
                    meter.push(gained);
                    input_tap.push(super::capture::Stage::InputGain, sample_rate, gained);
                    overflowed |= raw_tx.push(CaptureFrame { raw: mono, gained }).is_err();
                }
                if overflowed {
                    metrics.record_overflow();
                }
                metrics.set_input_meter(meter.finish());
                metrics.record_capture_callback(
                    started.elapsed().as_nanos().min(u64::MAX as u128) as u64
                );
            },
            move |error| error_metrics.record_driver_error(error, true),
            None,
        )
        .map(|stream| (stream, dsp_latency_frames, worker))
        .map_err(audio_error)
}

#[allow(clippy::too_many_arguments)]
fn build_output(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    consumer: Consumer<f32>,
    metrics: SharedAudioMetrics,
    controls: SharedGainControls,
    noise_controls: SharedNoiseSettings,
    playback_consumer: Consumer<f32>,
    recording_shared: std::sync::Arc<crate::audio::recording::Shared>,
    sound_consumer: Consumer<f32>,
    ai_shared: std::sync::Arc<crate::ai::transport::SharedTransport>,
    quality: super::quality::QualityProfile,
    monitor_tap: super::capture::Tap,
) -> Result<Stream, AudioDeviceError> {
    macro_rules! build {
        ($ty:ty) => {
            build_output_typed::<$ty>(
                device,
                config,
                consumer,
                metrics,
                controls,
                noise_controls,
                playback_consumer,
                recording_shared,
                sound_consumer,
                ai_shared,
                quality,
                monitor_tap,
            )
        };
    }
    match format {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::U16 => build!(u16),
        _ => Err(AudioDeviceError::backend(format!(
            "Unsupported output sample format: {format}"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_output_typed<T: RealtimeSample>(
    device: &Device,
    config: StreamConfig,
    consumer: Consumer<f32>,
    metrics: SharedAudioMetrics,
    controls: SharedGainControls,
    noise_controls: SharedNoiseSettings,
    mut playback_consumer: Consumer<f32>,
    recording_shared: std::sync::Arc<crate::audio::recording::Shared>,
    sound_consumer: Consumer<f32>,
    ai_shared: std::sync::Arc<crate::ai::transport::SharedTransport>,
    quality: super::quality::QualityProfile,
    mut monitor_tap: super::capture::Tap,
) -> Result<Stream, AudioDeviceError> {
    let channels = usize::from(config.channels);
    let sample_rate = config.sample_rate;
    let error_metrics = metrics.clone();
    let mut gain = SmoothedGain::new(controls.output_target(), config.sample_rate);
    let mut limiter = OutputLimiter::new(config.sample_rate);
    let mut privacy = crate::ai::transport::OutputPrivacyGate::new(ai_shared);
    let mut reader = super::resample::ClockedReader::new(
        consumer,
        config.sample_rate,
        config.sample_rate,
        quality.negotiated_startup_ms(config.sample_rate, config.buffer_size),
        quality.taps(),
    )
    .map_err(AudioDeviceError::backend)?;
    let mut sound_reader = super::resample::ClockedReader::new(
        sound_consumer,
        config.sample_rate,
        config.sample_rate,
        quality.negotiated_startup_ms(config.sample_rate, config.buffer_size),
        quality.taps(),
    )
    .map_err(AudioDeviceError::backend)?;
    metrics.set_playback_algorithm_frames(reader.delay_frames() + limiter.latency_frames());
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let started = Instant::now();
                let underruns = reader.underruns;
                let mut meter = SignalMeter::default();
                let noise = noise_controls.load();
                for frame in data.chunks_exact_mut(channels) {
                    if privacy.discard_if_changed(reader.consumer_mut()) {
                        reader.reset();
                    }
                    let source = reader.next_sample();
                    let source = privacy.filter(source);
                    let source = recording_shared
                        .playback_or_monitor(&mut playback_consumer, source)
                        + sound_reader.next_sample();
                    let sample = gain.process(source, controls.output_target());
                    let (sample, protected) =
                        limiter.process(sample, noise.limiter_enabled, noise.limiter_ceiling_dbfs);
                    if protected {
                        metrics.record_protected_clip();
                    }
                    meter.push(sample);
                    monitor_tap.push(
                        super::capture::Stage::Monitor,
                        sample_rate,
                        T::from_f32(sample).to_f32(),
                    );
                    frame.fill(T::from_f32(sample));
                }
                if reader.underruns > underruns {
                    metrics.record_underrun();
                }
                metrics.set_output_meter(meter.finish());
                metrics.set_ring_depth(reader.depth());
                metrics.record_output_callback(
                    started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                    reader.correction_ppm(),
                    reader.discarded,
                );
            },
            move |error| error_metrics.record_driver_error(error, false),
            None,
        )
        .map_err(audio_error)
}

fn audio_error(error: cpal::Error) -> AudioDeviceError {
    AudioDeviceError::backend(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_a_dsp_preset_restores_processed_audio_after_neural_voice() {
        for preview in [false, true] {
            let mut runtime = AudioStreamManager::default();
            let mut input = runtime.ai.attach(48_000);
            let mut output = crate::ai::transport::OutputPrivacyGate::new(runtime.ai.shared.clone());
            let (mut tx, mut rx) = RingBuffer::new(32);
            runtime.ai.begin_request("neural");
            assert_eq!(input.process(0.2), Some(0.0));
            assert_eq!(output.filter(0.2), 0.0);
            output.discard_if_changed(&mut rx);
            tx.push(0.6).unwrap();

            let settings = EffectParameters {
                pitch_semitones: -4.0,
                ..EffectParameters::default()
            };
            // Shared effect editing is also used by Hybrid and must keep AI selected.
            runtime.set_effect_parameters(settings);
            assert_eq!(runtime.ai.status().mode, "wOkada");
            if preview {
                runtime.preview_dsp_preset(settings);
            } else {
                runtime.apply_preset("deep", settings);
            }

            assert!(output.discard_if_changed(&mut rx));
            assert!(rx.pop().is_err(), "Old neural output must be discarded");
            assert_eq!(runtime.ai.status().mode, "classicDsp");
            assert!(input.process(0.2).is_none());
            let mut pipeline = AudioPipeline::new(
                48_000,
                runtime.effect_controls.clone(),
                runtime.noise_controls.clone(),
            );
            let mut peak: f32 = 0.0;
            for frame in 0..12_000 {
                let sample = (frame as f32 * std::f32::consts::TAU * 220.0 / 48_000.0).sin() * 0.2;
                pipeline.push(sample, |value| peak = peak.max(output.filter(value).abs()));
            }
            assert!(peak > 0.01, "Classic DSP audio must reach the output gate");
        }
    }
    #[test]
    fn stopped_soundboard_explains_engine_state_before_clip_lookup() {
        let mut manager = AudioStreamManager::default();
        assert!(
            manager
                .play_sound("not-loaded")
                .unwrap_err()
                .message
                .contains("Start audio processing")
        );
        assert!(manager.stop_sound(Some("not-loaded")).is_ok());
        assert_eq!(manager.status().state, MonitoringState::Stopped);
    }
    #[test]
    fn device_pcm_boundaries_are_symmetric_saturated_and_finite() {
        assert_eq!(<i16 as RealtimeSample>::to_f32(i16::MIN), -1.0);
        assert_eq!(<u16 as RealtimeSample>::to_f32(32768), 0.0);
        assert_eq!(<i32 as RealtimeSample>::to_f32(i32::MIN), -1.0);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(<i32 as RealtimeSample>::from_f32(value), 0);
            assert_eq!(<i16 as RealtimeSample>::from_f32(value), 0);
            assert_eq!(<u16 as RealtimeSample>::from_f32(value), 32768);
            assert_eq!(<f32 as RealtimeSample>::from_f32(value), 0.0);
        }
        assert_eq!(<i32 as RealtimeSample>::from_f32(1.2), i32::MAX);
        assert_eq!(<i32 as RealtimeSample>::from_f32(-1.2), i32::MIN);
        assert_eq!(
            <i16 as RealtimeSample>::from_f32(1.2).to_le_bytes(),
            [255, 127]
        );
        assert_eq!(
            <i16 as RealtimeSample>::from_f32(-1.2).to_le_bytes(),
            [0, 128]
        );
        assert_eq!(<u16 as RealtimeSample>::from_f32(-1.2), 0);
        assert_eq!(<u16 as RealtimeSample>::from_f32(1.2), 65535);
    }

    #[test]
    fn converts_common_sample_formats() {
        assert_eq!(i16::MIN.to_f32(), -1.0);
        assert_eq!(i16::MAX.to_f32(), 32767.0 / 32768.0);
        assert_eq!(u16::MAX.to_f32(), 32767.0 / 32768.0);
        assert_eq!(i16::from_f32(0.0), 0);
        assert!((u16::from_f32(0.0) as i32 - 32_768).abs() <= 1);
    }

    #[test]
    fn ring_buffer_handles_overflow_and_underflow_without_panicking() {
        let (mut producer, mut consumer) = RingBuffer::new(1);
        assert!(producer.push(0.5).is_ok());
        assert!(producer.push(0.7).is_err());
        assert_eq!(consumer.pop().ok(), Some(0.5));
        assert!(consumer.pop().is_err());
    }

    #[test]
    fn stop_is_idempotent() {
        let mut manager = AudioStreamManager::default();
        assert_eq!(manager.stop().state, MonitoringState::Stopped);
        assert_eq!(manager.stop().state, MonitoringState::Stopped);
    }

    #[test]
    fn start_reports_missing_devices() {
        let mut manager = AudioStreamManager::default();
        assert!(
            manager
                .start("wasapi:missing-input", "wasapi:missing-output")
                .is_err()
        );
        assert_eq!(manager.status().state, MonitoringState::Stopped);
    }
}

#[cfg(test)]
mod manual_hardware_tests {
    use super::*;

    #[test]
    #[ignore = "manual local VB-Cable loopback: synthetic tone only, microphone/headphone gain muted; close calls first"]
    fn synthetic_soundboard_reaches_paired_virtual_capture() {
        use std::sync::{Arc, Mutex, atomic::AtomicBool};
        let host = cpal::default_host();
        let duration = std::env::var("VOICEMORPH_LOOPBACK_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1)
            .clamp(1, 120);
        let profile = match std::env::var("VOICEMORPH_LOOPBACK_PROFILE").as_deref() {
            Ok("lowLatency") => super::super::quality::QualityProfile::LowLatency,
            Ok("highQuality") => super::super::quality::QualityProfile::HighQuality,
            _ => super::super::quality::QualityProfile::Balanced,
        };
        let find = |input: bool, prefix: &str| {
            let devices = if input {
                host.input_devices().unwrap()
            } else {
                host.output_devices().unwrap()
            };
            devices
                .into_iter()
                .find(|device| device.description().unwrap().name().starts_with(prefix))
                .unwrap()
        };
        let cable = find(false, "CABLE Input (");
        let capture = find(true, "CABLE Output (");
        let supported = capture.default_input_config().unwrap();
        assert_eq!(supported.sample_format(), SampleFormat::F32);
        let rate = supported.sample_rate();
        let channels = usize::from(supported.channels());
        let limit = rate as usize * (duration as usize + 6);
        let captured = Arc::new(Mutex::new(Vec::<f32>::with_capacity(limit)));
        let data_copy = captured.clone();
        let failed = Arc::new(AtomicBool::new(false));
        let fail_copy = failed.clone();
        let receiver = capture
            .build_input_stream(
                supported.config(),
                move |data: &[f32], _| {
                    if let Ok(mut samples) = data_copy.try_lock() {
                        let remaining = limit.saturating_sub(samples.len());
                        samples.extend(
                            data.chunks_exact(channels)
                                .take(remaining)
                                .map(|frame| frame[0]),
                        );
                    }
                },
                move |error| {
                    if !matches!(
                        error.kind(),
                        cpal::ErrorKind::Xrun
                            | cpal::ErrorKind::DeviceChanged
                            | cpal::ErrorKind::RealtimeDenied
                    ) {
                        fail_copy.store(true, Ordering::Release);
                    }
                },
                None,
            )
            .unwrap();
        receiver.play().unwrap();
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../.ai-runtime-build/virtual-mic-check");
        std::fs::create_dir_all(&project).unwrap();
        let tone = project.join("synthetic-997hz.wav");
        let mut writer = hound::WavWriter::create(
            &tone,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for n in 0..48000 * duration {
            writer
                .write_sample(
                    ((n as f32 * std::f32::consts::TAU * 997.0 / 48000.0).sin()
                        * 0.04
                        * i16::MAX as f32) as i16,
                )
                .unwrap();
        }
        writer.finalize().unwrap();
        let mut manager = AudioStreamManager::with_settings(
            VolumeSettings {
                input_muted: true,
                output_muted: true,
                ..Default::default()
            },
            NoiseSettings::default(),
            project.join("empty-recording"),
        );
        manager
            .set_quality(super::super::quality::QualitySettings { profile })
            .unwrap();
        manager.set_soundboard_entries(vec![crate::soundboard::SoundEntry {
            id: "loopback-tone".into(),
            name: "Local synthetic test".into(),
            file_path: tone.to_string_lossy().into(),
            category: "test".into(),
            favorite: false,
            volume_percent: 100.0,
            route: crate::soundboard::SoundRoute::VirtualMicrophone,
            hotkey: None,
            duration_seconds: duration as f32,
            missing: false,
        }]);
        manager.set_routing_configuration(RoutingConfiguration {
            output_device_id: Some(cable.id().unwrap().to_string()),
            output_device_fingerprint: None,
        });
        manager
            .start(
                &host
                    .default_input_device()
                    .unwrap()
                    .id()
                    .unwrap()
                    .to_string(),
                &host
                    .default_output_device()
                    .unwrap()
                    .id()
                    .unwrap()
                    .to_string(),
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            captured
                .lock()
                .unwrap()
                .iter()
                .all(|sample| sample.abs() < 0.001),
            "Cable already has audio from another source; stop the other source before testing"
        );
        assert_eq!(manager.start_routing().unwrap().state, RoutingState::Active);
        if std::env::var("VOICEMORPH_LOOPBACK_CAPTURE").as_deref() == Ok("1") {
            manager
                .start_capture(
                    vec![
                        super::super::capture::Stage::Mixer,
                        super::super::capture::Stage::VirtualOutput,
                    ],
                    u64::from(duration.min(30)),
                    true,
                )
                .unwrap();
        }
        manager.play_sound("loopback-tone").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(
            u64::from(duration) * 1000 + 600,
        ));
        let route = manager.routing_status();
        let audio_status = manager.refresh_status();
        let capture_status = manager.capture_status();
        manager.stop();
        receiver.pause().unwrap();
        assert!(!failed.load(Ordering::Acquire));
        assert_ne!(route.state, RoutingState::Faulted);
        let samples = captured.lock().unwrap();
        let peak = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        assert!(peak > 0.005, "No test tone arrived at paired CABLE Output");
        let first = samples
            .iter()
            .position(|sample| sample.abs() > 0.005)
            .unwrap();
        let last = samples
            .iter()
            .rposition(|sample| sample.abs() > 0.005)
            .unwrap();
        let voiced = &samples[first..=last];
        let crossings = voiced
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        let frequency = crossings as f64 * f64::from(rate) / voiced.len() as f64;
        assert!(
            (frequency - 997.0).abs() < 15.0,
            "Unexpected received frequency {frequency}"
        );
        let report = serde_json::json!({"kind":"Actual local VoiceMorph soundboard -> VB-Cable playback -> paired capture", "profile":profile,"durationSeconds":duration,"audio":audio_status,"stageCapture":capture_status,"microphoneMuted":true,"physicalOutputMuted":true,"recordedMicrophonePcm":false,"captureRate":rate,"receivedFrames":samples.len(),"peak":peak,"frequencyHz":frequency,"expectedFrequencyHz":997,"route":route,"limit":"Not a Discord call or Discord Mic Test. Only synthetic tone was sent; no microphone audio saved. DSP receives muted microphone; this measures transport, not voiced-speech DSP CPU."});
        std::fs::write(
            project.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        std::fs::write(
            project.join(format!("report-{profile:?}-{duration}s.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        println!("{report}");
    }

    #[test]
    #[ignore = "manual hardware diagnostic: opens microphone briefly with output muted; no recording"]
    fn monitors_muted_without_latching_recoverable_driver_notifications() {
        let host = cpal::default_host();
        let input = host
            .default_input_device()
            .unwrap()
            .id()
            .unwrap()
            .to_string();
        let output = host
            .default_output_device()
            .unwrap()
            .id()
            .unwrap()
            .to_string();
        let mut manager = AudioStreamManager::with_settings(
            VolumeSettings {
                output_muted: true,
                ..Default::default()
            },
            NoiseSettings::default(),
            std::env::temp_dir().join("voicemorph-muted-diagnostic"),
        );
        manager.start(&input, &output).unwrap();
        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let status = manager.refresh_status();
            assert_eq!(status.state, MonitoringState::Running, "{status:?}");
        }
        println!("Muted hardware diagnostic: {:?}", manager.status());
        // Inject a terminal notification after real streaming, without disabling a user's device.
        manager
            .streams
            .as_ref()
            .unwrap()
            .metrics
            .record_driver_error(cpal::ErrorKind::DeviceNotAvailable.into(), false);
        let fault = manager.refresh_status();
        assert_eq!(fault.state, MonitoringState::Faulted);
        assert!(manager.streams.is_none());
        assert!(
            fault
                .metrics
                .unwrap()
                .last_stream_error
                .unwrap()
                .contains("Audio output")
        );
        assert!(!manager.recording_status().recording);
        assert_eq!(manager.status().state, MonitoringState::Faulted);
        assert_eq!(manager.stop().state, MonitoringState::Stopped);
    }

    #[test]
    #[ignore = "manual hardware test: wear headphones before opening live microphone monitoring"]
    fn starts_and_stops_default_hardware_twice() {
        let host = cpal::default_host();
        let input_id = host
            .default_input_device()
            .and_then(|device| device.id().ok())
            .expect("No default input device")
            .to_string();
        let output_id = host
            .default_output_device()
            .and_then(|device| device.id().ok())
            .expect("No default output device")
            .to_string();
        let mut manager = AudioStreamManager::default();
        for _ in 0..2 {
            manager
                .start(&input_id, &output_id)
                .expect("Could not start live monitoring");
            std::thread::sleep(std::time::Duration::from_millis(500));
            assert_eq!(manager.stop().state, MonitoringState::Stopped);
        }
    }

    #[test]
    #[ignore = "manual hardware test: requires microphone, output and virtual cable endpoints"]
    fn routes_live_pipeline_to_virtual_cable() {
        let host = cpal::default_host();
        let input_id = host
            .default_input_device()
            .and_then(|d| d.id().ok())
            .expect("No default input")
            .to_string();
        let output_id = host
            .default_output_device()
            .and_then(|d| d.id().ok())
            .expect("No default output")
            .to_string();
        let virtual_device = host
            .output_devices()
            .expect("WASAPI output enumeration failed")
            .find(|device| {
                device.description().ok().is_some_and(|description| {
                    crate::routing::likely_virtual_device(description.name())
                })
            })
            .expect("No virtual cable playback endpoint");
        let virtual_id = virtual_device
            .id()
            .expect("Virtual endpoint has no stable ID")
            .to_string();
        let mut manager = AudioStreamManager::default();
        manager.set_routing_configuration(RoutingConfiguration {
            output_device_id: Some(virtual_id),
            output_device_fingerprint: None,
        });
        manager
            .start(&input_id, &output_id)
            .expect("Could not start shared microphone pipeline");
        let active = manager
            .start_routing()
            .expect("Could not start virtual routing");
        assert_eq!(active.state, RoutingState::Active);
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert_ne!(manager.routing_status().state, RoutingState::Faulted);
        manager.stop();
        assert_eq!(manager.routing_status().state, RoutingState::Ready);
    }
}
