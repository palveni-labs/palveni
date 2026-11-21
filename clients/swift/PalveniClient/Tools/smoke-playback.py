"""Run the Swift package tests against a fresh local Palveni server with two sessions."""

import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
from urllib.error import URLError
from urllib.request import Request, urlopen
import wave


def run(binary, root):
    with socket.socket() as reserved:
        reserved.bind(("127.0.0.1", 0))
        port = reserved.getsockname()[1]
    origin = f"http://127.0.0.1:{port}"

    def api(path, body=None, token=None):
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        request = Request(origin + "/api/v1" + path, headers=headers,
                          data=json.dumps(body).encode() if body is not None else None)
        with urlopen(request, timeout=5) as response:
            return json.load(response)

    media = root / "media"
    media.mkdir()
    titles = ("Session test", "海边来信", "Glass Harbor")
    for title in titles:
        with wave.open(str(media / (title + ".wav")), "wb") as audio:
            audio.setparams((1, 2, 8000, 0, "NONE", ""))
            audio.writeframes(bytes(160_000))
    # The fixture uses its own server, data, configuration and synthetic media.
    env = {key: value for key, value in os.environ.items() if not key.startswith("PALVENI_")}
    server_name = "swift-fixture-" + secrets.token_hex(8)
    server = subprocess.Popen([binary, "serve", "--listen", "127.0.0.1", "--port", str(port),
                               "--data-dir", str(root / "data")], cwd=root,
                              env={**env, "PALVENI_SERVER_NAME": server_name},
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.monotonic() + 30
        while True:
            if server.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError("Temporary server did not start")
            try:
                if api("/system/info")["name"] != server_name:
                    raise RuntimeError("Port belongs to a different server")
                break
            except URLError:
                time.sleep(0.1)
        account = {"username": "swift-fixture", "password": secrets.token_urlsafe(24)}
        api("/setup", account)
        first, second = [api("/auth/login", {**account, "deviceName": device})["accessToken"]
                         for device in ("swift-phone", "swift-tv")]
        library = api("/libraries", {"name": "Sessions", "kind": "music", "paths": [str(media)]}, first)["id"]
        job = api(f"/libraries/{library}/scan", {}, first)["jobId"]
        deadline = time.monotonic() + 60
        while True:
            states = {j["id"]: j["state"] for j in api("/jobs", token=first)["jobs"]}
            if states.get(job) == "succeeded":
                break
            if states.get(job) in ("failed", "cancelled") or time.monotonic() > deadline:
                raise RuntimeError("Fixture scan failed")
            time.sleep(0.1)
        tracks = {track["title"]: track["id"]
                  for track in api(f"/items?libraryId={library}", token=first)["items"]}
        subprocess.run(["swift", "test"], cwd=Path(__file__).resolve().parents[1], check=True,
                       env={**env, "PALVENI_SWIFT_TEST_URL": origin, "PALVENI_SWIFT_TEST_ITEM": tracks[titles[0]],
                            "PALVENI_SWIFT_FAVORITE_A": tracks[titles[1]], "PALVENI_SWIFT_FAVORITE_B": tracks[titles[2]],
                            "PALVENI_SWIFT_TEST_TOKEN_A": first, "PALVENI_SWIFT_TEST_TOKEN_B": second})
    finally:
        server.terminate()
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temp:
        run(args.binary.resolve(), Path(temp))
