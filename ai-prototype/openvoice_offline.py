from __future__ import annotations

import argparse
import json
from pathlib import Path
import platform
import sys
import time

from audio_io import peak_and_rms, read_pcm16_mono


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Consent-gated OpenVoice V2 offline conversion")
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--checkpoint-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metrics", type=Path, required=True)
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    parser.add_argument("--acknowledge-rights", action="store_true")
    return parser.parse_args()


def validate(args: argparse.Namespace) -> tuple[Path, Path]:
    if not args.acknowledge_rights:
        raise ValueError("Pass --acknowledge-rights only when you have rights to both audio files")
    for value in (args.source, args.reference):
        if not value.is_file() or value.suffix.lower() != ".wav":
            raise ValueError(f"Input must be an existing WAV file: {value}")
    config = args.checkpoint_dir / "config.json"
    checkpoint = args.checkpoint_dir / "checkpoint.pth"
    if not config.is_file() or not checkpoint.is_file():
        raise ValueError("Checkpoint directory must contain config.json and checkpoint.pth")
    if args.output.resolve() in {args.source.resolve(), args.reference.resolve()}:
        raise ValueError("Output must not overwrite source or reference audio")
    return config, checkpoint


def run(args: argparse.Namespace) -> dict[str, object]:
    config, checkpoint = validate(args)
    try:
        import psutil
        import torch
        from openvoice import se_extractor
        from openvoice.api import ToneColorConverter
    except ImportError as error:
        raise RuntimeError("Install PyTorch and pinned OpenVoice dependencies before inference") from error

    if args.device == "cuda" and not torch.cuda.is_available():
        raise RuntimeError("CUDA was requested but torch.cuda.is_available() is false")

    source_samples, source_rate = read_pcm16_mono(args.source)
    reference_samples, reference_rate = read_pcm16_mono(args.reference)
    started_load = time.perf_counter()
    converter = ToneColorConverter(str(config), device=args.device)
    converter.load_ckpt(str(checkpoint))
    load_seconds = time.perf_counter() - started_load

    started = time.perf_counter()
    source_embedding, _ = se_extractor.get_se(str(args.source), converter, vad=True)
    target_embedding, _ = se_extractor.get_se(str(args.reference), converter, vad=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    converter.convert(
        audio_src_path=str(args.source),
        src_se=source_embedding,
        tgt_se=target_embedding,
        output_path=str(args.output),
        message="VoiceMorph Studio: authorized offline conversion",
    )
    inference_seconds = time.perf_counter() - started
    output_samples, output_rate = read_pcm16_mono(args.output)
    peak, rms = peak_and_rms(output_samples)
    duration = len(source_samples) / source_rate
    metrics: dict[str, object] = {
        "mode": "offline",
        "backend": "OpenVoice V2 / PyTorch",
        "device": args.device,
        "python": platform.python_version(),
        "torch": torch.__version__,
        "cudaVerified": bool(args.device == "cuda" and torch.cuda.is_available()),
        "modelLoadSeconds": load_seconds,
        "inferenceSeconds": inference_seconds,
        "sourceDurationSeconds": duration,
        "realtimeFactor": inference_seconds / duration if duration else None,
        "processRssMiB": psutil.Process().memory_info().rss / (1024 * 1024),
        "sourceSampleRate": source_rate,
        "referenceSampleRate": reference_rate,
        "outputSampleRate": output_rate,
        "outputPeak": peak,
        "outputRms": rms,
        "endToEndLatencyMeasured": False,
    }
    args.metrics.parent.mkdir(parents=True, exist_ok=True)
    args.metrics.write_text(json.dumps(metrics, indent=2), encoding="utf-8")
    return metrics


def main() -> int:
    try:
        metrics = run(parse_args())
    except Exception as error:
        print(f"AI conversion failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(metrics, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
