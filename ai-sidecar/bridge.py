"""VoiceMorph's private stdio adapter for the pinned w-okada RVC implementation.

No web server, local audio devices, downloads or recording. User voice models are
ONNX only; the trusted runtime publisher supplies the digest-verified HuBERT asset.
The portable runtime (including Python) is a separately verified optional package.
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import logging
import math
import os
from pathlib import Path
import sys
import time
import struct

PROTOCOL = 1
UPSTREAM_COMMIT = "7e2ce5c4800010bbc7604968bd29b802534fac05"
MAX_MESSAGE = 256 * 1024
MAX_CHUNK = 16384
RVC_CONTENTVEC_FILE = "assets/contentvec-rvc.pt"
RVC_CONTENTVEC_SHA256 = "f54b40fd2802423a5643779c4861af1e9ee9c1564dc9d32f54f20b5ffba7db96"


def resolve_content_encoder(root, manifest, expected, hash_file):
    default = manifest["contentVecModel"]
    if expected is None:
        return contained(root, default)
    # Imported pack metadata can select a verified asset, never supply a pickle path.
    candidates = {default}
    if expected == RVC_CONTENTVEC_SHA256:
        candidates.add(RVC_CONTENTVEC_FILE)
    for entry in manifest["files"]:
        if entry["path"] in candidates and entry["sha256"].lower() == expected:
            asset = contained(root, entry["path"])
            if hash_file(asset) == expected:
                return asset
    raise ValueError("Required feature encoder is unavailable or failed verification; repair the managed runtime")


def load_flat_index(path, expected_sha, np):
    """Decode the bounded Phase 6 format, never FAISS's unrestricted deserializer."""
    import faiss
    with path.open("rb") as stream:
        header = stream.read(45)
        if len(header) != 45 or header[:4] != b"IxF2":
            raise ValueError("Expected a FAISS IndexFlatL2")
        dimension, rows = struct.unpack_from("<IQ", header, 4)
        floats, = struct.unpack_from("<Q", header, 37)
        if (dimension != 768 or not 8 <= rows <= 16000 or floats != rows * dimension
                or header[32:37] != b"\x01\x01\x00\x00\x00"):
            raise ValueError("Invalid RVC v2 feature index header")
        values = stream.read(4 * floats + 1)
    digest = hashlib.sha256(header)
    digest.update(values)
    if len(values) != 4 * floats or digest.hexdigest() != expected_sha:
        raise ValueError("Feature index checksum or storage length mismatch")
    matrix = np.frombuffer(values, dtype="<f4").reshape(rows, dimension)
    if not np.isfinite(matrix).all() or float(np.max(np.abs(matrix))) > 1000:
        raise ValueError("Invalid values in feature index")
    index = faiss.IndexFlatL2(dimension)
    index.add(matrix)
    return index, matrix


def align_indexed_pipeline(pipeline, np, torch):
    """Keep rolling F0 on the audio clock and protect unvoiced, pre-retrieval features."""
    extract_pitch = pipeline.extractPitch
    extract_features = pipeline.extractFeatures
    infer = pipeline.infer
    execute = pipeline.exec
    base_features = None
    protection = None

    def aligned_pitch(audio, uses_f0, history, shift, silence):
        if uses_f0:
            frames = audio.shape[0] // pipeline.window
            history = np.asarray(history)
            missing = frames - len(history)
            history = np.pad(history, (missing, 0)) if missing > 0 else history[-frames:]
        return extract_pitch(audio, uses_f0, history, shift, silence)

    def capture_features(audio, layer, project):
        nonlocal base_features
        features = extract_features(audio, layer, project)
        if protection is not None:
            base_features = features
        return features

    def protect_unvoiced(features, length, pitch, f0, speaker, out_size):
        if protection is not None and base_features is not None:
            original = base_features.repeat_interleave(2, dim=1)[:, -features.shape[1]:, :]
            mask = torch.where(f0 > 0, 1.0, protection).unsqueeze(-1)
            features = (features * mask + original * (1 - mask)).to(features.dtype)
        return infer(features, length, pitch, f0, speaker, out_size)

    def run(sid, audio, pitchf, feature, shift, rate, uses_f0, silence, layer, project,
            repeat, protect=0.5, out_size=None):
        nonlocal protection, base_features
        protection = protect if rate and uses_f0 and protect < 0.5 else None
        try:
            # The pinned upstream Protect block uses a post-retrieval copy and can
            # broadcast mismatched frame counts. Apply Protect above at inference instead.
            return execute(sid, audio, pitchf, feature, shift, rate, uses_f0, silence, layer,
                           project, repeat, 0.5, out_size)
        finally:
            protection = base_features = None

    pipeline.extractPitch = aligned_pitch
    pipeline.extractFeatures = capture_features
    pipeline.infer = protect_unvoiced
    pipeline.exec = run


def install_pcm_guards(model, np, torch):
    """Protect upstream's direct int16 casts; do not add another overlap/crossfade."""
    counters = {"modelSaturatedSamples": 0, "resampledSaturatedSamples": 0}
    infer = model.pipeline.inferencer.infer
    def bounded_infer(*args, **kwargs):
        output = infer(*args, **kwargs).float()
        if not bool(torch.isfinite(output).all()):
            raise ValueError("Non-finite model output before upstream PCM conversion")
        low, high = -32768.0 / 32767.5, 32767.0 / 32767.5
        counters["modelSaturatedSamples"] += int(((output < low) | (output > high)).sum().item())
        return output.clamp(low, high)
    model.pipeline.inferencer.infer = bounded_infer
    inference = model.inference
    def bounded_inference(*args, **kwargs):
        output = inference(*args, **kwargs)
        if not np.isfinite(output).all():
            raise ValueError("Non-finite resampled model output")
        counters["resampledSaturatedSamples"] += int(np.count_nonzero((output < -32768) | (output > 32767)))
        return np.clip(output, -32768, 32767)
    model.inference = bounded_inference
    return counters


class ContextualRvcStream:
    """RVC v2 with a sample-clock crop, past context and one block of lookahead."""

    def __init__(self, model, config, np, torch):
        from scipy import signal
        self.model, self.config, self.np, self.torch = model, config, np, torch
        self.signal = signal
        self.highpass = signal.butter(5, 48, btype="highpass", fs=16000, output="sos")
        self.history = np.zeros(max(14400, config["extraContextFrames"]), dtype=np.float32)
        self.pending = None
        self.tail = None
        self.position = 0
        self.overlap = config["crossfadeFrames"]
        block = config["chunkFrames"]
        self.right_context = min(block, max(2400, block // 2, self.overlap + 1440))
        window = len(self.history) + block + self.right_context
        # A fixed shape prevents the CUDA provider from choosing new convolution
        # algorithms at each fractional hop position.
        self.window_frames = ((window + 959 + 960 + 959) // 960) * 960
        self.fade = np.linspace(0, 1, self.overlap, endpoint=False, dtype=np.float32)

    def convert(self, pcm):
        np, torch = self.np, self.torch
        self.last_timing = {}
        next_block = pcm.astype(np.float32) / 32768.0
        current, self.pending = self.pending, next_block
        if current is None:
            return None
        block = len(current)
        left = len(self.history)
        pad_left = (self.position - left) % 960
        audio = np.concatenate((self.history, current, next_block[:self.right_context]))
        # Anchor both the 20ms encoder hop and 10ms F0 hop to time zero.
        audio = np.pad(audio, (pad_left, self.window_frames - pad_left - len(audio)), mode="reflect")
        target_start = left + pad_left
        old = np.concatenate((self.history, current))
        self.history = old[-left:]
        self.position += block
        if max(np.max(np.abs(current)), np.max(np.abs(next_block[:self.overlap]))) <= self.config["noiseThreshold"]:
            self.tail = None
            return np.zeros(block, dtype=np.int16)

        pipe = self.model.pipeline
        started = time.perf_counter()
        with torch.inference_mode():
            at_16k = self.signal.resample_poly(audio, 1, 3)
            at_16k = self.signal.sosfiltfilt(self.highpass, at_16k).astype(np.float32)
            resampled = time.perf_counter()
            pitch, pitchf = pipe.pitchExtractor.extract(
                at_16k, np.zeros(len(at_16k)//160 + 1, np.float32),
                self.config["pitchSemitones"], 16000, 160,
            )
            pitched = time.perf_counter()
            # RMVPE returns an extra endpoint for a hop-aligned input. Its first
            # frame belongs to sample zero; don't right-align it to a shorter tensor.
            features = pipe.extractFeatures(torch.from_numpy(at_16k[None, :]).to(pipe.device), 12, False)
            embedded = time.perf_counter()
            aligned_frames = min(features.shape[1] * 2, len(pitch), len(at_16k)//160)
            pitch = torch.as_tensor(pitch[None, :aligned_frames], device=pipe.device, dtype=torch.long)
            pitchf = torch.as_tensor(pitchf[None, :aligned_frames], device=pipe.device)
            original = features
            if self.model.settings.indexRatio > 0:
                query = features[0].float().cpu().numpy()
                distances, neighbors = pipe.index.search(query, 8)
                nearest = np.maximum(distances, 1e-6)
                weights = np.square(nearest[:, :1] / nearest)
                weights /= weights.sum(axis=1, keepdims=True)
                retrieved = np.einsum("ij,ijk->ik", weights, pipe.big_npy[neighbors])
                features = torch.lerp(original, torch.as_tensor(retrieved, device=pipe.device, dtype=original.dtype)[None],
                                      self.model.settings.indexRatio)
            features = features.repeat_interleave(2, dim=1)[:, :aligned_frames]
            if self.model.settings.indexRatio > 0 and self.model.settings.protect < 0.5:
                original = original.repeat_interleave(2, dim=1)[:, :aligned_frames]
                strength = torch.where(pitchf > 0, 1.0, self.model.settings.protect).to(features.dtype)
                features = torch.lerp(original, features, strength.unsqueeze(-1))
            length = torch.tensor([aligned_frames], device=pipe.device, dtype=torch.long)
            sid = torch.tensor([self.model.settings.dstId], device=pipe.device, dtype=torch.long)
            prepared = time.perf_counter()
            waveform = pipe.inferencer.infer(features, length, pitch, pitchf, sid, None).cpu().numpy()
            generated = time.perf_counter()

        rate = self.model.slotInfo.samplingRate
        divisor = math.gcd(rate, 48000)
        at_48k = self.signal.resample_poly(waveform, 48000//divisor, rate//divisor)
        end = target_start + block + self.overlap
        if end > len(at_48k):
            raise RuntimeError("Neural context did not cover the requested audio interval")
        output = at_48k[target_start:target_start+block].copy()
        if self.tail is not None:
            output[:self.overlap] = self.tail * (1-self.fade) + output[:self.overlap] * self.fade
        self.tail = at_48k[target_start+block:end].copy()
        pcm = np.clip(np.rint(output * 32768.0), -32768, 32767).astype(np.int16)
        self.last_timing = {
            "inputResample": resampled - started, "rmvpe": pitched - resampled,
            "contentVecDispatch": embedded - pitched, "featuresAndFaiss": prepared - embedded,
            "onnxAndTransfers": generated - prepared, "outputResampleAndOverlap": time.perf_counter() - generated,
        }
        return pcm


def contained(root: Path, relative: str) -> Path:
    if not relative or "\\" in relative or ":" in relative or any(
        part in ("", ".", "..") for part in relative.split("/")
    ):
        raise ValueError("Unsafe relative asset path")
    path = root.joinpath(relative).resolve(strict=True)
    if not path.is_relative_to(root.resolve(strict=True)) or not path.is_file():
        raise ValueError("Asset escapes its package")
    return path


def validate_config(value: dict) -> dict:
    if value.get("provider") not in ("auto", "cpu", "cuda"):
        raise ValueError("Unsupported provider")
    if type(value.get("pitchSemitones")) is not int or not -12 <= value["pitchSemitones"] <= 12:
        raise ValueError("Pitch must be -12..12")
    if value.get("f0Method") not in ("dio", "harvest", "rmvpe_onnx"):
        raise ValueError("Unsupported F0 method")
    chunk, crossfade = value.get("chunkFrames", 0), value.get("crossfadeFrames", 0)
    if chunk not in (2048, 4096, 8192, 16384) or not 128 <= crossfade <= min(4096, chunk):
        raise ValueError("Invalid chunk/crossfade")
    if type(value.get("cpuThreads")) is not int or not 1 <= value["cpuThreads"] <= 8:
        raise ValueError("Invalid CPU budget")
    if not 0 <= value.get("extraContextFrames", -1) <= 32768:
        raise ValueError("Invalid context")
    if not 0 <= value.get("noiseThreshold", -1) <= 0.1:
        raise ValueError("Invalid noise threshold")
    if value.get("failurePolicy") not in ("mute", "classicDsp"):
        raise ValueError("Invalid failure policy")
    for name, upper in (("indexRate", 1), ("protect", 0.5)):
        setting = value.get(name)
        if setting is not None and (type(setting) not in (int, float) or not 0 <= setting <= upper):
            raise ValueError(f"Invalid {name}; expected 0..{upper}")
    return value


class WOkadaEngine:
    def __init__(self, root: Path):
        self.root = root.resolve(strict=True)
        self.manifest = json.loads(contained(root, "runtime-manifest.json").read_text("utf-8"))
        if self.manifest["upstreamCommit"] != UPSTREAM_COMMIT:
            raise ValueError("Unexpected upstream revision")
        self.vc = None
        self.model = None
        self.config = None
        self.provider = None
        self.warmup_chunks = 0
        self.context_stream = None

    def probe(self):
        import torch
        import onnxruntime as ort
        import platform
        return {"upstreamCommit": UPSTREAM_COMMIT, "sampleRate": 48000,
                "channels": 1, "pcmFormat": "s16le", "providers": ort.get_available_providers(),
                "cudaDevices": [torch.cuda.get_device_name(i) for i in range(torch.cuda.device_count())],
                "torchVersion": torch.__version__, "onnxRuntimeVersion": ort.__version__,
                "pythonVersion": platform.python_version(), "cpuThreads": os.cpu_count(),
                "torchCudaAvailable": torch.cuda.is_available(), "torchCudaVersion": torch.version.cuda,
                "gpuMemoryBytes": [torch.cuda.get_device_properties(i).total_memory for i in range(torch.cuda.device_count())]}

    def load(self, request):
        config = validate_config(request["config"])
        path = Path(request["modelPath"]).resolve(strict=True)
        if path.suffix != ".onnx" or not path.is_file() or path.stat().st_size > 2 * 1024**3:
            raise ValueError("Only bounded RVC ONNX models are supported")
        digest = self.hash_file(path)
        if digest.lower() != request["sha256"].lower():
            raise ValueError("Model changed after import")
        if request.get("consentConfirmed") is not True:
            raise ValueError("Voice rights acknowledgement is required")

        # Validate ONNX graph before ORT can follow external tensor references/custom libraries.
        import onnx
        graph = onnx.load_model(str(path), load_external_data=False)
        def inspect_tensor(tensor):
            if tensor.external_data or tensor.data_location == onnx.TensorProto.EXTERNAL:
                raise ValueError("External ONNX tensor data is unsupported")
        def inspect_sparse(tensor):
            inspect_tensor(tensor.values)
            inspect_tensor(tensor.indices)
        def inspect_graph(g):
            for tensor in g.initializer:
                inspect_tensor(tensor)
            for tensor in g.sparse_initializer:
                inspect_sparse(tensor)
            for node in g.node:
                if node.domain not in ("", "ai.onnx"):
                    raise ValueError("Custom ONNX operators are unsupported")
                for attribute in node.attribute:
                    if attribute.HasField("t"):
                        inspect_tensor(attribute.t)
                    for tensor in attribute.tensors:
                        inspect_tensor(tensor)
                    if attribute.HasField("sparse_tensor"):
                        inspect_sparse(attribute.sparse_tensor)
                    for tensor in attribute.sparse_tensors:
                        inspect_sparse(tensor)
                    if attribute.type == onnx.AttributeProto.GRAPH:
                        inspect_graph(attribute.g)
                    for child in attribute.graphs:
                        inspect_graph(child)
        if graph.functions or graph.training_info:
            raise ValueError("ONNX local functions and training graphs are unsupported")
        inspect_graph(graph.graph)
        graph_metadata = {entry.key: entry.value for entry in graph.metadata_props}
        del graph
        import numpy as np
        import torch
        import onnxruntime as ort
        torch.set_num_threads(config["cpuThreads"])
        # Upstream creates sessions without thread limits. Apply the user's actual
        # budget to metadata, voice and F0 sessions instead of reporting Torch's
        # thread count while ORT silently starts a machine-wide thread pool.
        session_class = getattr(ort, "_voicemorph_original_session", ort.InferenceSession)
        ort._voicemorph_original_session = session_class
        class BoundedSession(session_class):
            def __init__(self, path_or_bytes, sess_options=None, *args, **kwargs):
                options = sess_options or ort.SessionOptions()
                options.intra_op_num_threads = config["cpuThreads"]
                options.inter_op_num_threads = 1
                super().__init__(path_or_bytes, options, *args, **kwargs)
        ort.InferenceSession = BoundedSession
        # Upstream's optional fallback/checkpoint loaders are not allowed to unpickle anything.
        def no_pickle(*_args, **_kwargs):
            raise ValueError("Checkpoint/pickle loading is disabled by VoiceMorph; use verified ONNX assets")
        trusted_torch_load = torch.load
        torch.load = no_pickle
        torch.hub.load = no_pickle
        server = self.root.joinpath(self.manifest["serverDirectory"]).resolve(strict=True)
        if not server.is_relative_to(self.root):
            raise ValueError("Invalid server directory")
        sys.path.insert(0, str(server))
        from data.ModelSlot import RVCModelSlot
        from voice_changer.RVC.RVCModelSlotGenerator import RVCModelSlotGenerator
        from voice_changer.RVC.RVCr2 import RVCr2
        from voice_changer.VoiceChangerV2 import VoiceChangerV2
        from voice_changer.utils.VoiceChangerParams import VoiceChangerParams

        # The upstream worker installs a NullHandler for its warnings unless its web UI starts.
        # Preserve that diagnostic channel for short/failed realtime chunks.
        logger = logging.getLogger("vcclient")
        if not logger.handlers:
            handler = logging.StreamHandler(sys.stderr)
            handler.setLevel(logging.WARNING)
            logger.addHandler(handler)

        # The manager starts ServerDevice threads. Use only RVCr2 + VoiceChangerV2.
        # A private per-session model directory is supplied by Rust; no user path is synthesized.
        model_root = path.parent
        # createPipeline uses model_dir / slotIndex / basename(modelFile).
        slot = RVCModelSlot(slotIndex=0, modelFile=path.name, isONNX=True)
        if request.get("featureEncoderSha256") == RVC_CONTENTVEC_SHA256:
            # The graph was already parsed above. Opening a second CPU inference
            # session solely to read these fields delays every neural voice start.
            from const import EnumInferenceTypes
            metadata = json.loads(graph_metadata.get("metadata", "{}"))
            if (metadata.get("embChannels") != 768 or metadata.get("embOutputLayer") != 12
                    or metadata.get("useFinalProj") is not False or metadata.get("f0") is not True
                    or metadata.get("version") not in ("2", "2.1", "2.2")):
                raise ValueError("Expected RVC v2 F0 metadata for the verified ContentVec encoder")
            slot.embChannels = 768
            slot.embOutputLayer = 12
            slot.useFinalProj = False
            slot.embedder = metadata.get("embedder", "hubert_base")
            slot.f0 = True
            slot.modelType = EnumInferenceTypes.onnxRVC.value
            slot.samplingRate = metadata.get("samplingRate")
            slot.deprecated = False
            slot.version = "v" + metadata["version"]
        else:
            slot = RVCModelSlotGenerator._setInfoByONNX(str(path), slot)
        if slot.deprecated or slot.embedder not in ("hubert_base", "contentvec") or slot.embChannels not in (256, 768) or slot.samplingRate not in (32000, 40000, 48000):
            raise ValueError("Unsupported RVC metadata")
        # Model is staged under session_root/0 by the Rust supervisor.
        if model_root.name != "0":
            raise ValueError("Expected managed model slot")
        index_data = None
        if request.get("featureIndexSha256") is not None:
            if slot.embChannels != 768:
                raise ValueError("Phase 6 retrieval requires RVC v2 features")
            index_data = load_flat_index(path.with_suffix(".index"), request["featureIndexSha256"], np)
            import faiss
            faiss.omp_set_num_threads(config["cpuThreads"])
        assets = self.manifest
        encoder_sha = request.get("featureEncoderSha256")
        if encoder_sha is None and slot.embedder == "contentvec":
            encoder_sha = RVC_CONTENTVEC_SHA256
        content = str(resolve_content_encoder(self.root, assets, encoder_sha, self.hash_file))
        # OnnxContentvec.loadModel is a NotImplemented stub at the pinned revision.
        # Only the runtime publisher's digest-verified HuBERT checkpoint is permitted;
        # imported voice checkpoints and arbitrary pickle paths remain rejected.
        if Path(content).suffix not in (".pt", ".bin"):
            raise ValueError("This upstream revision requires a trusted bundled HuBERT checkpoint; its ONNX content embedder is not implemented")
        def trusted_asset_load(file, *args, **kwargs):
            name = getattr(file, "name", file)
            if not isinstance(name, (str, os.PathLike)) or Path(name).resolve() != Path(content):
                raise ValueError("Only the verified runtime HuBERT asset may use the checkpoint loader")
            # fairseq's explicitly trusted embedding checkpoint contains config objects.
            # PyTorch 2.6 defaults to weights_only; never relax this for a user voice model.
            kwargs["weights_only"] = False
            return trusted_torch_load(file, *args, **kwargs)
        torch.load = trusted_asset_load
        rmvpe = str(contained(self.root, assets["rmvpeModel"])) if assets.get("rmvpeModel") else ""
        if config["f0Method"] == "rmvpe_onnx" and not rmvpe:
            raise ValueError("This runtime package has no RMVPE ONNX asset")
        params = VoiceChangerParams(model_dir=str(model_root.parent), content_vec_500="", content_vec_500_onnx=content,
            content_vec_500_onnx_on=False, hubert_base=content, hubert_base_jp="", hubert_soft="", nsf_hifigan="",
            sample_mode="", crepe_onnx_full="", crepe_onnx_tiny="", rmvpe="", rmvpe_onnx=rmvpe, whisper_tiny="")
        providers = ort.get_available_providers()
        cuda_available = torch.cuda.device_count() > 0 and "CUDAExecutionProvider" in providers
        if config["provider"] == "cuda" and not cuda_available and not config["allowCpuFallback"]:
            raise ValueError("CUDA is unavailable; CPU fallback was not enabled")
        gpu = 0 if config["provider"] != "cpu" and cuda_available else -1

        def initialize(device):
            model = RVCr2(params, slot)
            model.settings.gpu = device
            model.settings.f0Detector = config["f0Method"]
            model.initialize()
            if model.pipeline is None:
                raise RuntimeError("w-okada failed to initialize the RVC pipeline")
            actual_providers = model.pipeline.inferencer.getInferencerInfo().get("onnxExecutionProvider", [])
            expected_provider = "CUDAExecutionProvider" if device >= 0 else "CPUExecutionProvider"
            if expected_provider not in actual_providers:
                raise RuntimeError(f"Requested provider was not loaded: {expected_provider}")
            model.settings.tran = config["pitchSemitones"]
            model.settings.indexRatio = 0
            if config.get("protect") is not None:
                model.settings.protect = config["protect"]
            if index_data is not None:
                model.pipeline.index, model.pipeline.big_npy = index_data
                model.settings.indexRatio = 0.25 if config.get("indexRate") is None else config["indexRate"]
                align_indexed_pipeline(model.pipeline, np, torch)
            elif config.get("indexRate"):
                raise ValueError("An index rate requires a verified feature index")
            model.settings.extraConvertSize = config["extraContextFrames"]
            model.settings.silentThreshold = config["noiseThreshold"]
            self.pcm_guard_counters = install_pcm_guards(model, np, torch)
            vc = VoiceChangerV2(params)
            vc.setModel(model)
            vc.setInputSampleRate(48000)
            vc.setOutputSampleRate(48000)
            vc.settings.crossFadeOverlapSize = config["crossfadeFrames"]
            vc.settings.recordIO = 0
            return model, vc
        start = time.perf_counter()
        try:
            self.model, self.vc = initialize(gpu)
        except Exception:
            if gpu < 0 or not config["allowCpuFallback"]:
                raise
            gpu = -1
            self.model, self.vc = initialize(gpu)
        self.provider = "cuda" if gpu >= 0 else "cpu"
        self.config = config
        self.np = np
        self.context_stream = (ContextualRvcStream(self.model, config, np, torch)
                               if encoder_sha == RVC_CONTENTVEC_SHA256 and slot.embChannels == 768
                               and slot.f0 and config["f0Method"] == "rmvpe_onnx" else None)
        return {"provider": self.provider, "loadingMs": (time.perf_counter() - start) * 1000,
                "modelSampleRate": slot.samplingRate, "f0": slot.f0, "indexAvailable": index_data is not None,
                "featureEncoderSha256": encoder_sha,
                "lookaheadFrames": config["chunkFrames"] if self.context_stream is not None else 0,
                "indexRate": self.model.settings.indexRatio, "protect": self.model.settings.protect,
                "f0Method": self.model.settings.f0Detector,
                "torchCpuThreads": torch.get_num_threads(),
                "onnxCpuThreads": self.model.pipeline.inferencer.model.get_session_options().intra_op_num_threads}

    def metrics(self):
        import torch
        import psutil
        process = psutil.Process()
        return {"rssBytes": process.memory_info().rss,
                "cpuSeconds": sum(process.cpu_times()[:2]),
                "cudaAllocatedBytes": torch.cuda.memory_allocated() if self.provider == "cuda" else None,
                "cudaReservedBytes": torch.cuda.memory_reserved() if self.provider == "cuda" else None,
                "cudaPeakAllocatedBytes": torch.cuda.max_memory_allocated() if self.provider == "cuda" else None,
                "cudaPeakReservedBytes": torch.cuda.max_memory_reserved() if self.provider == "cuda" else None,
                "pcmGuardCounters": getattr(self, "pcm_guard_counters", {})}

    def convert_approved_voice(self, request):
        if self.vc is not None or self.model is not None:
            raise ValueError("Stop conversion before preparing an approved model")
        checkpoint = Path(request["checkpointPath"]).resolve(strict=True)
        expected = "cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020"
        if checkpoint.name != "tsukuyomi-official-normal1.pth" or self.hash_file(checkpoint) != expected:
            raise ValueError("Only the reviewed original-owner checkpoint may be converted")
        output_dir = checkpoint.parent / "converted"
        output_dir.mkdir(exist_ok=False)
        server = self.root.joinpath(self.manifest["serverDirectory"]).resolve(strict=True)
        if not server.is_relative_to(self.root):
            raise ValueError("Invalid server directory")
        sys.path.insert(0, str(server))
        import torch
        from const import EnumInferenceTypes
        from voice_changer.RVC.onnxExporter.export2onnx import _export2onnx
        metadata = {"application": "VC_CLIENT", "version": "2.1", "modelType": EnumInferenceTypes.pyTorchRVCv2.value,
            "samplingRate": 40000, "f0": True, "embChannels": 768, "embedder": "hubert_base",
            "embOutputLayer": 12, "useFinalProj": False,
            "sourceCheckpointSha256": expected, "sourceUrl": "https://tyc.rei-yumesaki.net/files/voice/tyc-rvc.zip",
            "licenseUrl": "https://tyc.rei-yumesaki.net/work/software/rvc/terms/",
            "attribution": "Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)"}
        original_load = torch.load
        def safe_weights(file, *args, **kwargs):
            if Path(getattr(file, "name", file)).resolve() != checkpoint:
                raise ValueError("Unexpected checkpoint loader path")
            kwargs["weights_only"] = True
            return original_load(file, *args, **kwargs)
        output = output_dir / "tsukuyomi-normal1.onnx"
        try:
            torch.load = safe_weights
            torch.set_num_threads(4)
            _export2onnx(str(checkpoint), str(output_dir / "tsukuyomi-normal1-raw.onnx"), str(output), False, metadata)
        finally:
            torch.load = original_load
        return {"modelPath": str(output), "sha256": self.hash_file(output), "sourceSha256": expected}

    @staticmethod
    def hash_file(path):
        digest = hashlib.sha256()
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(65536), b""):
                digest.update(chunk)
        return digest.hexdigest()

    def infer(self, request):
        if self.vc is None:
            raise RuntimeError("No AI model loaded")
        samples = request["samples"]
        if not isinstance(samples, list) or len(samples) != self.config["chunkFrames"] or len(samples) > MAX_CHUNK:
            raise ValueError("PCM chunk length mismatch")
        if any(type(value) is not int or not -32768 <= value <= 32767 for value in samples):
            raise ValueError("PCM must contain signed 16-bit samples")
        start = time.perf_counter()
        if self.context_stream is not None:
            output = self.context_stream.convert(self.np.asarray(samples, dtype=self.np.int16))
            return {"samples": [0] * len(samples) if output is None else output.tolist(),
                    "inferenceMs": (time.perf_counter() - start) * 1000, "warmingUp": output is None,
                    "stageSeconds": self.context_stream.last_timing}
        had_sola = hasattr(self.vc, "sola_buffer")
        output, performance = self.vc.on_request(self.np.asarray(samples, dtype=self.np.int16))
        elapsed = (time.perf_counter() - start) * 1000
        # Upstream reports several inference errors as a short zero buffer. Treat as failure,
        # never pad it with original input or report it as successful conversion.
        warming = len(output) != len(samples) or not had_sola
        if warming:
            self.warmup_chunks += 1
            if self.warmup_chunks > 8:
                raise RuntimeError("w-okada did not produce a full chunk within the bounded warmup window")
            return {"samples": [0] * len(samples), "inferenceMs": elapsed, "warmingUp": True}
        if not self.np.isfinite(output).all():
            raise RuntimeError("w-okada returned non-finite samples")
        self.warmup_chunks = 0
        return {"samples": self.np.clip(output, -32768, 32767).astype(self.np.int16).tolist(), "inferenceMs": elapsed,
                "upstreamPerformanceSeconds": performance, "warmingUp": False}


def serve(engine, source, destination):
    while True:
        line = source.readline(MAX_MESSAGE + 1)
        if not line:
            return
        if len(line) > MAX_MESSAGE or not line.endswith("\n"):
            raise ValueError("Protocol frame exceeds limit")
        request = json.loads(line)
        response = {"id": request.get("id"), "protocol": PROTOCOL, "ok": False}
        shutdown = False
        try:
            if request.get("protocol") != PROTOCOL or type(request.get("id")) is not int:
                raise ValueError("Protocol version/id mismatch")
            operation = request.get("op")
            if operation == "hello":
                response.update(engine.probe())
            elif operation == "load":
                response.update(engine.load(request))
            elif operation == "infer":
                response.update(engine.infer(request))
            elif operation == "health":
                response.update(ready=engine.vc is not None)
            elif operation == "metrics":
                response.update(engine.metrics())
            elif operation == "convertApprovedVoice":
                response.update(engine.convert_approved_voice(request))
            elif operation in ("unload", "shutdown"):
                engine.vc = None
                engine.model = None
                engine.context_stream = None
                shutdown = operation == "shutdown"
            else:
                raise ValueError("Unknown operation")
            response["ok"] = True
        except Exception as error:
            response["error"] = f"{type(error).__name__}: {str(error)[:768]}"
            print(response["error"], file=sys.stderr, flush=True)
        destination.write(json.dumps(response, allow_nan=False, separators=(",", ":")) + "\n")
        destination.flush()
        if shutdown:
            return


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime-root", type=Path, required=True)
    args = parser.parse_args()
    wire = sys.stdout
    # All third-party diagnostic prints are logs, never protocol/audio messages.
    with contextlib.redirect_stdout(sys.stderr):
        serve(WOkadaEngine(args.runtime_root), sys.stdin, wire)


if __name__ == "__main__":
    main()
