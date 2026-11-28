"""Extract RVC v2 layer 12 features with the exact verified VoiceMorph encoder."""
import hashlib
from pathlib import Path
import sys

ENCODER_SHA = "1703cf8d2cdc76f8c046f5f6a9bcd224e0e6caf4744cad1a1f4199c32cac8c8d"
DIMENSION = 768
MAX_VECTORS = 16000


def make_index(folder, destination):
    import faiss
    import numpy as np

    files = sorted(folder.glob("*.npy"))
    if not files:
        raise ValueError("No extracted features")
    arrays = [np.load(file, allow_pickle=False, mmap_mode="r") for file in files]
    if any(x.ndim != 2 or x.shape[1] != DIMENSION or x.dtype != np.float32 for x in arrays):
        raise ValueError("Expected RVC v2 768-dimensional float32 frames")
    total = sum(map(len, arrays))
    if total < 8:
        raise ValueError("At least 8 feature vectors are required")
    # Deterministic coverage over the entire corpus with bounded process memory.
    selected = np.linspace(0, total - 1, min(MAX_VECTORS, total), dtype=np.int64)
    index = faiss.IndexFlatL2(DIMENSION)
    offset = 0
    for features in arrays:
        end = offset + len(features)
        rows = selected[np.searchsorted(selected, offset):np.searchsorted(selected, end)] - offset
        if len(rows):
            batch = np.ascontiguousarray(features[rows], dtype=np.float32)
            if not np.isfinite(batch).all() or float(np.max(np.abs(batch))) > 1000:
                raise ValueError("Invalid feature values")
            index.add(batch)
        offset = end
    destination.write_bytes(faiss.serialize_index(index).tobytes())


def extract(runtime, experiment):
    import numpy as np
    import soundfile as sf
    import torch

    asset = runtime / "assets/hubert.pt"
    sha = hashlib.sha256()
    with asset.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(block)
    if sha.hexdigest() != ENCODER_SHA:
        raise ValueError("Unexpected managed HuBERT checkpoint")
    sys.path.insert(0, str(runtime / "server"))
    from voice_changer.RVC.embedder.FairseqHubert import FairseqHubert

    torch.set_num_threads(4)
    device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
    original_load = torch.load

    def trusted_asset_load(file, *args, **kwargs):
        if Path(getattr(file, "name", file)).resolve() != asset:
            raise ValueError("Only the fixed official HuBERT asset may be loaded")
        kwargs["weights_only"] = False
        return original_load(file, *args, **kwargs)

    try:
        torch.load = trusted_asset_load
        encoder = FairseqHubert().loadModel(str(asset), device, False)
    finally:
        torch.load = original_load
    target = experiment / "3_feature768"
    target.mkdir(exist_ok=False)
    files = sorted((experiment / "1_16k_wavs").glob("*.wav"))
    if not files:
        raise ValueError("Trainer produced no 16 kHz segments")
    for file in files:
        audio, rate = sf.read(file, dtype="float32")
        if rate != 16000 or audio.ndim != 1:
            raise ValueError(f"Invalid preprocessed segment: {file.name}")
        signal = torch.from_numpy(audio).unsqueeze(0)
        features = encoder.extractFeatures(signal, 12, False).squeeze(0).cpu().numpy()
        if features.ndim != 2 or features.shape[1] != DIMENSION or not np.isfinite(features).all():
            raise ValueError(f"Invalid HuBERT features for {file.name}")
        np.save(target / f"{file.stem}.npy", features, allow_pickle=False)


if __name__ == "__main__":
    runtime, experiment = (Path(v).resolve(strict=True) for v in sys.argv[1:3])
    if len(sys.argv) == 4:
        make_index(experiment / "3_feature768", Path(sys.argv[3]))
    else:
        extract(runtime, experiment)
