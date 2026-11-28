"""Dataset checks use generated one-second WAV fixtures, never training audio."""
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
import wave

spec = importlib.util.spec_from_file_location(
    "rvc_original", Path(__file__).resolve().parents[1] / "scripts/rvc-original.py"
)
pipeline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pipeline)
score_spec = importlib.util.spec_from_file_location(
    "score_original_voices", Path(__file__).resolve().parents[1] / "scripts/score-original-voices.py"
)
scoring = importlib.util.module_from_spec(score_spec)
score_spec.loader.exec_module(scoring)


class DatasetTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.path = self.root / "dataset.json"
        self.spec = dict(
            schemaVersion=1, profileId="wind-assassin", speakerId="test-owner",
            sourceUrl="https://example.test/source", licenseUrl="https://example.test/terms",
            attribution="Test only", consent=dict(training=True, voiceConversion=True), clips=[],
        )
        for number, split in enumerate(("train", "holdout")):
            file = self.root / f"{split}.wav"
            with wave.open(str(file), "wb") as target:
                target.setparams((1, 2, 16000, 0, "NONE", "not compressed"))
                target.writeframes(struct.pack("<hh", 300 + number, -300) * 8000)
            self.spec["clips"].append(dict(file=file.name, sha256=pipeline.sha(file), split=split,
                                           language="vi", transcript="Fixture"))

    def validate(self):
        self.path.write_text(json.dumps(self.spec), encoding="ascii")
        return pipeline.dataset(self.path)

    def test_accepts_hashed_wavs_and_reports_separate_holdout(self):
        _, _, seconds, fingerprint = self.validate()
        self.assertEqual(seconds, {"train": 1.0, "holdout": 1.0})
        self.spec["profileId"] = "jade-guardian"
        self.assertEqual(fingerprint, self.validate()[3])

    def test_rejects_missing_consent(self):
        self.spec["consent"]["training"] = False
        with self.assertRaisesRegex(ValueError, "consent"):
            self.validate()

    def test_rejects_holdout_leakage_and_checksum_mismatch(self):
        self.spec["clips"][1].update(file="train.wav", sha256=self.spec["clips"][0]["sha256"])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.validate()
        self.spec["clips"][1]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "Checksum"):
            self.validate()

    def test_rejects_escape(self):
        self.spec["clips"][0]["file"] = "../train.wav"
        with self.assertRaisesRegex(ValueError, "relative"):
            self.validate()

    def test_blind_trials_report_errors_and_wrong_identity(self):
        key = {"A": dict(profileId="wind-assassin", language="en", reference="Stay calm, please.")}
        trial = dict(listenerId="listener-1", sampleId="A", naturalness=4,
                     transcript="stay calm now", guess="jade-guardian")
        result = scoring.score(key, [trial])[0]
        self.assertAlmostEqual(result["wordErrorRate"], 1 / 3)
        self.assertEqual(result["meanNaturalness"], 4)
        self.assertEqual(result["identificationAccuracy"], 0)
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            scoring.score(key, [trial, trial])


if __name__ == "__main__":
    unittest.main()
