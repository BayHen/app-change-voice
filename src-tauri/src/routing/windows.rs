use super::{SharedRoutingMetrics, VirtualDeviceManager, VirtualOutputSink, errors::RoutingError};
use crate::{audio::noise::SharedNoiseSettings, effects::limiter::OutputLimiter};
use cpal::{
    BufferSize, Device, DeviceId, SampleFormat, Stream, StreamConfig, SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::Consumer;
use std::{
    str::FromStr,
    sync::{Arc, atomic::Ordering},
};

pub struct WindowsVirtualDeviceManager;
impl VirtualDeviceManager for WindowsVirtualDeviceManager {
    fn is_likely_virtual_output(name: &str) -> bool {
        super::likely_virtual_device(name)
    }
}

pub(crate) struct WindowsVirtualOutputSink {
    stream: Stream,
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) shared: Arc<SharedRoutingMetrics>,
}

impl WindowsVirtualOutputSink {
    pub(crate) fn build(
        device_id: &str,
        source_rate: u32,
        consumer: Consumer<f32>,
        shared: Arc<SharedRoutingMetrics>,
        noise_controls: SharedNoiseSettings,
    ) -> Result<Self, RoutingError> {
        let host = cpal::default_host();
        let parsed = DeviceId::from_str(device_id)
            .map_err(|e| RoutingError::new("invalidDevice", e.to_string()))?;
        let device = host.device_by_id(&parsed).ok_or_else(|| {
            RoutingError::new(
                "deviceNotFound",
                "Virtual output device is no longer available",
            )
        })?;
        let name = device
            .description()
            .map(|value| value.name().to_owned())
            .unwrap_or_else(|_| "Unknown virtual output".into());
        let supported = device
            .supported_output_configs()
            .map_err(route_error)?
            .max_by_key(|range| {
                let supports_source = range.min_sample_rate() <= source_rate
                    && source_rate <= range.max_sample_rate();
                (
                    supports_source,
                    format_rank(range.sample_format()),
                    range.max_sample_rate(),
                )
            })
            .ok_or_else(|| {
                RoutingError::new(
                    "unsupportedFormat",
                    "Virtual output has no supported stream format",
                )
            })?;
        let rate = if supported.min_sample_rate() <= source_rate
            && source_rate <= supported.max_sample_rate()
        {
            source_rate
        } else {
            source_rate.clamp(supported.min_sample_rate(), supported.max_sample_rate())
        };
        let format = supported.sample_format();
        let fixed = supported.with_sample_rate(rate);
        let config = StreamConfig {
            channels: fixed.channels(),
            sample_rate: rate,
            buffer_size: match fixed.buffer_size() {
                SupportedBufferSize::Range { min, max } => {
                    BufferSize::Fixed(256_u32.clamp(*min, *max))
                }
                SupportedBufferSize::Unknown => BufferSize::Default,
            },
        };
        let channels = config.channels;
        let stream = build_stream(
            &device,
            config,
            format,
            source_rate,
            consumer,
            shared.clone(),
            noise_controls,
        )?;
        Ok(Self {
            stream,
            device_id: device_id.into(),
            device_name: name,
            sample_rate: rate,
            channels,
            shared,
        })
    }
}

impl VirtualOutputSink for WindowsVirtualOutputSink {
    fn play(&self) -> Result<(), RoutingError> {
        self.stream.play().map_err(route_error)
    }
    fn pause(&self) -> Result<(), RoutingError> {
        self.stream.pause().map_err(route_error)
    }
}

trait RouteSample: cpal::SizedSample + Copy + Send + 'static {
    fn from_f32(value: f32) -> Self;
}
impl RouteSample for f32 {
    fn from_f32(value: f32) -> Self {
        value
    }
}
impl RouteSample for i16 {
    fn from_f32(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
    }
}
impl RouteSample for u16 {
    fn from_f32(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f32).round() as u16
    }
}

fn build_stream(
    device: &Device,
    config: StreamConfig,
    format: SampleFormat,
    source_rate: u32,
    consumer: Consumer<f32>,
    shared: Arc<SharedRoutingMetrics>,
    noise_controls: SharedNoiseSettings,
) -> Result<Stream, RoutingError> {
    match format {
        SampleFormat::F32 => build_typed::<f32>(
            device,
            config,
            source_rate,
            consumer,
            shared,
            noise_controls,
        ),
        SampleFormat::I16 => build_typed::<i16>(
            device,
            config,
            source_rate,
            consumer,
            shared,
            noise_controls,
        ),
        SampleFormat::U16 => build_typed::<u16>(
            device,
            config,
            source_rate,
            consumer,
            shared,
            noise_controls,
        ),
        _ => Err(RoutingError::new(
            "unsupportedFormat",
            format!("Unsupported virtual output format: {format}"),
        )),
    }
}

fn build_typed<T: RouteSample>(
    device: &Device,
    config: StreamConfig,
    source_rate: u32,
    mut consumer: Consumer<f32>,
    shared: Arc<SharedRoutingMetrics>,
    noise_controls: SharedNoiseSettings,
) -> Result<Stream, RoutingError> {
    let channels = usize::from(config.channels);
    let output_rate = config.sample_rate;
    let errors = shared.clone();
    let mut phase = output_rate;
    let mut held = 0.0_f32;
    let mut limiter = OutputLimiter::new(output_rate);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut underrun = false;
                let mut peak = 0.0_f32;
                for frame in data.chunks_exact_mut(channels) {
                    phase = phase.saturating_add(source_rate);
                    while phase >= output_rate {
                        phase -= output_rate;
                        match consumer.pop() {
                            Ok(sample) => held = sample,
                            Err(_) => {
                                held = 0.0;
                                underrun = true;
                            }
                        }
                    }
                    let sample = if held.is_finite() {
                        held.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    };
                    let noise = noise_controls.load();
                    let (sample, _) =
                        limiter.process(sample, noise.limiter_enabled, noise.limiter_ceiling_dbfs);
                    peak = peak.max(sample.abs());
                    frame.fill(T::from_f32(sample));
                }
                if underrun {
                    shared.underruns.fetch_add(1, Ordering::Relaxed);
                }
                shared.set_peak(peak);
            },
            move |_| {
                errors.faulted.store(true, Ordering::Release);
                errors.active.store(false, Ordering::Release);
            },
            None,
        )
        .map_err(route_error)
}

fn format_rank(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 3,
        SampleFormat::I16 => 2,
        SampleFormat::U16 => 1,
        _ => 0,
    }
}
fn route_error(error: cpal::Error) -> RoutingError {
    RoutingError::new("backend", error.to_string())
}
