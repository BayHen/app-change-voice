from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description="Repeat the offline OpenVoice prototype benchmark")
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("prototype_args", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not 1 <= args.runs <= 20:
        parser.error("--runs must be between 1 and 20")
    metrics_path = Path("ai-benchmarks/results/current.json")
    values = []
    for _ in range(args.runs):
        command = [sys.executable, "ai-prototype/openvoice_offline.py", *args.prototype_args,
                   "--metrics", str(metrics_path)]
        completed = subprocess.run(command, check=False)
        if completed.returncode:
            return completed.returncode
        values.append(json.loads(metrics_path.read_text(encoding="utf-8")))
    summary = {
        "runs": len(values),
        "medianInferenceSeconds": statistics.median(item["inferenceSeconds"] for item in values),
        "medianRealtimeFactor": statistics.median(item["realtimeFactor"] for item in values),
        "maxRssMiB": max(item["processRssMiB"] for item in values),
        "endToEndLatencyMeasured": False,
    }
    output = Path("ai-benchmarks/results/summary.json")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
