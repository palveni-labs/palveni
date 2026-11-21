#!/usr/bin/env python3
"""Exercise a CUE album using synthetic PCM and a newly created library."""
import json
import math
import os
from pathlib import Path
import struct
import sys
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import wave

BASE, TEMP, ENCODING = sys.argv[1:]
BEARER = os.environ["PALVENI_SMOKE_BEARER"]
SHEET = """PERFORMER "Cue Ensemble"
TITLE "Cue Sessions"
FILE "Suite.wav" WAVE
  TRACK 01 AUDIO
    TITLE "Overture"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Adagio"
    PERFORMER "Guest Strings"
    INDEX 00 00:00:55
    INDEX 01 00:01:01
FILE "Encore.wav" WAVE
  TRACK 03 AUDIO
    TITLE "Finale"
    INDEX 01 00:00:15
"""


def request(path, *, method="GET", body=None, auth=True, headers=None, status=200):
    h = {"Authorization": "Bearer " + BEARER} if auth else {}
    h.update(headers or {})
    data = None
    if body is not None:
        h["Content-Type"] = "application/json"
        data = json.dumps(body).encode()
    try:
        response = urlopen(Request(BASE + path, data=data, headers=h, method=method), timeout=30)
    except HTTPError as err:
        response = err
    with response:
        assert response.status == status, (method, path.split("?")[0], response.status, status, response.read()[:220])
        return response.headers, response.read()


def api(path, **kwargs):
    _, data = request("/api/v1" + path, **kwargs)
    return json.loads(data) if data else None


def wait_job(job):
    for _ in range(200):
        state = next((j for j in api("/jobs?limit=50")["jobs"] if j["id"] == job), None)
        if state is None:
            time.sleep(.1)
            continue
        if state["state"] == "succeeded":
            return state
        assert state["state"] in ("queued", "running"), state
        time.sleep(.1)
    raise RuntimeError("Background task exceeded 20 s")


def scan(library):
    wait_job(api("/libraries/" + library + "/scan", method="POST", status=202)["jobId"])


def tone(nframes, frequency, level=8000):
    return b"".join(struct.pack("<hh", s, -s) for s in
                    (round(level * math.sin(2 * math.pi * frequency * n / 44100)) for n in range(nframes * 588)))


def write_audio(path, samples):
    with wave.open(str(path), "wb") as fp:
        fp.setparams((2, 2, 44100, 0, "NONE", "not compressed"))
        fp.writeframes(samples)


def exercise(folder):
    folder.mkdir()
    first, second, third = tone(76, 220), tone(115, 550, 800), tone(90, 880)
    write_audio(folder / "Suite.wav", first + second)
    write_audio(folder / "Encore.wav", third)
    cue = folder / "Suite.cue"
    cue.write_text(SHEET, encoding="utf-8-sig")
    originals = {path: path.read_bytes() for path in folder.glob("*.wav")}
    library = api("/libraries", method="POST", body={"name": "Cue Sessions", "kind": "music", "paths": [str(folder)]}, status=201)["id"]
    music = "/libraries/" + library + "/music"
    try:
        scan(library)
        files = api("/items?libraryId=" + library)["items"]
        assert sorted(e["title"] for e in files) == ["Encore", "Suite"], files
        assert api("/libraries/" + library)["itemCount"] == 2
        artists = api(music)["entries"]
        assert [e["name"] for e in artists] == ["Cue Ensemble"], artists
        artist = artists[0]["id"]
        album = api(music + "?artist=" + artist)["entries"][0]["id"]
        browse = music + "?artist=" + artist + "&album=" + album
        tracks = api(browse)["entries"]
        assert [(t["title"], t["trackNumber"], t["durationMs"]) for t in tracks] == [
            ("Overture", 1, 1013), ("Adagio", 2, 1533), ("Finale", 3, 1000)], tracks
        track = tracks[1]["id"]
        if ENCODING == "true":
            job = api("/libraries/" + library + "/music/loudness", method="POST", status=202)
            assert wait_job(job["jobId"])["step"]["params"] == {"measured": 3, "skipped": 0}
            levels = [api("/items/" + entry["id"] + "/loudness")["measurement"] for entry in tracks]
            assert all(value["current"] for value in levels)
            assert abs(levels[0]["truePeakDbtp"] - levels[1]["truePeakDbtp"] - 20) < 0.25, levels
        assert tracks[1]["artist"] == "Guest Strings"
        assert api(music + "/search?q=adagio")["entries"][0]["id"] == track
        personal = "/me/items/" + track
        assert api(personal + "/favorite", method="PUT", body={"favorite": True})["favorite"]
        url = "/items/" + track + "/playback"
        if ENCODING == "true":
            native = api(url, method="POST")["url"]
            h, data = request(native, auth=False)
            assert h["Content-Type"] == "audio/wav" and data[:4] == b"RIFF" and data[44:] == second
            assert int(h["Content-Length"]) == 44 + len(second)
            head, _ = request(native, auth=False, method="HEAD")
            assert head["Content-Length"] == h["Content-Length"]
            for start, end in [(0, 1), (41, 48), (12345, 16789), (len(data)-5, len(data)-1)]:
                rh, part = request(native, auth=False, headers={"Range": f"bytes={start}-{end}"}, status=206)
                assert rh["Content-Range"] == f"bytes {start}-{end}/{len(data)}"
                assert part == data[start:end+1], (start, end)
            request(native, auth=False, headers={"Range": f"bytes={len(data)}-"}, status=416)
            api(url + "?format=mp3&startMs=1534", method="POST", status=400)
            codec = os.environ.get("PALVENI_SMOKE_AUDIO_FORMATS", "mp3,aac,opus").split(",")[0]
            encoded = api(url + "?format=" + codec + "&startMs=1000", method="POST")["url"]
            assert {"mp3": b"ID3", "aac": b"ftyp", "opus": b"OggS"}[codec] in request(encoded, auth=False)[1][:48]
        else:
            api(url, method="POST", status=503)

        cue.write_text(SHEET.replace("Adagio", "Adagio II").replace("00:01:01", "00:01:15"), encoding="utf-8")
        scan(library)
        renamed = api(browse)["entries"][1]
        assert (renamed["id"], renamed["title"], renamed["durationMs"]) == (track, "Adagio II", 1346), renamed
        assert api(personal)["favorite"]
        if ENCODING == "true":
            assert api("/items/" + track + "/loudness")["measurement"]["current"] is False
        cue.unlink()
        scan(library)
        assert not api(music + "/search?q=adagio")["entries"]
        assert api(music)["entries"][0]["name"] is None
        api(url, method="POST", status=404)
        cue.write_text(SHEET)
        scan(library)
        assert api(browse)["entries"][1]["id"] == track
        assert api(personal)["favorite"]
        assert all(path.read_bytes() == data for path, data in originals.items())
    finally:
        api("/libraries/" + library, method="DELETE", status=204)


if __name__ == "__main__":
    exercise(Path(TEMP).resolve() / "cue-album")
