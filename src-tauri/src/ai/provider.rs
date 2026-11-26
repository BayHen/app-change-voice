//! Worker-side conversion contract. Implementations never own microphone/output devices.
use super::{
    config::{AI_SAMPLE_RATE, EngineConfig, UPSTREAM_COMMIT},
    models::AIModelMetadata,
    process::ManagedProcess,
    runtime::VerifiedRuntime,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

pub struct ConvertedChunk {
    pub samples: Vec<i16>,
    pub inference_ms: f64,
    pub warming_up: bool,
}

/// Native providers can implement this boundary without changing CPAL or routing.
/// Classic mode continues through the existing Rust DSP pipeline, outside this worker.
pub trait VoiceConversionBackend {
    fn process_id(&self) -> Option<u32> {
        None
    }
    fn convert(&mut self, input: &[i16]) -> Result<ConvertedChunk, String>;
    fn health(&mut self) -> Result<(), String>;
    fn exited(&self) -> bool;
}

pub struct WOkadaBackend {
    process: ManagedProcess,
    frames: usize,
}
impl WOkadaBackend {
    pub fn load(
        runtime: &VerifiedRuntime,
        model: &Path,
        metadata: &AIModelMetadata,
        config: &EngineConfig,
        log: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<(Self, Value), String> {
        let mut process = ManagedProcess::spawn(runtime, log, cancel)?;
        let hello = process.call(json!({"op":"hello"}), Duration::from_secs(90))?;
        if hello["upstreamCommit"] != UPSTREAM_COMMIT
            || hello["sampleRate"] != AI_SAMPLE_RATE
            || hello["channels"] != 1
            || hello["pcmFormat"] != "s16le"
        {
            return Err("AI runtime negotiation is incompatible".into());
        }
        let loading = Instant::now();
        let mut loaded = process.call(
            json!({"op":"load", "modelPath":model, "sha256":metadata.sha256,
            "consentConfirmed":metadata.consent_confirmed, "config":config}),
            Duration::from_secs(120),
        )?;
        loaded["initializationMs"] = loaded["loadingMs"].clone();
        loaded["loadingMs"] = serde_json::json!(loading.elapsed().as_secs_f64() * 1000.0);
        if !matches!(loaded["provider"].as_str(), Some("cpu" | "cuda")) {
            return Err("AI runtime reported an unsupported loaded provider".into());
        }
        loaded["hardware"] = hello;
        Ok((
            Self {
                process,
                frames: config.chunk_frames,
            },
            loaded,
        ))
    }
}
impl VoiceConversionBackend for WOkadaBackend {
    fn process_id(&self) -> Option<u32> {
        self.process.process_id()
    }
    fn convert(&mut self, input: &[i16]) -> Result<ConvertedChunk, String> {
        if input.len() != self.frames {
            return Err("AI input chunk length mismatch".into());
        }
        let value = self.process.call(
            json!({"op":"infer", "samples":input}),
            Duration::from_secs(5),
        )?;
        decode_chunk(value, self.frames)
    }
    fn health(&mut self) -> Result<(), String> {
        let response = self
            .process
            .call(json!({"op":"health"}), Duration::from_secs(2))?;
        if response["ready"] != true {
            return Err("AI runtime lost the active model".into());
        }
        Ok(())
    }
    fn exited(&self) -> bool {
        self.process.exited()
    }
}
fn decode_chunk(value: Value, frames: usize) -> Result<ConvertedChunk, String> {
    let samples: Vec<i16> = serde_json::from_value(value["samples"].clone())
        .map_err(|_| "Invalid PCM in AI response")?;
    if samples.len() != frames {
        return Err("AI PCM frame count mismatch".into());
    }
    let inference_ms = value["inferenceMs"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("Invalid AI timing")?;
    let warming_up = value["warmingUp"]
        .as_bool()
        .ok_or("Missing AI warmup state")?;
    Ok(ConvertedChunk {
        samples,
        inference_ms,
        warming_up,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_provider_audio_and_timing() {
        for value in [
            json!({"samples":[32768],"inferenceMs":1,"warmingUp":false}),
            json!({"samples":[],"inferenceMs":1,"warmingUp":false}),
            json!({"samples":[0],"inferenceMs":-1,"warmingUp":false}),
            json!({"samples":[0],"inferenceMs":1}),
        ] {
            assert!(decode_chunk(value, 1).is_err());
        }
        let chunk = decode_chunk(
            json!({"samples":[-32768,32767],"inferenceMs":2.5,"warmingUp":false}),
            2,
        )
        .unwrap();
        assert_eq!(chunk.samples, [-32768, 32767]);
        assert!(!chunk.warming_up);
    }
}
