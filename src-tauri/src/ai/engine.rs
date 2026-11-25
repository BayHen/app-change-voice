use super::{
    config::{AI_SAMPLE_RATE, EngineConfig, FailurePolicy, UPSTREAM_COMMIT},
    models::AIModelMetadata,
    process::now_ms,
    provider::{VoiceConversionBackend, WOkadaBackend},
    runtime::{RuntimeInstallStatus, VerifiedRuntime},
    transport::{self, RealtimePort, Resampler, SharedTransport, TaggedSample, WorkerPort},
};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: String,
    pub mode: String,
    pub classic_fallback_active: bool,
    pub runtime_installed: bool,
    pub model_id: Option<String>,
    pub provider: Option<String>,
    pub upstream_commit: String,
    pub loading_ms: Option<f64>,
    pub conversion_ms: Option<f64>,
    pub round_trip_ms: Option<f64>,
    pub queued_input_samples: u64,
    pub queued_output_samples: u64,
    pub dropped_input_samples: u64,
    pub dropped_output_samples: u64,
    pub missing_output_samples: u64,
    pub completed_chunks: u64,
    pub restarts: u32,
    pub message: Option<String>,
    pub config: EngineConfig,
    pub install: RuntimeInstallStatus,
}
impl Default for EngineStatus {
    fn default() -> Self {
        Self {
            state: "notInstalled".into(),
            mode: "classicDsp".into(),
            classic_fallback_active: false,
            runtime_installed: false,
            model_id: None,
            provider: None,
            upstream_commit: UPSTREAM_COMMIT.into(),
            loading_ms: None,
            conversion_ms: None,
            round_trip_ms: None,
            queued_input_samples: 0,
            queued_output_samples: 0,
            dropped_input_samples: 0,
            dropped_output_samples: 0,
            missing_output_samples: 0,
            completed_chunks: 0,
            restarts: 0,
            message: None,
            config: EngineConfig::default(),
            install: RuntimeInstallStatus::default(),
        }
    }
}

pub struct AIEngine {
    pub shared: Arc<SharedTransport>,
    pub snapshot: Arc<Mutex<EngineStatus>>,
    attachment: Arc<Mutex<Option<WorkerPort>>>,
    runtime_root: PathBuf,
    config_path: PathBuf,
    config: EngineConfig,
    stop: Arc<AtomicBool>,
    hard_cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    pub install_cancel: Arc<AtomicBool>,
    pub installing: Arc<AtomicBool>,
}
impl Default for AIEngine {
    fn default() -> Self {
        Self {
            shared: Arc::new(SharedTransport::default()),
            snapshot: Arc::new(Mutex::new(EngineStatus::default())),
            attachment: Arc::new(Mutex::new(None)),
            runtime_root: PathBuf::new(),
            config_path: PathBuf::new(),
            config: EngineConfig::default(),
            stop: Arc::new(AtomicBool::new(false)),
            hard_cancel: Arc::new(AtomicBool::new(false)),
            worker: None,
            install_cancel: Arc::new(AtomicBool::new(false)),
            installing: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl AIEngine {
    pub fn configure(&mut self, runtime_root: PathBuf, config_path: PathBuf) {
        self.runtime_root = runtime_root;
        self.config_path = config_path;
        match EngineConfig::load(&self.config_path) {
            Ok(c) => self.config = c,
            Err(e) => self.update(|s| s.message = Some(e)),
        }
        self.shared.classic_fallback.store(
            self.config.failure_policy == FailurePolicy::ClassicDsp,
            Ordering::Release,
        );
        let installed = self.runtime_root.join("current.json").is_file();
        let config = self.config.clone();
        self.update(|s| {
            s.config = config;
            s.runtime_installed = installed;
            s.state = if installed { "stopped" } else { "notInstalled" }.into();
        });
    }
    pub fn runtime_root(&self) -> PathBuf {
        self.runtime_root.clone()
    }
    pub fn update(&self, update: impl FnOnce(&mut EngineStatus)) {
        if let Ok(mut state) = self.snapshot.lock() {
            update(&mut state);
        }
    }
    pub fn status(&self) -> EngineStatus {
        let mut status = self.snapshot.lock().map(|v| v.clone()).unwrap_or_default();
        let s = &self.shared;
        status.mode = if s.ai_selected.load(Ordering::Acquire) {
            "wOkada"
        } else {
            "classicDsp"
        }
        .into();
        status.classic_fallback_active = s.ai_selected.load(Ordering::Acquire)
            && s.faulted.load(Ordering::Acquire)
            && s.classic_fallback.load(Ordering::Acquire);
        status.queued_input_samples = s.queued_input_samples.load(Ordering::Relaxed);
        status.queued_output_samples = s.queued_output_samples.load(Ordering::Relaxed);
        status.dropped_input_samples = s.dropped_input_samples.load(Ordering::Relaxed);
        status.dropped_output_samples = s.dropped_output_samples.load(Ordering::Relaxed);
        status.missing_output_samples = s.missing_output_samples.load(Ordering::Relaxed);
        status.completed_chunks = s.completed_chunks.load(Ordering::Relaxed);
        if status.completed_chunks > 0
            && status.state == "converting"
            && s.ready.load(Ordering::Acquire)
        {
            status.conversion_ms =
                Some(s.conversion_micros.load(Ordering::Relaxed) as f64 / 1000.0);
            status.round_trip_ms =
                Some(s.round_trip_micros.load(Ordering::Relaxed) as f64 / 1000.0);
        }
        status
    }
    pub fn attach(&self, sample_rate: u32) -> RealtimePort {
        let (rt, worker) = transport::ports(sample_rate, self.shared.clone());
        if let Ok(mut pending) = self.attachment.lock() {
            *pending = Some(worker);
        }
        rt
    }
    pub fn begin_request(&mut self) -> u64 {
        self.shared.ai_selected.store(true, Ordering::Release);
        self.stop();
        self.shared.ready.store(false, Ordering::Release);
        self.shared.faulted.store(false, Ordering::Release);
        let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.update(|s| {
            s.state = "starting".into();
            s.message = None;
        });
        generation
    }
    pub fn reject_request(&self, message: String) {
        self.shared.ready.store(false, Ordering::Release);
        self.shared.faulted.store(true, Ordering::Release);
        self.update(|s| {
            s.state = "faulted".into();
            s.message = Some(message);
        });
    }
    pub fn save_config(&mut self, config: EngineConfig) -> Result<(), String> {
        if self.worker.as_ref().is_some_and(|w| !w.is_finished()) {
            return Err("Stop AI conversion before changing runtime settings".into());
        }
        config.save(&self.config_path)?;
        self.shared.classic_fallback.store(
            config.failure_policy == FailurePolicy::ClassicDsp,
            Ordering::Release,
        );
        self.config = config.clone();
        self.update(|s| s.config = config);
        Ok(())
    }
    pub fn start(
        &mut self,
        runtime: VerifiedRuntime,
        metadata: AIModelMetadata,
        path: PathBuf,
    ) -> Result<EngineStatus, String> {
        if self.installing.load(Ordering::Acquire) {
            return Err("Wait for runtime installation to finish".into());
        }
        self.config.validate()?;
        if metadata.backend != "w-okada-rvc"
            || metadata.architecture != "rvc"
            || !metadata.consent_confirmed
        {
            return Err("Choose a verified RVC ONNX model with acknowledged voice rights".into());
        }
        self.stop();
        self.stop = Arc::new(AtomicBool::new(false));
        self.hard_cancel = Arc::new(AtomicBool::new(false));
        self.shared.ready.store(false, Ordering::Release);
        self.shared.faulted.store(false, Ordering::Release);
        self.shared.ai_selected.store(true, Ordering::Release);
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        self.shared.classic_fallback.store(
            self.config.failure_policy == FailurePolicy::ClassicDsp,
            Ordering::Release,
        );
        self.shared.completed_chunks.store(0, Ordering::Relaxed);
        self.shared
            .dropped_input_samples
            .store(0, Ordering::Relaxed);
        self.shared
            .dropped_output_samples
            .store(0, Ordering::Relaxed);
        self.shared
            .missing_output_samples
            .store(0, Ordering::Relaxed);
        let id = metadata.id.clone();
        self.update(|s| {
            s.state = "starting".into();
            s.runtime_installed = true;
            s.model_id = Some(id);
            s.message = None;
            s.provider = None;
            s.restarts = 0;
            s.loading_ms = None;
        });
        let shared = self.shared.clone();
        let snapshot = self.snapshot.clone();
        let attachment = self.attachment.clone();
        let stop = self.stop.clone();
        let cancel = self.hard_cancel.clone();
        let config = self.config.clone();
        let root = self.runtime_root.clone();
        self.worker = Some(thread::spawn(move || {
            let session = root.join(format!("session-{}-{}", std::process::id(), now_ms()));
            let staged = (|| {
                fs::create_dir_all(session.join("0")).map_err(|e| e.to_string())?;
                let target = session.join("0").join(&metadata.model_file);
                let mut input = fs::File::open(&path).map_err(|e| e.to_string())?;
                let mut output = fs::File::create(&target).map_err(|e| e.to_string())?;
                let mut block = [0_u8; 65536];
                loop {
                    if stop.load(Ordering::Acquire) || cancel.load(Ordering::Acquire) {
                        return Err("Model staging cancelled".to_string());
                    }
                    let count = input.read(&mut block).map_err(|e| e.to_string())?;
                    if count == 0 {
                        break;
                    }
                    output
                        .write_all(&block[..count])
                        .map_err(|e| e.to_string())?;
                }
                drop(output);
                super::validator::verify_model(&metadata, &target)?;
                Ok::<_, String>(target)
            })();
            if let Ok(model_path) = staged.as_ref() {
                for attempt in 0..=2 {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    let generation = shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
                    let result = run_worker(
                        &runtime,
                        model_path,
                        &metadata,
                        &config,
                        &root.join("logs/ai-runtime.log"),
                        &shared,
                        &snapshot,
                        &attachment,
                        &stop,
                        cancel.clone(),
                        generation,
                    );
                    shared.ready.store(false, Ordering::Release);
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    if let Err(message) = result {
                        shared.faulted.store(true, Ordering::Release);
                        if let Ok(mut state) = snapshot.lock() {
                            state.state = "faulted".into();
                            state.message = Some(message);
                            state.restarts = attempt;
                        }
                    }
                    if attempt == 2 {
                        break;
                    }
                    thread::sleep(Duration::from_millis(200));
                    shared.faulted.store(false, Ordering::Release);
                    if let Ok(mut state) = snapshot.lock() {
                        state.state = "starting".into();
                        state.restarts = attempt + 1;
                    }
                }
            } else if let Err(error) = staged {
                shared.faulted.store(true, Ordering::Release);
                if let Ok(mut state) = snapshot.lock() {
                    state.state = "faulted".into();
                    state.message = Some(error);
                }
            }
            shared.ready.store(false, Ordering::Release);
            let _ = fs::remove_dir_all(session);
        }));
        Ok(self.status())
    }
    /// Stopping AI never silently returns to an unmodified/Classic voice.
    pub fn stop(&mut self) {
        // Explicit Stop always means mute, even if a prior failure allowed Classic DSP.
        self.shared.classic_fallback.store(false, Ordering::Release);
        self.shared.ready.store(false, Ordering::Release);
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_millis(500);
            while !worker.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            if !worker.is_finished() {
                self.hard_cancel.store(true, Ordering::Release);
            }
            let _ = worker.join();
        }
        let installed = self.runtime_root.join("current.json").is_file();
        self.shared.faulted.store(false, Ordering::Release);
        self.update(|s| {
            s.state = if installed { "stopped" } else { "notInstalled" }.into();
            s.message = None;
            s.provider = None;
        });
    }
    pub fn select_classic(&mut self) {
        self.stop();
        self.shared.ai_selected.store(false, Ordering::Release);
        self.shared.faulted.store(false, Ordering::Release);
    }
}
impl Drop for AIEngine {
    fn drop(&mut self) {
        self.install_cancel.store(true, Ordering::Release);
        self.stop();
    }
}

#[allow(clippy::too_many_arguments)]
fn run_worker(
    runtime: &VerifiedRuntime,
    model: &Path,
    metadata: &AIModelMetadata,
    config: &EngineConfig,
    log: &Path,
    shared: &Arc<SharedTransport>,
    snapshot: &Arc<Mutex<EngineStatus>>,
    attachment: &Arc<Mutex<Option<WorkerPort>>>,
    stop: &AtomicBool,
    cancel: Arc<AtomicBool>,
    generation: u64,
) -> Result<(), String> {
    let (mut backend, loaded) = WOkadaBackend::load(runtime, model, metadata, config, log, cancel)?;
    if let Ok(mut state) = snapshot.lock() {
        state.state = "ready".into();
        state.provider = loaded["provider"].as_str().map(str::to_owned);
        state.loading_ms = loaded["loadingMs"].as_f64();
        state.message = None;
    }
    shared.ready.store(true, Ordering::Release);
    let mut port: Option<WorkerPort> = None;
    let mut input_resampler = Resampler::new(AI_SAMPLE_RATE, AI_SAMPLE_RATE)?;
    let mut output_resampler = Resampler::new(AI_SAMPLE_RATE, AI_SAMPLE_RATE)?;
    let mut pending = Vec::<f32>::with_capacity(config.chunk_frames + 16);
    let mut output = Vec::<f32>::with_capacity(config.chunk_frames * 2);
    let mut health = Instant::now();
    let mut sent = 0;
    while !stop.load(Ordering::Acquire) {
        if let Some(next) = attachment
            .lock()
            .map_err(|_| "AI attachment lock poisoned")?
            .take()
        {
            input_resampler = Resampler::new(next.sample_rate, AI_SAMPLE_RATE)?;
            output_resampler = Resampler::new(AI_SAMPLE_RATE, next.sample_rate)?;
            pending.clear();
            port = Some(next);
        }
        if backend.exited() {
            return Err("AI sidecar exited unexpectedly".into());
        }
        if health.elapsed() > Duration::from_secs(2) {
            backend.health()?;
            health = Instant::now();
        }
        let Some(port) = port.as_mut() else {
            thread::sleep(Duration::from_millis(5));
            continue;
        };
        let available = port.input.slots();
        for _ in 0..available {
            let Ok(sample) = port.input.pop() else {
                break;
            };
            if sample.generation == generation {
                input_resampler.push(sample.value, &mut pending);
            }
            if pending.len() >= config.chunk_frames {
                break;
            }
        }
        shared
            .queued_input_samples
            .store(port.input.slots() as u64, Ordering::Relaxed);
        shared.queued_output_samples.store(
            (port.output.buffer().capacity() - port.output.slots()) as u64,
            Ordering::Relaxed,
        );
        if pending.len() < config.chunk_frames {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        let pcm: Vec<i16> = pending
            .drain(..config.chunk_frames)
            .map(transport::to_pcm16)
            .collect();
        let started = Instant::now();
        let response = backend.convert(&pcm)?;
        let conversion = response.inference_ms;
        shared.round_trip_micros.store(
            started.elapsed().as_micros().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        shared
            .conversion_micros
            .store((conversion * 1000.0) as u64, Ordering::Relaxed);
        output.clear();
        for sample in response.samples {
            output_resampler.push(f32::from(sample) / 32768.0, &mut output);
        }
        for value in output.iter().copied() {
            if port
                .output
                .push(TaggedSample { generation, value })
                .is_err()
            {
                shared
                    .dropped_output_samples
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        if response.warming_up {
            continue;
        }
        shared.completed_chunks.fetch_add(1, Ordering::Relaxed);
        sent += 1;
        if sent == 1 {
            if let Ok(mut state) = snapshot.lock() {
                state.state = "converting".into();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_stop_overrides_opted_in_fallback_until_classic_is_selected() {
        let mut engine = AIEngine::default();
        let mut port = engine.attach(48000);
        engine.shared.ai_selected.store(true, Ordering::Release);
        engine.shared.faulted.store(true, Ordering::Release);
        engine
            .shared
            .classic_fallback
            .store(true, Ordering::Release);
        assert!(port.process(0.8).is_none());
        assert!(engine.status().classic_fallback_active);
        engine.stop();
        assert!(!engine.status().classic_fallback_active);
        assert_eq!(port.process(0.8), Some(0.0));
        engine.select_classic();
        assert!(port.process(0.8).is_none());
    }
    #[test]
    fn cancelling_a_pending_start_invalidates_its_generation() {
        let mut engine = AIEngine::default();
        let generation = engine.begin_request();
        engine.stop();
        assert_ne!(generation, engine.shared.generation.load(Ordering::Acquire));
        assert_eq!(engine.status().mode, "wOkada");
        assert_eq!(engine.status().config.failure_policy, FailurePolicy::Mute);
    }
}
