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
import time
import urllib.request
from pathlib import Path

from verify_audio import verify_audio
from verify_covers import verify_cover_api, verify_covers


def command(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, **kwargs)


def verify_video(bin_dir, temp):
    for codec in ("h263", "mpeg4"):
        output = temp / f"{codec}.mov"
        command(bin_dir / "ffmpeg", "-nostdin", "-v", "error", "-filter_complex",
                "testsrc2=size=176x144:rate=4", "-frames:v", "3", "-threads", "1",
                "-c:v", codec, output, timeout=10)
        probe = command(bin_dir / "ffprobe", "-v", "error", "-count_frames", "-show_entries",
                        "stream=codec_name,width,height,nb_read_frames", "-of", "json", output,
                        timeout=10)
        assert json.loads(probe.stdout)["streams"] == [
            {"codec_name": codec, "width": 176, "height": 144, "nb_read_frames": "3"}
        ], (codec, probe.stdout)


def verify_playback(api, opener, url, token, temp, formats, ffprobe):
    media = temp / "media"
    media.mkdir()
    (temp / "levels.flac").rename(media / "levels.flac")
    api("system/settings", {"serverName": None, "perfTier": "lite"}, method="PUT")
    library = api("libraries", {"name": "Synthesized recording", "kind": "music", "paths": [str(media)]})
    job = api(f"libraries/{library['id']}/scan", method="POST")["jobId"]
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        state = next((row["state"] for row in api("jobs?limit=20")["jobs"] if row["id"] == job), None)
        if state == "succeeded":
            break
        assert state in (None, "queued", "running"), state
        time.sleep(0.1)
    else:
        raise TimeoutError("The synthetic recording was not indexed")
    item, = api(f"items?libraryId={library['id']}")["items"]
    credentials = api("me/subsonic-password", method="POST")
    verify_cover_api(opener, url, credentials, item["id"], media, temp, ffprobe)
    scripts = Path(__file__).resolve().parents[2] / "scripts"
    env = dict(os.environ, PALVENI_SMOKE_BEARER=token, PALVENI_SMOKE_AUDIO_FORMATS=",".join(sorted(formats)),
               no_proxy="127.0.0.1", NO_PROXY="127.0.0.1")
    subprocess.run([sys.executable, scripts / "smoke-audio.py", url, item["id"]], env=env, check=True, timeout=90)
    subprocess.run([sys.executable, scripts / "smoke-cue.py", url, temp, "true"], env=env, check=True, timeout=90)


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
    verify_covers(target / "bin", temp)
    verify_video(target / "bin", temp)
    formats = verify_audio(target / "bin/ffmpeg", target / "bin/ffprobe", temp)
    assert formats == {"aac"}, f"Unexpected live encoding profile in SPK: {formats}"
    ui = target / info["dsmuidir"].strip('"')
    menu = json.loads((ui / "config").read_text())[".url"][info["dsmappname"].strip('"')]
    for field, info_field in (("protocol", "adminprotocol"), ("port", "adminport"), ("url", "adminurl")):
        assert menu[field] == info[info_field].strip('"')
    for size in (16, 24, 32, 48, 64, 72, 256):
        data = (ui / menu["icon"].format(size)).read_bytes()
        assert data[:8] == b"\x89PNG\r\n\x1a\n" and int.from_bytes(data[16:20], "big") == int.from_bytes(data[20:24], "big") == size
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
        assert api("setup", admin)["user"]["role"] == "admin"
        token = api("auth/login", admin)["accessToken"]
        hardware = api("system/hardware")
        for tool in ("ffmpeg", "ffprobe"):
            assert Path(hardware[tool]["path"]).samefile(target / "bin" / tool), hardware[tool]
        verify_playback(api, opener, url, token, temp, formats, target / "bin/ffprobe")
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
    print("PASS: packaged media pipelines, non-root startup, health, dashboard, occupied port, "
          "idempotence, offline skins, stale PID isolation, account and skin persistence after "
          "target replacement, stop")


def md5(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "md5").hexdigest()


if __name__ == "__main__":
    if os.geteuid() == 0 or sys.platform != "linux":
        sys.exit("Run as a non-root user on the target Linux architecture.")
    with tempfile.TemporaryDirectory(prefix="palveni-spk-test-") as scratch:
        verify(Path(sys.argv[1]), Path(scratch))
