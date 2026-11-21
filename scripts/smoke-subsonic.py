"""Check the music client protocol against the smoke script's disposable library."""

import hashlib
import json
import os
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import Request, urlopen

base, username, track, non_track, media_file = sys.argv[1:]
bearer = os.environ["PALVENI_SMOKE_BEARER"]
original = Path(media_file).read_bytes()


def manage(method):
    request = Request(base + "/api/v1/me/subsonic-password", method=method,
                      headers={"Authorization": "Bearer " + bearer})
    with urlopen(request, timeout=15) as response:
        if response.status == 204:
            return None
        assert "no-store" in response.headers["Cache-Control"]
        return json.load(response)


def auth(secret):
    salt = os.urandom(12).hex()
    return {"u": username, "t": hashlib.md5((secret + salt).encode()).hexdigest(),
            "s": salt, "v": "1.16.1", "c": "music-client", "f": "json"}


def call(method, creds, *, params=None, headers=None, verb="GET", binary=False):
    query = dict(creds)
    query.update(params or {})
    request = Request(base + "/rest/" + method + "?" + urlencode(query), method=verb, headers=headers or {})
    with urlopen(request, timeout=15) as response:
        if binary:
            return response.status, response.headers, response.read()
        return json.load(response)["subsonic-response"]


def check():
    assert manage("GET") == {"enabled": False}
    issued = manage("POST")
    assert issued["username"] == username
    creds = auth(issued["password"])
    assert manage("GET") == {"enabled": True}
    assert call("ping.view", creds)["status"] == "ok"
    assert call("ping", {**creds, "t": "0" * 32})["error"]["code"] == 40
    assert call("ping", {"u": username, "p": issued["password"], "c": "test", "v": "1.16.1", "f": "json"})["error"]["code"] == 42
    assert call("ping", {**creds, "p": "unsupported"})["error"]["code"] == 43
    folders = call("getMusicFolders.view", creds)["musicFolders"]["musicFolder"]
    assert len(folders) == 1 and isinstance(folders[0]["id"], int)
    progress=call("getScanStatus",creds)["scanStatus"]
    assert not progress["scanning"] and progress["count"] >= 3 and progress["lastScan"]
    assert call("startScan",creds)["error"]["code"] == 50
    folder = folders[0]["id"]
    xml_creds = {k: v for k, v in creds.items() if k != "f"}
    _, _, xml = call("getMusicFolders.view", xml_creds, binary=True)
    xml_folder = ET.fromstring(xml).find(".//{http://subsonic.org/restapi}musicFolder")
    assert xml_folder.attrib == {"name": folders[0]["name"], "id": str(folder)}
    params = {"query": "", "albumCount": 0, "artistCount": 0, "songCount": 2, "musicFolderId": folder}
    first = call("search3.view", creds, params=params)["searchResult3"]["song"]
    more = call("search3.view", creds, params={**params, "songOffset": 2})["searchResult3"]["song"]
    assert len(first) == 2 and more and not ({s["id"] for s in first} & {s["id"] for s in more})
    hidden = call("search3", creds, params={**params, "musicFolderId": -1})["searchResult3"]
    assert hidden["song"] == []
    by_title = call("search3", creds, params={"query": "Coda"})["searchResult3"]["song"]
    assert any(song["title"] == "Coda" for song in by_title)
    song = call("getSong.view", creds, params={"id": track})["song"]
    assert song["id"] == track and song["contentType"] == "audio/flac" and "path" not in song
    detail = call("getAlbum.view", creds, params={"id": song["albumId"]})["album"]
    assert detail["songCount"] == len(detail["song"]) and any(s["id"] == track for s in detail["song"])
    albums = call("getAlbumList2.view", creds, params={"type": "alphabeticalByName", "musicFolderId": folder})["albumList2"]["album"]
    assert any(album["id"] == detail["id"] for album in albums)
    assert call("getSong", creds, params={"id": non_track})["error"]["code"] == 70
    assert call("stream.view", creds, params={"id": non_track})["error"]["code"] == 70
    status, hdr, data = call("stream.view", creds, params={"id": track, "format": "raw"},
                             headers={"Range": "bytes=1-3"}, binary=True)
    assert status == 206 and data == original[1:4] and hdr["Content-Range"] == f"bytes 1-3/{len(original)}"
    assert call("download.view", creds, params={"id": track}, binary=True)[2] == original
    assert call("stream.view", creds, params={"id": track}, verb="HEAD", binary=True)[1]["Content-Length"] == str(len(original))
    assert call("stream.view", creds, params={"id": track, "format": "mp3"})["status"] == "failed"
    next_creds = auth(manage("POST")["password"])
    assert call("ping", creds)["error"]["code"] == 40
    assert call("stream", creds, params={"id": track}, headers={"Range": "bytes=0-1"})["error"]["code"] == 40
    assert call("ping", next_creds)["status"] == "ok"
    manage("DELETE")
    assert manage("GET") == {"enabled": False} and call("ping", next_creds)["error"]["code"] == 40


if __name__ == "__main__":
    check()
