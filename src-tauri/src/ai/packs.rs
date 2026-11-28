//! Voice-pack jobs: bounded metadata lookup, resumable downloads, isolated conversion,
//! real inference validation, then model registration and Agent assignment.
use super::{
    agents::{AgentLibrary, Binding},
    bootstrap::{self, Component},
    config::EngineConfig,
    import::ImportRequest,
    manager::AIModelManager,
    models::AIModelMetadata,
    process::ManagedProcess,
    runtime,
    validator::sha256_file,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteArtifact {
    pub repository: String,
    pub revision: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub declared_license: Option<String>,
    pub state: String,
    pub download_url: String,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackStatus {
    pub phase: String,
    pub agent_id: Option<String>,
    pub model_id: Option<String>,
    pub message: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub detected: Option<Value>,
    pub validation: Option<Receipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub model_sha256: String,
    pub runtime_digest: String,
    pub provider: String,
    pub peak: i32,
    pub chunks: usize,
    pub inference_ms: f64,
    pub real_time_factor: f64,
    pub config: EngineConfig,
    pub source_sha256: String,
    pub detected: Value,
}
#[derive(Clone)]
pub struct PackManager {
    pub root: PathBuf,
    pub status: Arc<Mutex<PackStatus>>,
    pub cancel: Arc<AtomicBool>,
    pub busy: Arc<AtomicBool>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRequest {
    pub agent_id: String,
    pub local_path: Option<PathBuf>,
    pub hf_url: Option<String>,
    pub existing_model_id: Option<String>,
    pub index_path: Option<PathBuf>,
    pub identity_name: Option<String>,
    pub source_url: Option<String>,
    pub license_url: Option<String>,
    pub attribution: Option<String>,
    pub rights_confirmed: bool,
}
impl PackManager {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            status: Arc::new(Mutex::new(PackStatus {
                phase: "idle".into(),
                ..Default::default()
            })),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn snapshot(&self) -> PackStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
    pub fn phase(&self, phase: &str, message: &str) {
        if let Ok(mut s) = self.status.lock() {
            s.phase = phase.into();
            s.message = message.into();
        }
    }
    pub fn begin(&self, request: &InstallRequest) -> Result<(), String> {
        let source_count = [
            request.local_path.is_some(),
            request.hf_url.is_some(),
            request.existing_model_id.is_some(),
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count();
        if source_count != 1 {
            return Err("Choose exactly one source for the voice pack.".into());
        }
        super::agents::check_id(&request.agent_id)?;
        let mut status = self.status.lock().map_err(|_| "Pack status lock")?;
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("Another voice-pack operation is running. Wait or cancel it.".into());
        }
        self.cancel.store(false, Ordering::Release);
        *status = PackStatus {
            phase: "preparing".into(),
            agent_id: Some(request.agent_id.clone()),
            message: "Preparing voice pack; no model is active yet".into(),
            ..Default::default()
        };
        Ok(())
    }
    pub fn request_cancel(&self) -> Result<(), String> {
        let status = self.status.lock().map_err(|_| "Pack status lock")?;
        if status.phase == "assigning" {
            return Err("Assignment is finishing. Wait for it to complete.".into());
        }
        if self.busy.load(Ordering::Acquire) {
            self.cancel.store(true, Ordering::Release);
        }
        Ok(())
    }
    pub fn finish(&self, result: Result<String, String>) {
        match result {
            Ok(id) => {
                if let Ok(mut s) = self.status.lock() {
                    s.phase = "ready".into();
                    s.model_id = Some(id);
                    s.message="Neural inference validated and Agent assigned. Listening validation is still required.".into();
                }
            }
            Err(e) => {
                let phase = if self.cancel.load(Ordering::Acquire) {
                    "cancelled"
                } else if e.starts_with("License Unverified") {
                    "licenseUnverified"
                } else if e.starts_with("Runtime required") {
                    "runtimeRequired"
                } else if e.starts_with("Incompatible") {
                    "incompatible"
                } else {
                    "error"
                };
                self.phase(phase, &e);
            }
        }
        self.busy.store(false, Ordering::Release);
    }
    fn check(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Acquire) {
            Err("Voice-pack job cancelled; verified download cache retained".into())
        } else {
            Ok(())
        }
    }
    pub fn receipt(&self, sha: &str, runtime_digest: &str) -> Option<Receipt> {
        if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let path = self.root.join("receipts").join(format!("{sha}.json"));
        if path.metadata().ok()?.len() > 65536 {
            return None;
        }
        let r: Receipt = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        (r.model_sha256 == sha
            && r.runtime_digest == runtime_digest
            && r.peak > 8
            && r.chunks > 0
            && r.config.validate().is_ok())
        .then_some(r)
    }
    pub fn install(
        &self,
        mut req: InstallRequest,
        runtime_root: &Path,
        base: &EngineConfig,
        models: &Mutex<AIModelManager>,
        agents: &Mutex<AgentLibrary>,
    ) -> Result<String, String> {
        self.check()?;
        let original = if super::agents::original_name(&req.agent_id).is_some() {
            if !req.rights_confirmed {
                return Err(
                    "License Unverified: confirm consent to use the original voice pack".into(),
                );
            }
            let path = req
                .local_path
                .as_deref()
                .ok_or("Select a local original voice pack manifest (.json)")?;
            let pack = super::original::OriginalPack::read(path, &req.agent_id)?;
            let root = path.parent().ok_or("Missing voice pack directory")?;
            let onnx =
                super::original::verify_artifact(root, &pack.onnx, "onnx", 512 * 1024 * 1024)?;
            super::original::verify_artifact(root, &pack.checkpoint, "pth", 512 * 1024 * 1024)?;
            let index = super::original::verify_artifact(
                root,
                &pack.feature_index,
                "index",
                super::original::MAX_INDEX_BYTES,
            )?;
            if index != onnx.with_extension("index") {
                return Err(
                    "Place the feature index beside the ONNX with the same filename stem".into(),
                );
            }
            super::original::verify_index(&index, &pack.feature_index.sha256)?;
            reject_unlicensed_source(&pack.checkpoint.sha256)?;
            if pack.checkpoint.sha256
                == "cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020"
                || pack.onnx.sha256 == super::catalog::ONNX_SHA256
            {
                return Err("Original profiles require their own trained voice; Tsukuyomi belongs in the general AI library".into());
            }
            req.local_path = Some(onnx);
            req.identity_name = Some(if pack.pretrained_source.is_some() {
                format!(
                    "{} ({}, pretrained)",
                    super::agents::original_name(&req.agent_id).unwrap(),
                    pack.speaker_id
                )
            } else {
                super::agents::original_name(&req.agent_id).unwrap().into()
            });
            req.source_url = Some(pack.source_url.clone());
            req.license_url = Some(pack.license_url.clone());
            req.attribution = Some(pack.attribution.clone());
            Some(pack)
        } else {
            None
        };
        if req.index_path.is_some() {
            return Err("Incompatible optional index: FAISS retrieval is not qualified in this runtime adapter. Install without an index; Index Rate stays unavailable.".into());
        }
        let existing = if let Some(id) = &req.existing_model_id {
            Some(models.lock().map_err(|_| "Model lock")?.resolve(id)?)
        } else {
            None
        };
        if let Some((m, _)) = &existing {
            if !m.consent_confirmed {
                return Err(
                    "License Unverified: existing model has no voice-rights confirmation".into(),
                );
            }
            req.rights_confirmed = true;
            req.identity_name = Some(m.name.clone());
            req.source_url = Some(m.source_url.clone());
            req.license_url = m.license_url.clone();
            req.attribution = m.attribution.clone();
        }
        if !req.rights_confirmed {
            return Err(
                "License Unverified: confirm both model licensing and target voice rights".into(),
            );
        }
        let remote = if let Some(url) = &req.hf_url {
            self.phase(
                "checkingLicense",
                "Reading repository metadata; weights are not downloaded yet",
            );
            let artifact = inspect_hf(url)?;
            reject_unlicensed_source(&artifact.sha256)?;
            if artifact.state != "declaredLicense" {
                return Err("License Unverified: repository does not declare a supported model license. No weights were downloaded. Obtain model and voice-owner authorization or use a licensed local model.".into());
            }
            Some(artifact)
        } else {
            None
        };
        let source = if let Some((_, path)) = &existing {
            path.clone()
        } else if let Some(a) = &remote {
            if !req
                .license_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://"))
                || !req
                    .attribution
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
            {
                return Err("License Unverified: supply model terms/voice-authorization evidence URL and credited voice owner before downloading any weights".into());
            }
            if req.source_url.is_none() {
                req.source_url = Some(format!("https://huggingface.co/{}", a.repository));
            }
            check_space(
                &self.root,
                a.bytes.saturating_mul(6).saturating_add(1024 * 1024 * 1024),
            )?;
            self.phase(
                "downloading",
                "Downloading revision-pinned artifact; cache resumes partial transfers",
            );
            if let Ok(mut s) = self.status.lock() {
                s.total_bytes = a.bytes;
            }
            let cached = bootstrap::download(
                &Component {
                    name: a.filename.clone(),
                    url: a.download_url.clone(),
                    sha256: a.sha256.clone(),
                    bytes: a.bytes,
                    kind: "voice-pack".into(),
                },
                &self.root.join("downloads"),
                &self.cancel,
                |n| {
                    if let Ok(mut s) = self.status.lock() {
                        s.completed_bytes = n;
                    }
                },
            )?;
            let dir = self.root.join("sources").join(&a.sha256);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let name = Path::new(&a.filename)
                .file_name()
                .ok_or("Missing model filename")?;
            let target = dir.join(name);
            fs::copy(cached, &target).map_err(|e| e.to_string())?;
            target
        } else {
            req.local_path
                .clone()
                .ok_or("Choose a local PTH/ONNX, existing model, or Hugging Face file URL")?
        };
        self.check()?;
        let source_sha = sha256_file(&source)?;
        reject_unlicensed_source(&source_sha)?;
        let size = source.metadata().map_err(|e| e.to_string())?.len();
        if size == 0 || size > 512 * 1024 * 1024 {
            return Err("Model must be a non-empty file no larger than 512 MiB".into());
        }
        check_space(
            &self.root,
            size.saturating_mul(6).saturating_add(1024 * 1024 * 1024),
        )?;
        if source_sha == "cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020"
            || source_sha == super::catalog::ONNX_SHA256
        {
            let owner = super::catalog::metadata();
            req.identity_name = Some(owner.name);
            req.source_url = Some(owner.source_url);
            req.license_url = owner.license_url;
            req.attribution = owner.attribution;
        }
        let source_url = req
            .source_url
            .clone()
            .filter(|s| s.starts_with("https://"))
            .ok_or("Provide an HTTPS model source/owner URL")?;
        let license_url = req
            .license_url
            .clone()
            .filter(|s| s.starts_with("https://"))
            .ok_or("License Unverified: provide an HTTPS license/voice-permission URL")?;
        let attribution = req
            .attribution
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or("License Unverified: provide target voice-owner attribution")?;
        self.phase(
            "checkingRuntime",
            "Verifying the shared managed runtime and auxiliary models",
        );
        let runtime = runtime::discover(runtime_root).map_err(|e| {
            format!(
                "Runtime required: install or repair the managed runtime in AI Voice Studio. {e}"
            )
        })?;
        if let Some(pack) = &original {
            runtime.require_feature_encoder(&pack.embedder_sha256)?;
        }
        let script_hash = format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../../ai-sidecar/convert_worker.py"))
        );
        let work = self.root.join("conversion-cache").join(format!(
            "{}-{}-{}",
            source_sha,
            &runtime.digest[..12],
            &script_hash[..12]
        ));
        fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let extension = source
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (onnx,mut detected)=match extension.as_str(){"onnx"=>{self.phase("validating","Inspecting ONNX tensors and embedded metadata");let info=super::onnx::inspect(&source)?;(source.clone(),json!({"metadata":info.metadata,"precision":info.inputs.first().map(|t|t.element_type)}))},"pth"=>{
   self.phase("converting","Loading data-only weights and converting through the pinned RVC exporter");let checkpoint=work.join("source.pth");fs::copy(&source,&checkpoint).map_err(|e|e.to_string())?;if sha256_file(&checkpoint)?!=source_sha{return Err("Source checkpoint changed while copying".into());}
   let cache_meta=work.join("converted.json");let cache=fs::read(&cache_meta).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok());
   if let Some(value)=cache.filter(|v|v["sha256"].as_str().is_some_and(|sha|sha256_file(&work.join("voice.onnx")).ok().as_deref()==Some(sha))){(work.join("voice.onnx"),value)}else{
    self.phase("installingConverter","Preparing the separately pinned CPU conversion runtime");
    let safe_torch=super::converter_runtime::ensure(&self.root,&self.cancel,|name,n,total|{if let Ok(mut s)=self.status.lock(){s.message=format!("Downloading {name}");s.completed_bytes=n;s.total_bytes=total;}})?;
    self.phase("converting","Exporting restricted RVC weights with the qualified CPU converter");
    let mut child=ManagedProcess::spawn_converter(&runtime,&work,self.cancel.clone(),&safe_torch)?;
    let value=child.call(json!({"op":"convert","checkpointPath":checkpoint,"sha256":source_sha,"outputDirectory":work}),Duration::from_secs(180))?;
    self.check()?;let target=work.join("voice.onnx");if value["sha256"]!=sha256_file(&target)?{return Err("Converted ONNX checksum mismatch".into());}
    fs::write(cache_meta,serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;(target,value)
   }
  },_=>return Err("Incompatible model format: choose an RVC .pth or .onnx file; archives and arbitrary Python code are rejected".into())};
        self.check()?;
        self.phase(
            "validating",
            "Validating the converted ONNX contract before any registration",
        );
        let info = super::onnx::inspect(&onnx)?;
        if let Some(raw) = info.metadata.get("metadata") {
            if let Ok(metadata) = serde_json::from_str::<Value>(raw) {
                if let Some(hash) = metadata["sourceCheckpointSha256"].as_str() {
                    reject_unlicensed_source(hash)?;
                }
            }
        }
        if info.compatibility != "candidate" {
            return Err(format!("Incompatible model: {}", info.issues.join(" ")));
        }
        if let Some(pack) = &original {
            let metadata: Value = info
                .metadata
                .get("metadata")
                .and_then(|value| serde_json::from_str(value).ok())
                .ok_or("Incompatible original pack: missing ONNX metadata")?;
            if info.sha256 != pack.onnx.sha256
                || metadata["samplingRate"] != pack.sample_rate
                || metadata["embChannels"] != 768
                || metadata["f0"] != true
                || metadata["speakerCount"] != 1
                || metadata["embedder"] != "hubert_base"
                || metadata["sourceCheckpointSha256"] != pack.checkpoint.sha256
            {
                return Err("Incompatible original pack: ONNX provenance or RVC v2 tensor configuration disagrees with manifest".into());
            }
            let records = self.root.join("original-provenance");
            fs::create_dir_all(&records).map_err(|e| e.to_string())?;
            for (id, _) in super::agents::ORIGINAL_AGENTS {
                let path = records.join(format!("{id}.json"));
                if id != req.agent_id
                    && path.exists()
                    && pack.same_training_identity(&super::original::OriginalPack::read(&path, id)?)
                {
                    return Err("Original profiles require independent speakers, datasets, checkpoints and feature indexes".into());
                }
            }
            detected["originalPack"] = serde_json::to_value(pack).map_err(|e| e.to_string())?;
        }
        if let Ok(mut s) = self.status.lock() {
            s.detected = Some(detected.clone());
        }
        let name = req
            .identity_name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                source
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        let mut config = super::agents::quality_config(base, "highQuality")?;
        let uses_rvc_contentvec = original
            .as_ref()
            .is_some_and(|pack| pack.embedder_sha256 == runtime::RVC_CONTENTVEC_SHA256);
        if original.is_some() && runtime.manifest.rmvpe_model.is_some() {
            config.f0_method = "rmvpe_onnx".into();
        }
        if original
            .as_ref()
            .is_some_and(|pack| pack.pretrained_source.is_some())
        {
            if !uses_rvc_contentvec {
                config = super::agents::quality_config(&config, "balanced")?;
            }
            config.index_rate = Some(if uses_rvc_contentvec { 0.0 } else { 0.5 });
            config.protect = Some(0.33);
        }
        config.pitch_semitones = 0;
        config.hybrid_dsp = false;
        let candidate = AIModelMetadata {
            schema_version: 1,
            id: format!("candidate-{}", &info.sha256[..16]),
            name: name.clone(),
            description: "Pending real neural validation".into(),
            architecture: "rvc".into(),
            backend: "w-okada-rvc".into(),
            license: "LicenseRef-UserConfirmed".into(),
            source_url: source_url.clone(),
            model_file: onnx.file_name().unwrap().to_string_lossy().into_owned(),
            sha256: info.sha256.clone(),
            input_sample_rate: 48000,
            frame_samples: config.chunk_frames,
            supported_profiles: vec![],
            consent_confirmed: true,
            license_url: Some(license_url.clone()),
            attribution: Some(attribution.clone()),
            feature_index_sha256: original.as_ref().map(|p| p.feature_index.sha256.clone()),
            feature_encoder_sha256: original.as_ref().map(|p| p.embedder_sha256.clone()),
        };
        self.phase(
            "testingInference",
            "Running actual neural warm-up and non-silent inference; no audio devices are opened",
        );
        let receipt = validate_inference(
            &runtime,
            &onnx,
            &candidate,
            &config,
            &work,
            self.cancel.clone(),
            source_sha,
            detected,
        )?;
        self.check()?;
        let mut library = models.lock().map_err(|_| "Model lock")?;
        let model_id = if let Some(existing) = library
            .list()
            .into_iter()
            .find(|m| m.metadata.sha256 == info.sha256)
        {
            if existing.metadata.feature_index_sha256 != candidate.feature_index_sha256
                || existing.metadata.feature_encoder_sha256 != candidate.feature_encoder_sha256
            {
                return Err("This model is already registered with different encoder or index metadata. Remove it before reimporting the corrected pack.".into());
            }
            existing.metadata.id
        } else {
            self.phase(
                "registering",
                "Registering the verified ONNX and its model license record",
            );
            super::import::register_with_index(
                &mut library,
                &onnx,
                ImportRequest {
                    name: name.clone(),
                    source_url,
                    license_url,
                    attribution,
                    rights_confirmed: true,
                    expected_sha256: info.sha256.clone(),
                },
                candidate.feature_index_sha256.clone(),
                candidate.feature_encoder_sha256.clone(),
            )?
            .metadata
            .id
        };
        {
            let mut status = self.status.lock().map_err(|_| "Pack status lock")?;
            self.check()?;
            status.phase = "assigning".into();
            status.message =
                "Assigning the independent validated identity to the selected Agent".into();
        }
        // Assignment failures never mark a pack Ready or bind the same identity to another slot.
        agents.lock().map_err(|_| "Agent lock")?.assign(
            &req.agent_id,
            Binding {
                weights_sha256: None,
                model_id: model_id.clone(),
                sha256: info.sha256.clone(),
                identity_name: name,
                rights_confirmed: true,
                quality_mode: if !uses_rvc_contentvec && original
                    .as_ref()
                    .is_some_and(|pack| pack.pretrained_source.is_some())
                {
                    "balanced".into()
                } else {
                    "highQuality".into()
                },
            },
            &library,
        )?;
        let dir = self.root.join("receipts");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let target = dir.join(format!("{}.json", info.sha256));
        let tmp = target.with_extension("json.new");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        runtime::replace_file(&tmp, &target)?;
        if let Some(pack) = &original {
            let path = self
                .root
                .join("original-provenance")
                .join(format!("{}.json", pack.profile_id));
            let tmp = path.with_extension("json.new");
            fs::write(
                &tmp,
                serde_json::to_vec_pretty(pack).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            runtime::replace_file(&tmp, &path)?;
        }
        if let Ok(mut s) = self.status.lock() {
            s.validation = Some(receipt);
        }
        Ok(model_id)
    }
}
fn validate_inference(
    runtime: &runtime::VerifiedRuntime,
    path: &Path,
    metadata: &AIModelMetadata,
    config: &EngineConfig,
    work: &Path,
    cancel: Arc<AtomicBool>,
    source_sha256: String,
    detected: Value,
) -> Result<Receipt, String> {
    use super::provider::{VoiceConversionBackend, WOkadaBackend};
    let staged = crate::presets::preview::PreviewModel::new(work, path, metadata)?;
    let (mut backend, loaded) = WOkadaBackend::load(
        runtime,
        &staged.path,
        metadata,
        config,
        &work.join("inference.log"),
        cancel.clone(),
    )?;
    super::warmup::warm_up(&mut backend, config, &cancel)?;
    let mut peak = 0i32;
    let mut inference_ms = 0.0;
    let mut block = vec![0i16; config.chunk_frames];
    let timer = Instant::now();
    let chunks = 8;
    for chunk in 0..chunks {
        if cancel.load(Ordering::Acquire) {
            return Err("Validation cancelled".into());
        }
        for (i, s) in block.iter_mut().enumerate() {
            let t = (chunk * config.chunk_frames + i) as f32 / 48000.0;
            *s = (3000.0 * (std::f32::consts::TAU * 180.0 * t).sin()) as i16;
        }
        let result = backend.convert(&block)?;
        if result.warming_up {
            return Err("Model remained in warm-up during actual inference validation".into());
        }
        inference_ms += result.inference_ms;
        peak = peak.max(
            result
                .samples
                .iter()
                .map(|v| i32::from(*v).abs())
                .max()
                .unwrap_or(0),
        );
    }
    if peak <= 8 {
        return Err(
            "Model produced no usable non-silent neural output; it was not marked Ready".into(),
        );
    }
    Ok(Receipt {
        model_sha256: metadata.sha256.clone(),
        runtime_digest: runtime.digest.clone(),
        provider: loaded["provider"].as_str().unwrap_or("unknown").into(),
        peak,
        chunks,
        inference_ms,
        real_time_factor: timer.elapsed().as_secs_f64() / (chunks * config.chunk_frames) as f64
            * 48000.0,
        config: config.clone(),
        source_sha256,
        detected: {
            let mut details = detected;
            details["indexAvailable"] = loaded["indexAvailable"].clone();
            details
        },
    })
}
fn reject_unlicensed_source(sha: &str) -> Result<(), String> {
    if [
        "83745f80f560c727367f7e172a35605fbb5ca2f50749953e4a847c0111062bc7",
        "ef95e555298bac728d637174ebe7263fc1b12e7f6ed60b66b411ad96883045d2",
        "b7da32d31150d2191d5d2d6dd07dafd86b1cfdbe0a991871da4bb6c18cab0d39",
    ]
    .contains(&sha)
    {
        return Err("License Unverified: this known artifact lacks VoiceMorph model/voice authorization. Local selection does not bypass its licensing restriction.".into());
    }
    Ok(())
}
pub fn parse_hf_url(input: &str) -> Result<(String, String, String), String> {
    let url = reqwest::Url::parse(input).map_err(|_| "Use a Hugging Face file URL")?;
    if url.scheme() != "https"
        || url.host_str() != Some("huggingface.co")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use a clean HTTPS huggingface.co file URL without credentials, query or fragment"
                .into(),
        );
    }
    let parts: Vec<_> = url
        .path_segments()
        .ok_or("Invalid repository URL")?
        .collect();
    if parts.len() < 5
        || !matches!(parts[2], "blob" | "resolve")
        || parts
            .iter()
            .any(|s| s.is_empty() || s.contains('%') || s.contains('\\') || *s == ".." || *s == ".")
    {
        return Err(
            "Use https://huggingface.co/owner/repo/blob/revision/path/model.pth or .onnx".into(),
        );
    }
    if !["pth", "onnx"].contains(
        &parts
            .last()
            .and_then(|s| s.rsplit('.').next())
            .unwrap_or(""),
    ) {
        return Err(
            "Select a PTH or ONNX model URL; optional index files cannot be installed as a model"
                .into(),
        );
    }
    Ok((
        format!("{}/{}", parts[0], parts[1]),
        parts[3].into(),
        parts[4..].join("/"),
    ))
}
pub fn inspect_hf(input: &str) -> Result<RemoteArtifact, String> {
    let (repo, revision, file) = parse_hf_url(input)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(format!(
            "https://huggingface.co/api/models/{repo}/revision/{revision}?blobs=true"
        ))
        .send()
        .map_err(|e| format!("Could not read Hugging Face metadata: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Repository metadata is unavailable: {e}"))?;
    use std::io::Read;
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Repository metadata exceeds limit".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    artifact_from_metadata(&repo, &file, &value)
}
pub fn artifact_from_metadata(
    repo: &str,
    file: &str,
    value: &Value,
) -> Result<RemoteArtifact, String> {
    let revision = value["sha"]
        .as_str()
        .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Repository does not expose a pinned revision")?;
    let found = value["siblings"]
        .as_array()
        .ok_or("Repository file metadata is unavailable")?
        .iter()
        .find(|f| f["rfilename"] == file)
        .ok_or("Requested model file is not available in this repository revision")?;
    let sha = found["lfs"]["sha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Model has no publisher LFS SHA-256; use a locally verified file instead")?;
    let bytes = found["size"]
        .as_u64()
        .or_else(|| found["lfs"]["size"].as_u64())
        .filter(|n| *n > 0 && *n <= 512 * 1024 * 1024)
        .ok_or("Model size is unavailable or exceeds 512 MiB")?;
    let license = value["cardData"]["license"].as_str().map(str::to_owned);
    let approved = license.as_deref().is_some_and(|s| {
        [
            "mit",
            "apache-2.0",
            "bsd-2-clause",
            "bsd-3-clause",
            "cc-by-4.0",
            "cc0-1.0",
        ]
        .contains(&s)
    });
    Ok(RemoteArtifact {
        repository: repo.into(),
        revision: revision.into(),
        filename: file.into(),
        bytes,
        sha256: sha.into(),
        declared_license: license,
        state: if approved {
            "declaredLicense"
        } else {
            "licenseUnverified"
        }
        .into(),
        download_url: format!("https://huggingface.co/{repo}/resolve/{revision}/{file}"),
    })
}
#[cfg(windows)]
fn check_space(root: &Path, bytes: u64) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0;
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(format!(
            "Cannot check disk space: {}",
            std::io::Error::last_os_error()
        ));
    }
    if free < bytes {
        return Err("Insufficient disk space for checkpoint conversion, ONNX staging and model registration".into());
    }
    Ok(())
}
#[cfg(not(windows))]
fn check_space(_: &Path, _: u64) -> Result<(), String> {
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_metadata_is_not_a_voice_license() {
        let data = json!({"sha":"a".repeat(40),"siblings":[{"rfilename":"jett/jett.pth","size":55000000,"lfs":{"sha256":"b".repeat(64)}}],"cardData":{}});
        let a = artifact_from_metadata("fox3000foxy/valorant_rvc", "jett/jett.pth", &data).unwrap();
        assert_eq!(a.state, "licenseUnverified");
        assert!(a.download_url.contains(&"a".repeat(40)));
        assert!(artifact_from_metadata("x/y", "absent.pth", &data).is_err());
    }
    #[test]
    fn rejects_non_huggingface_and_ambiguous_urls() {
        for url in [
            "http://127.0.0.1/a/b/blob/main/model.pth",
            "https://huggingface.co.evil/a/b/blob/main/x.pth",
            "https://user@huggingface.co/a/b/blob/main/x.pth",
            "https://huggingface.co/a/b/blob/main/x.py",
            "https://huggingface.co/a/b/blob/main/x.pth?token=secret",
        ] {
            assert!(parse_hf_url(url).is_err());
        }
        assert!(
            parse_hf_url("https://huggingface.co/owner/model/blob/main/folder/voice.onnx").is_ok()
        );
    }
}
#[cfg(test)]
mod local_tests {
    use super::*;
    #[test]
    #[ignore = "Local VCTK test artifacts: real pack registration, preview and realtime PCM; requires offline models and input"]
    fn authorized_vctk_pair_pack_preview_and_realtime() {
        use crate::effects::parameters::EffectParameters;
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.ai-runtime-build");
        let phase7 = std::env::var("VOICEMORPH_TEST_CONTEXTUAL_RVC").as_deref() == Ok("1");
        let work = assets.join(if phase7 { "phase7-4" } else { "phase6-vctk-test" });
        let installed = std::env::var_os("VOICEMORPH_TEST_INSTALLED_LIBRARY").map(PathBuf::from);
        let runtime_root = installed.as_ref().map_or_else(|| work.join("runtime"), |p| p.join("ai-runtime"));
        let runtime = runtime::discover(&runtime_root).unwrap();
        let library = work.join("verified-library");
        let source_library = installed.as_ref().unwrap_or(&library);
        fs::create_dir_all(&library).unwrap();
        let manager = PackManager::new(source_library.join("voice-packs")).unwrap();
        let models = Mutex::new(AIModelManager::new(source_library.join("ai-models")).unwrap());
        let agents = Mutex::new(AgentLibrary::load(source_library.join("agent-voices.json")).unwrap());
        let source = work.join("original_input.wav");
        let wave: Vec<_> = hound::WavReader::open(&source)
            .unwrap()
            .samples::<i16>()
            .map(|sample| f32::from(sample.unwrap()) / 32768.0)
            .collect();
        let base = EngineConfig {
            provider: "cuda".into(),
            ..Default::default()
        };
        let mut outcomes = vec![];
        for (profile, speaker) in [("wind-assassin", "p231"), ("jade-guardian", "p238")] {
            let request = InstallRequest {
                agent_id: profile.into(),
                local_path: Some(work.join(speaker).join("voice-pack.json")),
                hf_url: None,
                existing_model_id: None,
                index_path: None,
                identity_name: None,
                source_url: None,
                license_url: None,
                attribution: None,
                rights_confirmed: true,
            };
            if installed.is_none() {
                manager.begin(&request).unwrap();
                let result = manager.install(request, &runtime_root, &base, &models, &agents);
                manager.finish(result.clone());
                result.unwrap();
            }
            let (metadata, path, binding) = agents
                .lock()
                .unwrap()
                .resolve(profile, &models.lock().unwrap())
                .unwrap();
            let model_id = binding.model_id;
            let receipt = manager.receipt(&metadata.sha256, &runtime.digest).unwrap();
            assert_eq!(receipt.detected["originalPack"]["profileId"], profile);
            assert_eq!(receipt.detected["indexAvailable"], true);
            assert_eq!(receipt.config.f0_method, "rmvpe_onnx");
            let mut test_config = receipt.config.clone();
            if phase7 {
                assert_eq!(metadata.feature_encoder_sha256.as_deref(), Some(runtime::RVC_CONTENTVEC_SHA256));
                test_config = super::super::agents::quality_config(&test_config, "highQuality").unwrap();
            }
            let preview = if installed.is_none() {
                let preview = super::super::comparison::compare(
                    &runtime_root,
                    &metadata,
                    &path,
                    &source,
                    true,
                    &test_config,
                    EffectParameters::default(),
                )
                .unwrap();
                assert!(preview.neural.peak > 0.001);
                fs::write(
                    library.join(format!("{profile}-app-preview.wav")),
                    &preview.neural.wav,
                )
                .unwrap();
                Some(preview)
            } else {
                None
            };

            let mut engine = super::super::engine::AIEngine::default();
            engine.configure(
                runtime_root.clone(),
                library.join(format!("{profile}-engine.json")),
            );
            engine.save_config(test_config).unwrap();
            let mut port = engine.attach(48_000);
            engine.start(runtime.clone(), metadata, path).unwrap();
            let loading_started = Instant::now();
            while engine.status().state == "starting" {
                assert!(loading_started.elapsed() < Duration::from_secs(120));
                std::thread::sleep(Duration::from_millis(40));
            }
            assert_eq!(engine.status().state, "ready", "{:?}", engine.status());
            let mut audible = 0;
            let blocks = if phase7 { wave.len().div_ceil(480) + 58 } else { 600 };
            let mut live = Vec::with_capacity(blocks * 480);
            let clock = Instant::now();
            for n in 0..blocks {
                for offset in 0..480 {
                    let value = port
                        .process(wave.get(n * 480 + offset).copied().unwrap_or(0.0))
                        .expect("Unexpected Classic DSP");
                    audible += usize::from(value.abs() > 0.0001);
                    live.push(super::super::transport::to_pcm16(value));
                }
                if let Some(remaining) = (clock + Duration::from_millis((n + 1) as u64 * 10))
                    .checked_duration_since(Instant::now())
                {
                    std::thread::sleep(remaining);
                }
            }
            let engine_status = engine.status();
            let realtime_pass = audible > 0
                && engine_status.completed_chunks > 0
                && engine_status.state != "faulted";
            engine.select_classic();
            assert!(port.process(0.2).is_none(), "Classic DSP must resume on the attached audio stream");
            if phase7 {
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: 48_000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut writer = hound::WavWriter::create(
                    work.join(format!("{speaker}_live_engine.wav")), spec,
                ).unwrap();
                for sample in live {
                    writer.write_sample(sample).unwrap();
                }
                writer.finalize().unwrap();
            }
            outcomes.push(json!({
                "profileId": profile, "modelId": model_id, "receipt": receipt,
                "previewPeak": preview.as_ref().map(|result| result.neural.peak),
                "previewProvider": preview.as_ref().map(|result| &result.provider),
                "previewRtf": preview.as_ref().map(|result| result.real_time_factor),
                "previewDeadlineMisses": preview.as_ref().map(|result| result.missed_deadlines),
                "realtimePass": realtime_pass, "realtimeAudibleSamples": audible, "engineStatus": engine_status,
            }));
            println!(
                "{profile}: realtime pass={realtime_pass}, {audible} audible samples, {:?}",
                engine_status.message
            );
            fs::write(
                library.join("app-verification.json"),
                serde_json::to_vec_pretty(&outcomes).unwrap(),
            )
            .unwrap();
        }
        if installed.is_some() {
            assert!(outcomes.iter().all(|result| result["realtimePass"] == true), "{outcomes:?}");
        }
    }
    #[test]
    #[ignore = "Executes the installed runtime with an authorized owner ONNX and a numerical FAISS fixture"]
    fn indexed_bridge_real_inference() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let assets = project.join(".ai-runtime-build");
        let work = assets.join("phase6-index-smoke");
        let model_path = work.join("voice.onnx");
        fs::copy(
            assets.join("converted-tsukuyomi/tsukuyomi-normal1.onnx"),
            &model_path,
        )
        .unwrap();
        let runtime = runtime::discover(&assets.join("bootstrap-installed")).unwrap();
        let mut metadata = super::super::catalog::metadata();
        metadata.feature_index_sha256 =
            Some(sha256_file(&model_path.with_extension("index")).unwrap());
        for provider in ["cpu", "cuda"] {
            let config = EngineConfig {
                provider: provider.into(),
                chunk_frames: 8192,
                crossfade_frames: 2048,
                extra_context_frames: 16384,
                ..Default::default()
            };
            let result = validate_inference(
                &runtime,
                &model_path,
                &metadata,
                &config,
                &work,
                Arc::new(AtomicBool::new(false)),
                metadata.sha256.clone(),
                json!({}),
            )
            .unwrap();
            assert_eq!(result.detected["indexAvailable"], true);
            assert_eq!(result.provider, provider);
            println!(
                "{provider}: peak {}, RTF {:.3}, {} blocks",
                result.peak, result.real_time_factor, result.chunks
            );
            fs::write(
                work.join(format!("{provider}.json")),
                serde_json::to_vec_pretty(&result).unwrap(),
            )
            .unwrap();
        }
    }
    #[test]
    #[ignore = "Approved owner PTH: restricted conversion, actual neural inference, receipt and automatic assignment; no devices"]
    fn authorized_pth_pack_end_to_end() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let root = project
            .join(".ai-runtime-build")
            .join(format!("pack-test-{}", super::super::process::now_ms()));
        let manager =
            PackManager::new(project.join(".ai-runtime-build/pack-manager-e2e/packs")).unwrap();
        let models = Mutex::new(AIModelManager::new(root.join("models")).unwrap());
        let agents = Mutex::new(AgentLibrary::load(root.join("agents.json")).unwrap());
        let request = InstallRequest {
            agent_id: "jett".into(),
            local_path: Some(
                project.join(".ai-runtime-build/assets/tsukuyomi-official-normal1.pth"),
            ),
            hf_url: None,
            existing_model_id: None,
            index_path: None,
            identity_name: None,
            source_url: None,
            license_url: None,
            attribution: None,
            rights_confirmed: true,
        };
        manager.begin(&request).unwrap();
        let result = manager.install(
            request,
            &project.join(".ai-runtime-build/bootstrap-installed"),
            &EngineConfig {
                provider: "cuda".into(),
                ..Default::default()
            },
            &models,
            &agents,
        );
        manager.finish(result.clone());
        let id = result.unwrap();
        let (model, _) = models.lock().unwrap().resolve(&id).unwrap();
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.phase, "ready");
        let receipt = snapshot.validation.unwrap();
        assert!(receipt.peak > 8);
        assert!(receipt.chunks > 0);
        assert!(
            manager
                .receipt(&model.sha256, &receipt.runtime_digest)
                .is_some()
        );
        assert!(manager.receipt(&model.sha256, &"0".repeat(64)).is_none());
        assert_eq!(
            agents
                .lock()
                .unwrap()
                .resolve("jett", &models.lock().unwrap())
                .unwrap()
                .0
                .id,
            id
        );
        std::fs::write(
            root.join("result.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        println!(
            "Real PTH pack: {} provider={} peak={} chunks={} RTF={:.3}; evidence {}",
            id,
            receipt.provider,
            receipt.peak,
            receipt.chunks,
            receipt.real_time_factor,
            root.display()
        );
    }

    #[test]
    #[ignore = "Approved owner ONNX: raw import and actual CPU inference; no devices"]
    fn authorized_onnx_pack_on_cpu() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let assets = project.join(".ai-runtime-build");
        let root = assets.join(format!(
            "onnx-pack-test-{}",
            super::super::process::now_ms()
        ));
        let manager = PackManager::new(root.join("packs")).unwrap();
        let models = Mutex::new(AIModelManager::new(root.join("models")).unwrap());
        let agents = Mutex::new(AgentLibrary::load(root.join("agents.json")).unwrap());
        let request = InstallRequest {
            agent_id: "sage".into(),
            local_path: Some(assets.join("converted-tsukuyomi/tsukuyomi-normal1.onnx")),
            hf_url: None,
            existing_model_id: None,
            index_path: None,
            identity_name: None,
            source_url: None,
            license_url: None,
            attribution: None,
            rights_confirmed: true,
        };
        manager.begin(&request).unwrap();
        let result = manager.install(
            request,
            &assets.join("bootstrap-installed"),
            &EngineConfig {
                provider: "cpu".into(),
                ..Default::default()
            },
            &models,
            &agents,
        );
        manager.finish(result.clone());
        let id = result.unwrap();
        let status = manager.snapshot();
        let receipt = status.validation.unwrap();
        assert_eq!(status.phase, "ready");
        assert_eq!(receipt.provider, "cpu");
        assert!(receipt.peak > 8);
        assert_eq!(receipt.chunks, 8);
        fs::write(
            root.join("result.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        println!(
            "Real raw ONNX pack: {id}, CPU, peak={}, RTF={:.3}; evidence {}",
            receipt.peak,
            receipt.real_time_factor,
            root.display()
        );
    }
}
