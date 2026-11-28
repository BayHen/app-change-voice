//! A deliberately small voice catalog with reviewed original-owner provenance.
use super::{
    bootstrap::{self, Component, Progress},
    manager::AIModelManager,
    models::AIModelMetadata,
    process::ManagedProcess,
    runtime::VerifiedRuntime,
    validator::sha256_file,
};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const VOICE_ID: &str = "tsukuyomi-official-normal1";
pub const TERMS_URL: &str = "https://tyc.rei-yumesaki.net/work/software/rvc/terms/";
pub const ATTRIBUTION: &str = "Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)";
pub const ONNX_SHA256: &str = "d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6";
pub fn metadata() -> AIModelMetadata {
    AIModelMetadata { schema_version: 1, id: VOICE_ID.into(), name: "Tsukuyomi-chan official · Normal 1".into(),
        description: "Original-owner RVC model, converted locally to ONNX. Custom attribution and use restrictions apply.".into(),
        architecture: "rvc".into(), backend: "w-okada-rvc".into(), license: "LicenseRef-TsukuyomiChan-Official".into(),
        source_url: "https://tyc.rei-yumesaki.net/work/software/rvc/".into(), model_file: "tsukuyomi-normal1.onnx".into(),
        sha256: ONNX_SHA256.into(), input_sample_rate: 48000, frame_samples: 4096, supported_profiles: vec![],
        consent_confirmed: true, license_url: Some(TERMS_URL.into()), attribution: Some(ATTRIBUTION.into()), feature_index_sha256: None, feature_encoder_sha256: None }
}
pub fn prepare(
    root: &Path,
    runtime: &VerifiedRuntime,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(Progress),
) -> Result<std::path::PathBuf, String> {
    let item = Component {
        name: "Tsukuyomi-chan official RVC archive".into(),
        url: "https://tyc.rei-yumesaki.net/files/voice/tyc-rvc.zip".into(),
        sha256: "1207590e7bac95bbbdeeced54714933390d73f0f7795a8f97fa1be60e542f201".into(),
        bytes: 255700053,
        kind: "approved-voice".into(),
    };
    let mut state = Progress {
        phase: "model-download".into(),
        component: item.name.clone(),
        total_bytes: item.bytes,
        total_components: 1,
        ..Default::default()
    };
    let archive = bootstrap::download(&item, &root.join("downloads"), &cancel, |bytes| {
        state.completed_bytes = bytes;
        progress(state.clone());
    })?;
    let staging = root.join(format!(
        ".voice-source-{}-{}",
        std::process::id(),
        super::process::now_ms()
    ));
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut archive = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if archive.len() > 100 {
            return Err("Unexpected owner archive layout".into());
        }
        let checkpoint = staging.join("tsukuyomi-official-normal1.pth");
        let mut found = false;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            if !name
                .rsplit('/')
                .next()
                .is_some_and(|n| n.starts_with("01 ") && n.ends_with(".pth"))
            {
                continue;
            }
            if found
                || entry.size() != 55237177
                || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err("Unexpected official model entry".into());
            }
            found = true;
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&checkpoint)
                .map_err(|e| e.to_string())?;
            let mut block = [0u8; 65536];
            let mut total = 0u64;
            loop {
                if cancel.load(Ordering::Acquire) {
                    return Err("Model installation cancelled".into());
                }
                let n = entry.read(&mut block).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                total += n as u64;
                if total > 55237177 {
                    return Err("Oversized model entry".into());
                }
                output.write_all(&block[..n]).map_err(|e| e.to_string())?;
            }
        }
        if !found
            || sha256_file(&checkpoint)?
                != "cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020"
        {
            return Err("Original-owner checkpoint integrity failed".into());
        }
        state.phase = "model-conversion".into();
        progress(state.clone());
        let mut process =
            ManagedProcess::spawn(runtime, &root.join("logs/model-preparation.log"), cancel)?;
        let response = process.call(
            serde_json::json!({"op":"convertApprovedVoice", "checkpointPath":checkpoint}),
            Duration::from_secs(120),
        )?;
        let model = staging.join("converted/tsukuyomi-normal1.onnx");
        if response["sha256"] != ONNX_SHA256 || sha256_file(&model)? != ONNX_SHA256 {
            return Err("Converted model differs from the audited ONNX artifact".into());
        }
        fs::write(
            staging.join("model.json"),
            serde_json::to_vec_pretty(&metadata()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(staging.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub fn import_prepared(manager: &mut AIModelManager, staging: &Path) -> Result<(), String> {
    manager.add_with_consent(
        &staging.join("model.json"),
        &staging.join("converted/tsukuyomi-normal1.onnx"),
        true,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_model_terms_are_bound_to_reviewed_artifact_and_credit() {
        let mut value = metadata();
        super::super::validator::validate_metadata(&value).unwrap();
        value.sha256 = "0".repeat(64);
        assert!(super::super::validator::validate_metadata(&value).is_err());
        value = metadata();
        value.attribution = None;
        assert!(super::super::validator::validate_metadata(&value).is_err());
    }
    #[test]
    #[ignore = "manual approved owner-model conversion and actual packaged runtime"]
    fn prepares_owner_voice_through_real_managed_runtime() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.ai-runtime-build/bootstrap-installed");
        let runtime = super::super::runtime::discover(&root).unwrap();
        let prepared = prepare(&root, &runtime, Arc::new(AtomicBool::new(false)), |p| {
            println!("{} {}", p.phase, p.component)
        })
        .unwrap();
        let mut manager = AIModelManager::new(root.join("test-model-library")).unwrap();
        import_prepared(&mut manager, &prepared).unwrap();
        let (model, path) = manager.resolve(VOICE_ID).unwrap();
        assert_eq!(sha256_file(&path).unwrap(), ONNX_SHA256);
        assert!(model.consent_confirmed);
        assert_eq!(model.license_url.as_deref(), Some(TERMS_URL));
        println!(
            "Actual approved ONNX model: {}; SHA-256 {}",
            path.display(),
            model.sha256
        );
        fs::remove_dir_all(prepared).unwrap();
    }
}
