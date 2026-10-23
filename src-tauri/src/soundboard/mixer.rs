use super::{SoundEntry, SoundRoute};
use std::path::Path;

pub const MAX_VOICES: usize = 16;
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
        for e in entries.iter().filter(|e| !e.missing) {
            clips.push(decode(e)?)
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
fn decode(e: &SoundEntry) -> Result<SoundClip, String> {
    let mut r = hound::WavReader::open(Path::new(&e.file_path)).map_err(|x| x.to_string())?;
    let spec = r.spec();
    let ch = usize::from(spec.channels);
    if ch == 0 {
        return Err("WAV has no channels".into());
    }
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r
            .samples::<f32>()
            .map(|x| x.map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = (1_i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            r.samples::<i32>()
                .map(|x| x.map(|v| v as f32 / scale).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?
        }
    };
    let samples = raw
        .chunks(ch)
        .map(|f| f.iter().copied().sum::<f32>() / f.len() as f32)
        .collect();
    Ok(SoundClip {
        id: e.id.clone(),
        samples,
        sample_rate: spec.sample_rate,
        volume: e.volume_percent / 100.0,
        route: e.route,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
