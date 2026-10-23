from __future__ import annotations

from pathlib import Path
from array import array
import math
import wave


def read_pcm16_mono(path: Path) -> tuple[list[float], int]:
    with wave.open(str(path), "rb") as source:
        if source.getsampwidth() != 2:
            raise ValueError("Only PCM16 WAV input is supported by the safe preprocessor")
        channels = source.getnchannels()
        sample_rate = source.getframerate()
        data = array("h")
        data.frombytes(source.readframes(source.getnframes()))
    if channels < 1:
        raise ValueError("WAV has no channels")
    frames = len(data) // channels
    mono = [sum(data[index * channels : (index + 1) * channels]) / channels / 32768.0 for index in range(frames)]
    return [max(-1.0, min(1.0, value)) for value in mono], sample_rate


def write_pcm16_mono(path: Path, samples: list[float], sample_rate: int) -> None:
    clean = (value if math.isfinite(value) else 0.0 for value in samples)
    pcm = array("h", (round(max(-1.0, min(1.0, value)) * 32767.0) for value in clean))
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(sample_rate)
        output.writeframes(pcm.tobytes())


def peak_and_rms(samples: list[float]) -> tuple[float, float]:
    if not samples:
        return 0.0, 0.0
    return max(abs(value) for value in samples), math.sqrt(sum(value * value for value in samples) / len(samples))
