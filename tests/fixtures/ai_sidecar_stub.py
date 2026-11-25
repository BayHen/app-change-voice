"""Test-only subprocess contract fixture. This is not an AI engine or a shipped runtime."""
import json
import sys
import time

for line in sys.stdin:
    request = json.loads(line)
    if request["op"] == "hang":
        time.sleep(30)
    response = {"id": request["id"], "protocol": 1, "ok": True, "testFixture": True}
    print(json.dumps(response), flush=True)
    if request["op"] == "shutdown":
        break
