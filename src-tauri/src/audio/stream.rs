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
    VirtualOutputSink, amplitude_to_dbfs, errors::RoutingError, routing_queue,
    windows::WindowsVirtualOutputSink,
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
const RING_BUFFER_MILLISECONDS: u32 = 40;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MonitoringState {
    Stopped,
    Running,
    Paused,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
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
}

struct ActiveStreams {
    input: Stream,
    output: Stream,
    input_device_id: String,
    output_device_id: String,
    sample_rate: u32,
    input_channels: u16,
    output_channels: u16,
    input_buffer_frames: u32,
    output_buffer_frames: u32,
    dsp_latency_frames: usize,
    metrics: SharedAudioMetrics,
    routing: Option<WindowsVirtualOutputSink>,
}

pub struct AudioStreamManager {
    streams: Option<ActiveStreams>,
    state: MonitoringState,
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
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self {
            streams: None,
            state: MonitoringState::Stopped,
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
        }
    }
}

impl AudioStreamManager {
    pub fn with_settings(
        settings: VolumeSettings,
        noise_settings: NoiseSettings,
        recordings_dir: PathBuf,
    ) -> Self {
        let settings = settings.validated();
        Self {
            streams: None,
            state: MonitoringState::Stopped,
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
        }
    }

    pub fn set_routing_configuration(&mut self, configuration: RoutingConfiguration) {
        self.routing_configuration = configuration;
    }

    pub fn routing_configuration(&self) -> &RoutingConfiguration {
        &self.routing_configuration
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

    pub fn apply_preset(&mut self, id: &str, parameters: EffectParameters) -> EffectParameters {
        let parameters = self.effect_controls.store(parameters);
        self.effect_parameters = parameters;
        self.active_preset_id = Some(id.to_owned());
        parameters
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
        let input_config = stream_config(input_supported);
        let output_config = stream_config(output_supported);

        let ring_capacity = ((sample_rate * RING_BUFFER_MILLISECONDS / 1_000) as usize)
            .max(REQUESTED_BUFFER_FRAMES as usize * 2)
            .next_power_of_two();
        let (mut producer, consumer) = RingBuffer::<f32>::new(ring_capacity);
        for _ in 0..REQUESTED_BUFFER_FRAMES.min(ring_capacity as u32) {
            let _ = producer.push(0.0);
        }
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
                let (producer, consumer, shared) = routing_queue(sample_rate);
                let sink = WindowsVirtualOutputSink::build(
                    device_id,
                    sample_rate,
                    consumer,
                    shared,
                    self.noise_controls.clone(),
                )
                .map_err(|error| AudioDeviceError::backend(error.message))?;
                (Some(producer), Some(sink))
            } else {
                (None, None)
            };
        let (input_stream, dsp_latency_frames) = build_input(
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
            input_buffer_frames,
            output_buffer_frames,
            dsp_latency_frames,
            metrics,
            routing: routing_sink,
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
        if let Some(recording) = &self.recording {
            let _ = recording.stop();
            recording.set_hear_myself(false);
        }
        self.streams.take();
        self.state = MonitoringState::Stopped;
        self.status()
    }

    pub fn start_routing(&mut self) -> Result<RoutingStatus, RoutingError> {
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
        sink.shared.faulted.store(false, Ordering::Release);
        sink.shared.active.store(true, Ordering::Release);
        if let Err(error) = sink.play() {
            sink.shared.active.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(self.routing_status())
    }

    pub fn status(&self) -> AudioRuntimeStatus {
        match &self.streams {
            Some(streams) => AudioRuntimeStatus {
                state: self.state,
                input_device_id: Some(streams.input_device_id.clone()),
                output_device_id: Some(streams.output_device_id.clone()),
                sample_rate: Some(streams.sample_rate),
                input_channels: Some(streams.input_channels),
                output_channels: Some(streams.output_channels),
                dsp_latency_frames: Some(streams.dsp_latency_frames),
                metrics: Some(streams.metrics.snapshot(
                    streams.sample_rate,
                    streams.input_buffer_frames,
                    streams.output_buffer_frames,
                    streams.dsp_latency_frames,
                )),
            },
            None => AudioRuntimeStatus {
                state: MonitoringState::Stopped,
                input_device_id: None,
                output_device_id: None,
                sample_rate: None,
                input_channels: None,
                output_channels: None,
                dsp_latency_frames: None,
                metrics: None,
            },
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
        SampleFormat::F32 => 3,
        SampleFormat::I16 => 2,
        SampleFormat::U16 => 1,
        _ => 0,
    }
}

fn stream_config(config: SupportedStreamConfig) -> StreamConfig {
    let buffer_size = match config.buffer_size() {
        SupportedBufferSize::Range { min, max } => {
            BufferSize::Fixed(REQUESTED_BUFFER_FRAMES.clamp(*min, *max))
        }
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
        self
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        sample
    }
}
impl RealtimeSample for i16 {
    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
    }
}
impl RealtimeSample for u16 {
    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / u16::MAX as f32 * 2.0 - 1.0
    }
    #[inline]
    fn from_f32(sample: f32) -> Self {
        ((sample.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f32).round() as u16
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
) -> Result<(Stream, usize), AudioDeviceError> {
    match format {
        SampleFormat::F32 => build_input_typed::<f32>(
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
        ),
        SampleFormat::I16 => build_input_typed::<i16>(
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
        ),
        SampleFormat::U16 => build_input_typed::<u16>(
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
        ),
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
) -> Result<(Stream, usize), AudioDeviceError> {
    let channels = usize::from(config.channels);
    let error_metrics = metrics.clone();
    let mut pipeline = AudioPipeline::new(config.sample_rate, effect_controls, noise_controls);
    let dsp_latency_frames = pipeline.latency_frames();
    let (mut original_producer, mut original_consumer) =
        RingBuffer::new((dsp_latency_frames + 4096).next_power_of_two());
    for _ in 0..dsp_latency_frames {
        let _ = original_producer.push(0.0);
    }
    let mut gain = SmoothedGain::new(controls.input_target(), config.sample_rate);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let started = Instant::now();
                let mut overflowed = false;
                let mut meter = SignalMeter::default();
                let mut frames = 0_u64;
                for frame in data.chunks_exact(channels) {
                    let mono =
                        frame.iter().map(|sample| sample.to_f32()).sum::<f32>() / channels as f32;
                    let _ = original_producer.push(mono);
                    pipeline.push(mono, |processed| {
                        let processed = gain.process(processed, controls.input_target());
                        let original = original_consumer.pop().unwrap_or(0.0);
                        recording_shared.push_recording(
                            &mut record_producer,
                            RecordingFrame {
                                original,
                                processed,
                            },
                        );
                        meter.push(processed);
                        if let Some(routing) = routing_producer.as_mut() {
                            routing.push(processed);
                        }
                        overflowed |= producer.push(processed).is_err();
                    });
                    frames += 1;
                }
                if overflowed {
                    metrics.record_overflow();
                }
                metrics.set_input_meter(meter.finish());
                metrics.set_ring_depth(producer.buffer().capacity() - producer.slots());
                metrics.record_processing(
                    frames,
                    started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                );
            },
            move |_| error_metrics.record_stream_error(),
            None,
        )
        .map(|stream| (stream, dsp_latency_frames))
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
) -> Result<Stream, AudioDeviceError> {
    match format {
        SampleFormat::F32 => build_output_typed::<f32>(
            device,
            config,
            consumer,
            metrics,
            controls,
            noise_controls,
            playback_consumer,
            recording_shared,
        ),
        SampleFormat::I16 => build_output_typed::<i16>(
            device,
            config,
            consumer,
            metrics,
            controls,
            noise_controls,
            playback_consumer,
            recording_shared,
        ),
        SampleFormat::U16 => build_output_typed::<u16>(
            device,
            config,
            consumer,
            metrics,
            controls,
            noise_controls,
            playback_consumer,
            recording_shared,
        ),
        _ => Err(AudioDeviceError::backend(format!(
            "Unsupported output sample format: {format}"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_output_typed<T: RealtimeSample>(
    device: &Device,
    config: StreamConfig,
    mut consumer: Consumer<f32>,
    metrics: SharedAudioMetrics,
    controls: SharedGainControls,
    noise_controls: SharedNoiseSettings,
    mut playback_consumer: Consumer<f32>,
    recording_shared: std::sync::Arc<crate::audio::recording::Shared>,
) -> Result<Stream, AudioDeviceError> {
    let channels = usize::from(config.channels);
    let error_metrics = metrics.clone();
    let mut gain = SmoothedGain::new(controls.output_target(), config.sample_rate);
    let mut limiter = OutputLimiter::new(config.sample_rate);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut underrun = false;
                let mut meter = SignalMeter::default();
                for frame in data.chunks_exact_mut(channels) {
                    let source = match consumer.pop() {
                        Ok(sample) => sample,
                        Err(_) => {
                            underrun = true;
                            0.0
                        }
                    };
                    let source =
                        recording_shared.playback_or_monitor(&mut playback_consumer, source);
                    let sample = gain.process(source, controls.output_target());
                    let noise = noise_controls.load();
                    let (sample, protected) =
                        limiter.process(sample, noise.limiter_enabled, noise.limiter_ceiling_dbfs);
                    if protected {
                        metrics.record_protected_clip();
                    }
                    meter.push(sample);
                    frame.fill(T::from_f32(sample));
                }
                if underrun {
                    metrics.record_underrun();
                }
                metrics.set_output_meter(meter.finish());
                metrics.set_ring_depth(consumer.slots());
            },
            move |_| error_metrics.record_stream_error(),
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
    fn converts_common_sample_formats() {
        assert!((i16::MAX.to_f32() - 1.0).abs() < f32::EPSILON);
        assert!((u16::MAX.to_f32() - 1.0).abs() < f32::EPSILON);
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
}
