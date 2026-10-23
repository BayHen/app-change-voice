use super::{
    models::{AIModelMetadata, AIModelStatus, AIProviderStatus},
    validator::{validate_metadata, verify_model},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct AIModelManager {
    root: PathBuf,
    active_model_id: Option<String>,
}

impl AIModelManager {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(Self {
            root,
            active_model_id: None,
        })
    }

    pub fn list(&self) -> Vec<AIModelStatus> {
        let mut models = Vec::new();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return models;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let status = fs::read(&path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<AIModelMetadata>(&bytes)
                        .map_err(|error| error.to_string())
                })
                .and_then(|metadata| {
                    validate_metadata(&metadata)?;
                    let model_path = self.root.join(&metadata.model_file);
                    let verified = verify_model(&metadata, &model_path).is_ok();
                    Ok(AIModelStatus {
                        enabled: self.active_model_id.as_deref() == Some(&metadata.id),
                        integrity_verified: verified,
                        runtime_available: false,
                        status_message: if verified {
                            "Integrity verified; native ONNX runtime is not enabled in this build"
                                .into()
                        } else {
                            "Model missing or integrity verification failed".into()
                        },
                        metadata,
                    })
                });
            if let Ok(status) = status {
                models.push(status);
            }
        }
        models.sort_by(|left, right| left.metadata.name.cmp(&right.metadata.name));
        models
    }

    pub fn add(
        &mut self,
        manifest_path: &Path,
        model_path: &Path,
    ) -> Result<AIModelStatus, String> {
        let bytes = fs::read(manifest_path).map_err(|error| error.to_string())?;
        if bytes.len() > 128 * 1024 {
            return Err("Model manifest exceeds 128 KiB".into());
        }
        let metadata: AIModelMetadata =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        validate_metadata(&metadata)?;
        if model_path.file_name().and_then(|value| value.to_str()) != Some(&metadata.model_file) {
            return Err("Selected model filename does not match modelFile in the manifest".into());
        }
        verify_model(&metadata, model_path)?;
        let target_model = self.root.join(&metadata.model_file);
        let target_manifest = self.root.join(format!("{}.json", metadata.id));
        let temporary_model = target_model.with_extension("onnx.tmp");
        let temporary_manifest = target_manifest.with_extension("json.tmp");
        fs::copy(model_path, &temporary_model).map_err(|error| error.to_string())?;
        fs::write(
            &temporary_manifest,
            serde_json::to_vec_pretty(&metadata).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if target_model.exists() {
            fs::remove_file(&target_model).map_err(|error| error.to_string())?;
        }
        if target_manifest.exists() {
            fs::remove_file(&target_manifest).map_err(|error| error.to_string())?;
        }
        fs::rename(temporary_model, &target_model).map_err(|error| error.to_string())?;
        fs::rename(temporary_manifest, &target_manifest).map_err(|error| error.to_string())?;
        Ok(AIModelStatus {
            metadata,
            enabled: false,
            integrity_verified: true,
            runtime_available: false,
            status_message: "Imported safely; native ONNX runtime is not enabled in this build"
                .into(),
        })
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let manifest = self.root.join(format!("{id}.json"));
        let bytes = fs::read(&manifest).map_err(|_| "AI model was not found".to_string())?;
        let metadata: AIModelMetadata =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        validate_metadata(&metadata)?;
        let model = self.root.join(metadata.model_file);
        if model.exists() {
            fs::remove_file(model).map_err(|error| error.to_string())?;
        }
        fs::remove_file(manifest).map_err(|error| error.to_string())?;
        if self.active_model_id.as_deref() == Some(id) {
            self.active_model_id = None;
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<AIProviderStatus, String> {
        if enabled {
            let model = self
                .list()
                .into_iter()
                .find(|value| value.metadata.id == id)
                .ok_or_else(|| "AI model was not found".to_string())?;
            if !model.integrity_verified {
                return Err("AI model integrity could not be verified".into());
            }
            return Err("Native ONNX inference is unavailable; DSP fallback remains active".into());
        }
        if self.active_model_id.as_deref() == Some(id) {
            self.active_model_id = None;
        }
        Ok(self.provider_status())
    }

    pub fn provider_status(&self) -> AIProviderStatus {
        AIProviderStatus {
            offline_only: true,
            inference_backend: "Python/OpenVoice prototype; native runtime deferred".into(),
            gpu_verified: false,
            active_model_id: self.active_model_id.clone(),
            fallback_active: true,
            message: "AI realtime is unavailable in this build; the realtime DSP chain is the safe fallback".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::models::AI_MODEL_SCHEMA_VERSION;
    use crate::ai::validator::sha256_file;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("voicemorph-ai-{suffix}"))
    }

    #[test]
    fn imports_lists_and_removes_verified_model() {
        let root = temporary_directory();
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        let model_path = source.join("tiny.onnx");
        fs::write(&model_path, b"test-only-onnx-placeholder").unwrap();
        let metadata = AIModelMetadata {
            schema_version: AI_MODEL_SCHEMA_VERSION,
            id: "tiny-test".into(),
            name: "Tiny Test".into(),
            description: "Integrity and repository test".into(),
            architecture: "test".into(),
            backend: "onnx-runtime".into(),
            license: "MIT".into(),
            source_url: "https://example.invalid/model".into(),
            model_file: "tiny.onnx".into(),
            sha256: sha256_file(&model_path).unwrap(),
            input_sample_rate: 48_000,
            frame_samples: 2_048,
            supported_profiles: vec![],
        };
        let manifest_path = source.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_vec(&metadata).unwrap()).unwrap();

        let mut manager = AIModelManager::new(root.join("library")).unwrap();
        let imported = manager.add(&manifest_path, &model_path).unwrap();
        assert!(imported.integrity_verified);
        assert_eq!(manager.list().len(), 1);
        manager.remove("tiny-test").unwrap();
        assert!(manager.list().is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_model_with_wrong_digest() {
        let root = temporary_directory();
        fs::create_dir_all(&root).unwrap();
        let model_path = root.join("tiny.onnx");
        fs::write(&model_path, b"model").unwrap();
        let metadata = AIModelMetadata {
            schema_version: 1,
            id: "bad-digest".into(),
            name: "Bad Digest".into(),
            description: "Must fail".into(),
            architecture: "test".into(),
            backend: "onnx-runtime".into(),
            license: "MIT".into(),
            source_url: "https://example.invalid/model".into(),
            model_file: "tiny.onnx".into(),
            sha256: "0".repeat(64),
            input_sample_rate: 48_000,
            frame_samples: 2_048,
            supported_profiles: vec![],
        };
        let manifest_path = root.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_vec(&metadata).unwrap()).unwrap();
        let mut manager = AIModelManager::new(root.join("library")).unwrap();
        assert!(manager.add(&manifest_path, &model_path).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
