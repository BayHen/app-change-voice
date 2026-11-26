use super::{SoundEntry, SoundRoute};
use std::{fs::File, io::BufReader, path::Path};

pub const MAX_VOICES: usize = 16;
const MAX_CLIP_SECONDS: u32 = 120;
const MAX_CLIP_FRAMES: usize = 6_000_000;
const MAX_SESSION_FRAMES: usize = 16_000_000;
#[derive(Clone)]
pub struct SoundClip {
    pub id: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub volume: f32,
    pub route: SoundRoute,
}
#[derive(Clone, Copy, Debug)]
pub enum MixerCommand {
    Play(usize),
    Stop(usize),
    StopAll,
}
#[derive(Clone, Copy, Default)]
struct Voice {
    clip: usize,
    position: f64,
    active: bool,
}
pub struct SoundboardMixer {
    clips: Vec<SoundClip>,
    voices: [Voice; MAX_VOICES],
    output_rate: u32,
}
impl SoundboardMixer {
    pub fn load(entries: &[SoundEntry], output_rate: u32) -> Result<Self, String> {
        let mut clips = Vec::new();
        let mut remaining_frames = MAX_SESSION_FRAMES;
        for e in entries.iter().filter(|e| !e.missing) {
            let clip =
                decode(e, remaining_frames).map_err(|error| format!("{}: {error}", e.name))?;
            remaining_frames -= clip.samples.len();
            clips.push(clip);
        }
        Ok(Self {
            clips,
            voices: [Voice::default(); MAX_VOICES],
            output_rate,
        })
    }
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.id == id)
    }
    pub fn command(&mut self, c: MixerCommand) {
        match c {
            MixerCommand::Play(i) if i < self.clips.len() => {
                let slot = self.voices.iter().position(|v| !v.active).unwrap_or(0);
                self.voices[slot] = Voice {
                    clip: i,
                    position: 0.0,
                    active: true,
                };
            }
            MixerCommand::Stop(i) => {
                for v in &mut self.voices {
                    if v.active && v.clip == i {
                        v.active = false
                    }
                }
            }
            MixerCommand::StopAll => {
                for v in &mut self.voices {
                    v.active = false
                }
            }
            _ => {}
        }
    }
    pub fn next_sample(&mut self) -> (f32, f32) {
        let mut hp = 0.0;
        let mut virt = 0.0;
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            let c = &self.clips[v.clip];
            let pos = v.position as usize;
            if pos >= c.samples.len() {
                v.active = false;
                continue;
            }
            let s = c.samples[pos] * c.volume;
            match c.route {
                SoundRoute::Headphones => hp += s,
                SoundRoute::VirtualMicrophone => virt += s,
                SoundRoute::Both => {
                    hp += s;
                    virt += s
                }
            }
            v.position += c.sample_rate as f64 / self.output_rate as f64;
        }
        (hp, virt)
    }
}
fn open_wav(path: &Path) -> Result<hound::WavReader<BufReader<File>>, String> {
    let reader = hound::WavReader::open(path).map_err(|error| format!("Invalid WAV: {error}"))?;
    let spec = reader.spec();
    validate_wav(spec, reader.len())?;
    Ok(reader)
}

fn validate_wav(spec: hound::WavSpec, samples: u32) -> Result<(), String> {
    let channels = u32::from(spec.channels);
    if !(1..=8).contains(&channels) || !(8_000..=192_000).contains(&spec.sample_rate) {
        return Err("WAV must have 1–8 channels and a sample rate of 8–192 kHz".into());
    }
    if samples == 0 || !samples.is_multiple_of(channels) {
        return Err("WAV has empty or incomplete audio frames".into());
    }
    let frames = samples / channels;
    if frames > MAX_CLIP_SECONDS * spec.sample_rate || frames as usize > MAX_CLIP_FRAMES {
        return Err("WAV exceeds the Soundboard clip duration or frame limit".into());
    }
    Ok(())
}

pub(super) fn wav_duration(path: &Path) -> Result<f32, String> {
    let reader = open_wav(path)?;
    Ok(reader.duration() as f32 / reader.spec().sample_rate as f32)
}

fn decode(e: &SoundEntry, remaining_frames: usize) -> Result<SoundClip, String> {
    let mut r = open_wav(Path::new(&e.file_path))?;
    let spec = r.spec();
    let frames = r.duration() as usize;
    if frames > remaining_frames {
        return Err("Decoded Soundboard exceeds the 64 MB session limit".into());
    }
    let channels = usize::from(spec.channels);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => downmix(r.samples::<f32>(), channels, frames)?,
        hound::SampleFormat::Int => {
            let scale = (1_i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            downmix(
                r.samples::<i32>().map(|x| x.map(|v| v as f32 / scale)),
                channels,
                frames,
            )?
        }
    };
    Ok(SoundClip {
        id: e.id.clone(),
        samples,
        sample_rate: spec.sample_rate,
        volume: e.volume_percent / 100.0,
        route: e.route,
    })
}

fn downmix(
    mut source: impl Iterator<Item = hound::Result<f32>>,
    channels: usize,
    frames: usize,
) -> Result<Vec<f32>, String> {
    let mut samples = Vec::with_capacity(frames);
    for _ in 0..frames {
        let mut sum = 0.0;
        for _ in 0..channels {
            let value = source
                .next()
                .ok_or("Incomplete WAV frame")?
                .map_err(|error| format!("Invalid WAV sample: {error}"))?;
            if value.is_finite() {
                sum += value.clamp(-1.0, 1.0);
            }
        }
        samples.push(sum / channels as f32);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_clip_limits_before_decoding() {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        assert!(validate_wav(spec, 2).is_ok());
        assert!(validate_wav(spec, 3).is_err());
        assert!(validate_wav(spec, 2 * 48_000 * 121).is_err());
        assert!(
            validate_wav(
                hound::WavSpec {
                    channels: 0,
                    ..spec
                },
                2
            )
            .is_err()
        );
    }

    #[test]
    fn downmix_sanitizes_invalid_samples_and_rejects_truncation() {
        let input = [Ok(1.0), Ok(-0.5), Ok(f32::NAN), Ok(f32::INFINITY)];
        assert_eq!(downmix(input.into_iter(), 2, 2).unwrap(), [0.25, 0.0]);
        assert!(downmix([Ok(0.5)].into_iter(), 2, 1).is_err());
    }
    #[test]
    fn repeated_play_is_bounded() {
        let mut m = SoundboardMixer {
            clips: vec![SoundClip {
                id: "x".into(),
                samples: vec![0.8],
                sample_rate: 48000,
                volume: 1.0,
                route: SoundRoute::Both,
            }],
            voices: [Voice::default(); MAX_VOICES],
            output_rate: 48000,
        };
        for _ in 0..100 {
            m.command(MixerCommand::Play(0))
        }
        let (a, b) = m.next_sample();
        assert!(a.is_finite() && b.is_finite());
        assert!(m.voices.len() == MAX_VOICES)
    }
}
