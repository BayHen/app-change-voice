use super::{
    errors::AudioDeviceError,
    metrics::{AudioMetrics, SharedAudioMetrics},
    pipeline::AudioPipeline,
    processor::PassthroughProcessor,
};
use cpal::{
    BufferSize, Device, DeviceId, SampleFormat, Stream, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig, SupportedStreamConfigRange,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::Serialize;
use std::{str::FromStr, time::Instant};

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
    metrics: SharedAudioMetrics,
}

pub struct AudioStreamManager {
    streams: Option<ActiveStreams>,
    state: MonitoringState,
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self {
            streams: None,
            state: MonitoringState::Stopped,
        }
    }
}

impl AudioStreamManager {
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
        let input_stream = build_input(
            &input,
            input_config,
            input_format,
            producer,
            metrics.clone(),
        )?;
        let output_stream = build_output(
            &output,
            output_config,
            output_format,
            consumer,
            metrics.clone(),
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
            metrics,
        });
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
        self.streams.take();
        self.state = MonitoringState::Stopped;
        self.status()
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
                metrics: Some(streams.metrics.snapshot(
                    streams.sample_rate,
                    streams.input_buffer_frames,
                    streams.output_buffer_frames,
                )),
            },
            None => AudioRuntimeStatus {
                state: MonitoringState::Stopped,
                input_device_id: None,
                output_device_id: None,
                sample_rate: None,
                input_channels: None,
                output_channels: None,
                metrics: None,
            },
        }
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

fn build_input(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    producer: Producer<f32>,
    metrics: SharedAudioMetrics,
) -> Result<Stream, AudioDeviceError> {
    match format {
        SampleFormat::F32 => build_input_typed::<f32>(device, config, producer, metrics),
        SampleFormat::I16 => build_input_typed::<i16>(device, config, producer, metrics),
        SampleFormat::U16 => build_input_typed::<u16>(device, config, producer, metrics),
        _ => Err(AudioDeviceError::backend(format!(
            "Unsupported input sample format: {format}"
        ))),
    }
}

fn build_input_typed<T: RealtimeSample>(
    device: &Device,
    config: StreamConfig,
    mut producer: Producer<f32>,
    metrics: SharedAudioMetrics,
) -> Result<Stream, AudioDeviceError> {
    let channels = usize::from(config.channels);
    let error_metrics = metrics.clone();
    let mut pipeline = AudioPipeline::new(PassthroughProcessor);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let started = Instant::now();
                let mut overflowed = false;
                let mut peak = 0.0_f32;
                let mut frames = 0_u64;
                for frame in data.chunks_exact(channels) {
                    let mono =
                        frame.iter().map(|sample| sample.to_f32()).sum::<f32>() / channels as f32;
                    let processed = pipeline.process(mono);
                    peak = peak.max(processed.abs());
                    overflowed |= producer.push(processed).is_err();
                    frames += 1;
                }
                if overflowed {
                    metrics.record_overflow();
                }
                metrics.set_input_level(peak);
                metrics.set_ring_depth(producer.buffer().capacity() - producer.slots());
                metrics.record_processing(
                    frames,
                    started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                );
            },
            move |_| error_metrics.record_stream_error(),
            None,
        )
        .map_err(audio_error)
}

fn build_output(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    consumer: Consumer<f32>,
    metrics: SharedAudioMetrics,
) -> Result<Stream, AudioDeviceError> {
    match format {
        SampleFormat::F32 => build_output_typed::<f32>(device, config, consumer, metrics),
        SampleFormat::I16 => build_output_typed::<i16>(device, config, consumer, metrics),
        SampleFormat::U16 => build_output_typed::<u16>(device, config, consumer, metrics),
        _ => Err(AudioDeviceError::backend(format!(
            "Unsupported output sample format: {format}"
        ))),
    }
}

fn build_output_typed<T: RealtimeSample>(
    device: &Device,
    config: StreamConfig,
    mut consumer: Consumer<f32>,
    metrics: SharedAudioMetrics,
) -> Result<Stream, AudioDeviceError> {
    let channels = usize::from(config.channels);
    let error_metrics = metrics.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut underrun = false;
                let mut peak = 0.0_f32;
                for frame in data.chunks_exact_mut(channels) {
                    let sample = match consumer.pop() {
                        Ok(sample) => sample,
                        Err(_) => {
                            underrun = true;
                            0.0
                        }
                    };
                    peak = peak.max(sample.abs());
                    frame.fill(T::from_f32(sample));
                }
                if underrun {
                    metrics.record_underrun();
                }
                metrics.set_output_level(peak);
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
