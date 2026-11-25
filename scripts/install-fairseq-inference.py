"""Assemble fairseq's Python inference modules from its verified official sdist.

The 0.12.2 sdist omits libbase C++ sources and cannot build on this Windows host.
HuBERT inference uses Torch operations, not optional training/binarization extensions.
No compiler installation and no third-party wheel mirror are used.
"""
import argparse
import base64
import csv
import hashlib
from pathlib import Path
import sysconfig
import tarfile

SHA256 = "34f1b18426bf3844714534162f065ab733e049597476daa35fffb4d06a92b524"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    args = parser.parse_args()
    if hashlib.sha256(args.source.read_bytes()).hexdigest() != SHA256:
        parser.error("Official fairseq sdist digest mismatch")
    site = Path(sysconfig.get_paths()["purelib"])
    package = site / "fairseq"
    if package.exists():
        parser.error("fairseq already exists; existing modules are never overwritten")
    records = []
    with tarfile.open(args.source, "r:gz") as archive:
        for item in archive:
            parts = Path(item.name).parts
            if len(parts) == 1 and parts[0] == "fairseq-0.12.2" and item.isdir():
                continue
            if len(parts) < 2 or parts[0] != "fairseq-0.12.2" or ".." in parts:
                raise ValueError("Unsafe source member")
            if not item.isfile() or item.size > 16 * 1024 * 1024:
                continue
            relative = Path(*parts[1:])
            if parts[1] == "fairseq":
                if relative.suffix not in (".py", ".yaml", ".yml", ".json", ".txt"):
                    continue
                target = site / relative
            elif str(relative) == "LICENSE":
                target = site / "fairseq-0.12.2.dist-info" / "LICENSE"
            else:
                continue
            payload = archive.extractfile(item).read()
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(payload)
            records.append((target.relative_to(site).as_posix(), "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(payload).digest()).rstrip(b"=").decode(), len(payload)))
    info = site / "fairseq-0.12.2.dist-info"
    info.mkdir(exist_ok=True)
    metadata = "Metadata-Version: 2.1\nName: fairseq\nVersion: 0.12.2\nLicense: MIT\nSummary: Official fairseq Python inference modules; optional native training extensions omitted by VoiceMorph\n"
    (info / "METADATA").write_text(metadata, "utf-8")
    (info / "INSTALLER").write_text("VoiceMorph verified source assembly\n", "utf-8")
    (info / "VOICE_MORPH_SOURCE.txt").write_text(f"Source: https://pypi.org/project/fairseq/0.12.2/\nSHA-256: {SHA256}\nOptional C++/Cython training extensions omitted. Python modules unmodified.\n", "utf-8")
    with (info / "RECORD").open("w", newline="", encoding="utf-8") as record:
        csv.writer(record).writerows(records)
    print(f"Assembled {len(records)} verified fairseq Python inference files in {package}")


if __name__ == "__main__":
    main()
