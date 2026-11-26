# Optional managed RVC runtime

`bridge.py` adapts the pinned w-okada `RVCr2` and `VoiceChangerV2` modules to private
bounded JSON-line stdio RPC. Rust owns audio devices, queues, routing and process lifecycle.
The upstream web server/manager is never started. End users choose **Install recommended
runtime** in AI Voice Studio or Settings; they do not install Python, run pip or open a
terminal. The base installer contains the pinned recipe/adapter, not the multi-GB runtime
or voice weights. See the [actual execution report](../docs/AI_RUNTIME_EXECUTION_REPORT.md).

## Managed installation

The Windows x64 recipe pins 69 official HTTPS artifacts and SHA-256 digests. It assembles
embedded Python, CUDA-compatible PyTorch/ORT, the pinned upstream source and Apache-2.0
HuBERT in the user's application data directory. Download size is 3.90 GiB; installation
requires 13 GiB free. Downloads are cached/resumable, extraction rejects traversal and
links, and activation is atomic. Cancel/retry preserves the previous active runtime.
Updates use a new reviewed recipe shipped with an application release; no floating pip
upgrade is run. Repair reconstructs and verifies the pinned release. Rollback is available
after changing releases. No administrator access, CUDA toolkit or GPU driver install is
performed. CPU fallback is opt-in. Private inference works offline after installation.

This tested compatibility baseline uses **Python 3.10.11, now end of support**. Qualify a
supported Python/dependency baseline and complete binary redistribution/security review
before general production redistribution. This is a working local integration candidate.
The fairseq 0.12.2 source distribution lacks a native training source file; the recipe uses
its unmodified Python inference modules and retains MIT notices, omitting unused training
extensions. It does not claim a successful stock fairseq native wheel build.

The curated **Get official Tsukuyomi-chan voice** action downloads the original owner's
archive, verifies the exact checkpoint, converts Normal 1 locally with the managed runtime,
and verifies the resulting ONNX digest. It requires acknowledgement of the owner's custom
terms. This narrow reviewed converter does not enable arbitrary checkpoint imports.

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
publisher trust. Copying is cancellable; the prior runtime remains active on failure.
This remains an alternative to the recommended recipe. Do not distribute test fixtures
as usable AI packages.

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
Generic imports support MIT, Apache-2.0, BSD-2-Clause and BSD-3-Clause. The curated
`LicenseRef-TsukuyomiChan-Official` exception is bound to the exact audited ONNX digest,
original-owner URL, full terms URL and required attribution; it is not an MIT license.
These metadata labels alone do not establish permission. Upstream ONNX RVC metadata must
also match supported embedding dimensions/model rates. Legacy generic ONNX imports remain
storage-only and cannot be selected for this RVC engine.

## Local tests

`python -m unittest discover -s ai-sidecar -p test_bridge.py` needs only stdlib. It verifies
protocol/config contracts and explicitly performs no real inference or benchmark.
Real CUDA/CPU inference, curated conversion and crash-recovery acceptance tests are
separate ignored-by-default tests requiring the approved local assets. See the
[execution report and remaining release blockers](../docs/AI_RUNTIME_EXECUTION_REPORT.md).
