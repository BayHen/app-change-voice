"""Protocol/config tests only. No inference quality or model benchmark is simulated."""
import io
import hashlib
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from bridge import (
    contained, resolve_content_encoder, serve, validate_config, MAX_MESSAGE,
    RVC_CONTENTVEC_FILE, RVC_CONTENTVEC_SHA256,
)


def config():
    return {"provider": "auto", "allowCpuFallback": False, "failurePolicy": "mute", "pitchSemitones": 0,
            "f0Method": "dio", "chunkFrames": 4096, "crossfadeFrames": 1024,
            "extraContextFrames": 4096, "noiseThreshold": 0.00001, "cpuThreads": 4}


class ProtocolFixture:
    vc = None
    model = None
    def probe(self):
        return {"fixture": True}


class BridgeTests(unittest.TestCase):
    def test_feature_encoder_selection_rejects_missing_or_changed_assets(self):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            default = root / "default.pt"
            default.write_bytes(b"legacy encoder fixture")
            manifest = {"contentVecModel": "default.pt", "files": [
                {"path": "default.pt", "sha256": hashlib.sha256(default.read_bytes()).hexdigest()},
                {"path": RVC_CONTENTVEC_FILE, "sha256": RVC_CONTENTVEC_SHA256},
            ]}
            hash_file = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
            self.assertEqual(resolve_content_encoder(root, manifest, None, hash_file), default)
            self.assertEqual(resolve_content_encoder(root, manifest, hash_file(default), hash_file), default)
            alternate = root / RVC_CONTENTVEC_FILE
            alternate.parent.mkdir()
            alternate.write_bytes(b"does not match the pinned ContentVec")
            with self.assertRaisesRegex(ValueError, "failed verification"):
                resolve_content_encoder(root, manifest, RVC_CONTENTVEC_SHA256, hash_file)
            with self.assertRaisesRegex(ValueError, "failed verification"):
                resolve_content_encoder(root, manifest, hash_file(alternate), hash_file)

    def test_rejects_invalid_configuration(self):
        validate_config(config())
        for change in ({"provider": "directml"}, {"chunkFrames": 0}, {"noiseThreshold": float("nan")},
                       {"pitchSemitones": 13}, {"cpuThreads": 100}, {"crossfadeFrames": 99999},
                       {"indexRate": 1.2}, {"indexRate": float("nan")}, {"protect": 0.51}):
            with self.assertRaises(ValueError):
                validate_config({**config(), **change})
        self.assertEqual(validate_config({**config(), "indexRate": 0.65, "protect": 0.33})["protect"], 0.33)

    def test_protocol_ids_errors_and_shutdown(self):
        requests = [{"id": 5, "protocol": 1, "op": "hello"},
                    {"id": 6, "protocol": 2, "op": "hello"},
                    {"id": 7, "protocol": 1, "op": "unknown"},
                    {"id": 8, "protocol": 1, "op": "shutdown"}]
        output = io.StringIO()
        serve(ProtocolFixture(), io.StringIO("".join(json.dumps(r) + "\n" for r in requests)), output)
        responses = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([r["id"] for r in responses], [5, 6, 7, 8])
        self.assertTrue(responses[0]["ok"])
        self.assertFalse(responses[1]["ok"])
        self.assertFalse(responses[2]["ok"])
        self.assertTrue(responses[3]["ok"])

    def test_rejects_oversized_protocol_lines(self):
        with self.assertRaises(ValueError):
            serve(ProtocolFixture(), io.StringIO("x" * (MAX_MESSAGE + 1)), io.StringIO())

    def test_asset_paths_cannot_escape(self):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "asset.pt").write_bytes(b"test fixture, never a real checkpoint")
            self.assertEqual(contained(root, "asset.pt"), root / "asset.pt")
            for path in ("../asset.pt", "C:/asset.pt", "a\\b.pt", "/asset.pt"):
                with self.assertRaises(ValueError):
                    contained(root, path)


if __name__ == "__main__":
    unittest.main()
