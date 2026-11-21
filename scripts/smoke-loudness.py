"""Measure known synthetic signals over the running server's native job API."""
import array
import hashlib
import json
import math
import os
from pathlib import Path
import sys
import tempfile
import time
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
import wave

base, parent = sys.argv[1:]
token = os.environ["PALVENI_SMOKE_BEARER"]


def api(path, method="GET", body=None, status=200, auth=True):
    headers = {"Content-Type": "application/json"}
    if auth:
        headers["Authorization"] = "Bearer " + token
    request = Request(base + "/api/v1" + path, headers=headers, method=method,
                      data=json.dumps(body).encode() if body is not None else None)
    try:
        with urlopen(request, timeout=10) as response:
            code, raw = response.status, response.read()
    except HTTPError as error:
        code, raw = error.code, error.read()
    assert code == status, (path, code, status, raw[:500])
    return json.loads(raw) if raw else None


def wait(job_id, states):
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        job = next((j for j in api("/jobs?limit=100")["jobs"] if j["id"] == job_id), None)
        if job and job["state"] in states:
            return job
        if job and job["state"] in {"succeeded", "failed", "cancelled"}:
            raise AssertionError((job["state"], job["error"]))
        time.sleep(0.04)
    raise TimeoutError("Loudness fixture job did not finish")


def tone(path, amplitude, seconds):
    rate = 48000
    phase = array.array("h", [round(32767 * amplitude * math.sin(2 * math.pi * 1000 * i / rate))
                              for i in range(rate)])
    whole_second = array.array("h", phase)
    if sys.byteorder != "little":
        whole_second.byteswap()
    with wave.open(str(path), "wb") as output:
        output.setparams((1, 2, rate, 0, "NONE", "not compressed"))
        for second in range(seconds):
            if second == 0 or second == seconds - 1:
                samples = array.array("h", (round(value * min(1, (second * rate + i) / 480,
                                                (seconds * rate - second * rate - i) / 480))
                                            for i, value in enumerate(phase)))
                if sys.byteorder != "little":
                    samples.byteswap()
                output.writeframesraw(samples.tobytes())
            else:
                output.writeframesraw(whole_second.tobytes())
    path.chmod(0o644)


def checksum(path):
    return hashlib.sha256(path.read_bytes()).digest()


def check(root, library):
    job_path = "/libraries/" + library + "/music/loudness"
    def schedule():
        return api(job_path, "POST", status=202)["jobId"]
    def scan():
        wait(api("/libraries/" + library + "/scan", "POST", status=202)["jobId"], {"succeeded"})
    def measurement(track):
        return api("/items/" + items[track] + "/loudness")["measurement"]
    scan()
    items = {i["title"]: i["id"] for i in api("/items?" + urlencode({"libraryId": library}))["items"]}
    hashes = {path: checksum(path) for path in root.glob("*.wav")}
    api(job_path, "POST", status=401, auth=False)
    api("/items/" + items["reference"] + "/loudness", status=401, auth=False)
    assert measurement("reference") is None
    result = wait(schedule(), {"succeeded"})
    assert result["step"]["params"]["measured"] == 3
    loud, soft, silence = (measurement(k) for k in ("reference", "quieter", "silence"))
    assert abs(loud["integratedLufs"] - soft["integratedLufs"] - 20) < 0.15
    assert abs(loud["truePeakDbtp"] + 6.02) < 0.15, loud
    assert loud["current"] and loud["rangeLu"] == 0
    assert all(silence[key] is None for key in ("integratedLufs", "rangeLu", "truePeakDbtp"))
    assert wait(schedule(), {"succeeded"})["step"]["params"] == {"measured": 0, "skipped": 3}
    assert all(checksum(path) == original for path, original in hashes.items())
    (root / "quieter.wav").write_bytes(b"synthetic unreadable audio")
    scan()
    assert measurement("quieter") == dict(soft, current=False)
    assert wait(schedule(), {"failed"})["error"]
    assert measurement("quieter") == dict(soft, current=False)
    tone(root / "quieter.wav", 0.25, 90)
    scan()
    job = schedule()
    assert schedule() == job
    wait(job, {"running"})
    time.sleep(0.15)
    api("/jobs/" + job + "/cancel", "POST", status=204)
    wait(job, {"cancelled"})
    api("/jobs/" + job + "/cancel", "POST", status=204)
    assert measurement("quieter") == dict(soft, current=False)
    print("Reference", loud["integratedLufs"], "LUFS;", "quieter", soft["integratedLufs"],
          "LUFS; true peak", loud["truePeakDbtp"], "dBTP; silence, resume and cancellation passed")


with tempfile.TemporaryDirectory(dir=parent, prefix="loudness-") as directory:
    root = Path(directory)
    root.chmod(0o755)
    for name, amplitude in (("reference", 0.5), ("quieter", 0.05), ("silence", 0)):
        tone(root / (name + ".wav"), amplitude, 4)
    album = api("/libraries", "POST", {"kind":"music", "name":"Loudness references", "paths":[str(root.resolve())]}, status=201)["id"]
    try:
        check(root, album)
    finally:
        api("/libraries/" + album, "DELETE", status=204)
