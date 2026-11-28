//! Private, bounded stdio RPC. No REST listener, browser surface or loopback port is opened.
use super::{
    config::PROTOCOL_VERSION,
    runtime::{VerifiedRuntime, package_path},
};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, sync_channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
// -I ignores PYTHONIOENCODING/PYTHONUTF8 environment variables. Explicit UTF-8
// mode is required for JSON paths on Windows legacy code-page installations.
const PYTHON_ARGS: &[&str] = &["-I", "-X", "utf8", "-u"];
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

pub fn read_message(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut line = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().map_err(|e| e.to_string())?;
        if available.is_empty() {
            return Err("AI runtime closed its protocol pipe".into());
        }
        let length = available
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(available.len());
        if line.len() + length > MAX_MESSAGE_BYTES {
            return Err("AI protocol message exceeds 256 KiB".into());
        }
        let complete = available[length - 1] == b'\n';
        line.extend_from_slice(&available[..length]);
        reader.consume(length);
        if complete {
            break;
        }
    }
    serde_json::from_slice(&line).map_err(|_| "Invalid AI protocol JSON".into())
}

#[cfg(windows)]
struct ProcessJob(isize);
#[cfg(windows)]
impl ProcessJob {
    fn attach(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        limits.ProcessMemoryLimit = 8 * 1024 * 1024 * 1024;
        if unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
            || unsafe { AssignProcessToJobObject(handle, child.as_raw_handle() as _) } == 0
        {
            let error = std::io::Error::last_os_error().to_string();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(Self(handle as isize))
    }
}
#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0 as _) };
    }
}

pub struct ManagedProcess {
    child: Arc<Mutex<Child>>,
    input: Option<ChildStdin>,
    response: Receiver<Result<Value, String>>,
    deadline: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
    reader: Option<JoinHandle<()>>,
    logger: Option<JoinHandle<()>>,
    next_id: u64,
    #[cfg(windows)]
    job: Option<ProcessJob>,
}
impl ManagedProcess {
    pub fn spawn(
        runtime: &VerifiedRuntime,
        log: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let entry = package_path(&runtime.root, &runtime.manifest.entry_point)?;
        Self::spawn_entry(runtime, &entry, log, cancel, None)
    }
    pub fn spawn_converter(
        runtime: &VerifiedRuntime,
        work: &Path,
        cancel: Arc<AtomicBool>,
        safe_torch: &Path,
    ) -> Result<Self, String> {
        fs::create_dir_all(work).map_err(|e| e.to_string())?;
        let script = work.join("convert_worker.py");
        fs::write(
            &script,
            include_bytes!("../../../ai-sidecar/convert_worker.py"),
        )
        .map_err(|e| e.to_string())?;
        let entry = script.canonicalize().map_err(|e| e.to_string())?;
        Self::spawn_entry(
            runtime,
            &entry,
            &work.join("conversion.log"),
            cancel,
            Some(safe_torch),
        )
    }
    pub fn spawn_indexed(
        runtime: &VerifiedRuntime,
        slot: &Path,
        log: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let entry = slot.join("voicemorph-indexed-bridge.py");
        fs::write(&entry, include_bytes!("../../../ai-sidecar/bridge.py"))
            .map_err(|e| e.to_string())?;
        Self::spawn_entry(runtime, &entry, log, cancel, None)
    }
    fn spawn_entry(
        runtime: &VerifiedRuntime,
        entry: &Path,
        log: &Path,
        cancel: Arc<AtomicBool>,
        safe_torch: Option<&Path>,
    ) -> Result<Self, String> {
        let python = package_path(&runtime.root, &runtime.manifest.python_executable)?;
        let mut command = Command::new(python);
        command
            .args(PYTHON_ARGS)
            .arg(entry)
            .arg("--runtime-root")
            .arg(&runtime.root)
            .current_dir(&runtime.root);
        if let Some(path) = safe_torch {
            command.arg("--safe-torch-root").arg(path);
        }
        command.env_clear();
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        // platformdirs' Windows KnownFolder lookup fails without a user profile.
        // Use its supported overrides and an app-owned cache profile, rather than
        // inheriting user credentials/environment or writing into the runtime release.
        let profile = log
            .parent()
            .unwrap_or(Path::new("."))
            .join("runtime-profile");
        fs::create_dir_all(profile.join("Local")).map_err(|e| e.to_string())?;
        fs::create_dir_all(profile.join("Roaming")).map_err(|e| e.to_string())?;
        let profile = profile.canonicalize().map_err(|e| e.to_string())?;
        command
            .env("USERPROFILE", &profile)
            .env("HOME", &profile)
            .env("LOCALAPPDATA", profile.join("Local"))
            .env("APPDATA", profile.join("Roaming"))
            .env("WIN_PD_OVERRIDE_LOCAL_APPDATA", profile.join("Local"))
            .env("WIN_PD_OVERRIDE_APPDATA", profile.join("Roaming"))
            .env("TORCH_HOME", profile.join("torch"))
            .env("HF_HOME", profile.join("huggingface"))
            .env("PYTHONIOENCODING", "utf-8")
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env(
                "NUMBA_CACHE_DIR",
                log.parent().unwrap_or(Path::new(".")).join("numba-cache"),
            )
            .env("OMP_NUM_THREADS", "4")
            .env("OPENBLAS_NUM_THREADS", "4");
        Self::spawn_command(command, log, cancel)
    }
    fn spawn_command(
        mut command: Command,
        log: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("AI runtime could not start: {e}"))?;
        #[cfg(windows)]
        let job = match ProcessJob::attach(&child) {
            Ok(job) => job,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Cannot isolate AI process in a Windows job: {e}"));
            }
        };
        let input = child.stdin.take().ok_or("Missing runtime input pipe")?;
        let stdout = child.stdout.take().ok_or("Missing runtime output pipe")?;
        let mut stderr = child.stderr.take().ok_or("Missing runtime log pipe")?;
        let (sender, response) = sync_channel(2);
        let reader = thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            loop {
                let result = read_message(&mut stdout);
                let failed = result.is_err();
                if sender.try_send(result).is_err() || failed {
                    break;
                }
            }
        });
        let log = log.to_path_buf();
        let logger = thread::spawn(move || {
            if let Some(parent) = log.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::metadata(&log).is_ok_and(|m| m.len() > 4 * 1024 * 1024) {
                let _ = fs::rename(&log, log.with_extension("previous.log"));
            }
            let mut file = OpenOptions::new().create(true).append(true).open(log).ok();
            let mut written = file
                .as_ref()
                .and_then(|f| f.metadata().ok())
                .map(|m| m.len())
                .unwrap_or(0);
            let mut buffer = [0_u8; 4096];
            while let Ok(length) = stderr.read(&mut buffer) {
                if length == 0 {
                    break;
                }
                if written < 4 * 1024 * 1024 {
                    if let Some(file) = file.as_mut() {
                        let _ = file.write_all(&buffer[..length]);
                    }
                    written += length as u64;
                }
            }
        });
        let child = Arc::new(Mutex::new(child));
        let deadline = Arc::new(AtomicU64::new(0));
        let watch_child = child.clone();
        let watch_deadline = deadline.clone();
        let watch_cancel = cancel.clone();
        let watcher = thread::spawn(move || {
            loop {
                if let Ok(mut child) = watch_child.lock() {
                    if child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    let limit = watch_deadline.load(Ordering::Acquire);
                    if watch_cancel.load(Ordering::Acquire) || (limit != 0 && now_ms() >= limit) {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        Ok(Self {
            child,
            input: Some(input),
            response,
            deadline,
            cancel,
            watcher: Some(watcher),
            reader: Some(reader),
            logger: Some(logger),
            next_id: 0,
            #[cfg(windows)]
            job: Some(job),
        })
    }
    pub fn call(&mut self, mut request: Value, timeout: Duration) -> Result<Value, String> {
        if self.cancel.load(Ordering::Acquire) {
            return Err("AI operation cancelled".into());
        }
        self.next_id = self.next_id.wrapping_add(1);
        request["id"] = self.next_id.into();
        request["protocol"] = PROTOCOL_VERSION.into();
        let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        if bytes.len() + 1 > MAX_MESSAGE_BYTES {
            return Err("AI request exceeds the protocol limit".into());
        }
        bytes.push(b'\n');
        // The watchdog kills a hung child even when its stdin stops consuming a write.
        self.deadline
            .store(now_ms() + timeout.as_millis() as u64, Ordering::Release);
        let result = (|| {
            self.input
                .as_mut()
                .ok_or("Runtime is stopped")?
                .write_all(&bytes)
                .map_err(|e| e.to_string())?;
            let response = self
                .response
                .recv_timeout(timeout)
                .map_err(|_| "AI operation timed out or the sidecar crashed".to_string())??;
            if response["protocol"].as_u64() != Some(PROTOCOL_VERSION as u64)
                || response["id"].as_u64() != Some(self.next_id)
            {
                return Err("AI response protocol/sequence mismatch".into());
            }
            if response["ok"].as_bool() != Some(true) {
                return Err(response["error"]
                    .as_str()
                    .unwrap_or("AI request failed")
                    .chars()
                    .take(1024)
                    .collect());
            }
            Ok(response)
        })();
        self.deadline.store(0, Ordering::Release);
        result
    }
    pub fn exited(&self) -> bool {
        self.child
            .lock()
            .map(|mut c| c.try_wait().ok().flatten().is_some())
            .unwrap_or(true)
    }
    pub fn process_id(&self) -> Option<u32> {
        self.child.lock().ok().map(|child| child.id())
    }
}
impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if !self.cancel.load(Ordering::Acquire) && !self.exited() {
            let _ = self.call(
                serde_json::json!({"op":"shutdown"}),
                Duration::from_millis(500),
            );
        }
        self.input.take();
        self.deadline.store(1, Ordering::Release);
        #[cfg(windows)]
        self.job.take();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(h) = self.watcher.take() {
            let _ = h.join();
        }
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        if let Some(h) = self.logger.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_rejects_oversized_and_non_json_lines() {
        assert!(
            read_message(&mut std::io::Cursor::new(vec![b'x'; MAX_MESSAGE_BYTES + 1])).is_err()
        );
        assert!(read_message(&mut std::io::Cursor::new(b"not JSON\n")).is_err());
    }
    #[test]
    fn real_child_roundtrip_timeout_and_cleanup() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/ai_sidecar_stub.py");
        let mut command = Command::new("python");
        command.args(PYTHON_ARGS).arg(fixture);
        let cancel = Arc::new(AtomicBool::new(false));
        let log = std::env::temp_dir().join(format!("vm-child-{}.log", now_ms()));
        let mut process = ManagedProcess::spawn_command(command, &log, cancel).unwrap();
        assert_eq!(
            process
                .call(serde_json::json!({"op":"hello"}), Duration::from_secs(3))
                .unwrap()["testFixture"],
            true
        );
        let unicode_path = format!(
            "C:/My recordings/gi{}ng n{}.onnx",
            char::from_u32(0x1ecd).unwrap(),
            char::from_u32(0x1eef).unwrap()
        );
        assert_eq!(
            process
                .call(
                    serde_json::json!({"op":"echo","text":unicode_path}),
                    Duration::from_secs(3)
                )
                .unwrap()["receivedText"],
            unicode_path
        );
        assert!(
            process
                .call(serde_json::json!({"op":"hang"}), Duration::from_millis(100))
                .is_err()
        );
        let child = process.child.clone();
        drop(process);
        assert!(child.lock().unwrap().try_wait().unwrap().is_some());
        let _ = fs::remove_file(log);
    }
}
