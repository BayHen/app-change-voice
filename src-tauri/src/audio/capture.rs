//! Explicit opt-in local stage captures. Samples cross bounded SPSC queues; all file I/O is on one writer thread.
use rtrb::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    RawInput,
    BypassReference,
    InputGain,
    ClassicDsp,
    AiInference,
    Mixer,
    Monitor,
    PcmBridge,
    VirtualOutput,
}
impl Stage {
    fn index(self) -> usize {
        self as usize
    }
    fn bit(self) -> u64 {
        1 << self.index()
    }
    fn name(self) -> &'static str {
        match self {
            Self::RawInput => "raw-input",
            Self::BypassReference => "bypass-reference",
            Self::InputGain => "input-gain",
            Self::ClassicDsp => "classic-dsp",
            Self::AiInference => "ai-inference",
            Self::Mixer => "mixer",
            Self::Monitor => "monitor",
            Self::PcmBridge => "pcm-bridge",
            Self::VirtualOutput => "virtual-output",
        }
    }
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub running: bool,
    pub folder: Option<String>,
    pub files: Vec<StageReport>,
    pub dropped_samples: u64,
    pub message: Option<String>,
    pub incomplete: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageReport {
    pub stage: Stage,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
    pub frames: u64,
    pub peak: f32,
    pub rms: f64,
    pub clipped_samples: u64,
    pub non_finite_samples: u64,
    pub file: String,
}
#[derive(Default)]
pub struct CaptureControl {
    mask: AtomicU64,
    generation: AtomicU64,
    seconds: AtomicU64,
    dropped: AtomicU64,
    recording: AtomicBool,
    state: Mutex<CaptureStatus>,
}
impl CaptureControl {
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Acquire)
    }
    pub fn start(
        &self,
        stages: Vec<Stage>,
        seconds: u64,
        consent: bool,
        folder: PathBuf,
    ) -> Result<CaptureStatus, String> {
        if !consent {
            return Err("Explicit consent to local diagnostic audio recording is required".into());
        }
        if stages.is_empty() || stages.len() > 9 || !(1..=30).contains(&seconds) {
            return Err("Choose 1–9 stages and 1–30 seconds".into());
        }
        let mut state = self.state.lock().map_err(|_| "Diagnostic state lock")?;
        if state.running {
            return Err("Stop the current diagnostic capture first".into());
        }
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        *state = CaptureStatus {
            running: true,
            folder: Some(folder.to_string_lossy().into()),
            message: Some(
                "Explicit local capture active. Unavailable stages produce no file.".into(),
            ),
            ..Default::default()
        };
        self.seconds.store(seconds, Ordering::Release);
        self.dropped.store(0, Ordering::Release);
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.recording.store(true, Ordering::Release);
        self.mask.store(
            stages.iter().fold(0, |mask, stage| mask | stage.bit()),
            Ordering::Release,
        );
        Ok(state.clone())
    }
    pub fn stop(&self) -> CaptureStatus {
        self.mask.store(0, Ordering::Release);
        self.status()
    }
    pub fn status(&self) -> CaptureStatus {
        let mut status = self.state.lock().map(|s| s.clone()).unwrap_or_default();
        status.dropped_samples = self.dropped.load(Ordering::Relaxed);
        status
    }
}
#[derive(Clone, Copy)]
struct Sample {
    stage: Stage,
    rate: u32,
    channels: u16,
    value: f32,
    generation: u64,
}
pub struct Tap {
    producer: Producer<Sample>,
    control: Arc<CaptureControl>,
    generation: u64,
    counts: [u64; 9],
}
impl Tap {
    pub fn push_frame(
        &mut self,
        stage: Stage,
        rate: u32,
        channels: u16,
        sample: impl Fn(usize) -> f32,
    ) {
        if self.control.mask.load(Ordering::Acquire) & stage.bit() == 0 {
            return;
        }
        let generation = self.control.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.generation = generation;
            self.counts = [0; 9];
        }
        let count = &mut self.counts[stage.index()];
        if *count
            >= self.control.seconds.load(Ordering::Relaxed) * u64::from(rate) * u64::from(channels)
        {
            self.control.mask.fetch_and(!stage.bit(), Ordering::AcqRel);
            return;
        }
        if self.producer.slots() < usize::from(channels) {
            self.control
                .dropped
                .fetch_add(u64::from(channels), Ordering::Relaxed);
            return;
        }
        for channel in 0..usize::from(channels) {
            let _ = self.producer.push(Sample {
                stage,
                rate,
                channels,
                value: sample(channel),
                generation,
            });
        }
        *count += u64::from(channels);
    }
    #[inline]
    pub fn push(&mut self, stage: Stage, rate: u32, value: f32) {
        self.push_channels(stage, rate, 1, value);
    }
    pub fn push_channels(&mut self, stage: Stage, rate: u32, channels: u16, value: f32) {
        if self.control.mask.load(Ordering::Acquire) & stage.bit() == 0 {
            return;
        }
        let generation = self.control.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.generation = generation;
            self.counts = [0; 9];
        }
        let count = &mut self.counts[stage.index()];
        if *count
            >= self.control.seconds.load(Ordering::Relaxed) * u64::from(rate) * u64::from(channels)
        {
            self.control.mask.fetch_and(!stage.bit(), Ordering::AcqRel);
            return;
        }
        *count += 1;
        if self
            .producer
            .push(Sample {
                stage,
                rate,
                channels,
                value,
                generation,
            })
            .is_err()
        {
            self.control.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
pub struct Taps {
    pub input: Tap,
    pub dsp: Tap,
    pub monitor: Tap,
    pub virtual_output: Tap,
    pub ai: Tap,
}
pub struct CaptureWriter {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for CaptureWriter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
type Wav = hound::WavWriter<std::io::BufWriter<std::fs::File>>;
struct FileState {
    writer: Wav,
    report: StageReport,
    power: f64,
    samples: u64,
}
impl CaptureWriter {
    pub fn new(control: Arc<CaptureControl>) -> Result<(Self, Taps), String> {
        let mut consumers = Vec::<Consumer<Sample>>::new();
        let mut tap = || {
            let (producer, consumer) = RingBuffer::new(65536);
            consumers.push(consumer);
            Tap {
                producer,
                control: control.clone(),
                generation: 0,
                counts: [0; 9],
            }
        };
        let taps = Taps {
            input: tap(),
            dsp: tap(),
            monitor: tap(),
            virtual_output: tap(),
            ai: tap(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop_copy = stop.clone();
        let thread=std::thread::Builder::new().name("audio-diagnostic-writer".into()).spawn(move || {
            let mut generation=0;let mut files:[Option<FileState>;9]=std::array::from_fn(|_|None);
            let mut started=Instant::now();let mut update=Instant::now();let mut folder=None::<PathBuf>;let mut idle=None::<Instant>;
            loop {
                let next=control.generation.load(Ordering::Acquire);
                if next!=generation {
                    for slot in &mut files {if let Some(file)=slot.take(){let _=file.writer.finalize();}}
                    generation=next;started=Instant::now();idle=None;folder=control.status().folder.map(PathBuf::from);
                }
                for consumer in &mut consumers {
                    let queued=consumer.slots();
                    for _ in 0..queued {
                        let Ok(sample)=consumer.pop() else {break;};if sample.generation!=generation || !control.recording.load(Ordering::Acquire) {continue;}
                        let slot=&mut files[sample.stage.index()];
                        if slot.is_none() {
                            let Some(folder)=folder.as_ref() else {continue;};
                            let path=folder.join(format!("{}.wav",sample.stage.name()));
                            match hound::WavWriter::create(&path,hound::WavSpec {channels:sample.channels,sample_rate:sample.rate,bits_per_sample:32,sample_format:hound::SampleFormat::Float}) {
                                Ok(writer)=>*slot=Some(FileState {writer,power:0.0,samples:0,report:StageReport {stage:sample.stage,sample_rate:sample.rate,channels:sample.channels,sample_format:"f32 WAV, little-endian; normalized stage".into(),frames:0,peak:0.0,rms:0.0,clipped_samples:0,non_finite_samples:0,file:path.to_string_lossy().into()}}),
                                Err(error)=>{if let Ok(mut status)=control.state.lock(){status.incomplete=true;status.message=Some(error.to_string());}control.mask.store(0,Ordering::Release);continue;}
                            }
                        }
                        if let Some(file)=slot {
                            let value=if sample.value.is_finite(){sample.value}else{file.report.non_finite_samples+=1;0.0};
                            if let Err(error)=file.writer.write_sample(value){if let Ok(mut status)=control.state.lock(){status.incomplete=true;status.message=Some(error.to_string());}control.mask.store(0,Ordering::Release);continue;}
                            file.samples+=1;file.report.frames=file.samples/u64::from(file.report.channels);file.report.peak=file.report.peak.max(value.abs());file.power+=f64::from(value).powi(2);
                            file.report.clipped_samples+=u64::from(value.abs()>=1.0);file.report.rms=(file.power/file.samples as f64).sqrt();
                        }
                    }
                }
                if started.elapsed()>Duration::from_secs(control.seconds.load(Ordering::Relaxed)+3) {control.mask.store(0,Ordering::Release);}
                if control.mask.load(Ordering::Acquire)==0 {if idle.is_none(){idle=Some(Instant::now());}}else{idle=None;}
                let done=stop_copy.load(Ordering::Acquire)||(idle.is_some_and(|time|time.elapsed()>Duration::from_millis(30)) && consumers.iter().all(|c|c.slots()==0));
                if update.elapsed()>Duration::from_millis(100)||done {
                    if let Ok(mut status)=control.state.lock(){if status.running {status.files=files.iter().filter_map(|slot|slot.as_ref().map(|file|file.report.clone())).collect();status.dropped_samples=control.dropped.load(Ordering::Relaxed);}}
                    update=Instant::now();
                }
                if done {
                    if let Ok(mut status)=control.state.lock(){
                      if status.running && generation!=0 && generation==control.generation.load(Ordering::Acquire) && (stop_copy.load(Ordering::Acquire)||control.mask.load(Ordering::Acquire)==0) {
                        control.mask.store(0,Ordering::Release);
                        control.recording.store(false,Ordering::Release);
                        for slot in &mut files {if let Some(file)=slot.take(){if let Err(error)=file.writer.finalize(){status.incomplete=true;status.message=Some(error.to_string());}}}
                        status.running=false;if !status.incomplete {status.message=Some("Local capture finalized; stages without a live source produce no file. Full-scale sample counts describe float headroom, not necessarily clipping.".into());}
                        if let Some(folder)=&folder {if let Ok(json)=serde_json::to_vec_pretty(&*status){if let Err(error)=std::fs::write(folder.join("manifest.json"),json){status.incomplete=true;status.message=Some(error.to_string());}}}
                      }
                    }
                }
                if stop_copy.load(Ordering::Acquire){break;}
                std::thread::sleep(Duration::from_millis(2));
            }
        }).map_err(|e|e.to_string())?;
        Ok((
            Self {
                stop,
                thread: Some(thread),
            },
            taps,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consent_is_required_and_stereo_frames_are_preserved_without_automatic_files() {
        let control = Arc::new(CaptureControl::default());
        let path = std::env::temp_dir().join(format!(
            "vm-capture-test-{}-{}",
            std::process::id(),
            crate::ai::process::now_ms()
        ));
        let (_writer, mut taps) = CaptureWriter::new(control.clone()).unwrap();
        // Starting after an idle writer previously raced with its finalization branch.
        std::thread::sleep(Duration::from_millis(70));
        assert!(!path.exists());
        assert!(
            control
                .start(vec![Stage::RawInput], 1, false, path.clone())
                .is_err()
        );
        assert!(!path.exists());
        control
            .start(vec![Stage::RawInput], 1, true, path.clone())
            .unwrap();
        for _ in 0..20 {
            taps.input.push_frame(Stage::RawInput, 48000, 2, |channel| {
                if channel == 0 { 0.25 } else { -0.5 }
            });
        }
        control.stop();
        for _ in 0..100 {
            if !control.status().running {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let status = control.status();
        assert!(!status.running);
        assert_eq!(status.files[0].frames, 20);
        let mut wav = hound::WavReader::open(path.join("raw-input.wav")).unwrap();
        assert_eq!(wav.spec().channels, 2);
        assert_eq!(wav.duration(), 20);
        assert_eq!(
            wav.samples::<f32>()
                .take(2)
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            [0.25, -0.5]
        );
        drop(wav);
        drop(taps);
        drop(_writer);
        std::fs::remove_dir_all(path).unwrap();
    }
}
