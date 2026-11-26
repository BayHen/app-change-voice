# VoiceMorph Studio design system

The redesign follows the supplied dark studio references: navy surfaces, restrained violet illumination, a persistent left workspace rail, compact audio controls, model/voice tiles, and an uninterrupted bottom audio bar. The references inform a unified desktop tool rather than a marketing page or a reproduction of another application's interface.

## Tokens and typography

The source of truth is `src/styles/index.css`; shared layouts and component states live in `src/styles/studio.css`.

| Role                               | Token / value                                                                     |
| ---------------------------------- | --------------------------------------------------------------------------------- |
| Workspace                          | `--background: #10121f`                                                           |
| Navigation                         | `--sidebar: #121423`                                                              |
| Panel                              | `--surface: #191d2e`                                                              |
| Raised control                     | `--elevated: #22263b`                                                             |
| Control track                      | `--control: #30354e`                                                              |
| Border                             | `--border: #30354a`                                                               |
| Accent                             | `--primary: #9360ff`                                                              |
| Action background                  | `--gradient-primary`, #8550e8 to #8f50e9                                          |
| Primary / secondary text           | #f5f5fa / #a8adc6                                                                 |
| Success / warning / error / info   | #22df8c / #f5b942 / #f04455 / #22b5ff                                             |
| Typography                         | Windows Segoe UI Variable, Segoe UI, system sans-serif; no external font requests |
| Body / controls / helper / heading | 13 / 12 / 11 / 28 px; compact desktop secondary annotations may be smaller        |
| Font weights                       | 400 / 500 / 600                                                                   |
| Panel / control / dialog radius    | 12 / 7 / 14 px                                                                    |
| Base spacing                       | 8 px; compact 4 px subdivisions                                                   |
| Standard control height / icon     | 36 / 18 px                                                                        |
| Motion                             | 160 ms feedback transitions, reduced-motion override                              |
| Keyboard focus                     | 2 px violet outline with 2 px offset                                              |

Legacy Tailwind zinc/cyan classes resolve to the same navy text and violet accent palette through `@theme`. This preserves existing component behavior during migration without shipping a second visual theme. Two-band bass/treble shelving is labeled honestly; there is no invented three-band equalizer.

Contrast calculations using WCAG relative luminance: primary text/panel 15.38:1; secondary text/panel 7.53:1; white text against the lightest primary-button endpoint 4.67:1. Hover uses #8950df (4.91:1). Disabled controls remain visibly distinct.

## Shared components

| Component                    | Source                              | Contract                                                                                                                                                                |
| ---------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Button / icon button         | `src/components/ui.tsx`             | Primary, secondary, ghost, danger; native disabled semantics; accessible names required for icon-only use                                                               |
| Card                         | `src/components/ui.tsx`             | One panel border, radius and surface shared across the five workspaces                                                                                                  |
| Badge                        | `src/components/ui.tsx`             | Neutral, success and warning; status includes text                                                                                                                      |
| SearchInput                  | `src/components/ui.tsx`             | Visible search affordance, accessible input name and focus-within ring                                                                                                  |
| Slider                       | `src/components/ui.tsx`             | Native keyboard-operable range, visible numeric value and units                                                                                                         |
| StudioDialog                 | `src/components/ui.tsx`             | Radix focus containment, Escape dismissal, scroll containment and trigger-focus restoration; used by preset creation, preset editing, WAV import and recording deletion |
| ToggleSwitch                 | `src/components/ToggleSwitch.tsx`   | Radix switch with controlled state and keyboard interaction                                                                                                             |
| EmptyState                   | `src/components/ui.tsx`             | Shared icon frame, title, helpful explanation and optional action                                                                                                       |
| AppHeader                    | `src/components/AppHeader.tsx`      | One heading/subtitle/action structure across the five pages                                                                                                             |
| AppSidebar                   | `src/components/AppSidebar.tsx`     | Exactly five primary destinations, current-page semantics, icon rail at narrow desktop widths                                                                           |
| AudioStatusBar               | `src/components/AudioStatusBar.tsx` | Shell-owned session status, measured latency, estimated DSP-core load, monitoring, recording and routing status                                                         |
| Waveform                     | `src/components/Waveform.tsx`       | Canvas history of measured input amplitude; 120 samples, 10 Hz paint, no generated PCM; timer cleanup and hidden-document pause                                         |
| InputLevelMeter / LevelMeter | Existing shared components          | Real backend peak/RMS dBFS, offline floor, clipping color semantics                                                                                                     |
| NotificationToast            | Existing shared component           | Application-wide desktop-action notifications; placed above persistent audio bar                                                                                        |

Native selects, details disclosures and range controls are used where they already provide accessible behavior. A vertical fader, tooltip package, animation runtime, WaveSurfer player and dropdown-menu dependency were not added merely to satisfy a component inventory. Unsupported file peaks/seeking and AI inference settings are not simulated.

All visible icons now use Lucide. The small `icons.tsx` adapter preserves existing component imports and the legacy `weight` prop while replacing Phosphor; the unused Phosphor dependency was removed.

## Layout and interaction

The five destinations are Voice Lab, Soundboard, AI Voice Studio, Recordings and Settings. Previous effects, DSP, audio-device, virtual-microphone and application-profile workspaces remain accessible through Settings. Existing desktop navigation-event destinations remain supported.

The native Windows title bar, window decorations, resize limits and DPI handling are unchanged. A shared in-content title strip supplies workspace identity without drawing fake native window controls.

Desktop breakpoints are 1800, 1250, 1200, 1000 and 760 CSS pixels, reflecting the existing 900 px minimum window. At narrower desktop widths the rail collapses to icons with retained accessible names; Voice Lab reduces its preset columns and panels can scroll vertically. The fixed footer always has reserved bottom content space. At wide widths a centered, bounded workspace prevents excessively long controls.

## State and audio ownership

`AudioSessionProvider` mounts once above navigation. Runtime, device selection, recording and routing hooks are owned by that provider; route changes never invoke stop, unload or recording teardown. Runtime telemetry is polled at 10 Hz (previously 25 Hz); recording state at 4 Hz. Canvas uses refs and a bounded buffer rather than React state for continuous history.

DSP load is a single-core estimate derived exactly like Rust diagnostics: `averageProcessingMicros * sampleRate / 10000`. It is not whole-process CPU or GPU utilization. Missing telemetry displays an em dash. No microphone is started automatically.

Voice-card selection derives from current DSP/noise parameters and survives navigation. The six templates use existing pitch, formant, ring modulation, makeup gain and suppression. Custom saves retain current EQ, creative processors, bypass and noise settings; Export targets the most recently created saved preset or an active library preset.

## Reproduction

```powershell
npm run build
npm run preview -- --host 127.0.0.1 --port 1422
# In a second terminal:
npm run test:visual
```

The visual script defaults to the production preview above. Override `VOICEMORPH_TEST_URL` to target another explicitly chosen server. Browser screenshots deliberately show the real disconnected bridge, without production demo data. `capture-before.mjs` is historical baseline tooling for the previous navigation and is not intended to run against the redesigned app.
