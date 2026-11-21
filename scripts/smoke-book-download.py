"""Check book attachments on a disposable local server. Set PALVENI_SMOKE_BEARER to its admin token."""

import argparse
import json
import os
import tempfile
import time
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen
from zipfile import ZipFile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("base", help="server origin")
parser.add_argument("media_parent", type=Path, help="temporary directory readable by the server")
args = parser.parse_args()
token = os.environ["PALVENI_SMOKE_BEARER"]


def request(path, method="GET", payload=None, *, expected=200, signed=False, headers=None):
    headers = dict(headers or {})
    if not signed:
        headers["Authorization"] = "Bearer " + token
    body = None
    if payload is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(payload).encode()
    try:
        response = urlopen(Request(args.base + path, data=body, headers=headers, method=method), timeout=30)
    except HTTPError as error:
        response = error
    with response:
        assert response.status == expected, (method, path.split("?")[0], response.status, expected)
        return response.headers, response.read()


def api(path, *parameters, **options):
    _, body = request("/api/v1" + path, *parameters, **options)
    return json.loads(body) if body else None


def scan(library):
    job_id = api(f"/libraries/{library}/scan", "POST", expected=202)["jobId"]
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        jobs = api("/jobs?limit=50")["jobs"]
        state = next((j["state"] for j in jobs if j["id"] == job_id), None)
        if state == "succeeded":
            return
        assert state in (None, "running", "queued"), state
        time.sleep(0.1)
    raise TimeoutError("Book scan timed out")


def fixture(path):
    with ZipFile(path, "w") as book:
        book.writestr("mimetype", "application/epub+zip")
        book.writestr("META-INF/container.xml", """<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
            <rootfiles><rootfile full-path="book.opf" media-type="application/oebps-package+xml"/></rootfiles>
        </container>""")
        book.writestr("book.opf", """<package version="3.0" unique-identifier="id" xmlns="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:identifier id="id">urn:palveni:sample:wind</dc:identifier>
                <dc:title>风之图</dc:title><dc:language>zh</dc:language>
                <meta property="dcterms:modified">2026-11-21T00:00:00Z</meta>
            </metadata>
            <manifest>
                <item id="p" href="one.xhtml" media-type="application/xhtml+xml"/>
                <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
            </manifest>
            <spine><itemref idref="p"/></spine>
        </package>""")
        book.writestr("nav.xhtml", """<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
            <head><title>目录</title></head>
            <body><nav epub:type="toc"><ol><li><a href="one.xhtml">第一章</a></li></ol></nav></body>
        </html>""")
        book.writestr("one.xhtml", """<html xmlns="http://www.w3.org/1999/xhtml">
            <head><title>风之图</title></head>
            <body><p>清晨，风掀起地图的一角。</p></body>
        </html>""")


def check(library, file, empty):
    scan(library)
    items = {i["title"]: i["id"] for i in api(f"/items?libraryId={library}")["items"]}
    path = f"/items/{items[file.stem]}/book-download"
    empty_path = f"/items/{items[empty.stem]}/book-download"
    api(path, "POST", expected=401, signed=True)
    api(path, expected=401, signed=True)
    data = file.read_bytes()
    issued = api(path, "POST")
    url = issued["url"]
    assert url.startswith("/api/v1" + path + "?ticket=") and issued["expiresAt"]
    headers, body = request(url, signed=True)
    assert body == data
    assert headers["Content-Type"] == "application/octet-stream"
    assert headers["Cache-Control"] == "private, no-store"
    assert headers["X-Content-Type-Options"] == "nosniff"
    assert headers["Content-Disposition"].endswith("filename*=UTF-8''" + quote(file.name))
    headers, body = request(url, "HEAD", signed=True, headers={"Range": "bytes=1-6"})
    assert not body and int(headers["Content-Length"]) == len(data)
    headers, body = request("/api/v1" + path, headers={"Range": "bytes=5-11"}, expected=206)
    assert body == data[5:12] and headers["Content-Range"] == f"bytes 5-11/{len(data)}"
    headers, body = request(url, expected=416, signed=True, headers={"Range": f"bytes={len(data)}-"})
    assert not body and headers["Content-Range"] == f"bytes */{len(data)}"
    assert request("/api/v1" + empty_path)[1] == b""
    assert api(empty_path + "?" + url.split("?", 1)[1], expected=401, signed=True)["error"]["code"] == "unauthorized"
    assert api(f"/items/{items['Shelf']}/book-download", expected=404)["error"]["code"] == "not_found"
    file.unlink()
    assert api(path, "POST", expected=404)["error"]["code"] == "not_found"
    request(url, signed=True, expected=404)
    return empty_path


with tempfile.TemporaryDirectory(prefix="book-download-", dir=args.media_parent) as tmp:
    root = Path(tmp).resolve()
    root.chmod(0o755)
    file, empty = root / "风之图 1%.epub", root / "Cover.pdf"
    fixture(file)
    empty.touch()
    shelf = root / "Shelf"
    shelf.mkdir()
    (shelf / "Index.mobi").write_bytes(b"Indexed")
    library = api("/libraries", "POST", {"kind": "book", "name": root.name, "paths": [str(root)]}, expected=201)["id"]
    try:
        path = check(library, file, empty)
    finally:
        api("/libraries/" + library, "DELETE", expected=204)
    assert api(path, expected=404)["error"]["code"] == "not_found"

print("Book download, attachment name, byte ranges, ticket scope and removed source passed")
