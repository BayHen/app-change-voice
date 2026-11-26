"""Build-time conversion of the owner's verified Tsukuyomi-chan RVC model.

Never used by the end-user runtime. Never accepts arbitrary pickle checkpoints.
The output retains the owner's model terms; it is not MIT-licensed by this tool.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

UPSTREAM = "7e2ce5c4800010bbc7604968bd29b802534fac05"
APPROVED_CHECKPOINT = "cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020"
SOURCE = "https://tyc.rei-yumesaki.net/files/voice/tyc-rvc.zip"
TERMS = "https://tyc.rei-yumesaki.net/work/software/rvc/terms/"


def sha256(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(65536), b""):
            value.update(block)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--upstream", required=True, type=Path)
    parser.add_argument("--destination", required=True, type=Path)
    args = parser.parse_args()
    if args.destination.exists():
        parser.error("Choose a new destination; existing output is never overwritten")
    if sha256(args.checkpoint) != APPROVED_CHECKPOINT:
        parser.error("Checkpoint is not the reviewed original-owner asset")
    revision = subprocess.check_output(["git", "-C", str(args.upstream), "rev-parse", "HEAD"], text=True).strip()
    if revision != UPSTREAM:
        parser.error("Upstream source revision mismatch")
    sys.path.insert(0, str((args.upstream / "server").resolve()))
    import torch
    # Refuse pickle globals outside the safe weights loader, even for this reviewed asset.
    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=True)
    if checkpoint.get("version") != "v2" or checkpoint.get("f0") != 1 or checkpoint["config"][-1] != 40000:
        raise ValueError("Unexpected official model architecture")
    del checkpoint
    from const import EnumInferenceTypes
    from voice_changer.RVC.onnxExporter.export2onnx import _export2onnx
    args.destination.mkdir(parents=True)
    metadata = {"application": "VC_CLIENT", "version": "2.1", "modelType": EnumInferenceTypes.pyTorchRVCv2.value,
        "samplingRate": 40000, "f0": True, "embChannels": 768, "embedder": "hubert_base",
        "embOutputLayer": 12, "useFinalProj": False,
        "sourceCheckpointSha256": APPROVED_CHECKPOINT, "sourceUrl": SOURCE, "licenseUrl": TERMS,
        "attribution": "Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)"}
    torch.set_num_threads(4)
    raw = args.destination / "tsukuyomi-normal1-raw.onnx"
    output = args.destination / "tsukuyomi-normal1.onnx"
    _export2onnx(str(args.checkpoint), str(raw), str(output), False, metadata)
    (args.destination / "provenance.json").write_text(json.dumps({
        "originalSource": SOURCE, "originalSha256": APPROVED_CHECKPOINT,
        "onnxSha256": sha256(output), "termsUrl": TERMS, "metadata": metadata,
        "conversion": "Pinned w-okada exporter, float32, ONNX opset 17; owner terms retained",
    }, indent=2), "utf-8")
    print(f"Converted approved owner model: {output}; SHA-256 {sha256(output)}")


if __name__ == "__main__":
    main()
