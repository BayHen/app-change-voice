"""Compare saved synthetic float WAVs, using the same least-squares residual estimator.
No device access, microphone recording, network or speech data.
"""
import json
from pathlib import Path
import numpy as np
from scipy.io import wavfile

root = Path(__file__).resolve().parents[1]
base = root / ".ai-runtime-build/audio-quality"
reports = {}
for label in ("before", "after"):
    cases = {}
    for file in sorted((base / label).glob("*.wav")):
        rate, signal = wavfile.read(file)
        signal = signal[rate:2*rate].astype(np.float64)
        phase = 2*np.pi*997*np.arange(len(signal))/rate
        basis = np.column_stack((np.sin(phase), np.cos(phase)))
        fit = basis @ np.linalg.lstsq(basis, signal, rcond=None)[0]
        residual = signal-fit
        cases[file.stem] = {
            "rate": rate, "frames": len(signal), "peak": float(np.max(np.abs(signal))),
            "rms": float(np.sqrt(np.mean(signal**2))),
            "exactRailSamples": int(np.count_nonzero(np.abs(signal)==1)),
            "overFullScaleFloatSamples": int(np.count_nonzero(np.abs(signal)>1)),
            "thdnDb": float(10*np.log10(np.mean(residual**2)/np.mean(fit**2))),
            "nonFinite": int(np.count_nonzero(~np.isfinite(signal))),
        }
    reports[label] = cases
destination = root / "docs/benchmarks/audio-quality/comparison.json"
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text(json.dumps({"method":"1-second steady 997 Hz sine; two-parameter least-squares fit; residual includes harmonics/noise; not perceptual speech quality or hardware latency", "cases":reports}, indent=2), encoding="utf8")
print(destination)
