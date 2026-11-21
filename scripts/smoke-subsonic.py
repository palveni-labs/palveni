"""Check the music client protocol against the smoke script's disposable library."""

import hashlib
import json
import os
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import Request, urlopen

base, username, track, non_track, media_file = sys.argv[1:]
bearer = os.environ["PALVENI_SMOKE_BEARER"]
original = Path(media_file).read_bytes()


def manage(method, *, personal=False):
    request = Request(base + "/api/v1/me/subsonic-password" + ("?personal=true" if personal else ""), method=method,
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
    request = Request(base + "/rest/" + method + "?" + urlencode(query, doseq=True), method=verb, headers=headers or {})
    with urlopen(request, timeout=15) as response:
        if binary:
            return response.status, response.headers, response.read()
        return json.load(response)["subsonic-response"]


def check():
    assert manage("GET") == {"enabled": False, "personal": False}
    issued = manage("POST")
    assert issued["username"] == username
    creds = auth(issued["password"])
    assert manage("GET") == {"enabled": True, "personal": False}
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
    cover = Path(media_file).with_name("Cover.PNG").read_bytes()
    for cover_id in (song["coverArt"], detail["coverArt"]):
        status, hdr, binary = call("getCoverArt.view", creds, params={"id": cover_id}, binary=True)
        assert status == 200 and hdr["Content-Type"] == "image/png" and binary == cover
    assert call("getCoverArt", creds, params={"id": non_track})["error"]["code"] == 70
    assert call("getCoverArt", creds, params={"id": song["coverArt"], "size": "0"})["error"]["code"] == 10
    _, header, head_body = call("getCoverArt.view", creds, params={"id": song["coverArt"]}, verb="HEAD", binary=True)
    assert int(header["Content-Length"]) == len(cover) and head_body == b""
    assert call("getStarred2", creds)["error"]["code"] == 50
    assert call("createPlaylist", creds, params={"name":"Denied"})["error"]["code"] == 50
    assert call("star", creds, params={"id":track})["error"]["code"] == 50
    assert call("getOpenSubsonicExtensions", creds)["openSubsonicExtensions"] == [{"name":"songLyrics","versions":[1]}]
    words = call("getLyricsBySongId", creds, params={"id":track})["lyricsList"]["structuredLyrics"][0]
    assert not words["synced"] and words["line"][0]["value"] == "Embedded test line"
    next_creds = auth(manage("POST", personal=True)["password"])
    assert manage("GET") == {"enabled": True,"personal":True}
    def personal(method, **params):
        return call(method, next_creds, params=params)
    assert personal("star", id=track)["status"] == "ok"
    assert track in [s["id"] for s in personal("getStarred2")["starred2"]["song"]]
    assert personal("unstar",id=track)["status"]=="ok"
    assert track not in [s["id"] for s in personal("getStarred2")["starred2"]["song"]]
    collection = personal("getAlbum", id=personal("getSong", id=track)["song"]["albumId"])["album"]
    for field, key, object_id in (("albumId", "album", collection["id"]), ("artistId", "artist", collection["artistId"])):
        for _ in range(2):
            assert personal("star", **{field: object_id})["status"] == "ok"
        saved = personal("getStarred2", musicFolderId=folder)["starred2"][key]
        assert [item["id"] for item in saved] == [object_id] and saved[0]["name"]
        assert personal("getStarred2", musicFolderId=-1)["starred2"][key] == []
        assert personal("unstar", **{field: object_id})["status"] == "ok"
        assert personal("getStarred2")["starred2"][key] == []
    assert personal("setRating",id=track,rating=4)["status"]=="ok"
    created=personal("createPlaylist",name="Practice",songId=[track,track])["playlist"]
    assert created["songCount"]==2 and len(created["entry"])==2
    pid=created["id"]
    assert personal("updatePlaylist",playlistId=pid,name="Practice set",comment="Working notes",songIndexToRemove=0,songIdToAdd=track)["status"]=="ok"
    detail=personal("getPlaylist",id=pid)["playlist"]
    assert detail["name"]=="Practice set" and detail["comment"]=="Working notes" and len(detail["entry"])==2
    assert any(p["id"]==pid for p in personal("getPlaylists")["playlists"]["playlist"])
    assert personal("deletePlaylist",id=pid)["status"]=="ok"
    assert personal("getPlaylist",id=pid)["error"]["code"]==70
    before=personal("getSong",id=track)["song"].get("playCount",0)
    at=int(time.time()*1000)-1500
    assert personal("scrobble",id=track,time=at,submission="false")["status"]=="ok"
    assert personal("getSong",id=track)["song"]["playCount"]==before
    for _ in range(2):
        assert personal("scrobble",id=track,time=at,submission="true")["status"]=="ok"
    song=personal("getSong",id=track)["song"]
    assert song["playCount"]==before+1 and song["userRating"]==4 and song["played"]
    assert call("getCoverArt", next_creds, params={"id": song["coverArt"]}, binary=True)[2] == cover
    index=personal("search3",query=song["title"])["searchResult3"]["song"]
    assert any(s["id"]==track and s["playCount"]==before+1 for s in index)

    assert call("ping", creds)["error"]["code"] == 40
    assert call("stream", creds, params={"id": track}, headers={"Range": "bytes=0-1"})["error"]["code"] == 40
    assert call("ping", next_creds)["status"] == "ok"
    manage("DELETE")
    assert manage("GET") == {"enabled": False, "personal": False} and call("ping", next_creds)["error"]["code"] == 40


if __name__ == "__main__":
    check()
