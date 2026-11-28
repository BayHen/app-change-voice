//! A small, separately pinned CPU conversion overlay. The CUDA inference release
//! is never modified. Wheels are downloaded once and verified before extraction.
use super::{
    bootstrap::{self, Component},
    runtime::{self, RuntimeFile},
    validator::sha256_file,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
pub fn ensure(
    root: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&str, u64, u64),
) -> Result<PathBuf, String> {
    let packages=[Component{name:"PyTorch 2.10.0 CPU converter".into(),url:"https://download-r2.pytorch.org/whl/cpu/torch-2.10.0%2Bcpu-cp310-cp310-win_amd64.whl".into(),sha256:"6c6f0df770144907092a0d067048d96ed4f278a6c840376d2ff0e27e7579b925".into(),bytes:113634294,kind:"wheel".into()},Component{name:"SymPy 1.14.0 converter dependency".into(),url:"https://files.pythonhosted.org/packages/a2/09/77d55d46fd61b4a135c444fc97158ef34a095e5681d0a6c10b75bf356191/sympy-1.14.0-py3-none-any.whl".into(),sha256:"e091cc3e99d2141a0ba2847328f5479b05d94a6635cb96148ccb3f34671bd8f5".into(),bytes:6299353,kind:"wheel".into()}];
    let target = root.join("converter-torch210-cpu");
    if target.exists() {
        let manifest = target.join("converter-files.json");
        if manifest
            .metadata()
            .map_err(|_| "Converter cache is incomplete; retry with a repaired converter cache")?
            .len()
            > 8 * 1024 * 1024
        {
            return Err("Invalid converter cache manifest".into());
        }
        let files: Vec<RuntimeFile> =
            serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if files.is_empty() || files.len() > 50000 {
            return Err("Invalid converter cache file list".into());
        }
        for file in files {
            if cancel.load(Ordering::Acquire) {
                return Err("Converter verification cancelled".into());
            }
            let path = runtime::package_path(&target, &file.path)?;
            if path.metadata().map_err(|e| e.to_string())?.len() != file.bytes
                || sha256_file(&path)? != file.sha256
            {
                return Err(format!("Converter cache integrity failed: {}", file.path));
            }
        }
        return Ok(target.join("python/Lib/site-packages"));
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let stage = root.join(format!(
        ".converter-stage-{}-{}",
        std::process::id(),
        super::process::now_ms()
    ));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let mut expanded = 0;
    for package in packages {
        let archive = bootstrap::download(&package, &root.join("downloads"), cancel, |n| {
            progress(&package.name, n, package.bytes)
        })?;
        bootstrap::extract(&package, &archive, &stage, &mut expanded, cancel)?;
    }
    let mut files = Vec::new();
    bootstrap::collect(&stage, &stage, &mut files, cancel)?;
    fs::write(
        stage.join("converter-files.json"),
        serde_json::to_vec_pretty(&files).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&stage, &target).map_err(|e| e.to_string())?;
    Ok(target.join("python/Lib/site-packages"))
}
