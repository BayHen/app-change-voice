use super::{
    noise::{NoiseSettings, SharedNoiseSettings},
    pipeline::AudioPipeline,
};
use crate::effects::parameters::{EffectParameters, SharedEffectParameters};
use std::{path::PathBuf, time::Instant};

#[test]
fn neutral_impulse_alignment_and_sweep_are_finite_without_noise_or_gain() {
    let mut pipeline = AudioPipeline::new(
        48000,
        SharedEffectParameters::new(EffectParameters::default()),
        SharedNoiseSettings::new(NoiseSettings {
            enabled: false,
            gate_enabled: false,
            ..Default::default()
        }),
    );
    let delay = pipeline.signal_latency_frames();
    let mut output = Vec::new();
    for i in 0..24000 {
        pipeline.push(if i == 4800 { 0.5 } else { 0.0 }, |v| output.push(v));
    }
    let peak = output
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap();
    assert_eq!(peak.0, 4800 + delay, "dry/processed sample-index alignment");
    assert!((*peak.1 - 0.5).abs() < 0.00001);
    assert!(output[..4800].iter().all(|v| v.abs() < 0.000001));
    let mut phase = 0.0_f64;
    for i in 0..96000 {
        let frequency = 30.0 * (18000.0_f64 / 30.0).powf(i as f64 / 96000.0);
        phase += std::f64::consts::TAU * frequency / 48000.0;
        let sample = (phase.sin() * 0.15 + (phase * 0.37).sin() * 0.05) as f32;
        pipeline.push(sample, |v| {
            assert!(v.is_finite());
            assert!(v.abs() < 0.3);
        });
    }
}

fn stats(samples: &[f32], rate: u32, frequency: f64) -> serde_json::Value {
    let n = samples.len().max(1) as f64;
    let peak = samples.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    let power = samples.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / n;
    let (mut sine, mut cosine, mut ss, mut cc, mut sc) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, sample) in samples.iter().enumerate() {
        let phase = std::f64::consts::TAU * frequency * i as f64 / f64::from(rate);
        sine += f64::from(*sample) * phase.sin();
        cosine += f64::from(*sample) * phase.cos();
        ss += phase.sin().powi(2);
        cc += phase.cos().powi(2);
        sc += phase.sin() * phase.cos();
    }
    let determinant = (ss * cc - sc * sc).max(1e-30);
    let a = (sine * cc - cosine * sc) / determinant;
    let b = (cosine * ss - sine * sc) / determinant;
    let fundamental = (a * a * ss + b * b * cc + 2.0 * a * b * sc) / n;
    let residual = samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let phase = std::f64::consts::TAU * frequency * i as f64 / f64::from(rate);
            (f64::from(*sample) - a * phase.sin() - b * phase.cos()).powi(2)
        })
        .sum::<f64>()
        / n;
    serde_json::json!({"frames":samples.len(),"peak":peak,"rms":power.sqrt(),"fullScaleSamples":samples.iter().filter(|v|v.abs()>=0.99999).count(),"exactRailSamples":samples.iter().filter(|v|v.abs()==1.0).count(),"nonFinite":samples.iter().filter(|v|!v.is_finite()).count(),"thdnDb":10.0*(residual/fundamental.max(1e-20)).log10()})
}

#[test]
#[ignore = "explicit developer synthetic diagnostic, writes local WAVs and measurements; no microphone"]
fn generate_quality_baseline() {
    let label = std::env::var("VOICEMORPH_QUALITY_LABEL").unwrap_or_else(|_| "before".into());
    assert!(
        label
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    );
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../.ai-runtime-build/audio-quality")
        .join(&label);
    std::fs::create_dir_all(&folder).unwrap();
    let mut cases = Vec::new();
    for (name, params, amplitude) in [
        ("neutral", EffectParameters::default(), 0.2_f32),
        (
            "bypass",
            EffectParameters {
                bypassed: true,
                ..Default::default()
            },
            0.2,
        ),
        (
            "boost-six-db",
            EffectParameters {
                makeup_gain_db: 6.0,
                ..Default::default()
            },
            0.6,
        ),
    ] {
        let mut pipeline = AudioPipeline::new(
            48000,
            SharedEffectParameters::new(params),
            SharedNoiseSettings::new(NoiseSettings {
                gate_enabled: false,
                enabled: false,
                limiter_enabled: false,
                ..Default::default()
            }),
        );
        let mut output = Vec::with_capacity(48000 * 3);
        let start = Instant::now();
        for i in 0..48000 * 3 {
            pipeline.push(
                (std::f64::consts::TAU * 997.0 * i as f64 / 48000.0).sin() as f32 * amplitude,
                |value| output.push(value),
            );
        }
        let elapsed = start.elapsed().as_secs_f64();
        let steady = &output[48000..96000];
        let mut report = stats(steady, 48000, 997.0);
        report["case"] = name.into();
        report["wallSeconds"] = elapsed.into();
        report["dspLatencyFrames"] = pipeline.latency_frames().into();
        let mut wav = hound::WavWriter::create(
            folder.join(format!("{name}.wav")),
            hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for &value in &output {
            wav.write_sample(value).unwrap();
        }
        wav.finalize().unwrap();
        cases.push(report);
        let mut limiter = crate::effects::limiter::OutputLimiter::new(48000);
        let mut final_output = Vec::with_capacity(steady.len());
        for &sample in steady {
            final_output.push(limiter.process(sample, true, -1.0).0);
        }
        let mut final_report = stats(&final_output[4800..43200], 48000, 997.0);
        final_report["case"] = format!("{name}-final-limiter").into();
        cases.push(final_report);
    }
    let mut resampler = crate::ai::transport::Resampler::new(48000, 16000).unwrap();
    let mut alias = Vec::new();
    for i in 0..48000 {
        resampler.push(
            (std::f64::consts::TAU * 18000.0 * i as f64 / 48000.0).sin() as f32 * 0.2,
            &mut alias,
        );
    }
    cases.push(serde_json::json!({"case":"18kHz-downsample-48-to-16-alias","stats":stats(&alias,16000,2000.0)}));
    let report = serde_json::json!({"label":label,"scope":"Deterministic synthetic, compiled test profile; wall time is not whole-process CPU or hardware latency", "cases":cases});
    std::fs::write(
        folder.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
