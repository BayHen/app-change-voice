use super::meter::{MeterReading, amplitude_to_dbfs};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Default)]
pub struct SharedAudioMetrics(Arc<AudioMetricAtoms>);

#[derive(Debug, Default)]
struct AudioMetricAtoms {
    underruns: AtomicU64,
    overflows: AtomicU64,
    stream_errors: AtomicU64,
    processing_nanos: AtomicU64,
    processed_frames: AtomicU64,
    ring_depth: AtomicUsize,
    input_level_bits: AtomicU64,
    output_level_bits: AtomicU64,
    input_rms_bits: AtomicU64,
    output_rms_bits: AtomicU64,
    input_clipping: AtomicBool,
    output_clipping: AtomicBool,
    stream_faulted: AtomicBool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetrics {
    pub sample_rate: u32,
    pub input_buffer_frames: u32,
    pub output_buffer_frames: u32,
    pub underrun_count: u64,
    pub overflow_count: u64,
    pub stream_error_count: u64,
    pub average_processing_micros: f64,
    pub estimated_latency_ms: f64,
    pub ring_depth_frames: usize,
    pub input_level: f32,
    pub output_level: f32,
    pub input_peak_dbfs: f32,
    pub input_rms_dbfs: f32,
    pub output_peak_dbfs: f32,
    pub output_rms_dbfs: f32,
    pub input_clipping: bool,
    pub output_clipping: bool,
    pub stream_faulted: bool,
}

impl SharedAudioMetrics {
    #[inline]
    pub fn record_overflow(&self) {
        self.0.overflows.fetch_add(1, Ordering::Relaxed);
    }
    #[inline]
    pub fn record_underrun(&self) {
        self.0.underruns.fetch_add(1, Ordering::Relaxed);
    }
    #[inline]
    pub fn record_stream_error(&self) {
        self.0.stream_errors.fetch_add(1, Ordering::Relaxed);
        self.0.stream_faulted.store(true, Ordering::Release);
    }
    #[inline]
    pub fn record_processing(&self, frames: u64, nanos: u64) {
        self.0.processed_frames.fetch_add(frames, Ordering::Relaxed);
        self.0.processing_nanos.fetch_add(nanos, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_ring_depth(&self, frames: usize) {
        self.0.ring_depth.store(frames, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_input_meter(&self, reading: MeterReading) {
        self.0
            .input_level_bits
            .store(reading.peak.to_bits() as u64, Ordering::Relaxed);
        self.0
            .input_rms_bits
            .store(reading.rms.to_bits() as u64, Ordering::Relaxed);
        self.0
            .input_clipping
            .store(reading.clipping, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_output_meter(&self, reading: MeterReading) {
        self.0
            .output_level_bits
            .store(reading.peak.to_bits() as u64, Ordering::Relaxed);
        self.0
            .output_rms_bits
            .store(reading.rms.to_bits() as u64, Ordering::Relaxed);
        self.0
            .output_clipping
            .store(reading.clipping, Ordering::Relaxed);
    }

    pub fn snapshot(
        &self,
        sample_rate: u32,
        input_buffer_frames: u32,
        output_buffer_frames: u32,
    ) -> AudioMetrics {
        let frames = self.0.processed_frames.load(Ordering::Relaxed);
        let nanos = self.0.processing_nanos.load(Ordering::Relaxed);
        let ring_depth_frames = self.0.ring_depth.load(Ordering::Relaxed);
        let input_level = f32::from_bits(self.0.input_level_bits.load(Ordering::Relaxed) as u32);
        let output_level = f32::from_bits(self.0.output_level_bits.load(Ordering::Relaxed) as u32);
        let input_rms = f32::from_bits(self.0.input_rms_bits.load(Ordering::Relaxed) as u32);
        let output_rms = f32::from_bits(self.0.output_rms_bits.load(Ordering::Relaxed) as u32);
        AudioMetrics {
            sample_rate,
            input_buffer_frames,
            output_buffer_frames,
            underrun_count: self.0.underruns.load(Ordering::Relaxed),
            overflow_count: self.0.overflows.load(Ordering::Relaxed),
            stream_error_count: self.0.stream_errors.load(Ordering::Relaxed),
            average_processing_micros: if frames == 0 {
                0.0
            } else {
                nanos as f64 / frames as f64 / 1_000.0
            },
            estimated_latency_ms: if sample_rate == 0 {
                0.0
            } else {
                (input_buffer_frames as usize + output_buffer_frames as usize + ring_depth_frames)
                    as f64
                    * 1_000.0
                    / sample_rate as f64
            },
            ring_depth_frames,
            input_level,
            output_level,
            input_peak_dbfs: amplitude_to_dbfs(input_level),
            input_rms_dbfs: amplitude_to_dbfs(input_rms),
            output_peak_dbfs: amplitude_to_dbfs(output_level),
            output_rms_dbfs: amplitude_to_dbfs(output_rms),
            input_clipping: self.0.input_clipping.load(Ordering::Relaxed),
            output_clipping: self.0.output_clipping.load(Ordering::Relaxed),
            stream_faulted: self.0.stream_faulted.load(Ordering::Acquire),
        }
    }
}
