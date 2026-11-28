//! Run non-silent kernels before exposing a neural model as ready. The generated
//! signal is discarded inside this worker and never sent to any audio device.
use super::{config::EngineConfig, provider::VoiceConversionBackend};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
pub fn warm_up(
    backend: &mut dyn VoiceConversionBackend,
    config: &EngineConfig,
    cancel: &AtomicBool,
) -> Result<f64, String> {
    let timer = Instant::now();
    let mut tone = vec![0i16; config.chunk_frames];
    let fixed_lookahead = backend.uses_fixed_lookahead();
    let voiced_chunks = if fixed_lookahead {
        // A single lookahead block and one voiced inference prime the fixed CUDA shape.
        2
    } else {
        (config.extra_context_frames / config.chunk_frames + 4).max(4)
    };
    for chunk in 0..voiced_chunks {
        if cancel.load(Ordering::Acquire) {
            return Err("Neural warm-up cancelled".into());
        }
        for (i, sample) in tone.iter_mut().enumerate() {
            let t = (chunk * config.chunk_frames + i) as f32 / 48000.0;
            *sample = (2200.0
                * ((std::f32::consts::TAU * 160.0 * t).sin()
                    + 0.25 * (std::f32::consts::TAU * 480.0 * t).sin()))
                as i16;
        }
        backend.convert(&tone)?;
    }
    let silence = vec![0; config.chunk_frames];
    let flush_chunks = if fixed_lookahead {
        // Drain the pending lookahead, then replace all voiced history with silence.
        config.extra_context_frames.max(14400).div_ceil(config.chunk_frames) + 1
    } else {
        (config.extra_context_frames + config.chunk_frames * 2).div_ceil(config.chunk_frames) + 4
    };
    let mut primed = false;
    for _ in 0..flush_chunks {
        if cancel.load(Ordering::Acquire) {
            return Err("Neural warm-up cancelled".into());
        }
        primed = !backend.convert(&silence)?.warming_up;
    }
    if !primed {
        return Err("Model did not finish neural warm-up. Increase chunk size or check model compatibility.".into());
    }
    Ok(timer.elapsed().as_secs_f64() * 1000.0)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::ConvertedChunk;
    struct Probe {
        voiced: usize,
        silent: usize,
    }
    impl VoiceConversionBackend for Probe {
        fn convert(&mut self, samples: &[i16]) -> Result<ConvertedChunk, String> {
            if samples.iter().any(|s| *s != 0) {
                self.voiced += 1;
            } else {
                self.silent += 1;
            }
            Ok(ConvertedChunk {
                samples: vec![0; samples.len()],
                inference_ms: 0.0,
                warming_up: false,
            })
        }
        fn health(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn exited(&self) -> bool {
            false
        }
    }
    #[test]
    fn warmup_exercises_voiced_path_then_flushes_without_routing() {
        let mut p = Probe {
            voiced: 0,
            silent: 0,
        };
        warm_up(&mut p, &EngineConfig::default(), &AtomicBool::new(false)).unwrap();
        assert!(p.voiced >= 4 && p.silent >= 4);
        assert!(warm_up(&mut p, &EngineConfig::default(), &AtomicBool::new(true)).is_err());
    }
}
