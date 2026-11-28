# Phase 5 ? Neural Agent Voices engineering report

Branch: `feature/new-version`. Source implementation and tests only; no EXE/installer or production bundle was rebuilt. No commit or push was performed.

## Acceptance status

**Partially implemented; the full acceptance criteria are not met.** The project now has a real neural Agent workflow, neural-first selection, independent identity guards, post-neural effects and an operational four-way audio comparison/calibration lab. One authorized owner voice was executed on CUDA and CPU. There is no second verified independent feminine model or authorized male/female Vietnamese/English comparison pair available in this session. No perceptual listening results or new end-to-end virtual microphone latency result are claimed. All 19 Agent slots remain Not Installed until an authorized independent identity is assigned locally.

## Audit and root causes

1. The existing female collection contains 20 DSP presets and one reviewed neural identity. Those presets share pitch/formant/EQ/dynamics processing and cannot provide independently trained target speaker identities. Their similar sound is expected; changing numerical DSP parameters is not a neural identity change.
2. The existing RVC backend performs actual ONNX inference. However, its female preview action is hard-wired to the single official Tsukuyomi model. There were no per-Agent model bindings or guards against reusing one identity under different labels.
3. In the checked-out repository, the previous ONNX wizard files were absent. The Import ONNX action still selected a JSON manifest first. This extension adds direct ONNX inspection/registration again while preserving the JSON path.
4. Silence-only warm-up did not exercise the voiced inference kernels. Initial file comparisons measured roughly 2.9-second first-speech chunks. These were initialization costs leaking into the speech path, not acceptable live conversion latency.
5. Small chunks can exceed the compute budget even on CUDA. The qualified voice with 2048-frame chunks remained slower than real time in this test; changing the UI label does not fix that constraint.
6. The packaged runtime has no RMVPE ONNX asset. The UI now disables RMVPE unless the installed runtime declares it. DIO and Harvest remain functional options. No quality benefit from RMVPE is claimed.
7. A real Unicode-path test found that Python isolated mode (`-I`) ignores `PYTHONIOENCODING`. JSON containing Vietnamese paths was decoded using the Windows legacy encoding. Managed children now explicitly use `-X utf8`, without changing system locale, security policy or Python installations.
8. Windows CRLF checkout bytes also differed from the packaged LF bridge digest. Runtime verification now accepts only the exact current bridge source hashes for those line endings; every installed bundle file is still hashed against the manifest.

## Architecture and implementation

- RVC: reuses `VoiceConversionBackend` / `WOkadaBackend` and the existing pinned w-okada private-process ONNX pipeline. Content encoder, F0 estimator and decoder/vocoder execute outside audio callbacks. The managed session is reused for live chunks.
- DSP: the original explicit Classic DSP engine remains available. Its presets are not advertised as installed neural Agent identities.
- Hybrid: `NeuralPostProcessor` runs the optional existing radio/ring modulation/EQ/de-essing/dynamics chain after neural output, on the DSP worker. It does not use pitch/formant changes to manufacture another Agent identity. Effect bypass is smoothed, and processor history resets when the model generation changes. Neural-only mode bypasses this post chain.
- Advanced neural engines: Seed-VC was evaluated, not installed or represented as working. Its architecture/runtime contract differs from the pinned RVC model interface, and it introduces another dependency/model/qualification path. The upstream repository is archived and its code is GPL-3.0. No unnecessary second runtime was installed merely to populate an engine list.
- Warm-up: a generated non-silent harmonic signal exercises voiced kernels, followed by bounded silence to flush context. All warm-up output is discarded in the worker. The model becomes Ready only after warm-up. No generated calibration signal is routed to headphones or the virtual microphone.
- Streaming: retains the bounded ring buffers, generation tags, reusable sessions, sample-rate conversion, upstream overlap/crossfade, explicit CPU fallback, underrun counters and opt-in failure policy. Agent loading forces mute-on-failure; there is no hidden DSP fallback. Existing virtual routing receives the neural/hybrid result through the common mixer path.
- Resources: the bridge's real process CPU counter, RSS and PyTorch CUDA allocations are sampled by the inference worker. Live GPU utilization is unavailable and is displayed as such, not estimated. Metric sampling does not run in CPAL callbacks.

### Independent identities and availability

`agent-voices.json` stores a binding per requested Agent slot: Jett, Sage, Reyna, Viper, Killjoy, Neon, Clove, Fade, Raze, Skye, Deadlock, Omen, Cypher, Yoru, Chamber, Phoenix, Sova, Iso and Gekko. UI names are explicitly ?-inspired? slots, without bundled game voice assets or a claim that the model reproduces a character.

Assignment requires an RVC-compatible graph, matching file SHA-256, confirmed model/voice rights and a named identity. Reusing the same file digest, encoded-weight fingerprint, or identity name under another Agent is rejected. Weight fingerprints sort named tensor payload hashes, so a metadata-only re-export cannot bypass the same-identity guard. This is a duplicate-weight guard, not a perceptual speaker-separation score. Different weights are not definitive proof of different-sounding people; listening validation is still required. The pinned engine currently uses speaker ID 0; shared-weight multi-speaker conditioning is not exposed as supported.

Agent source files remain in the model library. Removing an assignment does not delete its source model. Missing/changed models produce an error, rather than selecting Classic DSP automatically. Runtime model state and active model name are shown separately from the selected library slot.

### Quality modes

| Mode               | Chunk frames | Crossfade frames | Extra context frames |
| ------------------ | -----------: | ---------------: | -------------------: |
| Low Latency        |         2048 |              512 |                 2048 |
| Balanced (default) |         4096 |             1024 |                 8192 |
| High Quality       |         8192 |             2048 |                16384 |

These are real runtime configuration presets, not guaranteed performance tiers or independent voices. Pitch compensation, F0 method, provider, thread limit and runtime controls remain in AI Voice Studio. Unsupported naturalness/similarity/index/protection sliders were not added.

### A/B comparison and local calibration

The Agent quality lab reads one explicitly authorized local WAV, up to five seconds, and returns original, Classic DSP, neural and hybrid WAV outputs. Neural comparison requires 48 kHz. The same source can be reused across every installed identity. Inputs remain local; no automatic microphone recording or external upload is introduced. Playback starts only after the user selects an output.

The comparison reports initialization and discarded warm-up separately from compute real-time factor; actual inference total, worst chunk and inference-budget misses are recorded. Neural startup/tail is retained; files are not sample-aligned. Budget misses are not measured virtual-device dropouts.

Calibration estimates input F0 and spread with bounded autocorrelation, RMS, peak, clipping and quiet-frame level, plus a conservative input-gain recommendation. These are signal estimates, not an identity, resonance or intelligibility assessment. Quiet-frame level is not a separately measured room-noise floor. Calibration statistics are stored in `voice-calibration/<agent>-<model-sha>.json`, without copying the source recording. Recommendations are applied manually; unsupported index/protection/formant controls are not invented.

## Actual executed model and rights

- Model: Tsukuyomi-chan official Normal 1, CV. Rei Yumesaki.
- Model SHA-256: `d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6`.
- Owner distribution: https://tyc.rei-yumesaki.net/work/software/rvc/
- Custom model/voice terms: https://tyc.rei-yumesaki.net/work/software/rvc/terms/
- Authorized original-owner test WAV: https://tyc.rei-yumesaki.net/material/voice/sample-voice/
- Local model: `.ai-runtime-build/converted-tsukuyomi/tsukuyomi-normal1.onnx`.
- Local test input: `.ai-runtime-build/assets/owner-sample.wav` (1.68 seconds).
- Runtime: `.ai-runtime-build/bootstrap-installed/releases/69c600bd252be1fd47f5bf9246294c976c32e27dfb4796b1712e92312d6504f7`.
- Existing dependencies: CPython 3.10.11; PyTorch 2.6.0+cu124; ONNX Runtime GPU 1.21.0; NumPy 1.23.5. No dependencies or models were installed for this extension.
- Hardware observed: Intel Core i5-13400F, NVIDIA GeForce RTX 3080, 10240 MiB device memory, driver 610.88.

The local integration test imported a real copy with a Vietnamese filename and spaces, generated its manifest, assigned it to an isolated test slot, checked consent and duplicate assignments, and loaded it for CUDA inference. A metadata-only re-export with a different whole-file digest but identical weights was rejected for a second Agent. Those test bindings are not shipped or written into the user's Agent library.

A second model candidate, ru-masha-200, was researched but not downloaded or treated as authorized: its publisher reports actor consent for the corpus but explicitly states the Snowie pretrain has no declared license. That leaves the complete model-rights chain unresolved. No character, TikTok or community voice was used without voice authorization.

Another reviewed candidate was w-okada's Amitaro RVC v2 simple export. Its pinned publisher terms explicitly limit that artifact to VCClient. VoiceMorph cannot be assumed to inherit that permission merely because it embeds w-okada inference code. Only repository metadata and terms were fetched; no model weights were downloaded or executed. Its publisher SHA-256 is `b7da32d31150d2191d5d2d6dd07dafd86b1cfdbe0a991871da4bb6c18cab0d39`, revision `c46962577db8cf3fba2b3fc3526af98eb6611840`. The owner's current general terms also restrict model redistribution, so it was not added to a shipped catalog. Sources: [publisher artifact terms](https://huggingface.co/wok000/vcclient_model/blob/c46962577db8cf3fba2b3fc3526af98eb6611840/rvc_v2_alpha/amitaro/terms_of_use.txt), [owner terms](https://amitaro.net/voice/terms/).

## Measured file-inference results

Short single-voice samples after discarded warm-up; the latest HQ row includes real Unicode-path import and weight-identity regression checks. These runs occurred at different times and are not controlled long-duration or perceptual experiments. RTF includes wall time for inference IPC and flush chunks, divided by source duration; startup/runtime discovery, loading and warm-up are excluded. CPU percentages use 100% per fully utilized logical core, not 100% for the entire machine. RAM is process RSS. VRAM is **PyTorch allocated memory only**, excluding ONNX Runtime allocations and other device processes.

| Provider / mode    | Compute RTF | Worst inference chunk ms | Budget misses | Process CPU (core %) | RSS MiB | Torch allocated VRAM MiB |
| ------------------ | ----------: | -----------------------: | ------------: | -------------------: | ------: | -----------------------: |
| CUDA / lowLatency  |       1.577 |                     88.6 |            39 |                94.4% |    1983 |                    221.9 |
| CUDA / balanced    |       0.899 |                     92.3 |             3 |                92.0% |    1995 |                    221.9 |
| CUDA / highQuality |       0.421 |                     73.5 |             0 |                88.3% |    1994 |                    221.9 |
| CPU / balanced     |       2.054 |                    180.7 |            20 |               359.7% |    1169 |                      N/A |

Interpretation: Low Latency and CPU Balanced were slower than real time for this model on these runs. CUDA Balanced was below 1 overall but missed some chunk budgets. CUDA High Quality met every measured chunk budget in the short sample, at the cost of larger buffering/context. This does not establish end-to-end microphone-to-Discord latency or sustained reliability.

Cold-start reports and WAVs are retained under `.ai-runtime-build/phase5-neural-cold-start/`. Updated CUDA reports and four-way WAVs are under `.ai-runtime-build/phase5-neural/`; CPU results are under `.ai-runtime-build/phase5-neural-cpu/`. No MOS, human-naturalness score, speaker-embedding separation or English/Vietnamese quality result is fabricated. The engineering assertions verify real non-silent output, bounded clipping, distinct DSP/neural bytes and active hybrid effects; they do not establish subjective sound quality.

## Tests and visual verification

- Full Rust unit suite: **122 passed, 0 failed, 15 ignored**. Hardware/model tests are intentionally opt-in.
- Real owner-model CUDA comparison executed for all three modes; CPU comparison executed for Balanced.
- Final opt-in CUDA HQ regression: passed raw Unicode/spaces import, runtime load, real neural conversion, rights rejection, duplicate file and metadata-only duplicate-weight assignment rejection, and four-way output checks.
- Actual streaming ring-buffer/crash-recovery test: passed. CUDA produced 258607 audible samples; explicit CPU fallback produced 246518; one owned-child restart and 68 completed chunks. No audio devices or microphone were opened. This validates the bounded streaming/recovery path, not hardware loopback latency.
- Real child protocol regression: Unicode text round-trip, malformed/oversized frames, timeout and cleanup passed.
- Frontend suite: **33 passed**, covering existing UI plus 19 uninstalled slots, explicit assignment rights, real IPC payloads, quality-mode persistence, duplicate errors, no implicit DSP switch, hybrid controls, recording consent and ONNX/JSON picker filters.
- TypeScript and ESLint: passed. Rust and changed frontend files formatted/checked.
- Playwright/Edge: four screenshots; 19 uninstalled slots verified; load blocked without weights; no page errors or document overflow at desktop/compact sizes. Browser IPC is mocked for visual tests; actual neural tests run separately through the Rust backend.
- Screenshots: `docs/screenshots/phase5-neural/`.
- No live capture, device playback, Discord call, production EXE rebuild or installer build was performed.

Reproduce in PowerShell:

```powershell
npm.cmd test -- --reporter=dot
npx.cmd tsc --noEmit
npm.cmd run lint
cargo test --manifest-path src-tauri/Cargo.toml --offline --lib
# Explicit real CUDA test, approved owner model/assets required:
$env:VM_NEURAL_TEST_PROVIDER='cuda'
cargo test --manifest-path src-tauri/Cargo.toml --offline --lib compare_authorized_owner_in_three_modes -- --ignored --nocapture
# Optional focused HQ run:
$env:VM_NEURAL_TEST_MODE='highQuality'
# For the CPU benchmark, use provider cpu and remove a conflicting mode filter.
```

The Vietnamese operational guide is [HUONG-DAN-AGENT-VOICE.md](HUONG-DAN-AGENT-VOICE.md), including the current Monitor/capture switch and Discord cable endpoint mapping.

## Changed source files

- `src-tauri/src/ai/agents.rs`: identity registry, licensing gates, duplicate identity guards and real quality presets.
- `ai/onnx.rs`, `ai/import.rs`, `ai/manager.rs`, `ai/validator.rs`: bounded runtime-independent graph/metadata inspection, encoded-weight fingerprint, direct manifest generation/import and legacy RVC validation.
- `ai/warmup.rs`, `ai/provider.rs`, `ai/engine.rs`, `ai/transport.rs`, `ai/config.rs`: voiced warm-up/flush, actual resource telemetry, hybrid selection, capabilities and worker integration.
- `ai/hybrid.rs`, `audio/stream.rs`: optional post-neural effects in the existing processing/routing worker.
- `ai/comparison.rs`: real four-way conversion, metrics, calibration analysis and authorized integration tests.
- `ai/process.rs`, `ai/runtime.rs`: explicit UTF-8 isolated child startup and bridge line-ending hash compatibility.
- `ai/mod.rs`, `src-tauri/src/lib.rs`: module registration and real inspection/Agent/preview/calibration IPC.
- `src/components/AgentVoiceCollection.tsx`, `NeuralModelImport.tsx`, `src/pages/VoiceLab.tsx`, `AIVoiceStudio.tsx`: neural-first Agent library, real import controls, honest availability and supported controls.
- `src/types/ai.ts`, `src/services/tauri.ts`, `src/styles/studio.css`: types, IPC wrappers and shared navy/purple UI styling.
- `tests/agent-voices.test.tsx`, `tests/app-ui.test.tsx`, `tests/fixtures/ai_sidecar_stub.py`: UI and private-protocol regression tests.
- `scripts/benchmark-ai-runtime.py`, `scripts/verify-agent-voices-visual.mjs`: configurable benchmark context/F0 and browser visual verification.

## Windows usage and installation

1. Restart the development app with `npm.cmd run tauri:dev` so the new Rust commands are compiled and loaded. A browser-only Vite session cannot run native inference. The existing EXE is unchanged.
2. In AI Voice Studio, install the managed runtime using the existing automatic runtime controls. End users do not need a terminal or manual Python installation for that runtime workflow. CUDA must be compatible with the installed driver; CPU fallback is opt-in and may be too slow for live use.
3. Use the official Tsukuyomi voice only within its model/voice terms. It remains one identity; it does not make all Agent slots available.
4. For another voice, choose Import ONNX model, select/drop a compatible ONNX or choose JSON plus its matching ONNX. Review detected metadata/tensors. Provide source/terms/attribution and confirm rights before registration. Incompatible files require a compatible export; they never become DSP under a neural label.
5. Open Voice Lab ? Agent Voices. Select a slot, choose an authorized independent model, enter its credited identity, choose a real quality mode and save the assignment. Each slot needs a distinct approved identity. Stop AI before changing runtime settings/models.
6. Load RVC neural for an unprocessed neural base. Choose Hybrid only when optional post-neural effects are wanted. Those effects can be bypassed. Use Classic DSP only through its explicit control.
7. For comparison/calibration, choose an authorized original WAV and confirm local processing. Neural comparison needs 48 kHz and live AI stopped. Render all four outputs and listen using the same source for each installed identity. No recording is uploaded.
8. For Discord, retain the existing routing: physical microphone ? VoiceMorph neural/hybrid ? CABLE Input playback endpoint; select CABLE Output as Discord input. Monitor through headphones. Confirm local meters and make a call-free test before entering a call. No game client modification is needed.

## Remaining work and blockers

- A second independent feminine neural identity with a verified complete model/voice/distribution-rights chain is still missing. Nineteen authorized character or original identities are not available and are not claimed.
- Authorized male and female input pairs, including Vietnamese and English, were requested but not supplied in this session. The prior TikTok link was style reference only and was not reused as test/training audio.
- No human listening panel or perceptual identity/naturalness validation has occurred. Different encoded weights and output bytes do not prove perceptual distinction.
- Live Windows virtual-microphone loopback and end-to-end call latency were not re-tested. A current call-free confirmation was requested and not received; no test audio was sent to active calling applications.
- Seed-VC/advanced runtime, DirectML, imported speaker embeddings, arbitrary speaker IDs and FAISS indices remain unsupported/unqualified. The only checkpoint conversion path remains the existing checksum-pinned approved-owner workflow; arbitrary Python/pickle checkpoints are not executed.
- RMVPE remains unavailable in the installed package; no RMVPE quality claim is made. Neural enhancement, adaptive chunk selection and learned speaker similarity controls were not implemented as decorative controls.
- The static ONNX reader validates structure/signature and embedded data, not the complete ONNX operator semantics. Actual runtime load/inference is required to qualify an imported candidate.
- Live GPU utilization and total model-specific VRAM are not available from the current private bridge. Torch allocations and process CPU/RAM are real but partial resource measurements.
- Calibration is an optional file-based local analysis workflow using existing microphone recordings, not an automatic background recorder. It provides conservative signal statistics and manual recommendations, not a learned voice-quality optimizer.
- Production packaging/build verification remains deferred per the user's instruction.

## Primary-source evaluation references

- Pinned w-okada RVC implementation: https://github.com/w-okada/voice-changer/tree/7e2ce5c4800010bbc7604968bd29b802534fac05
- Seed-VC architecture/runtime: https://github.com/Plachtaa/seed-vc
- Seed-VC code license: https://github.com/Plachtaa/seed-vc/blob/main/LICENSE
- RMVPE upstream implementation: https://github.com/Dream-High/RMVPE
- Second-model candidate and explicitly missing pretrain license: https://github.com/Friskes/voice-changer/blob/main/model/README.md
