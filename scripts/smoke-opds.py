"""Check OPDS navigation, acquisition and session revocation on a disposable local server."""

import argparse
import base64
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urljoin
from urllib.request import HTTPBasicAuthHandler, Request, build_opener, urlopen
from xml.etree import ElementTree as ET
from zipfile import ZipFile

ATOM = "{http://www.w3.org/2005/Atom}"
ACQUIRE = "http://opds-spec.org/acquisition"


def href(node, relation):
    return node.find(f"{ATOM}link[@rel='{relation}']").attrib["href"]


def run(binary, root):
    with socket.socket() as reserved:
        reserved.bind(("127.0.0.1", 0))
        port = reserved.getsockname()[1]
    origin = f"http://127.0.0.1:{port}"
    server_name = "opds-fixture-" + secrets.token_hex(6)
    env = {k: v for k, v in os.environ.items() if not k.startswith("PALVENI_")}
    server = subprocess.Popen([binary, "serve", "--listen", "127.0.0.1", "--port", str(port),
                               "--data-dir", str(root / "data")], cwd=root,
                              env={**env, "PALVENI_SERVER_NAME": server_name},
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def http(path, auth=None, method="GET", body=None, status=200, headers=None):
        headers = dict(headers or {})
        if auth:
            headers["Authorization"] = auth
        if body is not None:
            body = json.dumps(body).encode()
            headers["Content-Type"] = "application/json"
        url = urljoin(origin, path)
        assert url.startswith(origin + "/")
        try:
            response = urlopen(Request(url, data=body, headers=headers, method=method), timeout=10)
        except HTTPError as error:
            response = error
        with response:
            assert response.status == status, (method, path.split("?")[0], response.status, status)
            return response.headers, response.read()

    def api(path, *args, **kw):
        _, body = http("/api/v1" + path, *args, **kw)
        return json.loads(body) if body else None

    def credentials(bearer):
        issued = api("/me/opds-access", bearer)
        pair = (issued["username"] + ":" + issued["password"]).encode()
        return "Basic " + base64.b64encode(pair).decode()

    try:
        deadline = time.monotonic() + 30
        while server.poll() is None and time.monotonic() < deadline:
            try:
                assert api("/system/info")["name"] == server_name
                break
            except URLError:
                time.sleep(0.1)
        else:
            raise RuntimeError("Temporary server did not start")
        account = {"username": "opds-test", "password": secrets.token_urlsafe(24)}
        api("/setup", method="POST", body=account, status=201)
        owner, other = ["Bearer " + api("/auth/login", method="POST", body={**account, "deviceName": device})["accessToken"]
                        for device in ("reader-a", "reader-b")]
        reader, independent = credentials(owner), credentials(other)
        directory = root / "books"
        nested = directory / "系列"
        nested.mkdir(parents=True)
        epub = nested / "同名 & '样书'.epub"
        with ZipFile(epub, "w") as book:
            book.writestr("mimetype", "application/epub+zip")
            book.writestr("META-INF/container.xml", """<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                <rootfiles><rootfile full-path="book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>""")
            book.writestr("book.opf", """<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="id">
                <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>夜航</dc:title><dc:language>zh</dc:language>
                <dc:identifier id="id">urn:palveni:opds:fixture</dc:identifier></metadata>
                <manifest><item id="p" href="one.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="p"/></spine></package>""")
            book.writestr("one.xhtml", """<html xmlns="http://www.w3.org/1999/xhtml"><head><title>夜航</title></head><body><p>灯火缓缓靠岸。</p></body></html>""")
        (directory / "同名 & '样书'.pdf").write_bytes(b"%PDF-1.4\n%synthetic\n")
        lib = api("/libraries", owner, "POST", {"name": "架 & <一>", "kind": "book", "paths": [str(directory)]}, status=201)
        hidden = root / "photos"
        hidden.mkdir()
        other_lib = api("/libraries", owner, "POST", {"name": "Photos", "kind": "photo", "paths": [str(hidden)]}, status=201)
        job = api(f"/libraries/{lib['id']}/scan", owner, "POST", {}, status=202)["jobId"]
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            state = next((j["state"] for j in api("/jobs", owner)["jobs"] if j["id"] == job), None)
            if state == "succeeded":
                break
            assert state in (None, "queued", "running"), state
            time.sleep(0.1)
        else:
            raise TimeoutError("Book scan timed out")
        assert "Basic" in http("/opds", status=401)[0]["WWW-Authenticate"]
        packed = base64.b64decode(reader.split(" ", 1)[1])
        damaged = packed[:-1] + (b"0" if packed[-1:] != b"0" else b"1")
        http("/opds/", "Basic " + base64.b64encode(damaged).decode(), status=401)
        issued = api("/me/opds-access", owner)
        basic = HTTPBasicAuthHandler()
        basic.add_password("Palveni OPDS", origin + "/opds/", issued["username"], issued["password"])
        challenge_reader = build_opener(basic)
        with challenge_reader.open(origin + "/opds/", timeout=10) as response:
            assert ET.parse(response).getroot().tag == ATOM + "feed"
        http("/opds", owner, status=401)
        api("/auth/me", reader, status=401)
        http("/opds/libraries/" + other_lib["id"], reader, status=404)
        for path in ("/opds", "/opds/"):
            headers, xml = http(path, reader)
            assert "kind=navigation" in headers["Content-Type"]
            nav = ET.fromstring(xml)
            entries = nav.findall(ATOM + "entry")
            assert len(entries) == 1 and entries[0].findtext(ATOM + "title") == lib["name"]
            catalog = urljoin(path, href(entries[0], "subsection"))
            assert catalog == "/opds/libraries/" + lib["id"]
        http(catalog + "?cursor=bad", reader, status=400)
        page_url = catalog + "?limit=1"
        ids, files = [], {}
        for index in range(2):
            headers, xml = http(page_url, reader)
            assert "kind=acquisition" in headers["Content-Type"] and str(directory).encode() not in xml
            page = ET.fromstring(xml)
            assert len(page.findall(ATOM + "entry")) == 1
            entry = page.find(ATOM + "entry")
            assert entry.findtext(ATOM + "title") == epub.stem
            ids.append(entry.findtext(ATOM + "id"))
            link = entry.find(f"{ATOM}link[@rel='{ACQUIRE}']")
            files[link.attrib["type"]] = urljoin(page_url, link.attrib["href"])
            if index == 0:
                page_url = urljoin(page_url, href(page, "next"))
            else:
                assert page.find(f"{ATOM}link[@rel='next']") is None
        assert len(set(ids)) == 2 and set(files) == {"application/epub+zip", "application/pdf"}
        book_url = files["application/epub+zip"]
        headers, content = http(book_url, reader)
        assert content == epub.read_bytes() and headers["Content-Type"] == "application/epub+zip"
        assert headers["Content-Disposition"].endswith("filename*=UTF-8''" + quote(epub.name))
        assert http(book_url, reader, method="HEAD")[0]["Content-Length"] == str(len(content))
        assert http(book_url, reader, headers={"Range": "bytes=1-5"}, status=206)[1] == content[1:6]
        http(book_url, status=401)
        api("/auth/logout", owner, "POST", status=204)
        http("/opds/", reader, status=401)
        http(page_url, reader, status=401)
        http(book_url, reader, status=401)
        assert http(book_url, independent)[1] == content
        print("OPDS passed: two authenticated pages, duplicate titles, UTF-8 EPUB, byte range, independent session and revocation.")
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
    with tempfile.TemporaryDirectory(prefix="palveni-opds-") as temp:
        run(args.binary.resolve(), Path(temp))
