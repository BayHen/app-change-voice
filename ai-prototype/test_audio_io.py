from pathlib import Path
import math
import tempfile
import unittest

from audio_io import peak_and_rms, read_pcm16_mono, write_pcm16_mono


class AudioIoTests(unittest.TestCase):
    def test_pcm16_roundtrip(self) -> None:
        signal = [math.sin(index * 0.05) * 0.25 for index in range(4800)]
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "signal.wav"
            write_pcm16_mono(target, signal, 48_000)
            restored, rate = read_pcm16_mono(target)
        self.assertEqual(rate, 48_000)
        self.assertEqual(len(restored), len(signal))
        self.assertLess(max(abs(left - right) for left, right in zip(restored, signal)), 1e-4)

    def test_invalid_samples_are_sanitized(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "clean.wav"
            write_pcm16_mono(target, [math.nan, math.inf, -math.inf, 0.5], 16_000)
            restored, _ = read_pcm16_mono(target)
        self.assertTrue(all(math.isfinite(value) for value in restored))
        self.assertLessEqual(peak_and_rms(restored)[0], 1.0)


if __name__ == "__main__":
    unittest.main()
