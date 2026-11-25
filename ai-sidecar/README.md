# Optional managed RVC runtime

`bridge.py` adapts the pinned w-okada `RVCr2` and `VoiceChangerV2` modules to private
bounded JSON-line stdio RPC. Rust owns audio devices, queues, routing and process lifecycle.
The upstream web server/manager is never started. A host Python installation is unnecessary
for end users. This repository does not contain an approved portable runtime or voice weights.

## Release builder only

Prepare a self-contained Windows x64 Python **3.10** distribution with compatible dependencies
for upstream commit `7e2ce5c4800010bbc7604968bd29b802534fac05`. Review and pin that environment's
dependency versions, binary redistributability and vulnerabilities. Do not pip-install the old
upstream requirements into the host Python 3.14 environment. The builder tests dependency and
upstream imports, but that does not validate inference or CUDA.

Run `python scripts/package-ai-runtime.py --help`. Required inputs are the prepared Python
directory, exact clean upstream checkout, an approved HuBERT/contentvec checkpoint, its license
and HTTPS source, dependency notices, and a **new** destination folder. RMVPE ONNX and its notice
are optional. The builder performs no downloads. It copies the app's adapter, upstream source
and full license/notice files, and emits `runtime-manifest.json` plus its publisher SHA-256.

HuBERT's upstream ONNX embedder is a stub. The trusted runtime therefore contains an embedding
checkpoint. Only that exact verified packaged path may use `torch.load`. User voice imports
remain ONNX-only; arbitrary checkpoints and archives are never passed to that loader.
The publisher must independently approve every executable, DLL and embedding asset; a digest
does not turn an untrusted checkpoint or runtime into safe code.

Distribute the manifest digest independently through a trusted publisher channel. In AI Voice
Studio, choose **Install local package**, select the folder, enter the digest and acknowledge
publisher trust. Copying is cancellable; the prior runtime remains active on failure. No online
catalog is enabled yet. Do not distribute synthetic test fixtures as usable AI packages.

## Voice model manifest

Provide a JSON manifest alongside a licensed RVC `.onnx` voice model:

```json
{
  "schemaVersion": 1,
  "id": "licensed-voice",
  "name": "Licensed voice",
  "description": "Voice approved by its owner",
  "architecture": "rvc",
  "backend": "w-okada-rvc",
  "license": "MIT",
  "sourceUrl": "https://publisher.example/model",
  "modelFile": "voice.onnx",
  "sha256": "REPLACE_WITH_THE_REAL_64_CHARACTER_FILE_SHA256",
  "inputSampleRate": 48000,
  "frameSamples": 4096,
  "supportedProfiles": []
}
```

The example is intentionally not importable until real provenance/hash values are supplied.
`consentConfirmed` is set by the app only after the user's explicit import acknowledgement.
Supported license identifiers currently are MIT, Apache-2.0, BSD-2-Clause and BSD-3-Clause.
These metadata labels alone do not establish permission. Upstream ONNX RVC metadata must
also match supported embedding dimensions/model rates. Legacy generic ONNX imports remain
storage-only and cannot be selected for this RVC engine.

## Local tests

`python -m unittest discover -s ai-sidecar -p test_bridge.py` needs only stdlib. It verifies
protocol/config contracts and explicitly performs no real inference or benchmark.
See [implementation audit and release blockers](../docs/AI_ENGINE_IMPLEMENTATION.md).
