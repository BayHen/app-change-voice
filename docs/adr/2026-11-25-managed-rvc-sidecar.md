# ADR: Managed RVC sidecar behind the Rust audio engine

Status: accepted for an integration candidate; production approval blocked by authorized
runtime/model availability and real hardware validation.

## Context

VoiceMorph owns capture, DSP, monitoring, recording, soundboard mixing and WASAPI output.
Replacing it with the upstream web application would duplicate devices and lose the shared
studio workflow. Native Rust RVC support is not implemented. The audited w-okada revision
has a usable Python RVC API, but old dependency constraints and separate asset licenses.

## Decision

Use a separately installed, explicitly trusted portable runtime and one managed process.
Call only `RVCr2` and `VoiceChangerV2`; bypass upstream manager/server/device initialization.
Use inherited private stdio pipes instead of HTTP/Socket.IO. Bound messages and all audio
queues; inference/resampling/serialization run on a worker. `VoiceConversionBackend` is
the replacement boundary for future native inference. Classic continues in the existing
Rust pipeline and requires no AI download.

Default failure output is silence. Generation tags and downstream privacy gates discard
old voice audio. An explicit persisted fallback policy may restore Classic on an AI fault;
explicit Stop always mutes until Classic is deliberately selected. Never use raw samples
to pad missing AI output. Report unavailable metrics as unavailable.

Install only a bounded manifest of regular contained files. Verify a publisher-supplied
manifest digest and every file, bind the bridge digest to the app build, stage immutable
releases, atomically activate and retain a rollback pointer. Runtime code is trusted code,
not sandboxed untrusted data. No online catalog is enabled without an approved artifact.

## Consequences

No AI listener/port/authentication token is exposed. Users do not need Python/pip or a
separate browser, once a compatible optional package exists. A Python runtime increases
optional download size and needs dependency/license/security review. The pinned ONNX
content embedder is a stub, so a trusted licensed embedding checkpoint is still necessary;
user voice checkpoints remain prohibited. Native Rust inference, DirectML, Beatrice and
online downloads are deferred. The existing virtual output endpoint is preserved, while
a signed native cable-free driver remains separate blocked work.

See [audit, alternatives and compatibility matrix](../AI_ENGINE_IMPLEMENTATION.md) and
[verification evidence](../AI_ENGINE_VERIFICATION.md).
