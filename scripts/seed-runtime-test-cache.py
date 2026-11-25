"""Reuse already downloaded verified artifacts for the real bootstrap acceptance test."""
import hashlib
import json
from pathlib import Path
import shutil

root = Path(__file__).resolve().parent.parent
recipe = json.loads((root / "ai-sidecar/runtime-recipe.json").read_text("utf-8"))
by_size = {}
for component in recipe["components"]:
    by_size.setdefault(component["bytes"], []).append(component)
target = root / ".ai-runtime-build/bootstrap-installed/downloads"
target.mkdir(parents=True, exist_ok=True)
found = set()
for directory in (root / ".ai-runtime-build/downloads", root / ".ai-runtime-build/assets", root / ".ai-runtime-build/pip-cache"):
    for file in directory.rglob("*"):
        if not file.is_file() or file.stat().st_size not in by_size:
            continue
        candidates = by_size[file.stat().st_size]
        if all(component["sha256"] in found for component in candidates):
            continue
        hasher = hashlib.sha256()
        with file.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                hasher.update(block)
        digest = hasher.hexdigest()
        for component in candidates:
            if digest == component["sha256"]:
                destination = target / digest
                if not destination.exists():
                    shutil.copyfile(file, destination)
                found.add(digest)
                print(f"Verified cached {component['name']}", flush=True)
print(f"Reused {len(found)} / {len(recipe['components'])} official artifacts")
