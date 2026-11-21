#!/usr/bin/env python3
"""Exercise the actual packaged server as an unprivileged Linux user in a scratch directory."""
import hashlib
import http.server
import json
import os
import platform
import secrets
import socket
import subprocess
import sys
import tarfile
import tempfile
import threading
import urllib.request
import wave
from pathlib import Path


def command(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, **kwargs)


def verify(package, temp):
    with tarfile.open(package) as spk:
        spk.extractall(temp / "package", filter="data")
    root = temp / "package"
    payload = root / "package.tgz"
    info = dict(line.split("=", 1) for line in (root / "INFO").read_text().splitlines())
    arch = {"x86_64": '"x86_64"', "aarch64": '"armv8"'}[platform.machine()]
    assert info["arch"] == arch
    assert info["checksum"].strip('"') == md5(payload)
    assert json.loads((root / "conf/privilege").read_text())["defaults"]["run-as"] == "package"
    for icon, size in (("PACKAGE_ICON.PNG", 64), ("PACKAGE_ICON_256.PNG", 256)):
        data = (root / icon).read_bytes()
        assert data[:8] == b"\x89PNG\r\n\x1a\n" and int.from_bytes(data[16:20], "big") == int.from_bytes(data[20:24], "big") == size

    target, var, tmp = (temp / name for name in ("target", "var", "tmp"))
    for directory in (target, var, tmp):
        directory.mkdir()
    with tarfile.open(payload) as contents:
        contents.extractall(target, filter="data")
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        port = reserve.getsockname()[1]
    env = dict(os.environ, SYNOPKG_PKGDEST=str(target), SYNOPKG_PKGVAR=str(var),
               SYNOPKG_PKGTMP=str(tmp), PALVENI_PORT=str(port), PALVENI_LISTEN="127.0.0.1")
    url = f"http://127.0.0.1:{port}"
    admin = {"username": "spk-test", "password": secrets.token_urlsafe(24)}
    token = None
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def api(path, body=None, method=None):
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = "Bearer " + token
        request = urllib.request.Request(url + "/api/v1/" + path, headers=headers, method=method,
                                         data=json.dumps(body).encode() if body is not None else None)
        return json.load(opener.open(request, timeout=10))

    def control(action, code=0):
        result = subprocess.run([root / "scripts/start-stop-status", action], env=env, capture_output=True)
        assert result.returncode == code, (action, code, result.returncode, result.stderr)

    pidfile = tmp / "service/pid"
    try:
        control("status", 3)
        with http.server.ThreadingHTTPServer(("127.0.0.1", port), http.server.BaseHTTPRequestHandler) as busy:
            thread = threading.Thread(target=busy.serve_forever, daemon=True)
            thread.start()
            try:
                control("start", 1)
                assert not pidfile.exists()
            finally:
                busy.shutdown()
                thread.join()
        control("start")
        before = pidfile.read_text()
        control("start")
        control("status")
        assert pidfile.read_text() == before
        assert api("system/health")["status"] == "ok"
        assert b"<html" in opener.open(url, timeout=5).read().lower()
        with wave.open(str(temp / "test.wav"), "wb") as sample:
            sample.setparams((1, 2, 8000, 0, "NONE", ""))
            sample.writeframes(bytes(16000))
        assert b"pcm_s16le" in command(target / "bin/ffprobe", "-v", "error", "-show_streams", temp / "test.wav").stdout
        command(target / "bin/ffmpeg", "-nostdin", "-v", "error", "-i", temp / "test.wav", "-c:a", "flac", temp / "test.flac")
        assert b"flac" in command(target / "bin/ffprobe", "-v", "error", "-show_streams", temp / "test.flac").stdout
        assert api("setup", admin)["user"]["role"] == "admin"
        token = api("auth/login", admin)["accessToken"]
        skins = api("skins/registry")["skins"]
        assert {s["id"] for s in skins} == {"native", "ink", "gear"}
        for skin in skins:
            assert api("skins/registry/" + skin["id"] + "/install", method="POST")["id"] == skin["id"]
        assert (var / "data/palveni.db").is_file() and (var / "data/palveni-ops.db").is_file()
        control("stop")
        control("status", 3)
        with subprocess.Popen(["sleep", "60"]) as other:
            try:
                stat = Path(f"/proc/{other.pid}/stat").read_text().split()
                pidfile.write_text(f"{other.pid} {stat[21]}\n")
                control("status", 1)
                control("stop")
                assert other.poll() is None
            finally:
                other.terminate()
        # DSM replaces the target and temp trees on upgrade and retains PKGVAR.
        target.rename(temp / "old-target")
        tmp.rename(temp / "old-tmp")
        target.mkdir()
        tmp.mkdir()
        with tarfile.open(payload) as contents:
            contents.extractall(target, filter="data")
        control("start")
        assert api("system/info")["setupCompleted"]
        token = api("auth/login", admin)["accessToken"]
        assert {skin["id"] for skin in api("skins/installed")["skins"]} == {"native", "ink", "gear"}
    finally:
        control("stop")
    control("status", 3)
    print("PASS: non-root startup, health, dashboard, occupied port, idempotence, media encode/probe, offline skins, stale PID isolation, account and skin persistence after target replacement, stop")


def md5(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "md5").hexdigest()


if __name__ == "__main__":
    if os.geteuid() == 0 or sys.platform != "linux":
        sys.exit("Run as a non-root user on the target Linux architecture.")
    with tempfile.TemporaryDirectory(prefix="palveni-spk-test-") as scratch:
        verify(Path(sys.argv[1]), Path(scratch))
