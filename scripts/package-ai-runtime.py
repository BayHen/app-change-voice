"""Release-builder tool, not an end-user Python/pip installation workflow.

Assemble an already prepared, licensed portable Python 3.10 environment with the
pinned upstream source and explicitly approved embedding assets. Does not download.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

COMMIT = "7e2ce5c4800010bbc7604968bd29b802534fac05"


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(65536), b""):
            value.update(block)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-dir", type=Path, required=True)
    parser.add_argument("--upstream", type=Path, required=True)
    parser.add_argument("--hubert", type=Path, required=True)
    parser.add_argument("--hubert-license", type=Path, required=True)
    parser.add_argument("--hubert-source", required=True)
    parser.add_argument("--dependency-notices", type=Path, required=True)
    parser.add_argument("--rmvpe", type=Path)
    parser.add_argument("--rmvpe-license", type=Path)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    if args.destination.exists():
        parser.error("Destination must be new; an existing runtime is never overwritten")
    if not args.hubert_source.startswith("https://"):
        parser.error("Record the approved embedding weight's HTTPS source")
    if args.rmvpe and not args.rmvpe_license:
        parser.error("RMVPE assets require their own license notice")
    revision = subprocess.check_output(["git", "-C", str(args.upstream), "rev-parse", "HEAD"], text=True).strip()
    if revision != COMMIT:
        parser.error("Upstream revision differs from VoiceMorph's audited commit")
    if subprocess.check_output(["git", "-C", str(args.upstream), "status", "--porcelain"], text=True).strip():
        parser.error("Upstream reference must be clean")
    python = args.python_dir / "python.exe"
    if not python.is_file() or not (args.python_dir / "python310.dll").is_file():
        parser.error("Supply a self-contained Windows x64 Python 3.10 distribution")
    version = subprocess.check_output([str(python), "--version"], text=True)
    if not version.startswith("Python 3.10."):
        parser.error("The audited upstream dependency family requires Python 3.10")
    probe = "import torch,onnxruntime,onnx,numpy,scipy,resampy,pyworld,faiss,fairseq,torchcrepe,torchfcpe; print('dependency imports OK')"
    subprocess.run([str(python), "-I", "-c", probe], check=True)
    source_probe = "import sys; sys.path.insert(0, sys.argv[1]); from voice_changer.RVC.RVCr2 import RVCr2; from voice_changer.VoiceChangerV2 import VoiceChangerV2; print('upstream imports OK')"
    subprocess.run([str(python), "-I", "-c", source_probe, str((args.upstream / "server").resolve())], check=True)
    root = args.destination
    root.mkdir(parents=True)
    shutil.copytree(args.python_dir, root / "python", ignore=shutil.ignore_patterns("__pycache__", "*.pyc", ".git"))
    shutil.copytree(args.upstream / "server", root / "server", ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "test.wav", "Beatrice"))
    for directory in ("assets", "notices"):
        (root / directory).mkdir()
    project = Path(__file__).resolve().parent.parent
    shutil.copyfile(project / "ai-sidecar" / "bridge.py", root / "bridge.py")
    shutil.copyfile(args.hubert, root / "assets" / "hubert.pt")
    shutil.copyfile(args.hubert_license, root / "notices" / "hubert-license.txt")
    shutil.copyfile(args.dependency_notices, root / "notices" / "dependencies.txt")
    shutil.copyfile(args.upstream / "LICENSE", root / "notices" / "w-okada-LICENSE.txt")
    shutil.copyfile(args.upstream / "LICENSE-NOTICE", root / "notices" / "w-okada-NOTICE.txt")
    if args.rmvpe:
        shutil.copyfile(args.rmvpe, root / "assets" / "rmvpe.onnx")
        shutil.copyfile(args.rmvpe_license, root / "notices" / "rmvpe-license.txt")
    (root / "notices" / "asset-sources.json").write_text(json.dumps({"hubertSource": args.hubert_source,
        "hubertSha256": digest(args.hubert), "warning": "Publisher must independently approve redistribution of every asset and dependency."}, indent=2), "utf-8")
    files = [{"path": str(path.relative_to(root)).replace("\\", "/"), "sha256": digest(path), "bytes": path.stat().st_size}
             for path in sorted(root.rglob("*")) if path.is_file()]
    manifest = {"schemaVersion": 1, "protocolVersion": 1, "upstreamCommit": COMMIT,
        "version": f"w-okada-{COMMIT[:12]}-vm1", "pythonExecutable": "python/python.exe", "entryPoint": "bridge.py",
        "serverDirectory": "server", "contentVecModel": "assets/hubert.pt", "rmvpeModel": "assets/rmvpe.onnx" if args.rmvpe else None,
        "notices": [file["path"] for file in files if file["path"].startswith("notices/")], "files": files}
    target = root / "runtime-manifest.json"
    target.write_text(json.dumps(manifest, ensure_ascii=True, indent=2), "utf-8")
    print(f"Runtime assembled: {root}")
    print(f"Publisher manifest SHA-256: {digest(target)}")
    print("No redistribution approval or inference benchmark is implied by packaging success.")


if __name__ == "__main__":
    main()
