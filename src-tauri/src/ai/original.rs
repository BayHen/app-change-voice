use super::{agents::original_name, runtime::package_path, validator::sha256_file};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const MAX_INDEX_BYTES: u64 = 64 * 1024 * 1024;

pub fn verify_index(path: &Path, expected: &str) -> Result<(), String> {
    use std::io::Read;
    let mut file = fs::File::open(path).map_err(|e| format!("Feature index missing: {e}"))?;
    let bytes = file.metadata().map_err(|e| e.to_string())?.len();
    let mut header = [0u8; 45];
    file.read_exact(&mut header)
        .map_err(|_| "Truncated FAISS feature index")?;
    let rows = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let floats = u64::from_le_bytes(header[37..45].try_into().unwrap());
    if &header[..4] != b"IxF2"
        || header[4..8] != 768u32.to_le_bytes()
        || header[32] != 1
        || header[33..37] != 1u32.to_le_bytes()
        || !(8..=16000).contains(&rows)
        || floats != rows * 768
        || bytes != 45 + floats * 4
        || sha256_file(path)? != expected
    {
        return Err(
            "Feature index must be a verified FAISS IndexFlatL2 of 768-dimensional RVC v2 features"
                .into(),
        );
    }
    Ok(())
}

pub fn stage_index(source: &Path, target: &Path, expected: Option<&str>) -> Result<(), String> {
    if let Some(digest) = expected {
        let input = source.with_extension("index");
        let output = target.with_extension("index");
        verify_index(&input, digest)?;
        fs::copy(input, &output).map_err(|e| e.to_string())?;
        verify_index(&output, digest)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub file: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PretrainedSource {
    pub revision: String,
    pub dataset_url: String,
    pub upstream_index_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalPack {
    pub schema_version: u32,
    pub profile_id: String,
    pub speaker_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretrained_source: Option<PretrainedSource>,
    /// Encoder verified for inference. For a locally trained pack this is also the training encoder.
    pub embedder_sha256: String,
    pub license: String,
    pub license_url: String,
    pub source_url: String,
    pub attribution: String,
    pub architecture: String,
    pub sample_rate: u32,
    pub checkpoint: Artifact,
    pub onnx: Artifact,
    pub feature_index: Artifact,
}

pub fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn https_url(url: &str) -> bool {
    url.len() <= 1024
        && reqwest::Url::parse(url).ok().is_some_and(|parsed| {
            parsed.scheme() == "https"
                && parsed.host_str().is_some()
                && parsed.username().is_empty()
                && parsed.password().is_none()
        })
}

impl OriginalPack {
    pub fn read(path: &Path, expected_id: &str) -> Result<Self, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("Original voice pack manifest exceeds 64 KiB".into());
        }
        let pack: Self = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Invalid original voice pack manifest: {e}"))?;
        pack.validate(expected_id)?;
        Ok(pack)
    }

    fn validate(&self, expected_id: &str) -> Result<(), String> {
        if self.schema_version != 1
            || self.profile_id != expected_id
            || original_name(expected_id).is_none()
            || self.architecture != "rvc-v2-f0"
            || self.sample_rate != 40000
        {
            return Err("Incompatible original voice pack profile, schema, or architecture".into());
        }
        if self.speaker_id.is_empty()
            || self.speaker_id.len() > 80
            || !self
                .speaker_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !valid_hex(&self.embedder_sha256, 64)
        {
            return Err("Invalid original speaker or training provenance".into());
        }
        match &self.pretrained_source {
            None if self.license == "LicenseRef-UserConfirmed"
                && self
                    .dataset_sha256
                    .as_deref()
                    .is_some_and(|s| valid_hex(s, 64))
                && self
                    .training_revision
                    .as_deref()
                    .is_some_and(|s| valid_hex(s, 40)) =>
            {
                ()
            }
            Some(source)
                if self.license == "Apache-2.0"
                    && self.dataset_sha256.is_none()
                    && self.training_revision.is_none()
                    && valid_hex(&source.revision, 40)
                    && valid_hex(&source.upstream_index_sha256, 64)
                    && https_url(&source.dataset_url) =>
            {
                ()
            }
            _ => return Err(
                "Supply either local training provenance or pinned pretrained source provenance"
                    .into(),
            ),
        }
        if self.attribution.trim().is_empty()
            || self.attribution.len() > 160
            || [&self.license_url, &self.source_url]
                .into_iter()
                .any(|url| !https_url(url))
        {
            return Err(
                "License Unverified: supply the dataset source, use terms and owner attribution"
                    .into(),
            );
        }
        Ok(())
    }

    pub fn same_training_identity(&self, other: &Self) -> bool {
        self.speaker_id.eq_ignore_ascii_case(&other.speaker_id)
            || self
                .dataset_sha256
                .as_ref()
                .is_some_and(|sha| other.dataset_sha256.as_ref() == Some(sha))
            || self.checkpoint.sha256 == other.checkpoint.sha256
            || self.feature_index.sha256 == other.feature_index.sha256
    }
}

pub fn verify_artifact(
    root: &Path,
    artifact: &Artifact,
    ext: &str,
    max_bytes: u64,
) -> Result<PathBuf, String> {
    if !valid_hex(&artifact.sha256, 64)
        || artifact.file.contains(['/', '\\'])
        || Path::new(&artifact.file)
            .extension()
            .and_then(|v| v.to_str())
            != Some(ext)
    {
        return Err(format!("Invalid original pack {ext} artifact declaration"));
    }
    let path = package_path(root, &artifact.file)?;
    let metadata = path.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > max_bytes
        || sha256_file(&path)? != artifact.sha256
    {
        return Err(format!(
            "Original pack {ext} failed size or SHA-256 validation"
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(profile_id: &str) -> OriginalPack {
        let artifact = Artifact {
            file: "voice.onnx".into(),
            sha256: "a".repeat(64),
        };
        OriginalPack {
            schema_version: 1,
            profile_id: profile_id.into(),
            speaker_id: "speaker-a".into(),
            dataset_sha256: Some("b".repeat(64)),
            training_revision: Some("c".repeat(40)),
            pretrained_source: None,
            embedder_sha256: "d".repeat(64),
            license: "LicenseRef-UserConfirmed".into(),
            license_url: "https://example.org/voice/terms".into(),
            source_url: "https://example.org/voice".into(),
            attribution: "Test fixture".into(),
            architecture: "rvc-v2-f0".into(),
            sample_rate: 40000,
            checkpoint: artifact.clone(),
            onnx: artifact.clone(),
            feature_index: artifact,
        }
    }

    #[test]
    fn rejects_wrong_profile_rate_and_missing_license() {
        let mut pack = sample("wind-assassin");
        assert!(pack.validate("wind-assassin").is_ok());
        assert!(pack.validate("jade-guardian").is_err());
        pack.sample_rate = 48000;
        assert!(pack.validate("wind-assassin").is_err());
        pack.sample_rate = 40000;
        pack.license_url = "http://example.org".into();
        assert!(pack.validate("wind-assassin").is_err());
    }

    #[test]
    fn requires_independent_speaker_data_weights_and_index() {
        let wind = sample("wind-assassin");
        let mut jade = sample("jade-guardian");
        assert!(wind.same_training_identity(&jade));
        jade.speaker_id = "speaker-b".into();
        jade.dataset_sha256 = Some("1".repeat(64));
        jade.checkpoint.sha256 = "2".repeat(64);
        assert!(wind.same_training_identity(&jade));
        jade.feature_index.sha256 = "3".repeat(64);
        assert!(!wind.same_training_identity(&jade));
    }
}
