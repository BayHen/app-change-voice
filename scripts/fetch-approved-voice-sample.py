"""Fetch an owner-published voice-changer demonstration sample for local tests.

No microphone capture or upload. Keep sample/output outside tracked release assets.
"""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urljoin, urlsplit, quote, urlunsplit
from urllib.request import urlopen
import hashlib
import json

PAGE = "https://tyc.rei-yumesaki.net/material/voice/sample-voice/"
root = Path(__file__).resolve().parent.parent / ".ai-runtime-build" / "assets"


class Sources(HTMLParser):
    def __init__(self):
        super().__init__()
        self.urls = []

    def handle_starttag(self, tag, attrs):
        for key, value in attrs:
            if key in ("src", "href") and value and value.lower().endswith(".wav"):
                self.urls.append(urljoin(PAGE, value))


parser = Sources()
parser.feed((root / "sample-voice-page.html").read_text("utf-8"))
urls = list(dict.fromkeys(parser.urls))
if not urls:
    raise ValueError("No official WAV sample found")
source = urls[0]
parts = urlsplit(source)
if parts.scheme != "https" or parts.hostname != "tyc.rei-yumesaki.net":
    raise ValueError("Unexpected sample publisher")
source = urlunsplit((parts.scheme, parts.netloc, quote(parts.path, safe="/%"), parts.query, ""))
with urlopen(source, timeout=30) as response:
    payload = response.read(32 * 1024 * 1024 + 1)
if len(payload) > 32 * 1024 * 1024 or payload[:4] != b"RIFF":
    raise ValueError("Sample is not a bounded WAV file")
target = root / "owner-sample.wav"
target.write_bytes(payload)
(root / "owner-sample-provenance.json").write_text(json.dumps({
    "sourceUrl": source, "termsUrl": PAGE,
    "sha256": hashlib.sha256(payload).hexdigest(), "bytes": len(payload),
    "credit": "Original voice: Tsukuyomi-chan (CV. Rei Yumesaki), official sample voice",
    "rightsBasis": "Owner's page explicitly permits before/after voice effect and voice-changer demonstrations. Character/corpus restrictions and credit apply.",
    "scope": "Local engineering tests; no raw or converted audio bundled/published",
}, indent=2), "utf-8")
print(f"Downloaded owner sample: {target}; {len(payload)} bytes; SHA-256 {hashlib.sha256(payload).hexdigest()}")
