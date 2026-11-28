//! Reproducible recommended runtime assembly from an app-pinned official-source recipe.
//! All network/archive work stays off the audio callback and the Tauri UI thread.
use super::runtime::{self, RuntimeFile, RuntimeManifest, VerifiedRuntime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const MAX_DOWNLOAD: u64 = 8 * 1024 * 1024 * 1024;
const MAX_EXPANDED: u64 = 12 * 1024 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    pub kind: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recipe {
    pub schema_version: u32,
    pub version: String,
    pub upstream_commit: String,
    pub platform: String,
    pub python_version: String,
    pub notes: String,
    pub components: Vec<Component>,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub component: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub completed_components: usize,
    pub total_components: usize,
}

pub fn recipe() -> Result<Recipe, String> {
    let value: Recipe =
        serde_json::from_str(include_str!("../../../ai-sidecar/runtime-recipe.json"))
            .map_err(|e| e.to_string())?;
    if value.schema_version != 1
        || value.platform != "windows-x86_64"
        || value.upstream_commit != super::config::UPSTREAM_COMMIT
        || value.components.len() > 100
    {
        return Err("Incompatible embedded runtime recipe".into());
    }
    for item in &value.components {
        validate_component(item)?;
    }
    Ok(value)
}
fn validate_component(item: &Component) -> Result<(), String> {
    let url = reqwest::Url::parse(&item.url).map_err(|e| e.to_string())?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(
            url.host_str(),
            Some(
                "www.python.org"
                    | "codeload.github.com"
                    | "files.pythonhosted.org"
                    | "download.pytorch.org"
                    | "download-r2.pytorch.org"
                    | "dl.fbaipublicfiles.com"
                    | "tyc.rei-yumesaki.net"
                    | "huggingface.co"
            )
        )
        || item.bytes > MAX_DOWNLOAD
        || item.sha256.len() != 64
        || !item.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Runtime component is not a bounded official-source artifact".into());
    }
    Ok(())
}
fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Runtime download/installation cancelled; verified cache retained".into())
    } else {
        Ok(())
    }
}
fn digest(path: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut block = [0u8; 65536];
    loop {
        cancelled(cancel)?;
        let n = file.read(&mut block).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&block[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Cache names derive from the pinned digest, never from an untrusted URL filename.
pub fn download(
    item: &Component,
    cache: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<PathBuf, String> {
    validate_component(item)?;
    fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let final_path = cache.join(&item.sha256);
    if final_path.is_file() && digest(&final_path, cancel)? == item.sha256 {
        progress(fs::metadata(&final_path).map_err(|e| e.to_string())?.len());
        return Ok(final_path);
    }
    let partial = cache.join(format!("{}.part", item.sha256));
    if partial.exists() {
        runtime::package_path(cache, &format!("{}.part", item.sha256))?;
    }
    let mut offset = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
    if item.bytes > 0 && offset == item.bytes {
        if digest(&partial, cancel)? == item.sha256 {
            runtime::replace_file(&partial, &final_path)?;
            progress(offset);
            return Ok(final_path);
        }
        // A completed but corrupt partial must not get stuck on HTTP 416 forever.
        fs::File::create(&partial).map_err(|e| e.to_string())?;
        offset = 0;
    }
    if offset > MAX_DOWNLOAD || (item.bytes > 0 && offset > item.bytes) {
        return Err(
            "Invalid partial download length; clear this component cache explicitly".into(),
        );
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.get(&item.url);
    if offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    let mut response = request
        .send()
        .map_err(|e| format!("{} download failed: {e}", item.name))?;
    if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        let range = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .ok_or("Missing partial response range")?;
        if !range.starts_with(&format!("bytes {offset}-")) {
            return Err("Download resume range mismatch".into());
        }
    } else if response.status().is_success() {
        offset = 0;
    } else {
        return Err(format!("{} download HTTP {}", item.name, response.status()));
    }
    let mut output = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(offset == 0)
        .append(offset > 0)
        .open(&partial)
        .map_err(|e| e.to_string())?;
    let mut count = offset;
    let mut block = [0u8; 65536];
    let mut update = Instant::now();
    loop {
        cancelled(cancel)?;
        let n = response.read(&mut block).map_err(|e| {
            format!("Download interrupted; retry resumes verified-length cache: {e}")
        })?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > MAX_DOWNLOAD || (item.bytes > 0 && count > item.bytes) {
            return Err("Download exceeds pinned size".into());
        }
        output.write_all(&block[..n]).map_err(|e| e.to_string())?;
        if update.elapsed() > Duration::from_millis(100) {
            progress(count);
            update = Instant::now();
        }
    }
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    if (item.bytes > 0 && count != item.bytes) || digest(&partial, cancel)? != item.sha256 {
        return Err("Runtime artifact size/SHA-256 mismatch; activation refused".into());
    }
    runtime::replace_file(&partial, &final_path)?;
    progress(count);
    Ok(final_path)
}

fn ignored(relative: &str) -> bool {
    relative.split('/').any(|part| {
        matches!(
            part,
            "__pycache__"
                | "include"
                | "Include"
                | "test"
                | "tests"
                | "benchmarks"
                | "Scripts"
                | "Beatrice"
        )
    }) || relative.ends_with(".pyc")
}
fn mapped(kind: &str, name: &str) -> Result<Option<String>, String> {
    let name = name.trim_end_matches('/');
    if name.is_empty() {
        return Ok(None);
    }
    // Raw paths may be longer than the final extracted path, but never ambiguous.
    if name.len() > 1024
        || name.contains(['\\', ':'])
        || name.chars().any(char::is_control)
        || name.starts_with('/')
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
    {
        return Err("Unsafe runtime archive member".into());
    }
    let result = match kind {
        "python" => Some(format!("python/{name}")),
        "wheel" => {
            if ignored(name) {
                return Ok(None);
            }
            if let Some((_, path)) = name
                .split_once(".data/purelib/")
                .or_else(|| name.split_once(".data/platlib/"))
            {
                Some(format!("python/Lib/site-packages/{path}"))
            } else if name.contains(".data/") {
                None
            } else {
                Some(format!("python/Lib/site-packages/{name}"))
            }
        }
        "upstream" => {
            let prefix = format!("voice-changer-{}/", super::config::UPSTREAM_COMMIT);
            let Some(path) = name.strip_prefix(&prefix) else {
                return Ok(None);
            };
            if path == "LICENSE" {
                Some("notices/w-okada-LICENSE.txt".into())
            } else if path == "LICENSE-NOTICE" {
                Some("notices/w-okada-NOTICE.txt".into())
            } else if path.starts_with("server/") && !ignored(path) {
                Some(path.into())
            } else {
                None
            }
        }
        "fairseq" => {
            let Some(path) = name.strip_prefix("fairseq-0.12.2/") else {
                return Ok(None);
            };
            if path == "LICENSE" {
                Some("notices/fairseq-LICENSE.txt".into())
            } else if path.starts_with("fairseq/")
                && !ignored(path)
                && [".py", ".yaml", ".yml", ".json", ".txt"]
                    .iter()
                    .any(|ext| path.ends_with(ext))
            {
                Some(format!("python/Lib/site-packages/{path}"))
            } else {
                None
            }
        }
        "antlr" => name
            .split_once("/src/")
            .map(|(_, path)| format!("python/Lib/site-packages/{path}")),
        _ => return Err("Unsupported component archive kind".into()),
    };
    if let Some(path) = &result {
        runtime::validate_relative(path)?;
    }
    Ok(result)
}
fn write_member(
    root: &Path,
    relative: &str,
    size: u64,
    reader: &mut impl Read,
    expanded: &mut u64,
    cancel: &AtomicBool,
) -> Result<(), String> {
    cancelled(cancel)?;
    runtime::validate_relative(relative)?;
    if size > 2 * 1024 * 1024 * 1024 {
        return Err("Runtime member exceeds 2 GiB".into());
    }
    *expanded = expanded
        .checked_add(size)
        .ok_or("Runtime archive size overflow")?;
    if *expanded > MAX_EXPANDED {
        return Err("Runtime archives exceed 12 GiB expanded".into());
    }
    let target = root.join(relative);
    fs::create_dir_all(target.parent().ok_or("Invalid member target")?)
        .map_err(|e| e.to_string())?;
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&target)
        .map_err(|e| format!("Duplicate/invalid archive member {relative}: {e}"))?;
    let mut copied = 0u64;
    let mut block = [0u8; 65536];
    loop {
        cancelled(cancel)?;
        let n = reader.read(&mut block).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > size {
            return Err("Archive member grew beyond declared size".into());
        }
        output.write_all(&block[..n]).map_err(|e| e.to_string())?;
    }
    if copied != size {
        return Err("Truncated archive member".into());
    }
    Ok(())
}
pub(super) fn extract(
    item: &Component,
    archive: &Path,
    root: &Path,
    expanded: &mut u64,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if matches!(item.kind.as_str(), "hubert" | "rvc-contentvec") {
        let mut input = fs::File::open(archive).map_err(|e| e.to_string())?;
        let destination = if item.kind == "hubert" {
            "assets/hubert.pt"
        } else {
            runtime::RVC_CONTENTVEC_FILE
        };
        return write_member(
            root,
            destination,
            fs::metadata(archive).map_err(|e| e.to_string())?.len(),
            &mut input,
            expanded,
            cancel,
        );
    }
    if matches!(item.kind.as_str(), "fairseq" | "antlr") {
        let input = fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(input));
        for entry in archive.entries().map_err(|e| e.to_string())? {
            cancelled(cancel)?;
            let mut entry = entry.map_err(|e| e.to_string())?;
            if !entry.header().entry_type().is_file() {
                if entry.header().entry_type().is_dir() {
                    continue;
                }
                return Err("Non-regular TAR entry rejected".into());
            }
            let name = entry
                .path()
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("Non-UTF8 TAR path")?
                .to_owned();
            if let Some(path) = mapped(&item.kind, &name)? {
                write_member(root, &path, entry.size(), &mut entry, expanded, cancel)?;
            }
        }
    } else {
        let mut archive = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if archive.len() > 100000 {
            return Err("Too many ZIP archive members".into());
        }
        for index in 0..archive.len() {
            cancelled(cancel)?;
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            if entry.is_dir() {
                continue;
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err("ZIP symbolic link rejected".into());
            }
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            if let Some(path) = mapped(&item.kind, &name)? {
                write_member(root, &path, entry.size(), &mut entry, expanded, cancel)?;
            }
        }
    }
    Ok(())
}

pub(super) fn collect(
    root: &Path,
    directory: &Path,
    files: &mut Vec<RuntimeFile>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        cancelled(cancel)?;
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            collect(root, &path, files, cancel)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("Invalid runtime path")?
                .replace('\\', "/");
            runtime::package_path(root, &relative)?;
            files.push(RuntimeFile {
                path: relative,
                sha256: digest(&path, cancel)?,
                bytes: fs::metadata(path).map_err(|e| e.to_string())?.len(),
            });
            if files.len() > 20000 {
                return Err("Recommended runtime exceeds file limit".into());
            }
        }
    }
    Ok(())
}
pub fn install(
    root: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Result<VerifiedRuntime, String> {
    let recipe = recipe()?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut available = 0u64;
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                path.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if available < 13 * 1024 * 1024 * 1024 {
            return Err("Recommended runtime needs at least 13 GiB free on its volume for downloads, staging and recovery".into());
        }
    }
    let staging = root.join(format!(
        ".bootstrap-{}-{}",
        std::process::id(),
        super::process::now_ms()
    ));
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut state = Progress {
            total_bytes: recipe.components.iter().map(|c| c.bytes).sum(),
            total_components: recipe.components.len(),
            ..Default::default()
        };
        let mut expanded = 0u64;
        let mut completed = 0u64;
        for (index, item) in recipe.components.iter().enumerate() {
            state.phase = "downloading".into();
            state.component = item.name.clone();
            state.completed_components = index;
            let archive = download(item, &root.join("downloads"), cancel, |bytes| {
                state.completed_bytes = completed + bytes;
                progress(state.clone());
            })?;
            state.phase = "extracting".into();
            progress(state.clone());
            extract(item, &archive, &staging, &mut expanded, cancel)?;
            completed += fs::metadata(archive).map_err(|e| e.to_string())?.len();
        }
        state.phase = "verifying".into();
        state.completed_components = state.total_components;
        progress(state.clone());
        fs::write(
            staging.join("python/python310._pth"),
            "python310.zip\n.\nLib/site-packages\nimport site\n",
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            staging.join("bridge.py"),
            include_bytes!("../../../ai-sidecar/bridge.py"),
        )
        .map_err(|e| e.to_string())?;
        fs::create_dir_all(staging.join("notices")).map_err(|e| e.to_string())?;
        fs::write(
            staging.join("notices/runtime-recipe.json"),
            include_bytes!("../../../ai-sidecar/runtime-recipe.json"),
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            staging.join("notices/hubert-LICENSE.txt"),
            include_bytes!("../../../ai-sidecar/notices/hubert-LICENSE.txt"),
        )
        .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        collect(&staging, &staging, &mut files, cancel)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let manifest = RuntimeManifest {
            schema_version: 1,
            protocol_version: super::config::PROTOCOL_VERSION,
            upstream_commit: recipe.upstream_commit,
            version: recipe.version,
            python_executable: "python/python.exe".into(),
            entry_point: "bridge.py".into(),
            server_directory: "server".into(),
            content_vec_model: "assets/hubert.pt".into(),
            rmvpe_model: None,
            notices: files
                .iter()
                .filter(|f| f.path.starts_with("notices/"))
                .map(|f| f.path.clone())
                .collect(),
            files,
        };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        fs::write(staging.join("runtime-manifest.json"), &bytes).map_err(|e| e.to_string())?;
        let expected = format!("{:x}", Sha256::digest(&bytes));
        state.phase = "installing".into();
        progress(state.clone());
        runtime::activate_staged(&staging, root, &expected, cancel)
    })();
    // Only this newly created contained staging folder is removed. Verified cache survives.
    if staging
        .canonicalize()
        .ok()
        .is_some_and(|p| root.canonicalize().ok().is_some_and(|r| p.starts_with(r)))
    {
        let _ = fs::remove_dir_all(staging);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn complete_partial_cache_is_verified_and_promoted_without_network() {
        let root = std::env::temp_dir().join(format!(
            "vm-cache-test-{}-{}",
            std::process::id(),
            super::super::process::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let payload = b"verified-cache";
        let expected = format!("{:x}", Sha256::digest(payload));
        let item = Component {
            name: "test inert cache".into(),
            url: "https://www.python.org/fixture.zip".into(),
            sha256: expected.clone(),
            bytes: payload.len() as u64,
            kind: "python".into(),
        };
        fs::write(root.join(format!("{expected}.part")), payload).unwrap();
        fs::write(root.join(&expected), b"corrupt-old-cache").unwrap();
        let cancel = AtomicBool::new(false);
        let result = download(&item, &root, &cancel, |_| {}).unwrap();
        assert_eq!(fs::read(result).unwrap(), payload);
        assert!(!root.join(format!("{expected}.part")).exists());
        cancel.store(true, Ordering::Release);
        assert!(download(&item, &root, &cancel, |_| {}).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn real_zip_traversal_is_rejected_before_writing() {
        let root = std::env::temp_dir().join(format!(
            "vm-zip-test-{}-{}",
            std::process::id(),
            super::super::process::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let zip =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/bootstrap-unsafe.zip");
        let item = Component {
            name: "inert security fixture".into(),
            url: "https://www.python.org/fixture.zip".into(),
            sha256: "0".repeat(64),
            bytes: 1,
            kind: "wheel".into(),
        };
        assert!(extract(&item, &zip, &root, &mut 0, &AtomicBool::new(false)).is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn recipe_and_archive_paths_are_constrained() {
        let value = recipe().unwrap();
        assert!(value.components.len() > 10);
        let mut names = HashSet::new();
        for item in value.components {
            assert!(names.insert(item.name));
        }
        for path in [
            "../escape",
            "x/../../escape",
            "C:/evil",
            "x\\evil",
            "/absolute",
            "x./evil",
        ] {
            assert!(mapped("wheel", path).is_err());
        }
        assert_eq!(
            mapped("wheel", "example.data/purelib/lib.py")
                .unwrap()
                .unwrap(),
            "python/Lib/site-packages/lib.py"
        );
        assert!(mapped("wheel", "torch/include/a.h").unwrap().is_none());
    }
    #[test]
    #[ignore = "manual approved multi-GB runtime assembly and real hardware probe"]
    fn assembles_recommended_runtime_and_probes_real_dependencies() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.ai-runtime-build/bootstrap-installed");
        let cancel = AtomicBool::new(false);
        let mut last = String::new();
        let runtime = install(&root, &cancel, |p| {
            let next = format!("{} {}", p.phase, p.component);
            if next != last {
                println!(
                    "{next}: {} / {}",
                    p.completed_components, p.total_components
                );
                last = next;
            }
        })
        .unwrap();
        let mut process = super::super::process::ManagedProcess::spawn(
            &runtime,
            &root.join("probe.log"),
            std::sync::Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        let result = process
            .call(serde_json::json!({"op":"hello"}), Duration::from_secs(90))
            .unwrap();
        assert_eq!(
            result["upstreamCommit"],
            super::super::config::UPSTREAM_COMMIT
        );
        assert!(
            result["providers"]
                .as_array()
                .is_some_and(|p| !p.is_empty())
        );
        println!(
            "Actual assembled runtime: {}; digest {}; probe {}",
            runtime.root.display(),
            runtime.digest,
            result
        );
    }
}
