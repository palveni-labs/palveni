"""Verify live audio representations of the smoke script's one-second WAV."""
import json
import os
import sys
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

base, item = sys.argv[1:]
authorization = {"Authorization": "Bearer " + os.environ["PALVENI_SMOKE_BEARER"]}
api = "/api/v1/items/" + item


def request(path, method="GET", headers=None):
    req = Request(base + path, method=method, headers=headers or {})
    try:
        with urlopen(req, timeout=20) as response:
            return response.status, response.headers, response.read()
    except HTTPError as err:
        return err.code, err.headers, err.read()


def prepare(**options):
    return request(api + "/playback?" + urlencode(options), "POST", authorization)


assert prepare(format="unknown")[0] == 400
assert prepare(format="mp3", bitrateKbps=999)[0] == 400
assert prepare(format="original", startMs=50)[0] == 400
assert request(api + "/audio?format=mp3")[0] == 401
expected = set(os.environ.get("PALVENI_SMOKE_AUDIO_FORMATS", "mp3,aac,opus").split(","))
assert expected and expected <= {"mp3", "aac", "opus"}, "Invalid expected audio formats"
for codec, mime, magic in (("mp3", "audio/mpeg", b"ID3"), ("aac", "audio/mp4", b"ftyp"), ("opus", "audio/ogg", b"OggS")):
    status, _, body = prepare(format=codec, bitrateKbps=96, startMs=125)
    if codec not in expected:
        assert status == 503 and json.loads(body)["error"]["code"] == "transcode_format_unavailable", (codec, status)
        continue
    assert status == 200, (codec, status)
    info = json.loads(body)
    assert info["mode"] == "transcode"
    url = info["url"]
    assert request(url.replace("startMs=125", "startMs=0"))[0] == 401
    assert request(url.replace("bitrateKbps=96", "bitrateKbps=64"))[0] == 401
    assert request(url.replace("/audio?", "/stream?"))[0] == 401
    status, headers, body = request(url, "HEAD")
    assert status == 200 and not body and headers["Content-Type"] == mime
    status, headers, body = request(url, headers={"Range": "bytes=0-"})
    assert status == 200 and headers["Accept-Ranges"] == "none"
    assert headers["Content-Type"] == mime and magic in body[:48] and len(body) > 200
