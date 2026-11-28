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
    #[cfg(test)]
    pub(crate) fn build(
        device_id: &str,
        source_rate: u32,
        consumer: Consumer<f32>,
        shared: Arc<SharedRoutingMetrics>,
        noise_controls: SharedNoiseSettings,
    ) -> Result<Self, RoutingError> {
        Self::build_with_quality(
            device_id,
            source_rate,
            consumer,
            shared,
            noise_controls,
            crate::audio::quality::QualityProfile::Balanced,
            None,
        )
    }
    pub(crate) fn build_with_quality(
        device_id: &str,
        source_rate: u32,
        consumer: Consumer<f32>,
        shared: Arc<SharedRoutingMetrics>,
        noise_controls: SharedNoiseSettings,
        quality: crate::audio::quality::QualityProfile,
        tap: Option<crate::audio::capture::Tap>,
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
                    BufferSize::Fixed(quality.requested_frames(rate).clamp(*min, *max))
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
            quality,
            tap,
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
impl RouteSample for i32 {
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
impl RouteSample for i16 {
    fn from_f32(value: f32) -> Self {
        crate::ai::transport::to_pcm16(value)
    }
}
impl RouteSample for u16 {
    fn from_f32(value: f32) -> Self {
        (i32::from(crate::ai::transport::to_pcm16(value)) + 32768) as u16
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
    quality: crate::audio::quality::QualityProfile,
    tap: Option<crate::audio::capture::Tap>,
) -> Result<Stream, RoutingError> {
    macro_rules! build {
        ($ty:ty) => {
            build_typed::<$ty>(
                device,
                config,
                source_rate,
                consumer,
                shared,
                noise_controls,
                quality,
                tap,
            )
        };
    }
    match format {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::U16 => build!(u16),
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
    consumer: Consumer<f32>,
    shared: Arc<SharedRoutingMetrics>,
    noise_controls: SharedNoiseSettings,
    quality: crate::audio::quality::QualityProfile,
    mut tap: Option<crate::audio::capture::Tap>,
) -> Result<Stream, RoutingError> {
    let channels = usize::from(config.channels);
    let output_rate = config.sample_rate;
    let errors = shared.clone();
    let mut reader = crate::audio::resample::ClockedReader::new(
        consumer,
        source_rate,
        output_rate,
        quality.negotiated_startup_ms(config.sample_rate, config.buffer_size),
        quality.taps(),
    )
    .map_err(|e| RoutingError::new("resampler", e))?;
    let mut route_epoch = shared.route_generation.load(Ordering::Acquire);
    let mut limiter = OutputLimiter::new(output_rate);
    let mut privacy = crate::ai::transport::OutputPrivacyGate::new(shared.voice_guard.clone());
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let underruns = reader.underruns;
                let mut peak = 0.0_f32;
                let noise = noise_controls.load();
                let generation = shared.route_generation.load(Ordering::Acquire);
                if generation != route_epoch {
                    route_epoch = generation;
                    let queued = reader.consumer_mut().slots();
                    for _ in 0..queued {
                        let _ = reader.consumer_mut().pop();
                    }
                    reader.reset();
                }
                if !shared.active.load(Ordering::Acquire) {
                    data.fill(T::from_f32(0.0));
                    shared.set_peak(0.0);
                    return;
                }
                for frame in data.chunks_exact_mut(channels) {
                    if privacy.discard_if_changed(reader.consumer_mut()) {
                        reader.reset();
                    }
                    let held = privacy.filter(reader.next_sample());
                    let sample = if held.is_finite() { held } else { 0.0 };
                    let (sample, _) =
                        limiter.process(sample, noise.limiter_enabled, noise.limiter_ceiling_dbfs);
                    peak = peak.max(sample.abs());
                    if let Some(tap) = tap.as_mut() {
                        tap.push(
                            crate::audio::capture::Stage::VirtualOutput,
                            output_rate,
                            sample,
                        );
                    }
                    frame.fill(T::from_f32(sample));
                }
                if reader.underruns > underruns {
                    shared.underruns.fetch_add(1, Ordering::Relaxed);
                }
                shared.set_peak(peak);
            },
            move |error| match error.kind() {
                cpal::ErrorKind::Xrun => {
                    errors.underruns.fetch_add(1, Ordering::Relaxed);
                }
                cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied => {}
                _ => {
                    errors.faulted.store(true, Ordering::Release);
                    errors.active.store(false, Ordering::Release);
                }
            },
            None,
        )
        .map_err(route_error)
}

fn format_rank(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 4,
        SampleFormat::I32 => 3,
        SampleFormat::I16 => 2,
        SampleFormat::U16 => 1,
        _ => 0,
    }
}
fn route_error(error: cpal::Error) -> RoutingError {
    RoutingError::new("backend", error.to_string())
}

#[cfg(test)]
mod manual_hardware_tests {
    use super::*;
    use crate::{audio::noise::NoiseSettings, routing::routing_queue};
    use std::{thread, time::Duration};

    fn virtual_output() -> (String, String) {
        let host = cpal::default_host();
        host.output_devices()
            .expect("WASAPI output enumeration failed")
            .filter_map(|device| {
                let id = device.id().ok()?.to_string();
                let name = device.description().ok()?.name().to_owned();
                WindowsVirtualDeviceManager::is_likely_virtual_output(&name).then_some((id, name))
            })
            .next()
            .expect("No virtual cable playback endpoint was found")
    }

    #[test]
    #[ignore = "manual hardware test: requires an installed virtual audio cable"]
    fn opens_and_stops_virtual_cable_output() {
        let (id, name) = virtual_output();
        let (mut producer, consumer, shared) = routing_queue(48_000);
        let sink = WindowsVirtualOutputSink::build(
            &id,
            48_000,
            consumer,
            shared.clone(),
            SharedNoiseSettings::new(NoiseSettings::default()),
        )
        .expect("Could not build virtual output stream");
        sink.play().expect("Could not start virtual output stream");
        shared.active.store(true, Ordering::Release);
        for frame in 0..4_800 {
            producer.push((frame as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.05);
        }
        thread::sleep(Duration::from_millis(150));
        sink.pause().expect("Could not stop virtual output stream");
        assert!(
            name.to_ascii_lowercase().contains("cable")
                || name.to_ascii_lowercase().contains("virtual")
        );
        assert!(
            !shared.faulted.load(Ordering::Acquire),
            "Virtual output stream faulted"
        );
    }

    #[test]
    #[ignore = "manual hardware test: requires an installed virtual audio cable"]
    fn virtual_cable_accepts_sample_rate_mismatch() {
        let (id, _) = virtual_output();
        let (_, consumer, shared) = routing_queue(44_100);
        let sink = WindowsVirtualOutputSink::build(
            &id,
            44_100,
            consumer,
            shared,
            SharedNoiseSettings::new(NoiseSettings::default()),
        )
        .expect("Virtual output did not accept/adapt a 44.1 kHz source");
        assert!(sink.sample_rate > 0);
    }

    #[test]
    #[ignore = "Manual VCTK WAV loopback: requires an idle VB-Cable capture endpoint"]
    fn vctk_wavs_reach_the_virtual_microphone() {
        use cpal::{FromSample, SizedSample};
        use std::{path::Path, sync::Mutex, time::Instant};
        assert_eq!(
            std::env::var("VOICEMORPH_TEST_IDLE_VB_CABLE").as_deref(),
            Ok("1"),
            "Check that VB-Audio capture is idle before enabling this test"
        );
        fn capture<T: SizedSample + Send + Copy>(
            device: &Device,
            config: &StreamConfig,
            values: Arc<Mutex<Vec<f32>>>,
        ) -> Stream
        where
            f32: FromSample<T>,
        {
            let channels = usize::from(config.channels);
            device
                .build_input_stream(
                    *config,
                    move |data: &[T], _| {
                        values.lock().unwrap().extend(
                            data.chunks_exact(channels)
                                .map(|frame| frame[0].to_sample::<f32>()),
                        );
                    },
                    |error| eprintln!("VB-Cable capture: {error}"),
                    None,
                )
                .expect("Could not open CABLE Output capture stream")
        }
        let host = cpal::default_host();
        let render =
            host.output_devices()
                .unwrap()
                .find(|device| {
                    device.description().ok().is_some_and(|d| {
                        d.name().starts_with("CABLE Input (VB-Audio Virtual Cable)")
                    })
                })
                .expect("Exact CABLE Input playback endpoint missing");
        let device = host
            .input_devices()
            .unwrap()
            .find(|device| {
                device.description().ok().is_some_and(|d| {
                    d.name()
                        .starts_with("CABLE Output (VB-Audio Virtual Cable)")
                })
            })
            .expect("Exact CABLE Output capture endpoint missing");
        let supported = device.default_input_config().unwrap();
        assert_eq!(supported.sample_rate(), 48_000);
        let config: StreamConfig = supported.clone().into();
        let folder =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.ai-runtime-build/phase6-vctk-test");
        for name in ["wind_assassin", "jade_guardian"] {
            let waveform: Vec<_> =
                hound::WavReader::open(folder.join(format!("{name}_output.wav")))
                    .unwrap()
                    .samples::<i16>()
                    .skip(48_000)
                    .take(3 * 48_000)
                    .map(|v| (f32::from(v.unwrap()) / 32768.0).clamp(-0.10, 0.10))
                    .collect();
            assert_eq!(waveform.len(), 3 * 48_000);
            let values = Arc::new(Mutex::new(Vec::with_capacity(5 * 48_000)));
            let recording = match supported.sample_format() {
                SampleFormat::F32 => capture::<f32>(&device, &config, values.clone()),
                SampleFormat::I16 => capture::<i16>(&device, &config, values.clone()),
                SampleFormat::I32 => capture::<i32>(&device, &config, values.clone()),
                format => panic!("Unsupported VB-Cable capture format: {format:?}"),
            };
            let (mut producer, consumer, shared) = routing_queue(48_000);
            let sink = WindowsVirtualOutputSink::build(
                &render.id().unwrap().to_string(),
                48_000,
                consumer,
                shared.clone(),
                SharedNoiseSettings::new(NoiseSettings::default()),
            )
            .unwrap();
            recording
                .play()
                .expect("Could not start CABLE Output capture");
            sink.play().expect("Could not start CABLE Input playback");
            shared.active.store(true, Ordering::Release);
            thread::sleep(Duration::from_millis(250));
            let started = Instant::now();
            for (index, frame) in waveform.chunks(480).enumerate() {
                for value in frame {
                    producer.push(*value);
                }
                if let Some(delay) = (started + Duration::from_millis((index + 1) as u64 * 10))
                    .checked_duration_since(Instant::now())
                {
                    thread::sleep(delay);
                }
            }
            thread::sleep(Duration::from_millis(400));
            sink.pause().unwrap();
            recording.pause().unwrap();
            let audio = values.lock().unwrap();
            let pcm = hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer =
                hound::WavWriter::create(folder.join(format!("{name}_cable_capture.wav")), pcm)
                    .unwrap();
            let peak = audio.iter().map(|v| v.abs()).fold(0.0, f32::max);
            for value in audio.iter() {
                writer
                    .write_sample(crate::ai::transport::to_pcm16(*value))
                    .unwrap();
            }
            writer.finalize().unwrap();
            println!(
                "{name} captured {} samples, peak {peak:.5}, overruns {}",
                audio.len(),
                shared.overflows.load(Ordering::Relaxed)
            );
            assert!(
                peak > 0.001 && !shared.faulted.load(Ordering::Acquire),
                "No neural WAV signal reached CABLE Output"
            );
        }
    }
}
