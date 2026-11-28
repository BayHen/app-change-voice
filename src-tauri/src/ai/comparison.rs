//! File-only, consent-gated comparisons. No capture devices or network operations.
use crate::{
    ai::{
        config::EngineConfig,
        provider::{VoiceConversionBackend, WOkadaBackend},
    },
    effects::parameters::{EffectParameters, SharedEffectParameters},
    presets::preview,
};
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub duration_seconds: f64,
    pub rms_dbfs: f64,
    pub peak_dbfs: f64,
    pub clipping_rate: f64,
    pub estimated_f0_hz: Option<f64>,
    pub f0_spread_hz: Option<f64>,
    pub voiced_frames: usize,
    pub quiet_frame_dbfs: f64,
    pub recommended_gain_db: f64,
}
pub fn analyze(rate: u32, input: &[f32]) -> Analysis {
    let power =
        input.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / input.len().max(1) as f64;
    let peak = input.iter().map(|s| s.abs()).fold(0f32, f32::max);
    let db = |v: f64| 20.0 * v.max(0.00000001).log10();
    let stride = (rate / 8000).max(1) as usize;
    let reduced: Vec<f64> = input
        .chunks(stride)
        .map(|c| c.iter().map(|v| f64::from(*v)).sum::<f64>() / c.len() as f64)
        .collect();
    let reduced_rate = rate as f64 / stride as f64;
    let window = (reduced_rate * 0.04) as usize;
    let mut pitches = Vec::new();
    let mut levels = Vec::new();
    for frame in reduced.chunks_exact(window) {
        let mean = frame.iter().sum::<f64>() / window as f64;
        let centered: Vec<_> = frame.iter().map(|v| v - mean).collect();
        let p = centered.iter().map(|v| v * v).sum::<f64>() / window as f64;
        levels.push(db(p.sqrt()));
        if p < 0.000032 {
            continue;
        }
        let min = (reduced_rate / 450.0) as usize;
        let max = (reduced_rate / 65.0) as usize;
        let mut candidates = Vec::new();
        for lag in min..max.min(window / 2) {
            let (a, b) = (&centered[..window - lag], &centered[lag..]);
            let dot = a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
            let norm = (a.iter().map(|v| v * v).sum::<f64>()
                * b.iter().map(|v| v * v).sum::<f64>())
            .sqrt();
            candidates.push((lag, dot / norm.max(1e-15)));
        }
        let best = candidates.iter().map(|v| v.1).fold(0.0, f64::max);
        if best < 0.80 {
            continue;
        }
        if let Some(c) = candidates
            .windows(3)
            .find(|w| w[1].1 >= best * 0.96 && w[1].1 > w[0].1 && w[1].1 >= w[2].1)
        {
            pitches.push(reduced_rate / c[1].0 as f64);
        }
    }
    pitches.sort_by(f64::total_cmp);
    levels.sort_by(f64::total_cmp);
    let f0 = (!pitches.is_empty()).then(|| pitches[pitches.len() / 2]);
    let spread =
        (pitches.len() > 3).then(|| pitches[pitches.len() * 9 / 10] - pitches[pitches.len() / 10]);
    let rms = db(power.sqrt());
    Analysis {
        duration_seconds: input.len() as f64 / rate as f64,
        rms_dbfs: rms,
        peak_dbfs: db(f64::from(peak)),
        clipping_rate: input.iter().filter(|s| s.abs() >= 0.999).count() as f64
            / input.len().max(1) as f64,
        estimated_f0_hz: f0,
        f0_spread_hz: spread,
        voiced_frames: pitches.len(),
        quiet_frame_dbfs: levels.get(levels.len() / 10).copied().unwrap_or(-160.0),
        recommended_gain_db: if power < 0.000001 {
            0.0
        } else {
            (-20.0 - rms).clamp(-12.0, 6.0)
        },
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub original: preview::Preview,
    pub dsp: preview::Preview,
    pub neural: preview::Preview,
    pub hybrid: preview::Preview,
    pub input_analysis: Analysis,
    pub neural_analysis: Analysis,
    pub resources: Option<serde_json::Value>,
    pub initialization_ms: f64,
    pub warmup_ms: f64,
    pub inference_ms: f64,
    pub real_time_factor: f64,
    pub worst_chunk_ms: f64,
    pub deadline_ms: f64,
    pub missed_deadlines: usize,
    pub provider: String,
    pub input_sha256: String,
    pub model_sha256: String,
    pub source_frames: usize,
}
pub fn compare(
    root: &Path,
    model: &super::models::AIModelMetadata,
    model_path: &Path,
    source: &Path,
    consent: bool,
    config: &EngineConfig,
    effects: EffectParameters,
) -> Result<Comparison, String> {
    if !consent {
        return Err("Confirm permission to process this recording locally".into());
    }
    let (rate, input) = preview::read_source(source, true)?;
    if rate != 48000 {
        return Err(
            "Neural comparison requires a 48 kHz WAV. Export the original recording at 48 kHz."
                .into(),
        );
    }
    let input_sha256 = super::validator::sha256_file(source)?;
    let runtime = super::runtime::discover(root)?;
    let staged = preview::PreviewModel::new(root, model_path, model)?;
    let init = Instant::now();
    let (mut backend, loaded) = WOkadaBackend::load(
        &runtime,
        &staged.path,
        model,
        config,
        &root.join(format!("comparison-{}.log", super::process::now_ms())),
        Arc::new(AtomicBool::new(false)),
    )?;
    let initialization_ms = init.elapsed().as_secs_f64() * 1000.0;
    let silence = vec![0i16; config.chunk_frames];
    let warmup_ms = super::warmup::warm_up(&mut backend, config, &AtomicBool::new(false))?;
    let pcm: Vec<_> = input
        .iter()
        .map(|v| super::transport::to_pcm16(*v))
        .collect();
    let mut output = Vec::new();
    let (mut inference_ms, mut worst_chunk_ms, mut missed_deadlines) = (0.0, 0f64, 0);
    let deadline_ms = config.chunk_frames as f64 / 48.0;
    let _ = backend.resource_metrics();
    let timer = Instant::now();
    for part in pcm.chunks(config.chunk_frames).chain([&[][..], &[][..]]) {
        let mut block = silence.clone();
        block[..part.len()].copy_from_slice(part);
        let result = backend.convert(&block)?;
        if result.warming_up {
            return Err("Unexpected neural warm-up during A/B comparison".into());
        }
        inference_ms += result.inference_ms;
        worst_chunk_ms = worst_chunk_ms.max(result.inference_ms);
        if result.inference_ms > deadline_ms {
            missed_deadlines += 1;
        }
        output.extend(result.samples.into_iter().map(|s| f32::from(s) / 32768.0));
    }
    let elapsed = timer.elapsed().as_secs_f64() * 1000.0;
    let resources = backend.resource_metrics().ok().flatten();
    let mut post =
        super::hybrid::NeuralPostProcessor::new(48000, SharedEffectParameters::new(effects));
    let hybrid: Vec<_> = output.iter().map(|v| post.process(*v, 1)).collect();
    let dsp_preset = crate::presets::registry::PresetRegistry::built_in()?
        .get("natural")?
        .clone();
    let input_analysis = analyze(rate, &input);
    let neural_analysis = analyze(rate, &output);
    let duration = input_analysis.duration_seconds;
    Ok(Comparison {
        original: preview::encode(rate, input.clone(), 0.0)?,
        dsp: preview::render_with_parameters(&dsp_preset, source, true, Some(effects))?,
        neural: preview::encode(rate, output, elapsed)?,
        hybrid: preview::encode(rate, hybrid, elapsed)?,
        input_analysis,
        neural_analysis,
        resources,
        initialization_ms,
        warmup_ms,
        inference_ms,
        real_time_factor: elapsed / (duration * 1000.0),
        worst_chunk_ms,
        deadline_ms,
        missed_deadlines,
        provider: loaded["provider"].as_str().unwrap_or("unknown").into(),
        input_sha256,
        model_sha256: model.sha256.clone(),
        source_frames: input.len(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_reports_signal_and_clipping_without_guessing_identity() {
        let silence = analyze(48000, &vec![0.0; 48000]);
        assert!(silence.estimated_f0_hz.is_none());
        assert_eq!(silence.recommended_gain_db, 0.0);
        let tone: Vec<_> = (0..48000)
            .map(|n| 0.2 * (std::f32::consts::TAU * 200.0 * n as f32 / 48000.0).sin())
            .collect();
        let a = analyze(48000, &tone);
        assert!((a.estimated_f0_hz.unwrap() - 200.0).abs() < 5.0);
        assert_eq!(a.clipping_rate, 0.0);
        let clipped = analyze(48000, &vec![1.0; 480]);
        assert_eq!(clipped.clipping_rate, 1.0);
    }
}

#[cfg(test)]
mod local_neural_tests {
    use super::*;
    #[test]
    #[ignore = "Actual licensed owner-model neural inference and four-way file comparison; no audio devices"]
    fn compare_authorized_owner_in_three_modes() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let runtime_root = project.join(".ai-runtime-build/bootstrap-installed");
        let source = project.join(".ai-runtime-build/assets/owner-sample.wav");
        let model = crate::ai::catalog::metadata();
        let path = project.join(".ai-runtime-build/converted-tsukuyomi/tsukuyomi-normal1.onnx");
        let report = crate::ai::onnx::inspect(&path).unwrap();
        assert_eq!(report.compatibility, "candidate", "{:?}", report.issues);
        let provider = std::env::var("VM_NEURAL_TEST_PROVIDER").unwrap_or_else(|_| "cuda".into());
        let destination = project.join(if provider == "cpu" {
            ".ai-runtime-build/phase5-neural-cpu"
        } else {
            ".ai-runtime-build/phase5-neural"
        });
        std::fs::create_dir_all(&destination).unwrap();
        let test_root = destination.join(format!("identity-test-{}", crate::ai::process::now_ms()));
        let mut models = crate::ai::manager::AIModelManager::new(test_root.join("models")).unwrap();
        let request = || crate::ai::import::ImportRequest {
            name: "Authorized Tsukuyomi Normal1".into(),
            source_url: model.source_url.clone(),
            license_url: model.license_url.clone().unwrap(),
            attribution: model.attribution.clone().unwrap(),
            rights_confirmed: true,
            expected_sha256: model.sha256.clone(),
        };
        let unicode_path =
            test_root.join("gi\u{1ecd}ng n\u{1eef} c\u{f3} kho\u{1ea3}ng tr\u{1eaf}ng.onnx");
        std::fs::copy(&path, &unicode_path).unwrap();
        let imported = crate::ai::import::register(&mut models, &unicode_path, request()).unwrap();
        let (registered_model, registered_path) = models.resolve(&imported.metadata.id).unwrap();
        assert!(
            crate::ai::import::register(&mut models, &path, request())
                .unwrap_err()
                .contains("already imported")
        );
        let mut library =
            crate::ai::agents::AgentLibrary::load(test_root.join("agents.json")).unwrap();
        let binding = crate::ai::agents::Binding {
            weights_sha256: None,
            model_id: imported.metadata.id,
            sha256: model.sha256.clone(),
            identity_name: "Tsukuyomi-chan official / CV. Rei Yumesaki".into(),
            rights_confirmed: true,
            quality_mode: "balanced".into(),
        };
        let mut denied = binding.clone();
        denied.rights_confirmed = false;
        assert!(library.assign("jett", denied, &models).is_err());
        library.assign("jett", binding.clone(), &models).unwrap();
        assert!(
            library
                .assign("sage", binding, &models)
                .unwrap_err()
                .contains("already assigned")
        );
        let altered_path = test_root.join("same-weights-different-metadata.onnx");
        std::fs::copy(&path, &altered_path).unwrap();
        let key = b"vm-test";
        let value = b"metadata-only export";
        let mut entry = vec![0x0a, key.len() as u8];
        entry.extend_from_slice(key);
        entry.extend_from_slice(&[0x12, value.len() as u8]);
        entry.extend_from_slice(value);
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&altered_path)
                .unwrap();
            file.write_all(&[0x72, entry.len() as u8]).unwrap();
            file.write_all(&entry).unwrap();
        }
        let altered = crate::ai::onnx::inspect(&altered_path).unwrap();
        assert_eq!(altered.weights_sha256, report.weights_sha256);
        assert_ne!(altered.sha256, report.sha256);
        let mut changed_request = request();
        changed_request.name = "Same voice different export".into();
        changed_request.expected_sha256 = altered.sha256;
        let second =
            crate::ai::import::register(&mut models, &altered_path, changed_request).unwrap();
        let mut same_weights = library.resolve("jett", &models).unwrap().2;
        same_weights.model_id = second.metadata.id;
        same_weights.sha256 = second.metadata.sha256;
        same_weights.identity_name = "Another label".into();
        assert!(
            library
                .assign("sage", same_weights, &models)
                .unwrap_err()
                .contains("already assigned")
        );
        let reloaded =
            crate::ai::agents::AgentLibrary::load(test_root.join("agents.json")).unwrap();
        assert_eq!(
            reloaded
                .list(&models)
                .iter()
                .filter(|v| v.state == "imported")
                .count(),
            1
        );
        let requested_mode = std::env::var("VM_NEURAL_TEST_MODE").ok();
        for mode in ["lowLatency", "balanced", "highQuality"] {
            if requested_mode
                .as_deref()
                .is_some_and(|requested| requested != mode)
            {
                continue;
            }
            if provider == "cpu" && mode != "balanced" {
                continue;
            }
            let config = crate::ai::agents::quality_config(
                &EngineConfig {
                    provider: provider.clone(),
                    ..Default::default()
                },
                mode,
            )
            .unwrap();
            let effects = EffectParameters {
                radio_mix: 0.2,
                de_ess_strength: 0.1,
                ..Default::default()
            };
            let report = compare(
                &runtime_root,
                &registered_model,
                &registered_path,
                &source,
                true,
                &config,
                effects,
            )
            .unwrap();
            assert!(report.neural.peak > 0.0001);
            assert_eq!(report.original.frames, report.source_frames);
            assert!(report.neural.full_scale_samples < report.neural.frames / 100);
            assert_ne!(report.neural.wav, report.dsp.wav);
            assert_ne!(report.neural.wav, report.hybrid.wav);
            for (kind, preview) in [
                ("original", &report.original),
                ("dsp", &report.dsp),
                ("neural", &report.neural),
                ("hybrid", &report.hybrid),
            ] {
                std::fs::write(destination.join(format!("{mode}-{kind}.wav")), &preview.wav)
                    .unwrap();
            }
            let mut json = serde_json::to_value(&report).unwrap();
            for key in ["original", "dsp", "neural", "hybrid"] {
                json[key].as_object_mut().unwrap().remove("wav");
            }
            std::fs::write(
                destination.join(format!("{mode}.json")),
                serde_json::to_vec_pretty(&json).unwrap(),
            )
            .unwrap();
            println!(
                "{mode}: provider={} rtf={:.3} inferenceMs={:.1} worstChunkMs={:.1} deadlineMisses={} peak={:.3}",
                report.provider,
                report.real_time_factor,
                report.inference_ms,
                report.worst_chunk_ms,
                report.missed_deadlines,
                report.neural.peak
            );
        }
    }
}
