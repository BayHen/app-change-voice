use super::{manager::AIModelManager, models::AIModelMetadata};
use serde::Deserialize;
use std::{fs, path::Path};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub name: String,
    pub source_url: String,
    pub license_url: String,
    pub attribution: String,
    pub rights_confirmed: bool,
    pub expected_sha256: String,
}
pub fn register(
    manager: &mut AIModelManager,
    path: &Path,
    request: ImportRequest,
) -> Result<super::models::AIModelStatus, String> {
    register_with_index(manager, path, request, None, None)
}

pub(super) fn register_with_index(
    manager: &mut AIModelManager,
    path: &Path,
    request: ImportRequest,
    feature_index_sha256: Option<String>,
    feature_encoder_sha256: Option<String>,
) -> Result<super::models::AIModelStatus, String> {
    if !request.rights_confirmed {
        return Err("Confirm model licensing and voice usage rights before import".into());
    }
    if !request.source_url.starts_with("https://")
        || !request.license_url.starts_with("https://")
        || request.attribution.trim().is_empty()
        || request.name.trim().is_empty()
    {
        return Err(
            "Provide a model name, HTTPS source and terms URLs, and credited voice identity".into(),
        );
    }
    let info = super::onnx::inspect(path)?;
    if info.sha256 != request.expected_sha256 {
        return Err("The model changed since inspection. Select it again.".into());
    }
    if info.compatibility != "candidate" {
        return Err(format!("Incompatible RVC model: {}", info.issues.join(" ")));
    }
    if manager
        .list()
        .iter()
        .any(|m| m.metadata.sha256 == info.sha256)
    {
        return Err(
            "This model checksum is already imported. Select the existing identity in the library."
                .into(),
        );
    }
    let slug: String = request
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    let metadata=AIModelMetadata{schema_version:1,id:format!("{}-{}",slug.trim_matches('-'),&info.sha256[..12]),name:request.name,description:"User-imported independent neural voice. Rights declared by the importing user; naturalness and identity separation require listening tests.".into(),architecture:"rvc".into(),backend:"w-okada-rvc".into(),license:"LicenseRef-UserConfirmed".into(),source_url:request.source_url,model_file:path.file_name().ok_or("Missing ONNX filename")?.to_string_lossy().into_owned(),sha256:info.sha256,input_sample_rate:48000,frame_samples:4096,supported_profiles:vec![],consent_confirmed:true,license_url:Some(request.license_url),attribution:Some(request.attribution),feature_index_sha256,feature_encoder_sha256};
    super::validator::validate_metadata(&metadata)?;
    let staging = std::env::temp_dir().join(format!(
        "voicemorph-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let manifest = staging.join("model.json");
    let result = (|| {
        fs::write(
            &manifest,
            serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        manager.add_with_consent(&manifest, path, true)
    })();
    let _ = fs::remove_file(&manifest);
    let _ = fs::remove_dir(&staging);
    result
}
