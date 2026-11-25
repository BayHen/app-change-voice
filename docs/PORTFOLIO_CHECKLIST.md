# Portfolio checklist

The project demonstrates a Rust audio engine behind a Tauri/React desktop application. Production claims require release evidence. The supplied Voice Lab image is a design reference. Current screenshots and measurement artifacts should identify their commit.

Prepare an English README with overview, current screenshot, demo link, technology and diagrams, installation and development commands, test and release links, limitations, license and acknowledgments. Preserve the existing Windows troubleshooting instructions.

## 75-second demo

- 0–10 s: Launch Voice Lab with capture off and show the audio device selectors.
- 10–30 s: Enable headphone monitoring and compare two real presets while the input meter responds.
- 30–50 s: Record and play original and processed tracks, then show WAV export.
- 50–65 s: Show received processed voice in OBS or Discord after capture is verified on the demo machine.
- 65–75 s: Show diagnostics and the verified installer or CI artifact.

## CV drafts

- Software Engineer: Built a Windows voice processing application with bounded audio queues, a shared DSP pipeline and automated regression coverage.
- Full-stack Developer: Connected a React and TypeScript desktop interface to Rust services through typed Tauri commands for presets, recording and audio diagnostics.
- Rust Developer: Implemented CPAL/WASAPI audio paths with lock-free SPSC buffering, pitch and formant processing, noise suppression and bounded recording queues.

Update these statements with reproducible measurements only after verification.
