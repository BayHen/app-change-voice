# Unified UI redesign: implementation and verification

Date: 2026-11-25, Asia/Saigon. Branch: `feature/new-version`, as requested. This report covers the implemented frontend redesign and distinguishes existing backend limitations from UI work.

## Audit and scope

Reviewed the application shell, every existing page, shared controls/styles, audio and preset hooks, Tauri service bindings, navigation events, runtime/recording types, Rust DSP parameter validation, preset storage/preview, AI manager, soundboard/routing interfaces and the existing architecture/audit/testing documentation. No project AGENTS.md was found. The explicit redesign request supplies authorization for application changes despite the older roadmap's general approval note.

Pre-existing changes were present in `docs/OPEN_SOURCE_RESEARCH.md`, `src-tauri/src/soundboard/mixer.rs`, `src-tauri/src/soundboard/repository.rs`, `.agents/` and `skills-lock.json`. They were preserved. The redesign does not modify Rust audio/DSP, AI, routing, driver or soundboard implementation.

| Capability             | Audited implementation                                                                               | UI result                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Real-time DSP          | Rust pitch/formant, gain, bass/treble shelves, creative processors, suppression/gate/limiter         | Voice Lab uses real IPC with backend-valid ranges; extended DSP controls remain in Settings      |
| Presets                | Built-in/custom libraries, preview/apply, JSON import/export, favorites, persistence                 | Six DSP templates plus working custom save/export; full library remains accessible               |
| Devices and monitoring | Native device enumeration/selection and start/stop/pause/resume                                      | Shared device/session state and persistent monitoring control; full diagnostics retained         |
| Recordings             | One aligned original/processed session, playback, delete, WAV export                                 | Dedicated management workspace using actual session duration/status; no fabricated file metadata |
| Soundboard             | Local WAV references, categories, favorites, gain, routes, hotkeys, plugin/effect registry           | Search/category/favorite filters, launch cards, progressive clip controls and import dialog      |
| Virtual microphone     | Existing virtual playback endpoint routing and PCM path                                              | Shared status plus preserved endpoint configuration and Windows settings controls                |
| Native AI              | Verified manifest/ONNX storage, integrity/license checks; native enable returns an unavailable error | Model management preserved, real metadata shown; inference/provider controls disabled honestly   |
| MASK / w-okada         | Architectural references in the existing research document; no connected provider in this app        | No integration removed or newly claimed; DSP remains independent                                 |

No fabricated voice portraits, recordings, waveforms, GPU percentages, inference latency or active playback indicators were added. The backend has no public sample-peak/file-waveform, playback-status or master-soundboard-gain contract, so those features are not falsely presented as working. File renaming/history/sorting, model downloads/RVC inference, automatic driver installation and GPU selection still require backend implementation. Driver signing/security policy is unchanged.

## Skills and dependencies

Applied project-scoped [frontend-design](../.agents/skills/frontend-design/SKILL.md) and [ui-ux-pro-max](../.agents/skills/ui-ux-pro-max/SKILL.md). Used the installed computer-use skill for native Windows verification. UI Pro Max's design-system search returned a marketing-oriented pattern; a narrower product search did not produce an audio-workstation match (the retry matched photography). Those off-topic recommendations were not adopted. The user references and task palette take precedence; keyboard-focus guidance informed Radix dialog verification.

Added only:

| Package                  | Installed version        | License    | Purpose                                                                        |
| ------------------------ | ------------------------ | ---------- | ------------------------------------------------------------------------------ |
| `@radix-ui/react-dialog` | 1.3.1                    | MIT        | Modal focus containment, keyboard dismissal and accessibility                  |
| `@radix-ui/react-switch` | 1.4.1                    | MIT        | Accessible consistent switches                                                 |
| `lucide-react`           | 1.80.0                   | ISC        | Single icon family throughout the app                                          |
| `@playwright/test`       | 1.65.1, development only | Apache-2.0 | Real-browser screenshots, responsive checks, keyboard modal checks and timings |

Verified installed package metadata and builds against the current React 19/TypeScript/Vite stack. Reviewed official [Radix switch documentation](https://www.radix-ui.com/primitives/docs/components/switch) and [Playwright screenshot documentation](https://playwright.dev/docs/screenshots). The owned components follow the composition approach used with shadcn/Radix; no shadcn registry component or CLI installation is claimed. Motion is unnecessary for the brief's lightweight feedback; Canvas serves the actual telemetry contract; WaveSurfer would introduce a second playback path without available file peaks/seeking. No additional skills or UI dependencies were installed. Removed the now-unused Phosphor dependency.

A live npm audit reported nine moderate development-tool findings in the existing ESLint/typescript-eslint/minimatch chain; no high or critical findings were reported. None named the newly added UI/runtime packages. The offline audit reported zero because it lacked live advisory data; that result is not used as evidence of a clean audit. Toolchain upgrades were kept outside this UI change.

## Visual verification and refinement

Captured the original five comparison views before changing source. The old app did not have a dedicated Recordings route, so its baseline is the Dashboard that contained recording controls. Before screenshots are historical; they use the old labels.

Refinement passes addressed the following observed issues:

- Reduced the large preset grid into six compact columns on suitable desktop widths; kept a multi-row layout on narrower workspaces.
- Reduced the signal panel's height to bring DSP controls higher in the viewport.
- Replaced raw missing-Tauri invocation errors with a clear disconnected-bridge message.
- Moved desktop notifications above the persistent audio bar.
- Unified legacy surfaces and icons with the same navy/violet tokens and Lucide family.
- Fixed dialog focus restoration, including conditionally mounted import/editor dialogs.
- Made card selection reflect real DSP settings after navigation; fixed export for a just-saved custom preset.
- Darkened the primary-button gradient/hover after numerical contrast checks.

The supplied references were compared qualitatively: dark navy hierarchy, violet active navigation, highlighted tiles, compact sliders, strong panel boundaries, an audio workspace and a persistent footer are carried through all pages. The delivered composition combines the references with the explicitly requested Voice Lab features; it is not a pixel-identical reproduction of one reference. No pixel-diff score is claimed because the supplied images were not available as local source files.

See [the design system and component contracts](UI_REDESIGN_DESIGN_SYSTEM.md) and [the screenshot gallery](screenshots/redesign/README.md). The cross-page contact sheet is assembled from genuine screenshots. Browser captures show actual offline states; native captures show the real connected desktop bridge and empty local libraries, without seeding user data.

## Validation results

- Frontend regression suite: 13 tests passed, including retained device/gain/mute, hotkey, preset and updater checks, plus cross-page audio-session ownership, modal focus/Escape and edited-preset save/export availability.
- `cargo check`: passed.
- Rust suite: 83 passed, 7 ignored. Ignored cases require real audio hardware/cable or manual performance measurement. Existing uncommitted soundboard changes were included in these local tests but were not authored by this redesign.
- ESLint: passed with no warnings after cleanup.
- Prettier: passed; installed skill/vendor content and generated measurement JSON are excluded rather than reformatted.
- Production frontend build: TypeScript and Vite passed.
- Tauri release build: `npm run tauri -- build --no-bundle` produced `src-tauri/target/release/voicemorph-studio.exe`. This verifies an optimized native executable; it does not verify signed MSI/NSIS installers or release publication.
- Final executable SHA-256: `88E91B6867155202D2BD074F11594E5237A4A8A45A6A21CE2C1A02178C9893F1`.
- Launched the final optimized executable and verified its embedded `http://tauri.localhost/` UI, native microphone/output enumeration and initial idle state. `native/voice-lab.png` records that release startup; the other native screenshots were captured during desktop development verification.
- Production-browser visual suite: all five pages at 900×600, 1280×720, 1366×768, 1440×900, 1600×900, 1920×1080 and 2560×1440; 35 page/viewport assertions for horizontal bounds, headings and persistent footer; browser DPR 1, 1.25 and 1.5 captures. Real Windows scaling changes were not performed.
- Real-browser WAV-import dialog: Tab stays inside the dialog, Escape closes it, focus returns to the trigger.
- Native Tauri smoke: Windows decorations rendered; native microphone/headphones enumerated; Deep Voice changed actual DSP to −4 st pitch/−2 st formant; Soundboard, AI, Recordings and Settings navigation rendered; native screenshots saved. No live microphone recording, Discord/OBS loopback or driver installation was performed.

Machine-readable browser measurements are in `screenshots/redesign/after/performance.json`; viewport assertions are in `layout-report.json`. The recorded 15 navigation-to-paint samples span 20.60–42.15 ms (median 25.35 ms); the observed JS heap used approximately 9.50 MiB. The production build reports 412.93 kB JavaScript / 121.52 kB gzip and 85.12 kB CSS / 16.30 kB gzip. Navigation timings include Playwright input and two animation frames and are not an INP benchmark. Heap observations are browser-only; CDP task counters were enabled immediately before the snapshot and are not an elapsed CPU measurement. These measurements do not establish real-time DSP CPU, audio latency, voice quality, memory-leak freedom or external-client acceptance. Missing telemetry stays unavailable in the product.

## Remaining acceptance work

The five-page frontend redesign and supported IPC controls are implemented. Full product-level acceptance still requires microphone/cable/Discord/OBS tests, long-running audio/load testing, actual Windows scaling review and signed-installer validation. AI conversion, multiple-file recording management, sample-based file waveforms, soundboard playback telemetry/master gain and model downloads remain backend limitations identified above. There was no merge, force push, release publication or privileged driver operation.
