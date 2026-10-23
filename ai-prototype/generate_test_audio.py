from __future__ import annotations

import argparse
from pathlib import Path
import math

from audio_io import write_pcm16_mono


def synthesize(seconds: float, sample_rate: int, fundamental: float) -> list[float]:
    output = []
    for index in range(round(seconds * sample_rate)):
        time = index / sample_rate
        envelope = min(1.0, time * 8.0) * min(1.0, (seconds - time) * 8.0)
        signal = sum(math.sin(2.0 * math.pi * fundamental * harmonic * time) / harmonic for harmonic in range(1, 7))
        output.append(0.12 * envelope * signal)
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate a synthetic, non-human test signal")
    parser.add_argument("output", type=Path)
    parser.add_argument("--frequency", type=float, default=140.0)
    args = parser.parse_args()
    write_pcm16_mono(args.output, synthesize(3.0, 48_000, args.frequency), 48_000)
