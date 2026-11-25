# Testing strategy

## P0 Windows acceptance procedure

Use a disposable Windows 10/11 x64 VM or an approved test machine. Record OS and WebView2 builds, commit, package SHA-256 and install mode. Test NSIS and MSI independently with clean snapshots. Install as a standard user where supported, launch, confirm working IPC and microphone idle on startup, create a preset and recording, upgrade with the same product identifier, compare the persisted data, uninstall, then reinstall to confirm the expected data remains. Capture MSI installer or WiX logs when a step fails. Preserve the production bundle policy while investigating failures.

Use one account for dependency installation and build. From a fresh checkout run `npm.cmd ci` and `npm.cmd run tauri:build:windows`. Inspect `Get-Service msiserver` and capture the full WiX message if ICE fails. If an MSI is generated, run `msiexec.exe /i "<candidate.msi>" /L*v "<install.log>"` in the VM and retain the log. For NSIS, use the app's normal installer and uninstall entry. Snapshot app-state directories before upgrades and compare the generated preset and recording after launch.

For cable acceptance, use headphones and a user-installed VB-CABLE. Record both endpoint names, driver version and negotiated sample rate. Send only an authorized test phrase. In VoiceMorph, select the physical microphone and headphones, then select CABLE Input as the virtual playback sink. In Discord Mic Test and OBS Audio Input Capture, select CABLE Output. Compare Natural and Robot/Deep Voice and verify mute by checking the captured audio. Repeat with Hear Myself and dual-track recording enabled. Open both clients, close one, switch the physical output, and unplug/reconnect the physical microphone. On an approved test machine, repeat with the cable endpoint unavailable and confirm the UI reports failure and can manually recover. A moving meter alone does not verify processed capture.

The seven ignored baseline tests are two Windows device discovery checks (one requires a physical input), two virtual sink checks (open/stop and sample-rate conversion), two live stream checks (headphones and, for routing, an installed cable), and one optimized synthetic DSP benchmark. Run hardware tests individually on a prepared machine. Test summaries must record which ignored tests were explicitly run.

P0 ran `cargo test --locked --manifest-path src-tauri/Cargo.toml platform::windows::manual_hardware_tests -- --ignored --nocapture` on the development machine. Both device discovery tests passed. They establish that CPAL sees at least one input and exposes unique endpoint IDs. They do not exercise audio capture, virtual routing or an external application.

Reuse [the existing test plan](TEST_PLAN.md). On feature/new-version at c5c419f the local baseline passed version validation, Prettier, ESLint, production TypeScript/Vite build, rustfmt, Cargo check and Clippy with warnings denied. Vitest passed 10 tests. Rust passed 80 tests with 7 hardware/performance cases ignored and a local incremental-cache cleanup permission warning during cargo test.

- Pull request gates: locked npm install on a clean runner, version, format, lint, Vitest, production frontend, rustfmt, Clippy, Rust tests and x64 artifact validation.
- Frontend gaps: error and loading recovery, keyboard focus, resize and scale, event cleanup, and IPC integration against the installed WebView2 app. Existing UI tests use mocks.
- Rust gaps: transactional failures, multi-client routing, device-loss recovery and callback saturation. Use injectable failures and deterministic authorized synthetic fixtures.
- Windows test matrix: current supported Windows 10/11 test images, microphone and headphone switching, VoiceMorph restart, device reconnect, sleep/resume, Discord/OBS and concurrent capture, recording/export, global hotkey conflicts, tray exit, clean installer and uninstall. Add a signed updater run with valid, invalid and unavailable update cases.
- Record commit, OS build, endpoint and driver versions, sample rate, frame count, expected result, observed result and status. Keep manual, automated and historical evidence separate.
