"""Protocol/config tests only. No inference quality or model benchmark is simulated."""
import io
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from bridge import contained, serve, validate_config, MAX_MESSAGE


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
    def test_rejects_invalid_configuration(self):
        validate_config(config())
        for change in ({"provider": "directml"}, {"chunkFrames": 0}, {"noiseThreshold": float("nan")},
                       {"pitchSemitones": 13}, {"cpuThreads": 100}, {"crossfadeFrames": 99999}):
            with self.assertRaises(ValueError):
                validate_config({**config(), **change})

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
