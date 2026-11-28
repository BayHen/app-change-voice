"""Security/metadata tests only. Malformed fixtures are not voice models or inference evidence."""
import hashlib
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
import sys

PROJECT=Path(__file__).resolve().parents[1]
OVERLAY=Path(os.environ.get('VM_TEST_CONVERTER_ROOT',PROJECT/'.ai-runtime-build/pack-manager-e2e/packs/converter-torch210-cpu/python/Lib/site-packages'))
if not OVERLAY.is_dir(): raise RuntimeError('Run the managed converter installation test before these security checks')
sys.path.insert(0,str(OVERLAY))
import torch
spec=importlib.util.spec_from_file_location('convert_worker',PROJECT/'ai-sidecar/convert_worker.py')
worker=importlib.util.module_from_spec(spec);spec.loader.exec_module(worker)

class SecurityTests(unittest.TestCase):
    def test_uses_exact_qualified_cpu_torch(self):
        self.assertEqual(torch.__version__,'2.10.0+cpu')
    def test_owner_checkpoint_is_detected_without_guessing(self):
        path=PROJECT/'.ai-runtime-build/assets/tsukuyomi-official-normal1.pth'
        _,metadata=worker.load_checkpoint(path,worker.digest(path))
        self.assertEqual(metadata['version'],'v2');self.assertEqual(metadata['samplingRate'],40000)
        self.assertTrue(metadata['f0']);self.assertEqual(metadata['embChannels'],768)
        self.assertEqual(metadata['converterTorchVersion'],'2.10.0+cpu')
    def test_malicious_pickle_global_is_rejected_without_execution(self):
        with tempfile.TemporaryDirectory(prefix='vm-checkpoint-security-') as temporary:
            root=Path(temporary);marker=root/'executed.txt'
            class Payload:
                def __reduce__(self):
                    return eval,(f"__import__('pathlib').Path({str(marker)!r}).write_text('executed')",)
            path=root/'malicious-test-fixture.pth';torch.save({'weight':Payload(),'config':[]},path)
            with self.assertRaises(Exception):worker.load_checkpoint(path,worker.digest(path))
            self.assertFalse(marker.exists())
    def test_legacy_pickle_and_checksum_change_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix='vm-checkpoint-legacy-') as temporary:
            path=Path(temporary)/'not-a-voice.pth';path.write_bytes(b'not a checkpoint')
            with self.assertRaisesRegex(ValueError,'changed'):worker.load_checkpoint(path,'0'*64)
            with self.assertRaisesRegex(ValueError,'ZIP'):worker.load_checkpoint(path,worker.digest(path))
    def test_missing_metadata_is_not_filled_with_guesses(self):
        with tempfile.TemporaryDirectory(prefix='vm-checkpoint-missing-') as temporary:
            path=Path(temporary)/'invalid-test-fixture.pth';torch.save({'weight':{},'config':[]},path)
            with self.assertRaises(ValueError):worker.load_checkpoint(path,worker.digest(path))
if __name__=='__main__':unittest.main()
