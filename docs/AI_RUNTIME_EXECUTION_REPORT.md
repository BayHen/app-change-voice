# VoiceMorph managed AI runtime — real execution report

Branch: `feature/new-version`. Work performed 2026-11-25/26, Asia/Saigon, following the
user's explicit runtime installation/execution authorization. This supersedes the
unavailable-assets checkpoint in `AI_ENGINE_IMPLEMENTATION.md` and `AI_ENGINE_VERIFICATION.md`.

## Implemented behavior

AI Voice Studio and Settings share the navy/purple design system, sidebar, status bar,
runtime installation/repair/update, progress/cancel, hardware detection and model controls.
No frontend dependencies were added for runtime management. The installed frontend-design
and ui-ux-pro-max guidance was applied to the shared components and visual review.

The recommended installer assembles 69 pinned official components (4,184,518,137 download
bytes, 3.90 GiB) into an application-owned portable runtime. It requires 13 GiB free,
verifies exact sizes/SHA-256, constrains redirects/hosts, rejects unsafe archive paths,
symlinks and nonregular TAR members, caches/resumes downloads, and atomically activates
the verified release. A failed/cancelled operation retains the previous active pointer.
Repair can quarantine a corrupt release; update recipes ship with reviewed app releases,
with rollback after changing releases. No floating remote dependency update is executed.

The app starts/stops the private Python worker, probes actual providers/hardware, loads
models only on request and performs bounded crash recovery. CPU fallback requires opt-in;
failure defaults to mute. Stop keeps AI selected/muted; Classic is restored explicitly.
The worker has an app-owned profile/cache, offline flags, cleared inherited environment,
bounded queues/watchdogs and a kill-on-close Windows Job. It opens no HTTP audio server.
Inference after installation is offline via private stdio. No microphone audio is uploaded.

The reviewed owner-model catalog is separate from generic ONNX import. It downloads the
exact owner archive, verifies the exact checkpoint, converts locally with `weights_only=True`,
and verifies the audited ONNX digest. Arbitrary voice checkpoints remain unsupported.
The HuBERT embedding loader's pickle exception is restricted to its verified packaged path.

Rust still owns capture, DSP, soundboard mixing, recording and virtual output. Existing
CPAL/Signalsmith/routing code and MASK reference architecture were preserved. The upstream
web server/manager and its audio-device ownership are bypassed.

## Installed environment and provenance

| Component                               | Tested value                                                      |
| --------------------------------------- | ----------------------------------------------------------------- |
| Upstream w-okada                        | `7e2ce5c4800010bbc7604968bd29b802534fac05`                        |
| Recipe                                  | `w-okada-vm-cuda124-2026.11.25.1`                                 |
| Python                                  | 3.10.11 x64, official embeddable distribution; **end of support** |
| PyTorch / torchaudio                    | 2.6.0+cu124 / 2.6.0+cu124                                         |
| ONNX Runtime GPU / ONNX                 | 1.21.0 / 1.17.0                                                   |
| NumPy / SciPy / resampy                 | 1.23.5 / 1.10.1 / 0.4.2                                           |
| fairseq / librosa / faiss-cpu / pyworld | 0.12.2 inference modules / 0.9.2 / 1.7.4 / 0.3.4                  |
| numba / llvmlite                        | 0.57.1 / 0.40.1                                                   |
| Hardware                                | RTX 3080, 10,240 MiB; 16 logical CPU threads                      |
| NVIDIA driver / Torch CUDA              | 610.88 / 12.4; no system toolkit/driver changed                   |
| Provider selection exercised            | CUDA and CPU; TensorRT listed by ORT but not selected/tested      |

The [complete recipe](../ai-sidecar/runtime-recipe.json) records all 69 URLs, versions,
sizes and hashes. Sources are Python.org, official PyTorch indexes, PyPI files,
GitHub's pinned source archive, Facebook's original model host and the original voice owner.
The pinned clone is `.references/w-okada`; the project-scoped developer Python is
`.ai-runtime-build/python310` (PSF installer Authenticode valid). The host Python 3.14
and PATH were not changed. The end-user installer uses embedded Python/wheel extraction,
not this developer installation, pip, a compiler or a terminal.

The fairseq PyPI source is missing `fairseq/clib/libbase/balanced_assignment.cpp`; a stock
native wheel build failed. The implemented recipe retains unmodified Python inference
modules and MIT notices while omitting unused native training extensions. Actual HuBERT
and RVC load/inference passed with this assembly. This is recorded rather than presented
as a successful stock fairseq build.

Portable acceptance runtime (9,867 verified files):

```text
D:\app\app-change-voice\.ai-runtime-build\bootstrap-installed\releases\713abbb506a839c33e82bec189ca97f44e9382fb688da139cbaba5bb2911d8cc
```

Runtime manifest SHA-256:
`713abbb506a839c33e82bec189ca97f44e9382fb688da139cbaba5bb2911d8cc`.
Native installation uses `%APPDATA%\com.voicemorph.studio\ai-runtime\releases\<digest>`;
models use `%APPDATA%\com.voicemorph.studio\ai-models`.
Developer assets, caches, audio and runtime binaries are ignored and are not committed.

## Voice rights and auxiliary assets

The model is **Tsukuyomi-chan official Normal 1 (CV. Rei Yumesaki)**, obtained from the
[original owner's RVC page](https://tyc.rei-yumesaki.net/work/software/rvc/) and
`https://tyc.rei-yumesaki.net/files/voice/tyc-rvc.zip`.
It has [custom owner terms](https://tyc.rei-yumesaki.net/work/software/rvc/terms/), identified
as `LicenseRef-TsukuyomiChan-Official`, **not MIT**. The original owner's terms permit
the reviewed private voice-conversion tests and integration subject to inherited terms
and attribution. Credit input and model as required; advocacy, attacks and reusable
voice-material use/redistribution have restrictions. The UI links the complete terms
and requires acknowledgement. This establishes rights for this reviewed voice and use,
not arbitrary imported voices or unrestricted redistribution.

Required model attribution: **Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)**.
The separately hosted w-okada Tsukuyomi sample marked “VCClient only” was read and excluded.

Input: the owner's [official sample voice](https://tyc.rei-yumesaki.net/material/voice/sample-voice/),
whose page explicitly allows voice-effect/changer demonstrations. WAV:
`https://tyc.rei-yumesaki.net/files/voice/samplevoice/tyc-samplevoice-1-titlecall.wav`.
Credit: **Tsukuyomi-chan official sample voice (CV. Rei Yumesaki)**.
This short sample is repeated for throughput testing. No live microphone was used.
Neither generated audio nor voice weights are published or bundled.

| Asset                 | SHA-256                                                            |
| --------------------- | ------------------------------------------------------------------ |
| Original owner ZIP    | `1207590e7bac95bbbdeeced54714933390d73f0f7795a8f97fa1be60e542f201` |
| Normal 1 checkpoint   | `cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020` |
| Locally exported ONNX | `d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6` |
| Owner sample WAV      | `607d1dbb201330d017308fbc841abacbe12d672688af21d047b49be2c44dedd5` |
| HuBERT base LS960     | `1703cf8d2cdc76f8c046f5f6a9bcd224e0e6caf4744cad1a1f4199c32cac8c8d` |
| fairseq source        | `34f1b18426bf3844714534162f065ab733e049597476daa35fffb4d06a92b524` |

HuBERT is the original Facebook checkpoint from
`https://dl.fbaipublicfiles.com/hubert/hubert_base_ls960.pt`; its
[publisher model card](https://huggingface.co/facebook/hubert-base-ls960) specifies Apache-2.0.
The complete [license](../ai-sidecar/notices/hubert-LICENSE.txt) and
[provenance](../ai-sidecar/notices/hubert-provenance.txt) are retained. No unknown model
or RMVPE asset was substituted. The tested F0 method is DIO.

## Verification and visual evidence

| Check                                              | Actual result                                                                                                                                  |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust default suite                                 | 103 passed, 10 ignored; no failures                                                                                                            |
| Real portable assembly/probe                       | Passed twice; latest 231.76 s, all 9,867 files verified                                                                                        |
| Real owner-model preparation                       | Passed; managed exporter reproduced the exact ONNX hash                                                                                        |
| Real Rust PCM transport/crash recovery             | Passed twice after fixes; final run recovered CUDA → opted-in CPU, 1 restart, 20 completed chunks                                              |
| Same-stream stop/start and privacy gates           | Passed in the actual transport test; no CPAL devices opened                                                                                    |
| Frontend / Python protocol tests                   | 18 / 4 passed                                                                                                                                  |
| TypeScript/Vite, ESLint, Prettier, Rust formatting | Passed                                                                                                                                         |
| Production Windows EXE + NSIS                      | Passed, optimized release build                                                                                                                |
| Browser visual checks                              | 5 pages × 7 viewports, DPR 1/1.25/1.5; no overflow/page errors                                                                                 |
| Actual native WebView2                             | Runtime installation, hardware probe, owner download/conversion, CUDA ready, explicit stop/Classic restore passed; microphone remained stopped |

The large installer acceptance reuses 60 hash-verified official artifacts downloaded
during environment preparation and downloaded the other 9 through Rust; the final
assembly rerun used all cached artifacts. Native production installation used the
verified assembled portable package through the actual import command, avoiding a
second 4 GiB download. The official voice was then downloaded/prepared through the
actual native UI. Initial harness execution had a `list_ai_models` command-name typo
after successful runtime installation; the corrected `get_ai_models` harness passed.
Network Range-resume handling is implemented; a forced mid-network interruption/resume
has not been separately exercised. Completed-part cache promotion, cancellation,
unsafe ZIP rejection, atomic activation and corrupt-release repair have tests.
Seven original hardware/manual Rust tests remain unexecuted; the three new real-AI
acceptance tests are intentionally ignored by default and were run explicitly.

Visual review compared the supplied references with the captured screens: dark navy
canvas, violet active states, compact left navigation, central voice workspace,
right-side controls/telemetry and persistent lower audio controls. The same tokens,
typography and components apply to all five pages. Runtime-specific copy includes
actual progress/errors and no fabricated audio telemetry. Captures:

- [Five-page comparison](screenshots/runtime-management/preview/all-pages.png)
- [Native CUDA-ready viewport](screenshots/runtime-management/native/ai-ready-cuda.png)
- [Original-owner terms dialog](screenshots/runtime-management/native/owner-model-terms.png)
- [Actual model preparation progress](screenshots/runtime-management/native/owner-model-progress.png)
- [Native command/status evidence](screenshots/runtime-management/native/report.json)
- [35-layout check report](screenshots/runtime-management/preview/layout-report.json)

The native report confirms actual `CABLE Input` and `CABLE In 16ch` VB-Audio endpoints
were enumerated. They were not started; this is not a loopback or conferencing test.
The scratch test EXE is byte-identical to the production executable. Owned test
processes are stopped after verification; the installed runtime/model remain available.

## Measured file-inference performance

Machine CPU: **Intel Core i5-13400F**, 10 cores / 16 logical threads; reported physical
memory 16,996,827,136 bytes (about 15.83 GiB). Each test uses the
same short owner sample repeated for about 10 s, DIO, four Torch/ORT CPU threads,
1,024 crossfade frames and 4,096 context frames. Profiles run sequentially with no
parallel build/model load. Reports retain individual hardware/provider/resource values.

| Profile                                                                                | Completed + warmup chunks | Inference median | IPC median | RTF incl. warmup | Full load RPC | Peak sampled RSS | CPU, machine normalized |
| -------------------------------------------------------------------------------------- | ------------------------- | ---------------- | ---------- | ---------------- | ------------- | ---------------- | ----------------------- |
| [CUDA, 4,096 frames](benchmarks/ai-runtime/cuda-4096.json)                             | 116 + 1                   | 28.84 ms         | 30.43 ms   | 0.625            | 9.64 s        | 2,353 MiB        | 5.27%                   |
| [CPU, 8,192 frames](benchmarks/ai-runtime/cpu-8192.json)                               | 57 + 1                    | 160.24 ms        | 163.82 ms  | 1.086            | 5.62 s        | 2,205 MiB        | 22.65%                  |
| [CUDA unavailable → opted-in CPU, 8,192](benchmarks/ai-runtime/cpu-fallback-8192.json) | 57 + 1                    | 181.18 ms        | 185.93 ms  | 1.190            | 5.92 s        | 2,202 MiB        | 22.35%                  |

RTF below 1 means this sequential file test ran faster than the input duration; it is
not a live-device guarantee. CPU profiles did **not** sustain real time including warmup.
The fallback test hides CUDA only in its child environment; the report confirms actual
CPU execution, while the separate Rust test kills the owned CUDA process and recovers
the real bounded transport. These are complementary tests, not simulated inference.

For CUDA, sampled whole-device GPU utilization ranged 4–40%, with up to 2,737 MiB
whole-device VRAM occupied during load/inference. Those figures include the desktop,
other applications and ORT; they are **not per-process GPU utilization/VRAM**. Torch-only
CUDA allocation peaked at 381,396,480 bytes and reserved memory at 461,373,440 bytes;
these counters exclude ORT allocations. Process RSS is sampled, not guaranteed absolute
peak. CPU percentages cover the inference interval normalized by 16 logical threads.
Full model-load RPC excludes process startup and Rust staging/integrity checks.
Native model load was also measured (24,952.7 ms in the first complete native run;
15,692.2 ms in the final viewport-evidence rerun).

The [earlier concurrent-install CUDA run](benchmarks/ai-runtime/cuda-concurrent-install.json)
was slower: median inference 57.99 ms, IPC 60.87 ms, RTF 1.085. Earlier exploratory CPU
4,096-frame runs were also slower than real time (RTF 1.31–1.70). These findings are
retained to avoid implying universal performance from the quiet CUDA result.
No audio is committed. Hardware loopback latency, natural speech quality, device
underruns/dropouts and conferencing latency remain unmeasured, represented as unavailable.

Reproduce locally after preparing the authorized ignored assets:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml ai::bootstrap::tests::assembles_recommended_runtime_and_probes_real_dependencies -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml ai::engine::tests::real_transport_recovers_from_cuda_child_exit_to_opted_in_cpu -- --ignored --nocapture
.\.ai-runtime-build\python310\python.exe scripts/benchmark-ai-runtime.py --help
# With the scratch production WebView2 instance listening on loopback CDP 9224:
node scripts/verify-runtime-management-native.mjs
```

The owner-catalog acceptance test also ran explicitly. Its test model library must be
fresh because duplicate IDs are intentionally rejected. CDP is a temporary test-launch
argument, absent from the normal application; no production audio listener is added.

Production artifacts:

```text
src-tauri/target/release/voicemorph-studio.exe
SHA-256 98e3cc54128233ee47a2ecf85e79d6b50ad525277b89bdc7f8a3baea063e239f
src-tauri/target/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe
SHA-256 2b2d14e7c616c8078eee0738e0ac92cea6fe159b9190ae2f6eb4790bcf9f0caa
```

## Resolved findings from real execution

1. Cleared subprocess profiles caused `platformdirs` Windows Known Folder lookup failure
   during model load. The process now uses isolated application-owned profile/cache
   directories and supported platformdirs overrides, without inheriting user credentials.
2. Actual CUDA-child termination exposed lost ring endpoints across worker restarts.
   Worker-port ownership now survives retries and is returned on explicit stop, so the
   same live stream can recover and start again. Actual CUDA-to-opted-in-CPU recovery passed.
3. ORT defaults created uncontrolled thread pools. Both Torch and upstream ORT sessions
   now honor the configured four-thread budget in the tested profiles.
4. Model-load UI timing now measures the full load RPC; upstream initialization is retained
   separately. It does not include all process startup, hashing or staging work.
5. Abandoned audio producers clear stale queues and return the UI to waiting-for-audio
   instead of presenting stale converting telemetry.

## Remaining release qualifications

- **Python 3.10.11 is end of support.** A supported interpreter/dependency baseline and
  vulnerability qualification are required before a general production AI release.
- A full redistribution review of every wheel, NVIDIA DLL and transitive notice remains
  required. The base NSIS deliberately contains no runtime binaries/voice weights.
- CPU default-size chunks are not guaranteed to meet real-time deadlines. CUDA measurements
  also vary with startup/other workloads. Hardware loopback latency, underruns and dropouts
  are not established by sequential file inference.
- No listening/identity-quality assessment, original-vs-processed recording alignment,
  long-duration thermal test, physical virtual-cable loopback, or Discord/Zoom/OBS acceptance
  was completed. A signed native virtual-microphone driver is outside this integration;
  existing virtual-output routing remains available when a compatible endpoint is installed.
- No administrator/kernel-driver install, system CUDA/toolkit change, Defender/signature
  bypass, public listener, paid service or microphone upload was performed.

The production EXE/NSIS build proves packaging. It does not clear these AI release qualifications.
