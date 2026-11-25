# AI engine verification — 2026-11-25

Scope: managed w-okada integration candidate on `feature/new-version`.

| Check                                | Result               | Evidence / limits                                                                                                                                   |
| ------------------------------------ | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust unit/integration tests          | 98 passed, 7 ignored | Real stdlib subprocess fixture, bounded transport, configuration, installation/rollback, privacy, existing DSP regressions; no AI inference fixture |
| Frontend tests                       | 15 passed            | Explicit opt-in, model-rights gate, absent-runtime loading disabled, settings persistence and existing workspace interactions                       |
| Python protocol tests                | 4 passed             | Standard library only; no model conversion                                                                                                          |
| TypeScript / Vite                    | Passed               | Production frontend build                                                                                                                           |
| Rust check / format                  | Passed               | Offline dependency cache                                                                                                                            |
| ESLint / Prettier                    | Passed               | Generated JSON is formatted before commit                                                                                                           |
| Playwright preview                   | Passed               | 5 workspaces × 7 viewport sizes; DPR 1/1.25/1.5; no page errors; screenshots in `docs/screenshots/ai-engine`                                        |
| Production Windows artifact          | Passed: EXE + NSIS   | Base application only, optional runtime not bundled                                                                                                 |
| Real runtime/model import            | Blocked              | User confirmed no authorized runtime/model available                                                                                                |
| Real RVC CPU/CUDA inference          | Unverified           | No authorized runtime/model                                                                                                                         |
| End-to-end latency, CPU/GPU/RAM/VRAM | Not measured         | IPC timing is not loopback latency; no synthetic benchmark substituted                                                                              |
| Discord / Zoom / OBS virtual mic     | Unverified           | Hardware/manual tests remain ignored                                                                                                                |
| Native signed virtual microphone     | Blocked              | No tracked driver/host PCM bridge; existing external endpoint routing preserved                                                                     |

## Source changes by responsibility

| Files                                                                                                  | Responsibility                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/ai/{config,runtime,process,provider,engine,transport}.rs`                               | Validated configuration, optional runtime installation, private RPC/process ownership, interchangeable worker backend contract, orchestration, bounded real-time queues/privacy |
| `src-tauri/src/ai/{manager,models,validator,mod}.rs`                                                   | RVC metadata, explicit consent, isolated imports, checksum/path validation and module exports                                                                                   |
| `src-tauri/src/audio/stream.rs`, `routing/{mod,windows}.rs`                                            | Connect AI conversion to existing capture/monitor/virtual route; discard stale downstream voice                                                                                 |
| `src-tauri/src/lib.rs`, `Cargo.toml`, `Cargo.lock`                                                     | Tauri commands, managed shutdown, Windows Job object API dependency                                                                                                             |
| `ai-sidecar/bridge.py`, `test_bridge.py`, `notices/`, `README.md`                                      | Pinned RVC adapter, protocol tests, retained source attribution and optional package instructions                                                                               |
| `scripts/package-ai-runtime.py`                                                                        | Publisher-side portable package assembly; no dependency downloads                                                                                                               |
| `src/hooks/{useAIEngine,AudioSession}.tsx/ts`, `src/services/tauri.ts`, `src/types/ai.ts`              | Shared engine state, typed IPC, persistent navigation-independent state                                                                                                         |
| `src/pages/{AIVoiceStudio,Settings}.tsx`, `src/components/AudioStatusBar.tsx`, `src/styles/studio.css` | Real runtime/model controls and explicit privacy/error/telemetry states in existing design                                                                                      |
| `tests/app-ui.test.tsx`, `tests/fixtures/ai_sidecar_stub.py`                                           | Meaningful UI gating and actual child-process contract fixtures                                                                                                                 |
| `scripts/visual-redesign.mjs`, `scripts/verify-ai-native.mjs`                                          | Isolated screenshot output path and real native bridge smoke test                                                                                                               |
| `.gitignore`, `.prettierignore`, `eslint.config.js`                                                    | Keep reference checkout/runtime build assets outside source/lint scope                                                                                                          |
| AI audit/setup/attribution docs and screenshots                                                        | Architecture, compatibility, packaging, evidence and explicit release blockers                                                                                                  |

The optional Python environment is not assembled on this host. `package-ai-runtime.py --help`
was exercised; a successful runtime package build or redistribution clearance is not claimed.
No additional frontend dependency, large model download, driver installation, signing operation,
push, merge, or release publication was performed.

## Production build and native smoke

The final `tauri build --bundles nsis -- --offline` passed and produced a 2.99 MiB installer:
`src-tauri/target/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe`.
The optimized EXE is `src-tauri/target/release/voicemorph-studio.exe`.

Native smoke passed on a byte-identical copy of that final EXE using real Tauri commands:
Classic default, loading disabled without runtime, actual start failure, AI fault with mute
policy, null conversion timing, zero completed chunks, Stop retains AI mode, and deliberate
Classic restoration. Microphone stayed stopped throughout. This tests state/control behavior;
physical speaker/virtual-device silence and real inference remain unverified.
See [native command report](screenshots/ai-engine/native/report.json) and screenshots.

SHA-256:

- EXE and tested copy: `72908e48bac222f336ae9ae0f99c42efe97ea9b45177f649b25417e8a1663e3f`
- NSIS: `aaf8fb9d6a8ce2bc21f75ef4873eeb717ab5dc9f09d97684fbec788d53e5e3a4`

A temporary loopback WebView2 debugging port was used only by the explicitly launched
test process; the shipping app enables no AI HTTP service or debugging listener. The NSIS
installer was built but not installed. Signing and installation were not performed.
