//! Explicit local WAV preview, outside device callbacks. Never captures microphone or changes live DSP.
use super::VoicePreset;
use crate::{
    audio::{
        noise::{NoiseSettings, SharedNoiseSettings},
        pipeline::AudioPipeline,
    },
    effects::{limiter::OutputLimiter, parameters::SharedEffectParameters},
};
use serde::Serialize;
use std::{io::Cursor, path::Path, time::Instant};
/// Upstream requires a private numeric slot. Never load directly from the user's model library.
pub struct PreviewModel {
    pub path: std::path::PathBuf,
    directory: std::path::PathBuf,
    parent: std::path::PathBuf,
}
impl PreviewModel {
    pub fn new(
        root: &Path,
        source: &Path,
        metadata: &crate::ai::models::AIModelMetadata,
    ) -> Result<Self, String> {
        crate::ai::validator::verify_model(metadata, source)?;
        let parent = root.canonicalize().map_err(|e| e.to_string())?;
        let directory = parent.join(format!(
            "preview-{}-{}",
            std::process::id(),
            crate::ai::process::now_ms()
        ));
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let staged = Self {
            path: directory.join("0").join(&metadata.model_file),
            directory,
            parent,
        };
        std::fs::create_dir(staged.directory.join("0")).map_err(|e| e.to_string())?;
        std::fs::copy(source, &staged.path).map_err(|e| e.to_string())?;
        crate::ai::validator::verify_model(metadata, &staged.path)?;
        Ok(staged)
    }
}
impl Drop for PreviewModel {
    fn drop(&mut self) {
        if self.directory.parent() == Some(self.parent.as_path())
            && self
                .directory
                .file_name()
                .is_some_and(|v| v.to_string_lossy().starts_with("preview-"))
        {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub wav: Vec<u8>,
    pub peak: f32,
    pub rms: f64,
    pub full_scale_samples: usize,
    pub frames: usize,
    pub sample_rate: u32,
    pub processing_ms: f64,
}
pub fn read_source(path: &Path, consent: bool) -> Result<(u32, Vec<f32>), String> {
    if !consent {
        return Err("Confirm permission to use the selected local WAV for preview".into());
    }
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 32 * 1024 * 1024 {
        return Err("Choose a WAV file no larger than 32 MiB".into());
    }
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if !(16000..=96000).contains(&spec.sample_rate) || !(1..=2).contains(&spec.channels) {
        return Err("Preview accepts mono/stereo WAV at 16–96 kHz".into());
    }
    let limit = spec.sample_rate as usize * usize::from(spec.channels) * 5;
    let values: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .take(limit)
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        (hound::SampleFormat::Int, 16 | 24 | 32) => {
            let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
            reader
                .samples::<i32>()
                .take(limit)
                .map(|v| v.map(|v| (f64::from(v) / scale) as f32))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?
        }
        _ => return Err("Unsupported WAV PCM format".into()),
    };
    if values.is_empty()
        || !values.len().is_multiple_of(usize::from(spec.channels))
        || values.iter().any(|v| !v.is_finite())
    {
        return Err("Empty, truncated or non-finite WAV".into());
    }
    Ok((
        spec.sample_rate,
        values
            .chunks_exact(usize::from(spec.channels))
            .map(|frame| frame.iter().sum::<f32>() / f32::from(spec.channels))
            .collect(),
    ))
}
pub fn render(preset: &VoicePreset, path: &Path, consent: bool) -> Result<Preview, String> {
    render_with_parameters(preset, path, consent, None)
}
pub fn render_with_parameters(
    preset: &VoicePreset,
    path: &Path,
    consent: bool,
    parameters: Option<crate::effects::parameters::EffectParameters>,
) -> Result<Preview, String> {
    super::validation::validate_preset(preset)?;
    let (rate, input) = read_source(path, consent)?;
    let start = Instant::now();
    let mut pipeline = AudioPipeline::new(
        rate,
        SharedEffectParameters::new(
            parameters
                .unwrap_or_else(|| preset.effect_parameters())
                .validated(),
        ),
        SharedNoiseSettings::new(NoiseSettings {
            enabled: false,
            gate_enabled: false,
            ..Default::default()
        }),
    );
    let signal_delay = pipeline.signal_latency_frames();
    let mut output = Vec::with_capacity(input.len() + pipeline.latency_frames() + 2048);
    for &sample in &input {
        pipeline.push(sample, |v| output.push(v));
    }
    for _ in 0..pipeline.latency_frames() + 2048 {
        pipeline.push(0.0, |v| output.push(v));
    }
    let mut limiter = OutputLimiter::new(rate);
    let delay = limiter.latency_frames();
    let limited: Vec<_> = output
        .into_iter()
        .skip(signal_delay)
        .map(|v| limiter.process(v, true, -1.0).0)
        .skip(delay)
        .take(input.len())
        .collect();
    encode(rate, limited, start.elapsed().as_secs_f64() * 1000.0)
}
pub fn encode(rate: u32, samples: Vec<f32>, processing_ms: f64) -> Result<Preview, String> {
    if samples.is_empty() || samples.iter().any(|v| !v.is_finite()) {
        return Err("Preview produced no valid PCM".into());
    }
    let peak = samples.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    let rms =
        (samples.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
    let full_scale_samples = samples.iter().filter(|v| v.abs() >= 1.0).count();
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: 1,
                sample_rate: rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(|e| e.to_string())?;
        for &v in &samples {
            writer
                .write_sample(crate::ai::transport::to_pcm16(v))
                .map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(Preview {
        wav: bytes.into_inner(),
        peak,
        rms,
        full_scale_samples,
        frames: samples.len(),
        sample_rate: rate,
        processing_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unconfirmed_file_reads_before_opening_any_path() {
        assert!(
            read_source(Path::new("not-opened.wav"), false)
                .unwrap_err()
                .contains("Confirm permission")
        );
    }
    #[test]
    #[ignore = "explicit licensed owner speech render benchmark; local WAV files, no microphone"]
    fn renders_every_female_preset_with_authorized_owner_speech() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let source = root.join(".ai-runtime-build/assets/owner-sample.wav");
        let destination = root.join(".ai-runtime-build/female-collection/previews");
        std::fs::create_dir_all(&destination).unwrap();
        let mut reports = Vec::new();
        let mut hashes = std::collections::HashSet::new();
        for preset in crate::presets::PresetRegistry::built_in()
            .unwrap()
            .all()
            .iter()
            .filter(|p| p.category == crate::presets::models::PresetCategory::Female)
        {
            let preview = render(preset, &source, true).unwrap();
            let hash = format!("{:x}", Sha256::digest(&preview.wav));
            assert!(
                hashes.insert(hash.clone()),
                "Duplicate rendered voice: {}",
                preset.name
            );
            assert_eq!(preview.full_scale_samples, 0);
            assert!(preview.peak > 0.0001 && preview.peak <= 0.892);
            std::fs::write(destination.join(format!("{}.wav", preset.id)), &preview.wav).unwrap();
            reports.push(serde_json::json!({"id":preset.id,"name":preset.name,"parameters":preset.effect_parameters(),"frames":preview.frames,"sampleRate":preview.sample_rate,"peak":preview.peak,"rms":preview.rms,"fullScaleSamples":preview.full_scale_samples,"renderMs":preview.processing_ms,"wavSha256":hash,"scope":"Offline local Japanese female owner sample; no male/Vietnamese speech or listening assessment"}));
        }
        assert_eq!(reports.len(), 20);
        let registry = crate::presets::PresetRegistry::built_in().unwrap();
        let first = registry
            .all()
            .iter()
            .find(|p| p.category == crate::presets::models::PresetCategory::Female)
            .unwrap();
        let custom = render_with_parameters(
            first,
            &source,
            true,
            Some(crate::effects::parameters::EffectParameters {
                pitch_semitones: -2.0,
                formant_semitones: -1.0,
                ..Default::default()
            }),
        )
        .unwrap();
        assert_ne!(
            format!("{:x}", Sha256::digest(&custom.wav)),
            reports[0]["wavSha256"].as_str().unwrap(),
            "Preview must honor edited parameters"
        );
        std::fs::write(
            destination.join("report.json"),
            serde_json::to_vec_pretty(&reports).unwrap(),
        )
        .unwrap();
        println!(
            "Rendered {} distinct speech WAVs; no microphone or output devices",
            reports.len()
        );
    }
}
