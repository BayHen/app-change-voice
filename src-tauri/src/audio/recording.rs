use rtrb::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::errors::AudioDeviceError;

const QUEUE_SECONDS: usize = 2;
const PLAYBACK_BUFFER_SECONDS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingSource {
    Original,
    Processed,
}

#[derive(Debug, Clone, Copy)]
pub struct RecordingFrame {
    pub original: f32,
    pub processed: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub recording: bool,
    pub playing: bool,
    pub hear_myself: bool,
    pub source: RecordingSource,
    pub duration_seconds: f64,
    pub playback_seconds: f64,
    pub dropped_frames: u64,
    pub has_recording: bool,
    pub incomplete: bool,
    pub error: Option<String>,
}

enum Command {
    Start,
    Stop,
    Play(RecordingSource),
    StopPlayback,
    Export(RecordingSource, PathBuf),
    Delete,
    Shutdown,
}

#[derive(Default)]
struct WorkerState {
    sample_rate: u32,
    total_frames: u64,
    has_recording: bool,
    error: Option<String>,
}
pub(crate) struct Shared {
    recording: AtomicBool,
    playing: AtomicBool,
    hear: AtomicBool,
    source: AtomicU8,
    dropped: AtomicU64,
    played: AtomicU64,
    playback_feed_complete: AtomicBool,
    state: Mutex<WorkerState>,
}

pub struct RecordingRuntime {
    shared: Arc<Shared>,
    command: Sender<Command>,
    worker: Option<JoinHandle<()>>,
}

pub(crate) struct RecordingEndpoints {
    pub(crate) record_producer: Producer<RecordingFrame>,
    pub(crate) playback_consumer: Consumer<f32>,
    pub(crate) shared: Arc<Shared>,
}

impl RecordingRuntime {
    pub(crate) fn new(
        sample_rate: u32,
        temp_dir: PathBuf,
    ) -> Result<(Self, RecordingEndpoints), AudioDeviceError> {
        fs::create_dir_all(&temp_dir).map_err(|e| AudioDeviceError::backend(e.to_string()))?;
        let record_capacity = (sample_rate as usize * QUEUE_SECONDS).next_power_of_two();
        let playback_capacity =
            (sample_rate as usize * PLAYBACK_BUFFER_SECONDS).next_power_of_two();
        let (record_producer, record_consumer) = RingBuffer::new(record_capacity);
        let (playback_producer, playback_consumer) = RingBuffer::new(playback_capacity);
        let shared = Arc::new(Shared {
            recording: AtomicBool::new(false),
            playing: AtomicBool::new(false),
            hear: AtomicBool::new(false),
            source: AtomicU8::new(1),
            dropped: AtomicU64::new(0),
            played: AtomicU64::new(0),
            playback_feed_complete: AtomicBool::new(false),
            state: Mutex::new(WorkerState {
                sample_rate,
                ..Default::default()
            }),
        });
        let (tx, rx) = mpsc::channel();
        let worker_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("voice-recording-writer".into())
            .spawn(move || {
                worker_loop(
                    record_consumer,
                    playback_producer,
                    rx,
                    worker_shared,
                    temp_dir,
                )
            })
            .map_err(|e| AudioDeviceError::backend(e.to_string()))?;
        Ok((
            Self {
                shared: shared.clone(),
                command: tx,
                worker: Some(worker),
            },
            RecordingEndpoints {
                record_producer,
                playback_consumer,
                shared,
            },
        ))
    }
    pub fn status(&self) -> RecordingStatus {
        let state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        RecordingStatus {
            recording: self.shared.recording.load(Ordering::Acquire),
            playing: self.shared.playing.load(Ordering::Acquire),
            hear_myself: self.shared.hear.load(Ordering::Acquire),
            source: if self.shared.source.load(Ordering::Relaxed) == 0 {
                RecordingSource::Original
            } else {
                RecordingSource::Processed
            },
            duration_seconds: state.total_frames as f64 / state.sample_rate.max(1) as f64,
            playback_seconds: self.shared.played.load(Ordering::Relaxed) as f64
                / state.sample_rate.max(1) as f64,
            dropped_frames: self.shared.dropped.load(Ordering::Relaxed),
            has_recording: state.has_recording,
            incomplete: self.shared.dropped.load(Ordering::Relaxed) > 0,
            error: state.error.clone(),
        }
    }
    pub fn start(&self) -> Result<(), AudioDeviceError> {
        self.send(Command::Start)
    }
    pub fn stop(&self) -> Result<(), AudioDeviceError> {
        self.send(Command::Stop)
    }
    pub fn play(&self, source: RecordingSource) -> Result<(), AudioDeviceError> {
        self.send(Command::Play(source))
    }
    pub fn stop_playback(&self) -> Result<(), AudioDeviceError> {
        self.send(Command::StopPlayback)
    }
    pub fn export(&self, source: RecordingSource, path: PathBuf) -> Result<(), AudioDeviceError> {
        self.send(Command::Export(source, path))
    }
    pub fn delete(&self) -> Result<(), AudioDeviceError> {
        self.send(Command::Delete)
    }
    pub fn set_hear_myself(&self, value: bool) {
        self.shared.hear.store(value, Ordering::Release)
    }
    fn send(&self, command: Command) -> Result<(), AudioDeviceError> {
        self.command
            .send(command)
            .map_err(|_| AudioDeviceError::backend("Recording worker is unavailable"))
    }
}
impl Drop for RecordingRuntime {
    fn drop(&mut self) {
        let _ = self.command.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Shared {
    #[inline]
    pub fn push_recording(&self, producer: &mut Producer<RecordingFrame>, frame: RecordingFrame) {
        if self.recording.load(Ordering::Acquire) && producer.push(frame).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[inline]
    pub fn playback_or_monitor(&self, consumer: &mut Consumer<f32>, monitor: f32) -> f32 {
        if self.playing.load(Ordering::Acquire) {
            match consumer.pop() {
                Ok(sample) => {
                    self.played.fetch_add(1, Ordering::Relaxed);
                    sample
                }
                Err(_) => {
                    if self.playback_feed_complete.load(Ordering::Acquire) {
                        self.playing.store(false, Ordering::Release);
                    }
                    0.0
                }
            }
        } else if self.hear.load(Ordering::Acquire) {
            monitor
        } else {
            0.0
        }
    }
}

fn worker_loop(
    mut input: Consumer<RecordingFrame>,
    mut playback: Producer<f32>,
    commands: Receiver<Command>,
    shared: Arc<Shared>,
    dir: PathBuf,
) {
    let original_path = dir.join("original.wav");
    let processed_path = dir.join("processed.wav");
    let mut original = Vec::<f32>::new();
    let mut processed = Vec::<f32>::new();
    let mut play_data = Vec::<f32>::new();
    let mut play_index = 0usize;
    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Start => {
                    original.clear();
                    processed.clear();
                    shared.dropped.store(0, Ordering::Relaxed);
                    let mut s = shared.state.lock().unwrap_or_else(|e| e.into_inner());
                    s.total_frames = 0;
                    s.has_recording = false;
                    s.error = None;
                    shared.playing.store(false, Ordering::Release);
                    shared.played.store(0, Ordering::Relaxed);
                    shared
                        .playback_feed_complete
                        .store(false, Ordering::Release);
                    shared.recording.store(true, Ordering::Release);
                }
                Command::Stop => {
                    shared.recording.store(false, Ordering::Release);
                    drain(&mut input, &mut original, &mut processed);
                    let rate = shared
                        .state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .sample_rate;
                    let result =
                        write_pair(&original_path, &processed_path, rate, &original, &processed);
                    let mut s = shared.state.lock().unwrap_or_else(|e| e.into_inner());
                    s.total_frames = original.len().min(processed.len()) as u64;
                    s.has_recording = result.is_ok() && s.total_frames > 0;
                    s.error = result.err();
                }
                Command::Play(source) => {
                    let path = match source {
                        RecordingSource::Original => &original_path,
                        RecordingSource::Processed => &processed_path,
                    };
                    match read_wav(path) {
                        Ok(data) => {
                            play_data = data;
                            play_index = 0;
                            shared.played.store(0, Ordering::Relaxed);
                            shared
                                .playback_feed_complete
                                .store(false, Ordering::Release);
                            shared.source.store(
                                if source == RecordingSource::Original {
                                    0
                                } else {
                                    1
                                },
                                Ordering::Relaxed,
                            );
                            shared.playing.store(true, Ordering::Release)
                        }
                        Err(e) => set_error(&shared, e),
                    }
                }
                Command::StopPlayback => {
                    shared.playing.store(false, Ordering::Release);
                    play_data.clear();
                    play_index = 0;
                    shared.played.store(0, Ordering::Relaxed);
                    shared
                        .playback_feed_complete
                        .store(false, Ordering::Release);
                }
                Command::Export(source, path) => {
                    let source = match source {
                        RecordingSource::Original => &original_path,
                        RecordingSource::Processed => &processed_path,
                    };
                    if let Err(e) = fs::copy(source, path)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                    {
                        set_error(&shared, e)
                    }
                }
                Command::Delete => {
                    shared.recording.store(false, Ordering::Release);
                    shared.playing.store(false, Ordering::Release);
                    original.clear();
                    processed.clear();
                    play_data.clear();
                    shared.played.store(0, Ordering::Relaxed);
                    shared
                        .playback_feed_complete
                        .store(false, Ordering::Release);
                    let _ = fs::remove_file(&original_path);
                    let _ = fs::remove_file(&processed_path);
                    let mut s = shared.state.lock().unwrap_or_else(|e| e.into_inner());
                    s.total_frames = 0;
                    s.has_recording = false;
                    s.error = None;
                }
                Command::Shutdown => return,
            }
        }
        if shared.recording.load(Ordering::Acquire) {
            drain(&mut input, &mut original, &mut processed);
            shared
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .total_frames = original.len().min(processed.len()) as u64;
        } else {
            while input.pop().is_ok() {}
        }
        if shared.playing.load(Ordering::Acquire) {
            while play_index < play_data.len() {
                if playback.push(play_data[play_index]).is_err() {
                    break;
                }
                play_index += 1;
            }
            if play_index >= play_data.len() {
                shared.playback_feed_complete.store(true, Ordering::Release)
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
}
fn drain(input: &mut Consumer<RecordingFrame>, original: &mut Vec<f32>, processed: &mut Vec<f32>) {
    while let Ok(frame) = input.pop() {
        original.push(frame.original);
        processed.push(frame.processed);
    }
}
fn set_error(shared: &Shared, error: String) {
    shared.state.lock().unwrap_or_else(|e| e.into_inner()).error = Some(error)
}
fn write_pair(
    original_path: &Path,
    processed_path: &Path,
    rate: u32,
    original: &[f32],
    processed: &[f32],
) -> Result<(), String> {
    write_wav(original_path, rate, original)?;
    write_wav(processed_path, rate, processed)
}
fn write_wav(path: &Path, rate: u32, data: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &sample in data {
        writer
            .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .map_err(|e| e.to_string())?
    }
    writer.finalize().map_err(|e| e.to_string())
}
fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    reader
        .samples::<i16>()
        .map(|s| {
            s.map(|v| v as f32 / i16::MAX as f32)
                .map_err(|e| e.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_round_trip_and_delete() {
        let dir = std::env::temp_dir().join(format!("voicemorph-test-{}", std::process::id()));
        let path = dir.join("test.wav");
        fs::create_dir_all(&dir).unwrap();
        write_wav(&path, 48_000, &[0.0, 0.5, -0.5]).unwrap();
        let data = read_wav(&path).unwrap();
        assert_eq!(data.len(), 3);
        assert!((data[1] - 0.5).abs() < 0.001);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn bounded_queue_reports_drops() {
        let shared = Arc::new(Shared {
            recording: AtomicBool::new(true),
            playing: AtomicBool::new(false),
            hear: AtomicBool::new(false),
            source: AtomicU8::new(1),
            dropped: AtomicU64::new(0),
            played: AtomicU64::new(0),
            playback_feed_complete: AtomicBool::new(false),
            state: Mutex::new(WorkerState {
                sample_rate: 48_000,
                ..Default::default()
            }),
        });
        let (mut p, _) = RingBuffer::new(1);
        let f = RecordingFrame {
            original: 0.0,
            processed: 0.0,
        };
        shared.push_recording(&mut p, f);
        shared.push_recording(&mut p, f);
        assert_eq!(shared.dropped.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn writes_controlled_five_and_thirty_second_wav_files() {
        let dir = std::env::temp_dir().join(format!("voicemorph-duration-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for seconds in [5, 30] {
            let path = dir.join(format!("{seconds}.wav"));
            let samples = vec![0.0; 48_000 * seconds];
            write_wav(&path, 48_000, &samples).unwrap();
            let reader = hound::WavReader::open(path).unwrap();
            assert_eq!(reader.spec().sample_rate, 48_000);
            assert_eq!(reader.duration(), 48_000 * seconds as u32);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reports_disk_write_failure() {
        let missing_parent = std::env::temp_dir()
            .join(format!("voicemorph-missing-{}", std::process::id()))
            .join("recording.wav");
        assert!(write_wav(&missing_parent, 48_000, &[0.0]).is_err());
    }
}
