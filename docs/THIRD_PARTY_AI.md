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
