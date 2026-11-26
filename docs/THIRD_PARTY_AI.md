# Optional AI runtime attribution

The managed adapter targets [w-okada/voice-changer](https://github.com/w-okada/voice-changer)
at commit `7e2ce5c4800010bbc7604968bd29b802534fac05`.
Its upstream MIT notices include Wataru Okada, Isle Tennos and the other copyright holders
listed in the complete [retained license](../ai-sidecar/notices/w-okada-LICENSE.txt).
The optional package builder retains both the complete upstream LICENSE and LICENSE-NOTICE.
The base application does not bundle upstream Python source or AI voice weights.

MIT permission for application source does not automatically license model weights,
datasets, voice identities or separately distributed binary dependencies. Python,
PyTorch, ONNX Runtime, fairseq, faiss, F0 libraries, HuBERT/contentvec and any optional
RMVPE assets need their own redistribution review and retained notices. The builder
requires publisher-supplied dependency and asset notices; no approved full distribution
or license clearance is claimed here.

Beatrice JVS assets have separate noncommercial and redistribution conditions and are
excluded from packaging. Diffusion/DDSP vocoders are not integrated. Do not redistribute
those assets merely because the repository's top-level source license is MIT.

MASK was reviewed as an architectural reference, not copied or added as a runtime
dependency. Existing Radix/Lucide/React licenses remain managed by the existing project.

## Authorized local runtime execution

The pinned [69-component recipe](../ai-sidecar/runtime-recipe.json) records exact official
download URLs, versions, sizes and SHA-256 hashes. Python's PSF license, each wheel's
license metadata/notices, fairseq MIT and w-okada notices are retained in the installed
runtime. This inventory is evidence of provenance, not a completed legal/security audit
of every transitive DLL. NVIDIA CUDA wheel redistribution conditions still require review
before a general bundled release. No system GPU driver/toolkit is redistributed or installed.

The original [Facebook HuBERT base model](https://huggingface.co/facebook/hubert-base-ls960)
is Apache-2.0. The recipe downloads the publisher's original fairseq checkpoint from
`dl.fbaipublicfiles.com`; its full [license](../ai-sidecar/notices/hubert-LICENSE.txt) and
[provenance](../ai-sidecar/notices/hubert-provenance.txt) are retained. Its exact packaged
hash/path is the sole trusted embedding-checkpoint loader exception.

The curated [Tsukuyomi-chan official RVC model](https://tyc.rei-yumesaki.net/work/software/rvc/)
is owned/published by Rei Yumesaki and has
[custom terms](https://tyc.rei-yumesaki.net/work/software/rvc/terms/), **not MIT**.
Required model credit: **Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)**.
The terms permit the reviewed local voice-conversion tests and software integration with
inherited conditions/credit; they restrict advocacy, attacks and reusable voice-material
use/redistribution. Read the complete terms for any intended distribution or output use.
The application requires a separate acknowledgement and binds this license exception to
the audited ONNX digest and owner/terms/attribution metadata. It does not infer rights for
other voices from a filename or top-level repository license.

The engineering input is the owner's
[official sample voice](https://tyc.rei-yumesaki.net/material/voice/sample-voice/), whose
page permits voice-effect/changer demonstrations. Credit: **Tsukuyomi-chan official
sample voice (CV. Rei Yumesaki)**. No microphone was captured; input/output audio stays
in the ignored local build directory. Neither voice weights nor generated audio are
committed or bundled. The w-okada-hosted Tsukuyomi sample marked **VCClient only** was
explicitly excluded; the integration uses the original owner's archive instead.

Exact asset hashes, installed paths, dependency versions and test limits are in the
[execution report](AI_RUNTIME_EXECUTION_REPORT.md).
