"""Requires the qualified project Torch/NumPy. Numeric boundary tests, not simulated voice quality."""
import unittest
from types import SimpleNamespace
import numpy as np
import torch
from bridge import install_pcm_guards

class PcmGuardTests(unittest.TestCase):
    def test_saturation_prevents_wrap_and_preserves_normal_values(self):
        model = SimpleNamespace(pipeline=SimpleNamespace(inferencer=SimpleNamespace(infer=lambda: torch.tensor([-2., -0.5, 0., 0.5, 2.]))), inference=lambda: np.array([-40000., 0., 40000.]))
        counters = install_pcm_guards(model, np, torch)
        pcm = (model.pipeline.inferencer.infer() * 32767.5).to(torch.int16).tolist()
        self.assertEqual(pcm[0], -32768)
        self.assertEqual(pcm[-1], 32767)
        self.assertEqual(pcm[1:4], [-16383, 0, 16383])
        self.assertEqual(model.inference().tolist(), [-32768, 0, 32767])
        self.assertEqual(counters, {"modelSaturatedSamples": 2, "resampledSaturatedSamples": 2})
    def test_nonfinite_is_an_error(self):
        model = SimpleNamespace(pipeline=SimpleNamespace(inferencer=SimpleNamespace(infer=lambda: torch.tensor([float("nan")]))), inference=lambda: np.array([0.]))
        install_pcm_guards(model, np, torch)
        with self.assertRaises(ValueError): model.pipeline.inferencer.infer()

if __name__ == "__main__": unittest.main()
