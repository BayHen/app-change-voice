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
    pub hardware: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub supported_f0_methods: Vec<String>,
    pub runtime_version: Option<String>,
    pub recommended_runtime_version: String,
    pub recommended_download_bytes: u64,
    pub download: Option<super::bootstrap::Progress>,
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
    pub missed_deadlines: u64,
    pub startup_wait_samples: u64,
    pub inference_budget_ms: Option<f64>,
    pub restarts: u32,
    pub message: Option<String>,
    pub config: EngineConfig,
    pub install: RuntimeInstallStatus,
}
impl Default for EngineStatus {
    fn default() -> Self {
        let recipe = super::bootstrap::recipe().ok();
        Self {
            state: "notInstalled".into(),
            mode: "classicDsp".into(),
            classic_fallback_active: false,
            hardware: None,
            resources: None,
            supported_f0_methods: vec!["dio".into(), "harvest".into()],
            runtime_version: None,
            recommended_runtime_version: recipe
                .as_ref()
                .map(|r| r.version.clone())
                .unwrap_or_default(),
            recommended_download_bytes: recipe
                .as_ref()
                .map(|r| r.components.iter().map(|c| c.bytes).sum())
                .unwrap_or(0),
            download: None,
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
            missed_deadlines: 0,
            startup_wait_samples: 0,
            inference_budget_ms: None,
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
        self.shared
            .hybrid_dsp
            .store(self.config.hybrid_dsp, Ordering::Release);
        let manifest = super::runtime::installed_manifest(&self.runtime_root).ok();
        let installed = manifest.is_some();
        let needs_adapter_update = !installed && self.runtime_root.join("current.json").exists();
        let config = self.config.clone();
        self.update(|s| {
            s.config = config;
            s.runtime_installed = installed;
            s.supported_f0_methods=vec!["dio".into(),"harvest".into()];
            if manifest.as_ref().is_some_and(|m|m.rmvpe_model.is_some()) {s.supported_f0_methods.push("rmvpe_onnx".into());}
            s.runtime_version = manifest.as_ref().map(|m| m.version.clone());
            s.install.installed = installed;
            s.install.version = s.runtime_version.clone();
            s.state = if installed { "stopped" } else { "notInstalled" }.into();
            if needs_adapter_update {s.message=Some("Installed AI adapter needs an update for PCM safety. Install the recommended runtime; verified artifact cache is reused.".into());}
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
        status.startup_wait_samples = s.startup_wait_samples.load(Ordering::Relaxed);
        status.inference_budget_ms = status
            .model_id
            .as_ref()
            .map(|_| status.config.chunk_frames as f64 * 1000.0 / f64::from(AI_SAMPLE_RATE));
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
        self.attach_with_diagnostics(sample_rate, None)
    }
    pub fn attach_with_diagnostics(
        &self,
        sample_rate: u32,
        diagnostics: Option<crate::audio::capture::Tap>,
    ) -> RealtimePort {
        let (rt, mut worker) = transport::ports(sample_rate, self.shared.clone());
        worker.diagnostics = diagnostics;
        if let Ok(mut pending) = self.attachment.lock() {
            *pending = Some(worker);
        }
        rt
    }
    pub fn begin_request(&mut self, model_id: &str) -> u64 {
        self.shared.ai_selected.store(true, Ordering::Release);
        self.stop();
        self.shared.ready.store(false, Ordering::Release);
        self.shared.faulted.store(false, Ordering::Release);
        let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.update(|s| {
            s.state = "starting".into();
            s.resources = None;
            s.model_id = Some(model_id.to_owned());
            s.message = Some("Verifying the local AI runtime…".into());
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
        self.shared
            .hybrid_dsp
            .store(config.hybrid_dsp, Ordering::Release);
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
            s.resources = None;
            s.runtime_installed = true;
            s.model_id = Some(id);
            s.message = Some("Loading the neural model…".into());
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
                super::original::stage_index(
                    &path,
                    &target,
                    metadata.feature_index_sha256.as_deref(),
                )?;
                Ok::<_, String>(target)
            })();
            // Ring endpoints belong to the audio stream, not to one sidecar process.
            // Retain them across crashes and return them for an explicit Stop/Start.
            let mut port: Option<WorkerPort> = None;
            if let Ok(model_path) = staged.as_ref() {
                for attempt in 0..=2 {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    let generation = shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
                    let mut attempt_config = config.clone();
                    if attempt > 0
                        && attempt_config.allow_cpu_fallback
                        && attempt_config.provider != "cpu"
                    {
                        attempt_config.provider = "cpu".into();
                    }
                    let result = run_worker(
                        &runtime,
                        model_path,
                        &metadata,
                        &attempt_config,
                        &root.join("logs/ai-runtime.log"),
                        &shared,
                        &snapshot,
                        &attachment,
                        &mut port,
                        &stop,
                        cancel.clone(),
                        generation,
                    );
                    shared.process_id.store(0, Ordering::Release);
                    shared.ready.store(false, Ordering::Release);
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    let budget_failure = result
                        .as_ref()
                        .err()
                        .is_some_and(|message| message.starts_with("Real-time budget:"));
                    if let Err(message) = result {
                        eprintln!("AI engine ({}, attempt {}): {message}", metadata.id, attempt + 1);
                        shared.faulted.store(true, Ordering::Release);
                        if let Ok(mut state) = snapshot.lock() {
                            state.state = "faulted".into();
                            state.message = Some(message);
                            state.restarts = attempt;
                        }
                    }
                    if attempt == 2 || budget_failure {
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
                eprintln!("AI engine ({}): model staging failed: {error}", metadata.id);
                shared.faulted.store(true, Ordering::Release);
                if let Ok(mut state) = snapshot.lock() {
                    state.state = "faulted".into();
                    state.message = Some(error);
                }
            }
            shared.ready.store(false, Ordering::Release);
            if let Ok(mut pending) = attachment.lock() {
                if pending.is_none() {
                    *pending = port.take();
                }
            }
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
        self.shared.process_id.store(0, Ordering::Release);
        self.shared.faulted.store(false, Ordering::Release);
        self.update(|s| {
            s.state = if installed { "stopped" } else { "notInstalled" }.into();
            s.message = None;
            s.provider = None;
        });
    }
    pub fn select_classic(&mut self) {
        if !self.shared.ai_selected.load(Ordering::Acquire) {
            return;
        }
        self.stop();
        self.shared.ai_selected.store(false, Ordering::Release);
        // Flush output once more at the mode boundary, including audio queued
        // while the previous worker was shutting down.
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        self.update(|state| state.model_id = None);
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
    port: &mut Option<WorkerPort>,
    stop: &AtomicBool,
    cancel: Arc<AtomicBool>,
    generation: u64,
) -> Result<(), String> {
    let (mut backend, loaded) = WOkadaBackend::load(runtime, model, metadata, config, log, cancel)?;
    shared.process_id.store(
        u64::from(backend.process_id().unwrap_or(0)),
        Ordering::Release,
    );
    if let Ok(mut state) = snapshot.lock() {
        state.message = Some("Preparing the audio engine…".into());
    }
    let warmup_ms = super::warmup::warm_up(&mut backend, config, stop)?;
    if let Ok(mut state) = snapshot.lock() {
        state.state = "ready".into();
        state.provider = loaded["provider"].as_str().map(str::to_owned);
        state.hardware = Some(loaded["hardware"].clone());
        state.loading_ms = loaded["loadingMs"].as_f64().map(|ms| ms + warmup_ms);
        state.message = if state.provider.as_deref() == Some("cpu")
            && state.config.provider != "cpu"
            && (state.restarts > 0 || state.config.provider == "cuda")
        {
            Some("CPU fallback is active under your saved policy. Larger chunks can help if output is missing.".into())
        } else {
            None
        };
    }
    shared.ready.store(true, Ordering::Release);
    let native_rate = port
        .as_ref()
        .map(|p| p.sample_rate)
        .unwrap_or(AI_SAMPLE_RATE);
    shared.output_prefill_frames.store(
        (config.chunk_frames as u64 * 2 * u64::from(native_rate)) / u64::from(AI_SAMPLE_RATE),
        Ordering::Release,
    );
    let mut input_resampler = Resampler::new(native_rate, AI_SAMPLE_RATE)?;
    let mut output_resampler = Resampler::new(AI_SAMPLE_RATE, native_rate)?;
    let mut pending = Vec::<f32>::with_capacity(config.chunk_frames + 16);
    let mut output = Vec::<f32>::with_capacity(config.chunk_frames * 2);
    let mut health = Instant::now();
    let mut sent = 0;
    let mut consecutive_over_budget = 0_u32;
    while !stop.load(Ordering::Acquire) {
        if let Some(next) = attachment
            .lock()
            .map_err(|_| "AI attachment lock poisoned")?
            .take()
        {
            input_resampler = Resampler::new(next.sample_rate, AI_SAMPLE_RATE)?;
            shared.output_prefill_frames.store(
                (config.chunk_frames as u64 * 2 * u64::from(next.sample_rate))
                    / u64::from(AI_SAMPLE_RATE),
                Ordering::Release,
            );
            output_resampler = Resampler::new(AI_SAMPLE_RATE, next.sample_rate)?;
            pending.clear();
            *port = Some(next);
            sent = 0;
            if let Ok(mut state) = snapshot.lock() {
                state.state = "ready".into();
            }
        }
        if port.as_ref().is_some_and(|p| p.input.is_abandoned()) {
            *port = None;
            pending.clear();
            sent = 0;
            shared.queued_input_samples.store(0, Ordering::Relaxed);
            shared.queued_output_samples.store(0, Ordering::Relaxed);
            if let Ok(mut state) = snapshot.lock() {
                state.state = "ready".into();
            }
        }
        if backend.exited() {
            return Err("AI sidecar exited unexpectedly".into());
        }
        if health.elapsed() > Duration::from_secs(2) {
            backend.health()?;
            if let Ok(resources) = backend.resource_metrics() {
                if let Ok(mut state) = snapshot.lock() {
                    state.resources = resources;
                }
            }
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
        if !response.warming_up {
            let over = started.elapsed().as_secs_f64()
                > config.chunk_frames as f64 / f64::from(AI_SAMPLE_RATE);
            consecutive_over_budget = if over { consecutive_over_budget + 1 } else { 0 };
            if over {
                if let Ok(mut state) = snapshot.lock() {
                    state.missed_deadlines += 1;
                    if consecutive_over_budget >= 4 {
                        state.message=Some("AI inference exceeds its real-time budget. Increase chunk size, use CUDA or select Classic DSP.".into());
                    }
                }
            }
            if consecutive_over_budget >= 16
                && port.input.slots() > port.input.buffer().capacity() / 2
            {
                return Err("Real-time budget: AI cannot keep up with audio input. Voice follows the explicit failure policy; choose a larger chunk/CUDA or Classic DSP.".into());
            }
        }
        for sample in response.samples {
            if let Some(tap) = port.diagnostics.as_mut() {
                tap.push(
                    crate::audio::capture::Stage::AiInference,
                    AI_SAMPLE_RATE,
                    f32::from(sample) / 32768.0,
                );
            }
            output_resampler.push(f32::from(sample) / 32768.0, &mut output);
        }
        for value in output.iter().copied() {
            if let Some(tap) = port.diagnostics.as_mut() {
                tap.push(
                    crate::audio::capture::Stage::PcmBridge,
                    port.sample_rate,
                    value,
                );
            }
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
    fn feed_file_audio(port: &mut RealtimePort, source: &[f32], seconds: u64) -> usize {
        let started = Instant::now();
        let mut audible = 0;
        for callback in 0..seconds * 100 {
            for offset in 0..480 {
                let value =
                    port.process(source[((callback * 480) as usize + offset) % source.len()]);
                assert!(value.is_some(), "AI unexpectedly selected Classic DSP");
                if value.unwrap().abs() > 0.0001 {
                    audible += 1;
                }
            }
            let target = started + Duration::from_millis((callback + 1) * 10);
            if let Some(delay) = target.checked_duration_since(Instant::now()) {
                thread::sleep(delay);
            }
        }
        audible
    }
    fn wait_provider(engine: &AIEngine, provider: &str) {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let status = engine.status();
            if matches!(status.state.as_str(), "ready" | "converting")
                && status.provider.as_deref() == Some(provider)
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "Runtime did not become ready: {status:?}"
            );
            assert!(
                status.state != "faulted" || status.restarts < 2,
                "Actual runtime failed: {status:?}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
    #[cfg(windows)]
    fn terminate_owned_child(pid: u32, executable: &Path) {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
                QueryFullProcessImageNameW, TerminateProcess,
            },
        };
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                0,
                pid,
            )
        };
        assert!(!handle.is_null(), "Cannot open owned runtime child");
        let mut path = vec![0u16; 32768];
        let mut size = path.len() as u32;
        let queried =
            unsafe { QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut size) };
        if queried == 0 {
            unsafe {
                CloseHandle(handle);
            }
            panic!("Cannot verify child executable");
        }
        let actual = PathBuf::from(std::ffi::OsString::from_wide(&path[..size as usize]))
            .canonicalize()
            .unwrap();
        let expected = executable.canonicalize().unwrap();
        if actual != expected {
            unsafe {
                CloseHandle(handle);
            }
            panic!("Refusing to terminate an unrelated process");
        }
        let killed = unsafe { TerminateProcess(handle, 77) };
        unsafe {
            CloseHandle(handle);
        }
        assert_ne!(killed, 0, "Owned runtime termination failed");
    }
    #[test]
    #[cfg(windows)]
    #[ignore = "manual actual CUDA inference, bounded PCM transport and owned-child crash recovery; no audio devices"]
    fn real_transport_recovers_from_cuda_child_exit_to_opted_in_cpu() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let root = project.join(".ai-runtime-build/bootstrap-installed");
        let runtime = super::super::runtime::discover(&root).unwrap();
        let executable = runtime.root.join(&runtime.manifest.python_executable);
        let manager =
            super::super::manager::AIModelManager::new(root.join("test-model-library")).unwrap();
        let (model, path) = manager.resolve(super::super::catalog::VOICE_ID).unwrap();
        let samples: Vec<f32> =
            hound::WavReader::open(project.join(".ai-runtime-build/assets/owner-sample.wav"))
                .unwrap()
                .samples::<i16>()
                .map(|s| f32::from(s.unwrap()) / 32768.0)
                .collect();
        let mut engine = AIEngine::default();
        // A fresh fixture avoids replacing a config owned by another Windows test identity.
        engine.configure(
            root.clone(),
            root.join(format!(
                "transport-test-{}-{}.json",
                std::process::id(),
                super::super::process::now_ms()
            )),
        );
        engine
            .save_config(EngineConfig {
                provider: "cuda".into(),
                allow_cpu_fallback: true,
                chunk_frames: 8192,
                ..EngineConfig::default()
            })
            .unwrap();
        let mut port = engine.attach(48000);
        engine
            .start(runtime.clone(), model.clone(), path.clone())
            .unwrap();
        wait_provider(&engine, "cuda");
        let cuda_audible = feed_file_audio(&mut port, &samples, 6);
        assert!(
            cuda_audible > 0,
            "No converted CUDA samples reached the Rust real-time boundary"
        );
        assert!(engine.status().completed_chunks > 0);
        let pid = engine.shared.process_id.load(Ordering::Acquire) as u32;
        assert!(pid > 0);
        terminate_owned_child(pid, &executable);
        wait_provider(&engine, "cpu");
        let cpu_audible = feed_file_audio(&mut port, &samples, 6);
        assert!(
            cpu_audible > 0,
            "No real CPU conversion after controlled CUDA-child crash"
        );
        let final_status = engine.status();
        assert!(final_status.restarts >= 1);
        engine.stop();
        for _ in 0..1024 {
            assert_eq!(port.process(0.9), Some(0.0));
        }
        engine
            .save_config(EngineConfig {
                provider: "cpu".into(),
                chunk_frames: 8192,
                ..EngineConfig::default()
            })
            .unwrap();
        engine.start(runtime, model, path).unwrap();
        wait_provider(&engine, "cpu");
        assert!(
            feed_file_audio(&mut port, &samples, 3) > 0,
            "Stop/Start lost the original audio stream endpoint"
        );
        engine.select_classic();
        assert!(port.process(0.9).is_none());
        println!(
            "Actual bounded transport: CUDA audible samples {cuda_audible}; CPU audible samples {cpu_audible}; restarts {}; completed chunks {}. No devices or microphone were opened.",
            final_status.restarts, final_status.completed_chunks
        );
    }
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
        let generation = engine.begin_request("test-model");
        engine.stop();
        assert_ne!(generation, engine.shared.generation.load(Ordering::Acquire));
        assert_eq!(engine.status().mode, "wOkada");
        assert_eq!(engine.status().config.failure_policy, FailurePolicy::Mute);
    }
}
