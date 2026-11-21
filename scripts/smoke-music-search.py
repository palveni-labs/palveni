"""Check native search on a disposable local server. Set PALVENI_SMOKE_BEARER to its admin token."""

import argparse
from contextlib import contextmanager
import json
import math
import os
from pathlib import Path
import struct
import tempfile
import time
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("base", help="server origin")
parser.add_argument("media_parent", type=Path, help="test directory readable by the server")
parser.add_argument("--benchmark-tracks", type=int, default=0,
                    help="add this many tracks per library and time warm searches")
args = parser.parse_args()
if args.benchmark_tracks < 0:
    parser.error("--benchmark-tracks must be nonnegative")
bearer = os.environ["PALVENI_SMOKE_BEARER"]
artist, album, title = "遠岸合奏", "霧港來信", "星河燈塔"


def request(path, *, method="GET", body=None, status=200, auth=True):
    headers = {"Authorization": "Bearer " + bearer} if auth else {}
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = Request(args.base + path, data=data, headers=headers, method=method)
    try:
        response = urlopen(req, timeout=30)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read()
        assert response.status == status, (method, path.split("?")[0], response.status, status)
        return raw


def api(path, *, params=None, **kwargs):
    query = "?" + urlencode(params) if params else ""
    raw = request("/api/v1" + path + query, **kwargs)
    return json.loads(raw) if raw else None


def search(q, *, library=None, status=200, **params):
    path = f"/libraries/{library}/music/search" if library else "/music/search"
    return api(path, params={"q": q, **params}, status=status)


def write_track(path, name, source, *, performer=artist, collection=album):
    # The 50 ms mono FLAC frame and STREAMINFO also used by the main smoke fixture.
    info = bytes.fromhex("0240024000000d00000d01f400f0000001906b431bf2da7c9312b3e5c21e67f6591b")
    frame = bytes.fromhex("fff8740800018f240000007d5f")
    comments = [f"TITLE={name}", f"ARTIST={performer}", f"ALBUM={collection}", f"COMMENT={source}"]
    tags = struct.pack("<I", 7) + b"Palveni" + struct.pack("<I", len(comments))
    for comment in comments:
        value = comment.encode()
        tags += struct.pack("<I", len(value)) + value
    path.write_bytes(b"fLaC\0\0\0\x22" + info + b"\x84" + len(tags).to_bytes(3, "big") + tags + frame)


def scan(library):
    job_id = api(f"/libraries/{library}/scan", method="POST", status=202)["jobId"]
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        jobs = api("/jobs", params={"limit": 50})["jobs"]
        job = next((row for row in jobs if row["id"] == job_id), None)
        if job and job["state"] == "succeeded":
            return
        assert not job or job["state"] in {"queued", "running"}, job
        time.sleep(0.1)
    raise TimeoutError("Music library scan timed out")


@contextmanager
def library(folder, name, companion):
    folder.mkdir()
    write_track(folder / "shared.flac", title, name)
    write_track(folder / "companion.flac", companion, name)
    for index in range(args.benchmark_tracks):
        write_track(folder / f"sample-{index}.flac", f"夜間錄音 {index:05}", name,
                    performer=f"合成樣本 {index % 100}", collection=f"合成唱片 {index % 500}")
    source = api("/libraries", method="POST", status=201,
                 body={"kind": "music", "name": name, "paths": [str(folder)]})
    try:
        yield source["id"]
    finally:
        api("/libraries/" + source["id"], method="DELETE", status=204)


def check_results(sources):
    groups = {}
    for kind, query, expected in (("artist", "远岸合奏", artist), ("album", "wuganglaixin", album),
                                  ("track", "xhdt", title)):
        first = search(query, kind=kind, limit=1)
        cursor = first["nextCursor"]
        assert cursor and len(first["entries"]) == 1, kind
        last = search(query, kind=kind, cursor=cursor, limit=1)
        assert last["nextCursor"] is None and len(last["entries"]) == 1, kind
        hits = first["entries"] + last["entries"]
        assert hits == search(query, kind=kind)["entries"], kind
        assert hits == sorted(hits, key=lambda h: (h["artistId"] or "", h["id"], h["libraryId"])), kind
        assert {h["libraryId"] for h in hits} == set(sources), kind
        for hit in hits:
            source = hit["libraryId"]
            assert (hit["libraryName"], hit["title"]) == (sources[source].name, expected)
            scoped = search(query, library=source, kind=kind)["entries"]
            assert scoped == [{k: v for k, v in hit.items() if k not in ("libraryId", "libraryName")}]
            groups[source, kind] = hit

    for source, folder in sources.items():
        browse = f"/libraries/{source}/music"
        artist_id, album_id, track_id = (groups[source, kind]["id"] for kind in ("artist", "album", "track"))
        assert groups[source, "album"]["artistId"] == artist_id
        albums = api(browse, params={"artist": artist_id})["entries"]
        assert [(entry["id"], entry["name"]) for entry in albums] == [(album_id, album)]
        tracks = api(browse, params={"artist": artist_id, "album": album_id})["entries"]
        assert [track["id"] for track in tracks if track["title"] == title] == [track_id]
        ticket = api(f"/items/{track_id}/playback", method="POST")["url"]
        assert request(ticket, auth=False) == (folder / "shared.flac").read_bytes()

    query = "远岸合奏"
    all_tracks = search(query)["entries"]
    assert [row["title"] for row in all_tracks] == ["Alto", title, title, "Zither"]
    cursor = {}
    for index, expected in enumerate(all_tracks):
        page = search(query, limit=1, **cursor)
        assert page["entries"] == [expected]
        assert bool(page["nextCursor"]) == (index + 1 < len(all_tracks))
        cursor = {"cursor": page["nextCursor"]}
    assert search("missing-search-fixture") == {"entries": [], "nextCursor": None}
    scope = next(iter(sources))
    local = search(query, library=scope, limit=1)["nextCursor"]
    global_cursor = search(query, limit=1)["nextCursor"]
    for kind in ("artist", "album"):
        assert search(query, kind=kind, cursor=global_cursor, status=400)["error"]["code"] == "validation_failed"
    for q, source, cursor in ((query, scope, global_cursor), (query, None, local),
                              ("changed", None, global_cursor), (query, None, "invalid")):
        assert search(q, library=source, cursor=cursor, status=400)["error"]["code"] == "validation_failed"


def benchmark():
    print(f"{2 * (2 + args.benchmark_tracks)} tracks in two libraries")
    for label, query, count in (("common initials", "yjly", min(30, 2 * args.benchmark_tracks)),
                                ("selective initials", f"yjly {args.benchmark_tracks - 1:05}", 2)):
        times = []
        for _ in range(31):
            start = time.perf_counter()
            result = search(query, limit=30)
            times.append((time.perf_counter() - start) * 1000)
            assert len(result["entries"]) == count
        warm = sorted(times[1:])
        print(f"{label}, {len(warm)} warm first-page HTTP queries, P50 {warm[14]:.2f} ms, "
              f"P95 {warm[math.ceil(0.95 * len(warm)) - 1]:.2f} ms")


def check(root):
    denied = api("/music/search", params={"q": title}, auth=False, status=401)
    assert denied["error"]["code"] == "unauthorized"
    assert "globalMusicSearch" in api("/system/info")["features"]
    assert search(" ", status=400)["error"]["code"] == "validation_failed"
    assert search(title) == {"entries": [], "nextCursor": None}
    first, second = root / "West Studio", root / "East Studio"
    with library(first, first.name, "Zither") as west:
        scan(west)
        query = "远岸合奏"
        old = search(query, limit=1)["nextCursor"]
        with library(second, second.name, "Alto") as east:
            assert search(query, cursor=old, status=409)["error"]["code"] == "conflict"
            scan(east)
            check_results({west: first, east: second})
            if args.benchmark_tracks:
                benchmark()
            old = search(query, limit=1)["nextCursor"]
            write_track(second / "new.flac", "源流", second.name)
            scan(east)
            assert search(query, cursor=old, status=409)["error"]["code"] == "conflict"
            assert [h["libraryId"] for h in search("源流")["entries"]] == [east]
            old = search(query, limit=1)["nextCursor"]
        assert search(query, cursor=old, status=409)["error"]["code"] == "conflict"
        assert {r["libraryId"] for r in search(query)["entries"]} == {west}
    assert search(title) == {"entries": [], "nextCursor": None}


with tempfile.TemporaryDirectory(dir=args.media_parent, prefix="music-search-") as fixture:
    root = Path(fixture).resolve()
    root.chmod(0o755)
    check(root)
    print("Cross-library search, source playback, scoped compatibility and cursor invalidation passed")
