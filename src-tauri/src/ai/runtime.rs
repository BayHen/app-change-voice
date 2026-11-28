use super::{
    config::{PROTOCOL_VERSION, UPSTREAM_COMMIT},
    validator::sha256_file,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const RVC_CONTENTVEC_FILE: &str = "assets/contentvec-rvc.pt";
pub const RVC_CONTENTVEC_SHA256: &str =
    "f54b40fd2802423a5643779c4861af1e9ee9c1564dc9d32f54f20b5ffba7db96";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeManifest {
    pub schema_version: u32,
    pub protocol_version: u32,
    pub upstream_commit: String,
    pub version: String,
    pub python_executable: String,
    pub entry_point: String,
    pub server_directory: String,
    pub content_vec_model: String,
    pub rmvpe_model: Option<String>,
    pub notices: Vec<String>,
    pub files: Vec<RuntimeFile>,
}
#[derive(Debug, Clone)]
pub struct VerifiedRuntime {
    pub root: PathBuf,
    pub manifest: RuntimeManifest,
    pub digest: String,
}
impl VerifiedRuntime {
    pub fn require_feature_encoder(&self, expected: &str) -> Result<(), String> {
        for file in &self.manifest.files {
            let allowed_path = file.path == self.manifest.content_vec_model
                || (file.path == RVC_CONTENTVEC_FILE && expected == RVC_CONTENTVEC_SHA256);
            if allowed_path && file.sha256.eq_ignore_ascii_case(expected) {
                let path = package_path(&self.root, &file.path)?;
                if sha256_file(&path)? == expected {
                    return Ok(());
                }
            }
        }
        Err("Required feature encoder is unavailable or failed verification. Install or repair the managed runtime.".into())
    }
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstallStatus {
    pub installed: bool,
    pub installing: bool,
    pub copied_files: usize,
    pub total_files: usize,
    pub version: Option<String>,
    pub message: Option<String>,
}

pub fn validate_relative(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 240
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == ".." || s.ends_with(['.', ' ']))
        || !Path::new(value)
            .components()
            .all(|v| matches!(v, Component::Normal(_)))
    {
        return Err("Only safe relative package paths are allowed".into());
    }
    Ok(())
}
fn reject_link(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_symlink() {
        return Err("Symbolic links are not allowed in AI packages".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Reparse points are not allowed in AI packages".into());
        }
    }
    Ok(())
}
pub fn package_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative(relative)?;
    reject_link(root)?;
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        path.push(part);
        reject_link(&path)?;
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let result = path.canonicalize().map_err(|e| e.to_string())?;
    if !result.starts_with(root) {
        return Err("Package path escapes its root".into());
    }
    Ok(result)
}
pub fn read_manifest(root: &Path, expected: &str) -> Result<(RuntimeManifest, String), String> {
    if expected.len() != 64 || !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(
            "Enter the runtime publisher's 64-character SHA-256 before installation".into(),
        );
    }
    let path = package_path(root, "runtime-manifest.json")?;
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("Runtime manifest exceeds 8 MiB".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Runtime manifest exceeds 8 MiB".into());
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if !digest.eq_ignore_ascii_case(expected) {
        return Err("Runtime manifest differs from the publisher SHA-256".into());
    }
    let manifest: RuntimeManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if manifest.schema_version != 1
        || manifest.protocol_version != PROTOCOL_VERSION
        || manifest.upstream_commit != UPSTREAM_COMMIT
        || manifest.version.is_empty()
    {
        return Err("Runtime protocol/upstream version is incompatible".into());
    }
    if manifest.files.is_empty() || manifest.files.len() > 20000 || manifest.notices.is_empty() {
        return Err("Runtime must contain bounded files and third-party notices".into());
    }
    let mut files = HashSet::new();
    let mut total = 0_u64;
    for file in &manifest.files {
        validate_relative(&file.path)?;
        if !files.insert(file.path.to_lowercase())
            || file.sha256.len() != 64
            || !file.sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Duplicate path or invalid runtime digest".into());
        }
        total = total
            .checked_add(file.bytes)
            .ok_or("Package size overflow")?;
        if total > 12 * 1024 * 1024 * 1024 {
            return Err("Runtime package exceeds 12 GiB".into());
        }
    }
    for required in [
        &manifest.python_executable,
        &manifest.entry_point,
        &manifest.content_vec_model,
    ]
    .into_iter()
    .chain(manifest.rmvpe_model.iter())
    .chain(manifest.notices.iter())
    {
        validate_relative(required)?;
        if !files.contains(&required.to_lowercase()) {
            return Err(format!("Runtime file missing from manifest: {required}"));
        }
    }
    validate_relative(&manifest.server_directory)?;
    if !files.contains(
        &format!("{}/voice_changer/RVC/RVCr2.py", manifest.server_directory).to_lowercase(),
    ) {
        return Err("Pinned w-okada RVC source is missing".into());
    }
    let bridge = manifest
        .files
        .iter()
        .find(|f| f.path == manifest.entry_point)
        .ok_or("Bridge entry missing")?;
    let expected_bridge = format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../ai-sidecar/bridge.py"))
    );
    let lf_bridge = format!(
        "{:x}",
        Sha256::digest(
            include_str!("../../../ai-sidecar/bridge.py")
                .replace("\r\n", "\n")
                .as_bytes()
        )
    );
    if !bridge.sha256.eq_ignore_ascii_case(&expected_bridge)
        && !bridge.sha256.eq_ignore_ascii_case(&lf_bridge)
        && !bridge.sha256.eq_ignore_ascii_case(
            "c72e3fbb6e45abe22a9b8c040a773a34dcb5fe22b96adf6812b4f94ceb3ff125",
        )
        && !bridge.sha256.eq_ignore_ascii_case(
            "1ea3d9fd85b206a59026d8e88ac691afcd4cc3e2a0490d15e2e7a32b9ecb83e2",
        )
        && !bridge.sha256.eq_ignore_ascii_case(
            "f87a23d4d34d283b58a9ddb73d10ed8acb2faf4d5a6dcce5029073f112d1f018",
        )
        && !bridge.sha256.eq_ignore_ascii_case(
            "9008fc368bb20feee26b1ce034d4cc5b38e33bb3c71ab70a75e5b28f202c477a",
        )
        && !bridge.sha256.eq_ignore_ascii_case(
            "8072800cd23838cbb1ebd7dc0baa371ea52687101e38b4594ab38f884f9f39ca",
        )
    {
        return Err("Runtime bridge is incompatible with this VoiceMorph build".into());
    }
    Ok((manifest, digest))
}
pub fn verify_bundle(root: &Path, expected: &str) -> Result<VerifiedRuntime, String> {
    let workers = std::thread::available_parallelism().map_or(1, |count| count.get().min(4));
    verify_bundle_with_workers(root, expected, workers)
}
fn verify_bundle_with_workers(
    root: &Path,
    expected: &str,
    workers: usize,
) -> Result<VerifiedRuntime, String> {
    let (manifest, digest) = read_manifest(root, expected)?;
    let mut files: Vec<_> = manifest.files.iter().collect();
    files.sort_unstable_by_key(|file| std::cmp::Reverse(file.bytes));
    std::thread::scope(|scope| {
        let files = &files;
        let handles: Vec<_> = (0..workers.min(files.len()))
            .map(|worker| {
                scope.spawn(move || {
                    for file in files.iter().skip(worker).step_by(workers) {
                        let path = package_path(root, &file.path)?;
                        if !path.is_file()
                            || path.metadata().map_err(|e| e.to_string())?.len() != file.bytes
                            || !sha256_file(&path)?.eq_ignore_ascii_case(&file.sha256)
                        {
                            return Err(format!("Runtime integrity failure: {}", file.path));
                        }
                    }
                    Ok(())
                })
            })
            .collect();
        let mut result = Ok(());
        for handle in handles {
            let verified = handle.join().unwrap_or_else(|_| Err("Runtime verification worker failed".into()));
            result = result.and(verified);
        }
        result
    })?;
    Ok(VerifiedRuntime {
        root: root.canonicalize().map_err(|e| e.to_string())?,
        manifest,
        digest,
    })
}

#[cfg(test)]
#[test]
#[ignore = "Measures the full SHA-256 check on the local managed runtime"]
fn measure_local_activation_verification() {
    let root = PathBuf::from(std::env::var_os("VOICEMORPH_TEST_RUNTIME_ROOT").unwrap());
    let digest = active_digest(&root).unwrap();
    let release = root.join("releases").join(&digest);
    for workers in [1, 4] {
        let started = std::time::Instant::now();
        verify_bundle_with_workers(&release, &digest, workers).unwrap();
        println!("{workers} verification workers: {:.2}s", started.elapsed().as_secs_f64());
    }
}
fn active_digest(root: &Path) -> Result<String, String> {
    let path = root.join("current.json");
    let file = fs::File::open(path).map_err(|_| "No managed AI runtime installed".to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 1024 {
        return Err("Invalid runtime pointer".into());
    }
    let digest: String = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid installed runtime digest".into());
    }
    Ok(digest)
}
/// Verify the small manifest at startup; executable/model files are fully hashed before use.
pub fn installed_manifest(root: &Path) -> Result<RuntimeManifest, String> {
    let digest = active_digest(root)?;
    read_manifest(&root.join("releases").join(&digest), &digest).map(|(manifest, _)| manifest)
}
pub fn discover(root: &Path) -> Result<VerifiedRuntime, String> {
    let digest = active_digest(root)?;
    verify_bundle(&root.join("releases").join(&digest), &digest)
}
pub fn install(
    source: &Path,
    root: &Path,
    expected: &str,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<VerifiedRuntime, String> {
    let (manifest, digest) = read_manifest(source, expected)?;
    fs::create_dir_all(root.join("releases")).map_err(|e| e.to_string())?;
    let destination = root.join("releases").join(&digest);
    if !destination.exists() {
        let staging = root.join(format!(
            ".staging-{}-{}",
            std::process::id(),
            super::process::now_ms()
        ));
        fs::create_dir(&staging).map_err(|e| e.to_string())?;
        let result = (|| {
            for (index, file) in manifest.files.iter().enumerate() {
                if cancel.load(Ordering::Acquire) {
                    return Err("Runtime installation cancelled; previous runtime retained".into());
                }
                let from = package_path(source, &file.path)?;
                let to = staging.join(&file.path);
                fs::create_dir_all(to.parent().ok_or("Invalid target")?)
                    .map_err(|e| e.to_string())?;
                // Stream bounded blocks; cancellation never waits for a multi-GB fs::copy.
                use std::io::{Read, Write};
                let mut input = fs::File::open(from).map_err(|e| e.to_string())?;
                let mut output = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(to)
                    .map_err(|e| e.to_string())?;
                let mut hasher = Sha256::new();
                let mut count = 0_u64;
                let mut buffer = [0_u8; 65536];
                loop {
                    if cancel.load(Ordering::Acquire) {
                        return Err("Runtime installation cancelled".into());
                    }
                    let read = input.read(&mut buffer).map_err(|e| e.to_string())?;
                    if read == 0 {
                        break;
                    }
                    count += read as u64;
                    if count > file.bytes {
                        return Err("Runtime file grew during installation".into());
                    }
                    hasher.update(&buffer[..read]);
                    output
                        .write_all(&buffer[..read])
                        .map_err(|e| e.to_string())?;
                }
                output.sync_all().map_err(|e| e.to_string())?;
                if count != file.bytes
                    || !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(&file.sha256)
                {
                    return Err(format!("Runtime integrity failure: {}", file.path));
                }
                progress(index + 1, manifest.files.len());
            }
            fs::copy(
                source.join("runtime-manifest.json"),
                staging.join("runtime-manifest.json"),
            )
            .map_err(|e| e.to_string())?;
            verify_bundle(&staging, &digest)?;
            fs::rename(&staging, &destination).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result?;
    }
    let verified = verify_bundle(&destination, &digest)?;
    if cancel.load(Ordering::Acquire) {
        return Err("Runtime installation cancelled before activation".into());
    }
    let current = root.join("current.json");
    if current.exists() {
        let previous = root.join("previous.json.new");
        fs::copy(&current, &previous).map_err(|e| e.to_string())?;
        replace_file(&previous, &root.join("previous.json"))?;
    }
    let pointer = root.join("current.json.new");
    fs::write(
        &pointer,
        serde_json::to_vec(&digest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace_file(&pointer, &current)?;
    Ok(verified)
}
pub fn rollback(root: &Path) -> Result<(), String> {
    let previous = root.join("previous.json");
    let digest: String = serde_json::from_slice(&fs::read(&previous).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid rollback pointer".into());
    }
    verify_bundle(&root.join("releases").join(digest.clone()), &digest)?;
    let temp = root.join("current.json.new");
    fs::copy(previous, &temp).map_err(|e| e.to_string())?;
    replace_file(&temp, &root.join("current.json"))
}

/// Publish an already assembled, contained runtime without copying several GB twice.
/// Files are fully verified before the atomic directory rename and pointer update.
pub fn activate_staged(
    staging: &Path,
    root: &Path,
    expected: &str,
    cancel: &AtomicBool,
) -> Result<VerifiedRuntime, String> {
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let canonical_stage = staging.canonicalize().map_err(|e| e.to_string())?;
    if canonical_stage.parent() != Some(canonical_root.as_path()) {
        return Err("Runtime staging must be an immediate contained child".into());
    }
    let mut verified = verify_bundle(staging, expected)?;
    if cancel.load(Ordering::Acquire) {
        return Err("Runtime activation cancelled".into());
    }
    fs::create_dir_all(root.join("releases")).map_err(|e| e.to_string())?;
    let destination = root.join("releases").join(&verified.digest);
    if destination.exists() {
        if verify_bundle(&destination, expected).is_err() {
            let quarantine = root.join(format!(
                ".corrupt-{}-{}",
                verified.digest,
                super::process::now_ms()
            ));
            fs::rename(&destination, &quarantine).map_err(|e| e.to_string())?;
            if let Err(error) = fs::rename(staging, &destination) {
                let _ = fs::rename(quarantine, &destination);
                return Err(error.to_string());
            }
        }
    } else {
        fs::rename(staging, &destination).map_err(|e| e.to_string())?;
    }
    if cancel.load(Ordering::Acquire) {
        return Err("Runtime activation cancelled; previous active pointer retained".into());
    }
    if active_digest(root)
        .ok()
        .is_some_and(|old| old != verified.digest)
    {
        let temp = root.join("previous.json.new");
        fs::copy(root.join("current.json"), &temp).map_err(|e| e.to_string())?;
        replace_file(&temp, &root.join("previous.json"))?;
    }
    use std::io::Write;
    let temp = root.join("current.json.new");
    let mut pointer = fs::File::create(&temp).map_err(|e| e.to_string())?;
    pointer
        .write_all(&serde_json::to_vec(&verified.digest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    pointer.sync_all().map_err(|e| e.to_string())?;
    drop(pointer);
    replace_file(&temp, &root.join("current.json"))?;
    verified.root = destination.canonicalize().map_err(|e| e.to_string())?;
    Ok(verified)
}
pub fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let a: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, target).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_activation_is_atomic_cancellable_and_repairs_corrupt_release() {
        let root = std::env::temp_dir().join(format!(
            "vm-staged-test-{}-{}",
            std::process::id(),
            super::super::process::now_ms()
        ));
        let stage = root.join(".bootstrap-one");
        let first = fixture(&stage, "fixture-one");
        let cancel = AtomicBool::new(false);
        activate_staged(&stage, &root, &first, &cancel).unwrap();
        assert!(!stage.exists());
        assert_eq!(discover(&root).unwrap().digest, first);
        let second_stage = root.join(".bootstrap-two");
        let second = fixture(&second_stage, "fixture-two");
        cancel.store(true, Ordering::Release);
        assert!(activate_staged(&second_stage, &root, &second, &cancel).is_err());
        assert_eq!(discover(&root).unwrap().digest, first);
        cancel.store(false, Ordering::Release);
        activate_staged(&second_stage, &root, &second, &cancel).unwrap();
        assert_eq!(discover(&root).unwrap().digest, second);
        rollback(&root).unwrap();
        fs::write(
            root.join("releases").join(&first).join("assets/hubert.pt"),
            b"corrupt",
        )
        .unwrap();
        assert!(discover(&root).is_err());
        let repair = root.join(".bootstrap-repair");
        assert_eq!(fixture(&repair, "fixture-one"), first);
        activate_staged(&repair, &root, &first, &cancel).unwrap();
        assert_eq!(discover(&root).unwrap().digest, first);
        fs::remove_dir_all(root).unwrap();
    }
    fn fixture(root: &Path, version: &str) -> String {
        let contents: Vec<(&str, &[u8])> = vec![
            ("python/python.exe", b"test fixture, never executed"),
            ("bridge.py", include_bytes!("../../../ai-sidecar/bridge.py")),
            ("server/voice_changer/RVC/RVCr2.py", b"fixture"),
            ("assets/hubert.pt", b"fixture, not a model"),
            ("NOTICE.txt", b"fixture notice"),
        ];
        let mut files = Vec::new();
        for (name, bytes) in contents {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
            files.push(RuntimeFile {
                path: name.into(),
                sha256: format!("{:x}", Sha256::digest(bytes)),
                bytes: bytes.len() as u64,
            });
        }
        let manifest = RuntimeManifest {
            schema_version: 1,
            protocol_version: PROTOCOL_VERSION,
            upstream_commit: UPSTREAM_COMMIT.into(),
            version: version.into(),
            python_executable: "python/python.exe".into(),
            entry_point: "bridge.py".into(),
            server_directory: "server".into(),
            content_vec_model: "assets/hubert.pt".into(),
            rmvpe_model: None,
            notices: vec!["NOTICE.txt".into()],
            files,
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        fs::write(root.join("runtime-manifest.json"), &bytes).unwrap();
        format!("{:x}", Sha256::digest(bytes))
    }
    #[test]
    fn installs_cancels_rejects_corruption_and_rolls_back_without_running_code() {
        let root = std::env::temp_dir().join(format!(
            "vm-runtime-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let managed = root.join("managed");
        let first = fixture(&source, "fixture-1");
        let cancel = AtomicBool::new(false);
        let mut progress = 0;
        install(&source, &managed, &first, &cancel, |n, _| progress = n).unwrap();
        assert_eq!(progress, 5);
        assert_eq!(discover(&managed).unwrap().digest, first);
        let second = fixture(&source, "fixture-2");
        cancel.store(true, Ordering::Release);
        assert!(install(&source, &managed, &second, &cancel, |_, _| {}).is_err());
        assert_eq!(discover(&managed).unwrap().digest, first);
        cancel.store(false, Ordering::Release);
        fs::write(source.join("assets/hubert.pt"), b"corrupted").unwrap();
        assert!(install(&source, &managed, &second, &cancel, |_, _| {}).is_err());
        assert_eq!(discover(&managed).unwrap().digest, first);
        let second = fixture(&source, "fixture-2");
        install(&source, &managed, &second, &cancel, |_, _| {}).unwrap();
        assert_eq!(discover(&managed).unwrap().digest, second);
        rollback(&managed).unwrap();
        assert_eq!(discover(&managed).unwrap().digest, first);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_cross_platform_traversal() {
        for p in [
            "../x", "/x", "C:/x", "a\\b", "a//b", "a/../b", "x.", "x ", ".",
        ] {
            assert!(validate_relative(p).is_err(), "{p}");
        }
        assert!(validate_relative("python/Lib/a.py").is_ok());
    }
    #[test]
    fn requires_external_publisher_digest() {
        let p = std::env::temp_dir();
        assert!(
            read_manifest(&p, "not-a-hash")
                .unwrap_err()
                .contains("publisher")
        );
    }
}
