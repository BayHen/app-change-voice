"""Exercise real private sidecar inference with an approved owner WAV, without devices.

Reports IPC/inference/throughput and actual process resources, never hardware loopback latency.
Input/output audio stays in the ignored project runtime directory.
"""
import argparse
from array import array
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import statistics
import subprocess
import threading
import time
import wave
from datetime import datetime, timezone

import psutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--provider", required=True, choices=("cpu", "cuda"))
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--chunk-frames", type=int, choices=(2048, 4096, 8192, 16384), default=4096)
    parser.add_argument("--allow-cpu-fallback", action="store_true")
    parser.add_argument("--hide-cuda", action="store_true", help="Test unavailable-CUDA fallback using CUDA_VISIBLE_DEVICES=-1 in the child only")
    args = parser.parse_args()
    runtime = args.runtime.resolve(strict=True)
    manifest = json.loads((runtime / "runtime-manifest.json").read_text("utf-8"))
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    model_dir = destination / "0"
    model_dir.mkdir(exist_ok=True)
    model = model_dir / args.model.name
    shutil.copyfile(args.model, model)
    digest = hashlib.sha256(model.read_bytes()).hexdigest()
    with wave.open(str(args.input), "rb") as wav:
        if (wav.getnchannels(), wav.getsampwidth(), wav.getframerate()) != (1, 2, 48000):
            raise ValueError("Benchmark source must be 48 kHz mono PCM16")
        samples = array("h", wav.readframes(wav.getnframes()))
    frames = args.chunk_frames
    count = max(12, int(args.seconds * 48000 / frames))
    config = {"provider": args.provider, "allowCpuFallback": args.allow_cpu_fallback, "failurePolicy": "mute",
        "pitchSemitones": 0, "f0Method": "dio", "chunkFrames": frames, "crossfadeFrames": 1024,
        "extraContextFrames": 4096, "noiseThreshold": 0.00001, "cpuThreads": 4}
    env = dict(os.environ, HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", PYTHONDONTWRITEBYTECODE="1",
               NUMBA_CACHE_DIR=str(destination / "numba-cache"), OMP_NUM_THREADS="4")
    if args.hide_cuda:
        env["CUDA_VISIBLE_DEVICES"] = "-1"
    responses = queue.Queue(maxsize=2)
    errors = destination / "sidecar.log"
    with errors.open("wb") as log:
        process = subprocess.Popen([str(runtime / manifest["pythonExecutable"]), "-I", "-u",
            str(runtime / manifest["entryPoint"]), "--runtime-root", str(runtime)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, cwd=runtime, env=env,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        monitor_stop = threading.Event()
        observations = {"peakRssBytes": 0, "deviceGpuSamples": []}
        child = psutil.Process(process.pid)
        def observe():
            last_gpu = 0
            while not monitor_stop.is_set():
                try:
                    observations["peakRssBytes"] = max(observations["peakRssBytes"], child.memory_info().rss)
                    if time.monotonic() - last_gpu >= 1 and shutil.which("nvidia-smi"):
                        sample = subprocess.run(["nvidia-smi", "--query-gpu=utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"],
                            capture_output=True, text=True, timeout=5, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
                        if sample.returncode == 0:
                            values = sample.stdout.strip().splitlines()[0].split(",")
                            observations["deviceGpuSamples"].append({"timestampUtc": datetime.now(timezone.utc).isoformat(),
                                "utilizationPercent": float(values[0]), "memoryUsedMiB": float(values[1]), "memoryTotalMiB": float(values[2])})
                        last_gpu = time.monotonic()
                except (psutil.Error, ValueError, IndexError, subprocess.SubprocessError):
                    pass
                monitor_stop.wait(0.1)
        observer = threading.Thread(target=observe, daemon=True)
        observer.start()
        def read_responses():
            try:
                while True:
                    line = process.stdout.readline(256 * 1024 + 1)
                    if not line:
                        raise RuntimeError("Sidecar exited; inspect sidecar.log")
                    if len(line) > 256 * 1024:
                        raise RuntimeError("Oversized protocol response")
                    responses.put(json.loads(line), timeout=5)
            except Exception as error:
                try:
                    responses.put(error, timeout=1)
                except queue.Full:
                    pass
        reader = threading.Thread(target=read_responses, daemon=True)
        reader.start()
        sequence = 0
        def call(operation, timeout=120, **payload):
            nonlocal sequence
            sequence += 1
            process.stdin.write((json.dumps({"protocol": 1, "id": sequence, "op": operation, **payload}, separators=(",", ":")) + "\n").encode())
            process.stdin.flush()
            value = responses.get(timeout=timeout)
            if isinstance(value, Exception):
                raise value
            if value.get("id") != sequence or value.get("protocol") != 1 or not value.get("ok"):
                raise RuntimeError(str(value.get("error", "Protocol mismatch")))
            return value
        try:
            hello = call("hello")
            load_started = time.perf_counter()
            loaded = call("load", modelPath=str(model), sha256=digest, consentConfirmed=True, config=config)
            load_rpc_ms = (time.perf_counter() - load_started) * 1000
            before = call("metrics")
            timings, inference, converted = [], [], array("h")
            warmup = 0
            started = time.perf_counter()
            for index in range(count):
                pcm = [samples[(index * frames + offset) % len(samples)] for offset in range(frames)]
                tick = time.perf_counter()
                output = call("infer", timeout=120, samples=pcm)
                elapsed = (time.perf_counter() - tick) * 1000
                if len(output.get("samples", [])) != frames:
                    raise RuntimeError("Converted frame count mismatch")
                converted.extend(output["samples"])
                if output.get("warmingUp"):
                    warmup += 1
                else:
                    timings.append(elapsed)
                    inference.append(output["inferenceMs"])
            elapsed = time.perf_counter() - started
            after = call("metrics")
            if not timings or max(map(abs, converted), default=0) == 0:
                raise RuntimeError("No non-silent real inference output")
            report = {"kind": "Real w-okada RVC inference, offline WAV-to-WAV through private stdio; no microphone/speaker/virtual device",
                "timestampUtc": datetime.now(timezone.utc).isoformat(),
                "hardwareLoopbackLatencyMs": None, "qualityListeningTest": "not performed by this script",
                "inputCredit": "Tsukuyomi-chan official sample voice (CV. Rei Yumesaki)",
                "modelCredit": "Tsukuyomi-chan official RVC normal1 (CV. Rei Yumesaki), converted from original-owner checkpoint",
                "modelTerms": "https://tyc.rei-yumesaki.net/work/software/rvc/terms/",
                "sourceTerms": "https://tyc.rei-yumesaki.net/material/voice/sample-voice/",
                "inputSha256": hashlib.sha256(args.input.read_bytes()).hexdigest(), "modelSha256": digest,
                "runtimeManifestSha256": hashlib.sha256((runtime / "runtime-manifest.json").read_bytes()).hexdigest(),
                "hello": hello, "load": loaded, "config": config, "chunks": count, "warmupChunks": warmup,
                "modelLoadRpcMs": load_rpc_ms,
                "completedChunks": len(timings), "inputDurationSeconds": count * frames / 48000,
                "wallSecondsIncludingWarmup": elapsed, "realTimeFactorIncludingWarmup": elapsed / (count * frames / 48000),
                "ipcRoundTripMedianMs": statistics.median(timings), "ipcRoundTripMaxMs": max(timings),
                "inferenceMedianMs": statistics.median(inference), "inferenceMaxMs": max(inference),
                "processCpuPercentOneCore": (after["cpuSeconds"] - before["cpuSeconds"]) / elapsed * 100,
                "processCpuPercentMachine": (after["cpuSeconds"] - before["cpuSeconds"]) / elapsed * 100 / (os.cpu_count() or 1),
                "resourcesAfter": after, "outputPeakPcm16": max(map(abs, converted)),
                "peakRssBytesSampled": observations["peakRssBytes"],
                "deviceGpuTelemetry": {"scope": "Whole device, includes other applications and ORT allocations; not attributed per-process GPU utilization", "samples": observations["deviceGpuSamples"]},
                "underruns": None, "dropouts": None,
                "limits": "Sequential throughput test on one machine with repeated same-speaker sample and instrumentation overhead. Not live audio latency, natural speech quality, device underruns, or per-process GPU utilization. CUDA allocator counters are PyTorch-only and exclude ORT allocations; nvidia-smi counters describe the whole device."}
            (destination / "report.json").write_text(json.dumps(report, indent=2), "utf-8")
            with wave.open(str(destination / "converted-local-only.wav"), "wb") as wav:
                wav.setnchannels(1)
                wav.setsampwidth(2)
                wav.setframerate(48000)
                wav.writeframes(converted.tobytes())
            call("shutdown", timeout=10)
            print(json.dumps({key: report[key] for key in ("completedChunks", "warmupChunks", "inferenceMedianMs", "ipcRoundTripMedianMs", "realTimeFactorIncludingWarmup")}))
        finally:
            monitor_stop.set()
            observer.join(timeout=6)
            if process.poll() is None:
                process.kill()
            process.wait(timeout=10)
            process.stdin.close()
            process.stdout.close()


if __name__ == "__main__":
    main()
