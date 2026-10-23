use super::models::{AI_MODEL_SCHEMA_VERSION, AIModelMetadata};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

const MAX_MODEL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ALLOWED_LICENSES: &[&str] = &["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause"];

pub fn validate_metadata(metadata: &AIModelMetadata) -> Result<(), String> {
    if metadata.schema_version != AI_MODEL_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported AI model schema {}; expected {}",
            metadata.schema_version, AI_MODEL_SCHEMA_VERSION
        ));
    }
    if metadata.id.is_empty()
        || !metadata
            .id
            .chars()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-')
    {
        return Err("Model id must use lowercase ASCII letters, digits or hyphens".into());
    }
    if metadata.name.trim().is_empty() || metadata.description.trim().is_empty() {
        return Err("Model name and description are required".into());
    }
    if metadata.backend != "onnx-runtime" {
        return Err("Only the declarative onnx-runtime backend is accepted".into());
    }
    if !ALLOWED_LICENSES.contains(&metadata.license.as_str()) {
        return Err(format!(
            "Model license is not allow-listed: {}",
            metadata.license
        ));
    }
    if !metadata.source_url.starts_with("https://") {
        return Err("Model source must be an HTTPS URL".into());
    }
    let file = Path::new(&metadata.model_file);
    if file.components().count() != 1
        || file.extension().and_then(|value| value.to_str()) != Some("onnx")
    {
        return Err("modelFile must be a relative .onnx filename without directories".into());
    }
    if metadata.sha256.len() != 64
        || !metadata
            .sha256
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err("sha256 must contain exactly 64 hexadecimal characters".into());
    }
    if !(8_000..=96_000).contains(&metadata.input_sample_rate) {
        return Err("Input sample rate is outside the supported range".into());
    }
    if metadata.frame_samples == 0 || metadata.frame_samples > 65_536 {
        return Err("Frame size is outside the supported range".into());
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let details = path.metadata().map_err(|error| error.to_string())?;
    if !details.is_file() || details.len() > MAX_MODEL_BYTES {
        return Err("Model must be a regular file no larger than 2 GiB".into());
    }
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn verify_model(metadata: &AIModelMetadata, model_path: &Path) -> Result<(), String> {
    validate_metadata(metadata)?;
    let actual = sha256_file(model_path)?;
    if !actual.eq_ignore_ascii_case(&metadata.sha256) {
        return Err("Model SHA-256 does not match its manifest".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> AIModelMetadata {
        AIModelMetadata {
            schema_version: 1,
            id: "openvoice-v2".into(),
            name: "OpenVoice V2".into(),
            description: "Offline tone-colour conversion".into(),
            architecture: "tone-color-converter".into(),
            backend: "onnx-runtime".into(),
            license: "MIT".into(),
            source_url: "https://github.com/myshell-ai/OpenVoice".into(),
            model_file: "openvoice-v2.onnx".into(),
            sha256: "a".repeat(64),
            input_sample_rate: 48_000,
            frame_samples: 2_048,
            supported_profiles: vec!["deep".into()],
        }
    }

    #[test]
    fn accepts_portable_onnx_manifest() {
        assert!(validate_metadata(&valid()).is_ok());
    }

    #[test]
    fn rejects_path_traversal_and_pickle_models() {
        let mut metadata = valid();
        metadata.model_file = "../unsafe.pth".into();
        assert!(validate_metadata(&metadata).is_err());
    }

    #[test]
    fn rejects_unknown_license() {
        let mut metadata = valid();
        metadata.license = "NOASSERTION".into();
        assert!(validate_metadata(&metadata).is_err());
    }
}
