"""Release-builder: pin official component URLs/hashes for terminal-free installation.

Run with the tested project Python environment. Resolves metadata, not executable code.
No arbitrary runtime URLs are accepted by the end-user recommended installer.
"""
import hashlib
from html.parser import HTMLParser
import importlib.metadata as metadata
import json
from pathlib import Path
from urllib.parse import unquote, urlsplit
from urllib.request import urlopen, Request

from packaging.tags import sys_tags
from packaging.utils import parse_wheel_filename

ROOT = Path(__file__).resolve().parent.parent
DOWNLOADS = ROOT / ".ai-runtime-build" / "downloads"
TAGS = {tag: index for index, tag in enumerate(sys_tags())}
COMMIT = "7e2ce5c4800010bbc7604968bd29b802534fac05"


def request_json(url):
    with urlopen(url, timeout=30) as response:
        return json.load(response)


def local(name, url, file, kind):
    value = hashlib.sha256()
    with file.open("rb") as source:
        for block in iter(lambda: source.read(65536), b""):
            value.update(block)
    return {"name": name, "url": url, "sha256": value.hexdigest(), "bytes": file.stat().st_size, "kind": kind}


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.values = []

    def handle_starttag(self, tag, attrs):
        if tag == "a":
            self.values.extend(value for key, value in attrs if key == "href")


components = [
    local("CPython 3.10.11 embeddable x64", "https://www.python.org/ftp/python/3.10.11/python-3.10.11-embed-amd64.zip", DOWNLOADS / "python-3.10.11-embed-amd64.zip", "python"),
    local("w-okada pinned source", f"https://codeload.github.com/w-okada/voice-changer/zip/{COMMIT}", DOWNLOADS / "w-okada-source.zip", "upstream"),
    local("fairseq 0.12.2 inference modules", "https://files.pythonhosted.org/packages/30/36/db42846570f479ac859be3b48d8b47f2ae9b0b9c77487a512f2f2ecbcb6b/fairseq-0.12.2.tar.gz", DOWNLOADS / "fairseq-0.12.2.tar.gz", "fairseq"),
    local("HuBERT Base embedding", "https://dl.fbaipublicfiles.com/hubert/hubert_base_ls960.pt", ROOT / ".ai-runtime-build" / "assets" / "hubert_base_ls960.pt", "hubert"),
    {
        "name": "RVC ContentVec embedding",
        "url": "https://huggingface.co/lj1995/VoiceConversionWebUI/resolve/fc74318833d751ccd0b7a8885291b1b290c8881b/hubert_base.pt",
        "sha256": "f54b40fd2802423a5643779c4861af1e9ee9c1564dc9d32f54f20b5ffba7db96",
        "bytes": 189507909,
        "kind": "rvc-contentvec",
    },
]
for distribution in sorted(metadata.distributions(), key=lambda d: d.metadata["Name"].lower()):
    name, version = distribution.metadata["Name"], distribution.version
    if name.lower() in ("fairseq", "pip", "wheel", "cython"):
        continue
    if name.lower() in ("torch", "torchaudio"):
        with urlopen(f"https://download.pytorch.org/whl/cu124/{name}/", timeout=30) as response:
            links = Links()
            links.feed(response.read().decode())
        expected = f"{name}-{version}-cp310-cp310-win_amd64.whl"
        url = next(url for url in links.values if unquote(urlsplit(url).path).endswith("/" + expected))
        digest = urlsplit(url).fragment.removeprefix("sha256=")
        if len(digest) != 64:
            raise ValueError("Missing official PyTorch index SHA-256")
        url = url.split("#")[0]
        with urlopen(Request(url, method="HEAD"), timeout=30) as response:
            size = int(response.headers.get("Content-Length", "0"))
        component = {"name": f"{name} {version}", "url": url, "sha256": digest, "bytes": size, "kind": "wheel"}
    else:
        release = request_json(f"https://pypi.org/pypi/{name}/{version}/json")
        wheels = []
        for file in release["urls"]:
            if file["filename"].endswith(".whl"):
                _, _, _, tags = parse_wheel_filename(file["filename"])
                ranks = [TAGS[tag] for tag in tags if tag in TAGS]
                if ranks:
                    wheels.append((min(ranks), file))
        if wheels:
            file = min(wheels, key=lambda pair: pair[0])[1]
            kind = "wheel"
        elif name.lower().replace("_", "-") == "antlr4-python3-runtime" and version == "4.8":
            file = next(file for file in release["urls"] if file["packagetype"] == "sdist")
            kind = "antlr"
        else:
            raise ValueError(f"No official compatible wheel for {name} {version}")
        component = {"name": f"{name} {version}", "url": file["url"], "sha256": file["digests"]["sha256"], "bytes": file["size"], "kind": kind}
    components.append(component)
    print(f"Pinned {component['name']}", flush=True)
recipe = {"schemaVersion": 1, "version": "w-okada-vm-cuda124-2026.11.25.1", "upstreamCommit": COMMIT,
    "platform": "windows-x86_64", "pythonVersion": "3.10.11", "components": components,
    "notes": "Validated legacy RVC compatibility candidate. Python 3.10 is end-of-life; update before approving general production distribution. No target voice weights included."}
target = ROOT / "ai-sidecar" / "runtime-recipe.json"
target.write_text(json.dumps(recipe, indent=2), "utf-8")
print(f"Pinned {len(components)} official components, {sum(c['bytes'] for c in components)} download bytes; {target}")
