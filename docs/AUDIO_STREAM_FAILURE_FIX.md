# Monitoring stream error correction — 2026-11-26

Branch: `feature/new-version`.

The reported screenshot showed the monitor enabled and engine running alongside a fatal
driver-error banner. The old input/output callbacks discarded the CPAL error itself and
latched `streamFaulted` for every notification. CPAL 0.18.2 uses the same callback for
recoverable `Xrun`, `DeviceChanged` and `RealtimeDenied` notifications. The original
screenshot's exact error kind cannot be recovered because it was discarded.

The corrected callbacks count input overrun/output underrun notifications without
declaring the stream permanently broken. Automatic route-change and scheduling-quality
notifications also leave a live stream active. Other errors retain the input/output
direction and actual CPAL error description. The same recoverable classification is
applied to virtual output.

For terminal monitoring failures, the next control-thread status poll (normally 100 ms)
releases both streams and stops recording/playback/routing through the existing session
cleanup. The returned state is `faulted`, not `running`. Diagnostics remain available
until explicit stop or retry. There is no automatic microphone restart or device switch.
The status bar turns Monitor off, disables recording, hides stale timing/CPU telemetry
and displays the actual driver error with refresh/retry instructions. Successful explicit
retry clears the derived driver banner.

## Verification

- Rust default suite: **104 passed, 11 ignored**, zero failures. Classification regression
  distinguishes recoverable notifications from actual device-unavailable failures.
- Frontend: **20 passed**, including actual hook polling/error clearing and the status
  bar's faulted → explicit retry transition, without automatic capture start.
- Muted hardware diagnostic explicitly run twice: approximately 3 s, 48 kHz, stereo,
  default WASAPI input/output, no recording or upload. Final sample: 3 underrun events,
  1 overflow event, 0 terminal stream errors; state remained running, output peak zero.
  These counters include application queues and are not all attributed to the driver.
  The recording worker initialized two 44-byte WAV headers in its diagnostic temp
  directory; no PCM samples were recorded to those files.
- The final hardware test then **injected** `DeviceNotAvailable` through the same metrics
  handler while real streams existed. It verified terminal fault status, stream release,
  retained output diagnostics and explicit stop clearing. This was an injected error,
  not physical unplug/replug or an OS device-setting change.
- TypeScript/Vite production build, ESLint, Rust formatting and modified-file Prettier
  checks passed. No dependency or DSP algorithm changed.
- Optimized Windows production EXE and NSIS installer rebuilt; latest artifacts written
  at 2026-11-26 00:41:39 (Asia/Saigon). The final rebuild also corrected the regression
  test's unsupported `getByRole` `exact` option to an anchored name expression; all
  20 frontend tests and TypeScript/Vite then passed.
  Artifacts: `src-tauri/target/release/voicemorph-studio.exe` and
  `src-tauri/target/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe`.

Artifact SHA-256 values:

```text
EXE  7d88fc5fc5ffcdbb1bed552be5ae4334ae4c4e1281b6d5ebace6f2be9242e10f
NSIS cd808ef86f3d304b6b0dea4cb44296be5d04dfc9add01835c5f4099b77dc0b9e
```

This short test does not establish long-session glitch-free playback or hardware
loopback latency. Persistent glitches still require the actual retained driver detail
and buffer/load investigation. No Windows driver/security setting was changed.

Old Cargo incremental-cache ownership produced cleanup warnings during tests; test
compilation/execution succeeded. The cache was not deleted and ACLs were not changed.
