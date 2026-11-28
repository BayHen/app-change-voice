"""Local Phase 6 RVC training and export. Inputs are consented, versioned datasets."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import wave

PROFILES = ("wind-assassin", "jade-guardian")
REVISION = "81eed5e8f68b6bed1789f682fe78cdd324495afc"
HUBERT_SHA = "1703cf8d2cdc76f8c046f5f6a9bcd224e0e6caf4744cad1a1f4199c32cac8c8d"
ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def read_json(path):
    with path.open("rb") as stream:
        data = stream.read(2 * 1024 * 1024 + 1)
    if len(data) > 2 * 1024 * 1024:
        raise ValueError(f"Manifest too large: {path.name}")
    return json.loads(data)


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def contained(root, relative):
    if Path(relative).is_absolute() or ".." in Path(relative).parts:
        raise ValueError("Manifest paths must be relative to the dataset directory")
    path = (root / relative).resolve(strict=True)
    if not path.is_relative_to(root):
        raise ValueError("Manifest path escapes the dataset directory")
    return path


def dataset(path):
    spec = read_json(path)
    if spec["schemaVersion"] != 1 or spec["profileId"] not in PROFILES:
        raise ValueError("Unsupported dataset schema or profile")
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,80}", spec["speakerId"]):
        raise ValueError("Supply a stable pseudonymous speaker ID")
    if spec["consent"]["training"] is not True or spec["consent"]["voiceConversion"] is not True:
        raise ValueError("Speaker consent must cover training and voice conversion")
    for field in ("sourceUrl", "licenseUrl"):
        from urllib.parse import urlsplit
        url = urlsplit(spec[field])
        if url.scheme != "https" or not url.hostname or url.username or url.password:
            raise ValueError(f"Supply an HTTPS {field}")
    if not 0 < len(spec["attribution"].strip()) <= 160:
        raise ValueError("Provide the required credit")
    root = path.resolve().parent
    if not 1 <= len(spec["clips"]) <= 10000:
        raise ValueError("Dataset must contain 1 to 10000 clips")
    paths, seen = [], set()
    seconds = {"train": 0.0, "holdout": 0.0}
    for clip in spec["clips"]:
        label = clip["split"]
        if label not in seconds or not clip["transcript"].strip() or not clip["language"].strip():
            raise ValueError("Every clip needs a split, language and transcript")
        audio = contained(root, clip["file"])
        actual = sha(audio)
        if actual != clip["sha256"] or actual in seen:
            raise ValueError(f"Checksum mismatch or duplicate recording: {audio.name}")
        seen.add(actual)
        with wave.open(str(audio), "rb") as wav:
            duration = wav.getnframes() / wav.getframerate()
            if (wav.getnchannels() != 1 or wav.getsampwidth() != 2
                    or wav.getframerate() not in (16000, 22050, 24000, 32000, 40000, 44100, 48000)
                    or not 0.5 <= duration <= 60):
                raise ValueError(f"Expected mono PCM16 WAV, 0.5-60 seconds: {audio.name}")
            samples = wav.readframes(wav.getnframes())
            expected_bytes = 2 * wav.getnframes()
        if len(samples) != expected_bytes:
            raise ValueError(f"Truncated WAV: {audio.name}")
        from array import array
        signal = array("h")
        signal.frombytes(samples)
        if sys.byteorder != "little":
            signal.byteswap()
        energy = sum(int(x) ** 2 for x in signal) / len(signal)
        if energy < 32 ** 2 or sum(abs(x) >= 32760 for x in signal) / len(signal) > 0.001:
            raise ValueError(f"Recording is silent or clips excessively: {audio.name}")
        seconds[label] += duration
        paths.append((clip, audio))
    if not all(seconds.values()):
        raise ValueError("Training and held-out audio must both be present")
    # Stable across filesystem locations, independent of the chosen profile name.
    descriptors = sorted(
        (clip["sha256"], clip["split"]) for clip, _ in paths
    )
    fingerprint = hashlib.sha256(json.dumps(descriptors).encode("ascii")).hexdigest()
    return spec, paths, seconds, fingerprint


def run(command, cwd, **kwargs):
    subprocess.run([str(part) for part in command], cwd=cwd, check=True, **kwargs)


def checked_trainer(path):
    path = path.resolve(strict=True)
    revision = subprocess.check_output(["git", "-C", str(path), "rev-parse", "HEAD"], text=True).strip()
    changes = subprocess.check_output(
        ["git", "-C", str(path), "status", "--porcelain", "--untracked-files=no"], text=True
    )
    if revision != REVISION or changes:
        raise ValueError("Use a clean checkout of the pinned RVC training revision")
    return path


def filelist(experiment):
    waves = sorted((experiment / "0_gt_wavs").glob("*.wav"))
    rows = []
    for path in waves:
        stem = path.stem
        artifacts = [
            path, experiment / "3_feature768" / f"{stem}.npy",
            experiment / "2a_f0" / f"{stem}.wav.npy",
            experiment / "2b-f0nsf" / f"{stem}.wav.npy",
        ]
        if not all(p.is_file() and p.stat().st_size for p in artifacts):
            raise ValueError(f"Incomplete F0 or HuBERT extraction: {stem}")
        rows.append("|".join([*(p.resolve().as_posix() for p in artifacts), "0"]))
    if len(rows) < 2:
        raise ValueError("Not enough processed training segments")
    (experiment / "filelist.txt").write_text("\n".join(rows) + "\n", encoding="utf-8")
    return len(rows)


def train(args):
    spec, paths, seconds, fingerprint = dataset(args.dataset)
    trainer = checked_trainer(args.trainer)
    runtime = args.runtime.resolve(strict=True)
    hubert = runtime / "assets/hubert.pt"
    if sha(hubert) != HUBERT_SHA:
        raise ValueError("The verified VoiceMorph HuBERT checkpoint is required")
    if not 1 <= args.epochs <= 2000 or not 1 <= args.batch <= 32 or not 0 <= args.gpu <= 15:
        raise ValueError("Invalid epoch, batch or CUDA device")
    if any(shutil.which(exe) is None for exe in ("ffmpeg", "ffprobe")):
        raise ValueError("Add FFmpeg and FFprobe to PATH")
    if args.rmvpe:
        asset = trainer / "assets/rmvpe/rmvpe.pt"
        if not re.fullmatch(r"[a-f0-9]{64}", args.rmvpe) or sha(asset) != args.rmvpe:
            raise ValueError("RMVPE PT SHA-256 does not match the approved local asset")
    if args.init:
        for weight, expected in args.init:
            if not re.fullmatch(r"[a-f0-9]{64}", expected) or sha(Path(weight).resolve()) != expected:
                raise ValueError("Pretraining weight checksum mismatch")
    python = args.python.resolve(strict=True)
    run([python, "-c", "import torch; assert torch.cuda.is_available(), 'CUDA training requires NVIDIA CUDA'"], trainer)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    environment = subprocess.check_output([str(python), "-m", "pip", "freeze"], text=True)
    (output / "dependencies.txt").write_text(environment, encoding="utf-8")
    staged = output / "training-input"
    staged.mkdir()
    for number, (clip, original) in enumerate(paths):
        if clip["split"] == "train":
            target = staged / f"{number:06d}.wav"
            shutil.copyfile(original, target)
            if sha(target) != clip["sha256"]:
                raise ValueError("Input changed during staging")
    exp_name = f"{spec['profileId']}-{fingerprint[:12]}"
    experiment = trainer / "logs" / exp_name
    experiment.mkdir(parents=True, exist_ok=False)
    (trainer / "assets/weights").mkdir(parents=True, exist_ok=True)
    weight = trainer / "assets/weights" / f"{exp_name}.pth"
    if weight.exists():
        raise ValueError("Checkpoint already exists for this experiment")
    # Upstream shares the 40 kHz config between v1 and v2. The CLI selects the v2 network.
    config = read_json(trainer / "configs/v1/40k.json")
    config["model"]["spk_embed_dim"] = 1
    write_json(experiment / "config.json", config)
    env = {**os.environ, "TORCH_FORCE_WEIGHTS_ONLY_LOAD": "1", "PYTHONUTF8": "1",
           "USE_LIBUV": "0", "PYTHONNOUSERSITE": "1"}
    run([python, "-m", "train.preprocess", staged, 40000, 1, experiment, "True", 3.7], trainer, env=env)
    run([python, "-m", "train.dataset.extract_f0", "cpu", experiment, 1, "rmvpe" if args.rmvpe else "pm"], trainer, env=env)
    run([runtime / "python/python.exe", "-I", "-X", "utf8", ROOT / "scripts/rvc-original-features.py",
         runtime, experiment], output)
    count = filelist(experiment)
    if count < args.batch:
        raise ValueError("Training batch is larger than the segment count")
    flags = []
    if args.init:
        flags = ["-pg", Path(args.init[0][0]).resolve(), "-pd", Path(args.init[1][0]).resolve()]
    run([python, "-m", "train.train", "-e", exp_name, "-sr", "40k", "-v", "v2", "-f0", 1,
         "-bs", args.batch, "-g", args.gpu, "-te", args.epochs, "-se", args.epochs,
         "-sw", 0, "-l", 1, "-c", 0, *flags], trainer, env=env)
    if not weight.is_file() or not weight.stat().st_size:
        raise ValueError("Trainer did not produce a final inference checkpoint. Inspect its log.")
    for property_name in ("profileId", "speakerId", "licenseUrl", "sourceUrl", "attribution"):
        config[property_name] = spec[property_name]
    config.update(datasetSha256=fingerprint, datasetManifestSha256=sha(args.dataset),
                  trainingRevision=REVISION, embedderSha256=HUBERT_SHA,
                  f0Training="rmvpe" if args.rmvpe else "pm", rmvpeSha256=args.rmvpe,
                  initializerSha256=[digest for _, digest in args.init] if args.init else [],
                  epochs=args.epochs, batch=args.batch, seconds=seconds, segments=count)
    shutil.copyfile(weight, output / "voice.pth")
    run([runtime / "python/python.exe", "-I", "-X", "utf8", ROOT / "scripts/rvc-original-features.py",
         runtime, experiment, output / "voice.index"], output)
    write_json(output / "training-provenance.json", config)
    print(f"Training finished. Export the local pack from {output}")


def export(args):
    run_dir = args.run.resolve(strict=True)
    runtime = args.runtime.resolve(strict=True)
    safe_torch = args.converter_site_packages.resolve(strict=True)
    provenance = read_json(run_dir / "training-provenance.json")
    checkpoint = run_dir / "voice.pth"
    checkpoint_sha = sha(checkpoint)
    request = dict(protocol=1, id=1, op="convert", checkpointPath=str(checkpoint),
                   outputDirectory=str(run_dir), sha256=checkpoint_sha)
    result = subprocess.run(
        [str(runtime / "python/python.exe"), "-I", "-X", "utf8",
         str(ROOT / "ai-sidecar/convert_worker.py"), "--runtime-root", str(runtime),
         "--safe-torch-root", str(safe_torch)],
        input=json.dumps(request) + "\n", stdout=subprocess.PIPE, check=True, cwd=run_dir,
        encoding="utf-8",
    )
    record = json.loads(result.stdout)
    if not record.get("ok"):
        raise ValueError(record.get("error", "Exporter failed"))
    if (record["version"], record["samplingRate"], record["embChannels"], record["f0"], record["speakerCount"]) != ("v2", 40000, 768, True, 1):
        raise ValueError("Export is not a single-speaker 40 kHz RVC v2 F0 model")
    manifest = {name: provenance[name] for name in (
        "profileId", "speakerId", "datasetSha256", "trainingRevision", "embedderSha256",
        "sourceUrl", "licenseUrl", "attribution"
    )}
    manifest.update(schemaVersion=1, architecture="rvc-v2-f0", sampleRate=40000,
                    license="LicenseRef-UserConfirmed")
    for key, file in (("checkpoint", "voice.pth"), ("onnx", "voice.onnx"), ("featureIndex", "voice.index")):
        manifest[key] = {"file": file, "sha256": sha(run_dir / file)}
    write_json(run_dir / "voice-pack.json", manifest)
    print(f"Exported {run_dir / 'voice-pack.json'} for app inference validation.")


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    commands = cli.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate")
    validate.add_argument("dataset", type=Path)
    pair = commands.add_parser("validate-pair")
    pair.add_argument("wind", type=Path)
    pair.add_argument("jade", type=Path)
    training = commands.add_parser("train")
    for item in ("dataset", "trainer", "python", "runtime", "output"):
        training.add_argument("--" + item, required=True, type=Path)
    training.add_argument("--epochs", type=int, default=200)
    training.add_argument("--batch", type=int, default=4)
    training.add_argument("--gpu", type=int, default=0)
    training.add_argument("--rmvpe", metavar="REVIEWED_SHA256")
    training.add_argument("--init", action="append", nargs=2, metavar=("PATH", "SHA256"),
                          help="Optional reviewed generator then discriminator")
    exporting = commands.add_parser("export")
    for item in ("run", "runtime", "converter-site-packages"):
        exporting.add_argument("--" + item, required=True, type=Path)
    args = cli.parse_args()
    try:
        if args.command == "validate":
            spec, _, seconds, fingerprint = dataset(args.dataset)
            print(json.dumps(dict(profileId=spec["profileId"], seconds=seconds, datasetSha256=fingerprint), indent=2))
        elif args.command == "validate-pair":
            wind, wind_clips, _, wind_sha = dataset(args.wind)
            jade, jade_clips, _, jade_sha = dataset(args.jade)
            if (wind["profileId"], jade["profileId"]) != PROFILES:
                raise ValueError("Pass Wind Assassin first, then Jade Guardian")
            if (wind["speakerId"].casefold() == jade["speakerId"].casefold()
                    or {clip["sha256"] for clip, _ in wind_clips}
                    & {clip["sha256"] for clip, _ in jade_clips}):
                raise ValueError("Original profiles must have independent speakers and recordings")
            print(json.dumps(dict(windAssassin=wind_sha, jadeGuardian=jade_sha), indent=2))
        elif args.command == "train":
            if args.init and len(args.init) != 2:
                raise ValueError("Specify exactly two initialization assets or train from scratch")
            train(args)
        else:
            export(args)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError, wave.Error) as error:
        cli.exit(1, f"VoiceMorph RVC: {error}\n")


if __name__ == "__main__":
    main()
