# Virtual microphone review — 2026-11-26

Branch: `feature/new-version`. User reports Discord silence/unchanged voice and confirms
Discord is not in a call before local cable testing.

## Findings and changes

- Actual saved routing configuration pointed to `Headphones (High Definition Audio Device)`;
  this was the physical monitoring endpoint, not the installed VB-Cable playback endpoint.
  Physical mic was HyperX QuadCast. Input/output gains were 100%, neither muted; gate and
  noise suppression were disabled. No driver reinstall was necessary.
- Virtual-output selection previously allowed ordinary speakers/headphones. The UI now
  disables unrecognized physical choices and flags an invalid saved choice. Backend
  selection/start rejects unrecognized virtual outputs before storing/activating them.
  Recognition is name-based, not a signed-driver attestation; standard VB-Cable names pass.
- Changing the virtual output intentionally stops the session, but instructions were unclear.
  The UI now states: select cable, enable Monitor, then Start routing. Start requires an
  actual running session; faulted streams are reconciled before attempting routing.
- Virtual microphone, Discord and game/app settings now use the persistent audio-session
  provider instead of independent hook instances with separate selection/busy/error state.
  Stop routing stays available for an active route even if runtime state changes.
- The saved VoiceMorph route was corrected to **CABLE Input (VB-Audio Virtual Cable)**
  through the real production command bridge. Temporary test input/output mutes were
  restored to their original values. Monitoring/routing were stopped at test completion.
  Discord's own settings were not modified; it still needs the user to select CABLE Output
  and verify Mic Test. Existing app instances must be restarted to load the corrected config.

## Evidence and limits

- Rust: **104 passed, 12 ignored**. New local loopback acceptance test was run explicitly
  twice. Frontend: **21 passed**, including invalid saved route → select cable → explicit
  Monitor start → routing activation through shared state.
- Actual local loopback: generated 997 Hz tone → VoiceMorph soundboard/processed-session
  clock → CABLE Input → paired CABLE Output capture. Input and physical output gains
  were muted. No microphone PCM was saved/sent; no Discord call was used.
- Final capture: 48 kHz, 84,960 received mono frames, peak 0.039978, 0 routing overflows,
  2 underrun events. Whole-voiced-span zero-crossing estimate was 982.61 Hz versus 997 Hz
  source; the smoke permits ±15 Hz and is **signal-delivery evidence, not pitch accuracy,
  glitch-free quality or measured hardware-loopback latency**. A previous run during
  compilation had 4 underruns and is also retained. Longer quality/clock-drift analysis
  remains outside this short routing check.
- Actual production WebView2: rejects headphone routing, preserves config on failed choice,
  accepts CABLE Input, leaves capture stopped after selection, requires explicit Monitor,
  activates routing and updates the persistent footer, then stops and restores volume.
  First harness attempt expected a hidden embedded header badge; changing the assertion
  to the visible Stop routing button fixed the harness. Native application commands worked.
- Optimized EXE and NSIS build passed (Rust release compilation 2m 40s); TypeScript/Vite,
  ESLint, formatting and diff checks passed. Native scratch EXE hash matches release EXE.
- No physical device disconnect, long call, Discord Mic Test, network call delivery,
  or AI-to-Discord quality test was performed. AI inference was verified in the preceding
  runtime audit, independently of this routing review. No driver/security policy changed.

Evidence:

- [Native report](screenshots/virtual-mic-review/native-report.json)
- [Cable loopback report](screenshots/virtual-mic-review/loopback-report.json)
- [Loopback during build](screenshots/virtual-mic-review/loopback-during-build.json)
- [Native active route screenshot](screenshots/virtual-mic-review/routing-active.png)
- [Vietnamese user guide](HUONG_DAN_SU_DUNG.md)

Current artifacts:

```text
src-tauri/target/release/voicemorph-studio.exe
SHA-256 b0826ced7ca36f8e58b7704fbaa63b1ec739cd3fe848939b98a9664dd8d9aacf
src-tauri/target/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe
SHA-256 2b9577fc645758a1060f6557584fc1b7396ffa77042576240772497394dfabe0
```
