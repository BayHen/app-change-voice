# Managed w-okada integration — implementation and verification

Branch: `feature/new-version`. Audit date: 2026-11-25.

> Historical pre-authorization audit. Its unavailable-runtime/model statements describe
> that earlier checkpoint. The user subsequently authorized installation and testing;
> see [current real execution results](AI_RUNTIME_EXECUTION_REPORT.md) for the implemented
> automatic installer, authorized assets, CUDA/CPU inference and remaining blockers.

## Readiness

This is an implemented integration candidate, **not a production-approved AI release**.
The user confirmed that no authorized runtime or voice model is available on this machine.
No real RVC inference, listening test, CPU/GPU performance benchmark, or conferencing
application test has been performed. Protocol fixtures never stand in for inference results.

Classic DSP remains available without Python, AI weights, or a network connection.
The optional AI package is imported from a publisher-prepared local folder. There is no
approved online runtime/model catalog, download endpoint, or automatic dependency install.

## Existing application audit

React renders the shared navy/violet studio shell. Tauri commands manage state. Rust CPAL
captures microphone audio, runs the existing DSP pipeline, records, mixes the soundboard,
monitors, and sends output through the existing WASAPI virtual-cable route. The root
`AudioSessionProvider` keeps engine state alive across page navigation.

Previously, AI had a model metadata/checksum library, an offline OpenVoice prototype, and
a fixed-size worker abstraction disconnected from live capture. It had no working live
model provider. MASK is an architectural reference in project documentation; no installed
MASK engine was found. No tracked native virtual-audio driver source or host PCM bridge
was found in `drivers/voicemorph-virtual-audio`. The integration preserves the existing
output route; it cannot provide a signed, cable-free native virtual microphone.

## Source audit and compatibility

Audited upstream: [w-okada/voice-changer at 7e2ce5c4800010bbc7604968bd29b802534fac05](https://github.com/w-okada/voice-changer/tree/7e2ce5c4800010bbc7604968bd29b802534fac05).
The isolated checkout is ignored under `.references/w-okada`; it is not part of the base app.

| Source / provider                          | Finding                                                                  | Integration decision                                                                                                             |
| ------------------------------------------ | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `server/restapi/MMVC_Rest_VoiceChanger.py` | `/test` takes timestamp + base64 signed int16 PCM; returns converted PCM | Audited contract; no public server started                                                                                       |
| `server/sio/MMVC_Namespace.py`             | Socket.IO `/test`, request/response audio events                         | Audited; avoided browser and port surface                                                                                        |
| `VoiceChangerManager.py`                   | Starts ServerDevice and manages server defaults                          | Bypassed to preserve sole Rust capture ownership                                                                                 |
| `VoiceChangerV2.py`                        | Owns SOLA/crossfade; can return short zeros on warmup/failure            | Called from adapter, bounded warmup and exact output validation                                                                  |
| `RVC/RVCr2.py`                             | RVC pipeline with transpose, F0, context, threshold                      | Optional RVC voice ONNX path implemented, real inference unverified                                                              |
| `RVC/RVCModelSlotGenerator.py`             | Reads ONNX metadata, deprecated fallback                                 | Reject deprecated metadata and unsupported embedding/rate combinations                                                           |
| CPU                                        | PyTorch embedding + ORT CPU voice inference                              | Implemented selection, unverified with real package                                                                              |
| NVIDIA CUDA                                | Requires both Torch CUDA and ORT CUDA; ORT may silently fall back        | Check actual loaded provider; CPU retry only with explicit opt-in                                                                |
| DirectML / AMD / Intel                     | Upstream branches exist; dependency/operator compatibility unverified    | Deferred; not advertised in UI                                                                                                   |
| ONNX embedding                             | `RVC/embedder/OnnxContentvec.py` is a NotImplemented stub                | Do not claim an all-ONNX engine                                                                                                  |
| HuBERT/contentvec embedding                | Uses fairseq checkpoint loading                                          | Only an explicitly trusted publisher's digest-verified runtime asset may load; arbitrary imported voice checkpoints are rejected |
| Beatrice                                   | Separate assets and noncommercial/redistribution conditions              | Excluded; legal and technical approval required                                                                                  |
| Native Rust inference                      | Existing prototype worker is separate from the new managed engine        | Deferred; transport/engine boundary permits future replacement                                                                   |

The optional source dependency family targets Python 3.10. Host Python 3.14 is not used to
install old ML dependencies. Detecting an RTX 3080 does not verify inference compatibility.
RVC voice metadata must describe supported HuBERT/contentvec, 256/768 embedding channels,
and 32/40/48 kHz model rate. Wire audio is always mono 48 kHz signed PCM16. Imported voice
models are ONNX only. FAISS indexes, arbitrary archives, custom ONNX domains, external
tensor data, local functions and training graphs are not accepted. Index ratio stays zero.

## Architecture decision: private managed sidecar

**Decision:** retain the Rust real-time engine and use a single managed optional Python
process for the pinned RVC implementation. Private inherited stdin/stdout pipes replace
the upstream HTTP/Socket.IO server. No loopback listener, port reservation or bearer token
is needed because no network service is exposed. This deliberately provides a smaller
access surface than a loopback-only server. It is process isolation, not an OS sandbox.
Trusted runtime code still has the current user's filesystem/network rights.

Capture -> bounded tagged input ring -> worker resampling/PCM -> private RPC ->
RVC + upstream SOLA -> output resampling -> bounded tagged output ring -> Rust gain,
recording, monitor/soundboard mix and existing virtual output.

The callback never waits for inference or a mutex, serializes JSON, or spawns a process.
Worker queues hold at most approximately half a second of native audio; overload drops
samples and increments real counters. The RPC frame limit is 256 KiB. Linear resampling
preserves duration in tests; non-48 kHz aliasing/quality must be listened to before release.
IPC round trip excludes capture, chunk accumulation, queue delay, routing and device latency.
It is labeled accordingly and **is not end-to-end latency**. Warmup zeros do not increment
completed inference chunks. Original/processed AI recording alignment is unverified.

Runtime state is independent of audio stream and navigation lifetime. Replacing a stream
replaces its ring attachment without automatically spawning another AI process. Epoch tags
discard stale inference results. Monitor and virtual output also discard queued Classic
audio on a mode change. During AI startup, stop, or failure, default output is silence.
The virtual mixed bus temporarily mutes during these states to prevent stale voice leakage.
Classic DSP on failure requires explicit saved opt-in. Explicit Stop overrides that opt-in;
only selecting Classic DSP deliberately restores the Classic voice.

The process has bounded RPC deadlines, health checks, at most two automatic restarts,
bounded logs, and a Windows kill-on-close Job object with an 8 GiB per-process memory cap.
Shutdown requests a graceful exit, then kills the process tree if needed. Environment is
restricted and offline Hugging Face flags are set. These flags are not a firewall guarantee.
No audio samples are deliberately logged or uploaded; upstream recording is disabled.

## Packaging and installation

See [sidecar packaging instructions](../ai-sidecar/README.md). Base Tauri packaging includes
the UI and Rust adapter; it does not embed a huge AI runtime or licensed model. The optional
builder assembles an already prepared portable Python 3.10 distribution, pinned source,
bridge and explicitly licensed assets. End users do not run pip or launch a separate UI.

The user supplies the publisher manifest SHA-256 through an independent trusted channel
and acknowledges publisher trust. Every declared file is hashed; paths reject traversal,
symlinks and Windows reparse points. The bridge hash must match this exact app build.
This provides integrity and explicit trust, **not a digital publisher signature**.
Installation copies only manifest files in bounded blocks, reports verified-file progress,
supports cancellation, stages immutable releases, and atomically updates the active pointer.
Failure/cancellation keeps the previous active runtime. A verified previous release can be
restored. Partially staged directories after abrupt power loss are not activated; automatic
garbage collection and resumable network downloads are deferred.

Model imports require a bounded manifest, allow-listed license, HTTPS provenance, checksum,
ONNX filename and explicit voice-rights acknowledgement. This acknowledgement does not prove
the license or identity rights; the operator remains responsible for provenance. IDs are
isolated, duplicates do not overwrite, and active models cannot be removed. Interrupted
model imports retain temporary files and return an actionable error for manual inspection.

## Validation evidence

- Rust: **98 passed, 7 ignored**. Covers existing DSP/routing behavior, PCM/resampling,
  bounded queues, stale generations, privacy gates, explicit stop, pending-start cancellation,
  config persistence, model integrity, real stdlib child lifecycle/timeout, and runtime
  installation/cancellation/corruption/rollback with inert fixtures.
- Frontend: **15 passed**; includes runtime opt-in, loading disabled without a runtime,
  explicit failure-policy persistence, Classic selection, and model-rights gating.
- Python: **4 passed**; protocol/config/path/error bounds only, no inference.
- ESLint and TypeScript/Vite production build passed.
- Playwright production preview: five workspaces, **35 page/viewport checks**, DPR
  1/1.25/1.5, no page errors. Evidence: [screenshots and report](screenshots/ai-engine/).
  Browser preview has no Tauri bridge and shows unavailable values; it does not simulate live telemetry.
- Native production build/smoke result is recorded in the final verification report.

## Release blockers and deferred work

1. No authorized portable runtime, HuBERT asset and RVC voice model are available. Package
   import smoke, actual RVC API compatibility, warmup, quality, CPU and CUDA inference remain unverified.
2. Approve dependency versions, redistributable DLLs, model rights and all license notices;
   perform vulnerability review and sign/publish a runtime catalog before offering downloads.
3. Measure true loopback latency, inference time, CPU/RAM/GPU/VRAM, underruns, drops,
   model switching, long sessions, recording alignment and device reconnects on CPU/GPU hosts.
4. Validate Discord, Zoom and OBS using real Windows hardware and an installed endpoint.
   A production virtual microphone without manually installed VB-CABLE needs a separately
   implemented and signed native driver/host bridge. No driver/signing/security changes were made.
5. Download resume/catalog, automatic hardware preflight without loading a model, DirectML,
   Beatrice, indexes, quality presets and native Rust inference are deferred.

Do not present this candidate as completed production AI voice conversion until these gates
have evidence. No CPU/GPU/RAM/VRAM figures or end-to-end benchmark results were fabricated.
