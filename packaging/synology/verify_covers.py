"""Exercise the server's album thumbnail command with the packaged media tools."""
import base64
import hashlib
import json
import secrets
import subprocess
from urllib.parse import urlencode


# Same synthetic 64 x 32 stripe pattern, encoded ahead of time so the verifier
# can test a decoder even when the SPK ships no encoder for that input format.
COVERS = {
    "jpeg": ("mjpeg", (
        "/9j/2wBDAAgICAkICQsLCwsLCw0MDQ0NDQ0NDQ0NDQ0ODg4REREODg4NDQ4OEBARERITEhER"
        "ERETExQUFBgYFxccHB0iIin/xABOAAEAAAAAAAAAAAAAAAAAAAAAAQEAAAAAAAAAAAAAAAAAAAAA"
        "EAADAQEAAAAAAAAAAAAAAAAAB0XEhREBAAAAAAAAAAAAAAAAAAAAAP/AABEIACAAQAMBEgACEgAD"
        "EgD/2gAMAwEAAhEDEQA/ADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6O"
        "QADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhd"
        "HIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6OQADzhdHIHnC6O"
        "QADzhdHIHnC6OQAP/Z"
    )),
    "png": ("png", (
        "iVBORw0KGgoAAAANSUhEUgAAAEAAAAAgAQAAAACK5kMpAAAAEElEQVR4nGP4AAUMowxsDAAJ"
        "CfABki5ERgAAAABJRU5ErkJggg=="
    )),
    "webp-lossy": ("webp", (
        "UklGRkQAAABXRUJQVlA4IDgAAACQAwCdASpAACAAPpE0pFIloiOjkgCwEglpAYTP+AAAwJrC"
        "CSWAAPUwg7MfWZO61HhdrKaUkpAAAA=="
    )),
    "webp-lossless": ("webp", "UklGRiYAAABXRUJQVlA4TBkAAAAvP8AHAA8w//M///MfeAAKBCF/6m5E/1MfAA=="),
}


def verify_jpeg(ffprobe, path, data):
    assert data.startswith(b"\xff\xd8\xff") and data.endswith(b"\xff\xd9"), path.name
    path.write_bytes(data)
    probe = subprocess.run([
        ffprobe, "-v", "error", "-f", "image2", "-count_frames",
        "-show_entries", "stream=codec_name,width,height,nb_read_frames", "-of", "json", path,
    ], stdout=subprocess.PIPE, check=True, timeout=5)
    streams = json.loads(probe.stdout)["streams"]
    expected = {"codec_name": "mjpeg", "width": 32, "height": 16, "nb_read_frames": "1"}
    assert streams == [expected], (path.name, streams)


def verify_pixels(path):
    # Decode the package's output independently to detect a valid-sized but corrupted image.
    pixels = subprocess.check_output([
        "ffmpeg", "-nostdin", "-v", "error", "-threads", "1", "-i", path,
        "-frames:v", "1", "-pix_fmt", "gray", "-threads", "1", "-f", "rawvideo", "pipe:1",
    ], timeout=5)
    assert len(pixels) == 32 * 16, path.name
    assert all(value >= 220 if x % 4 < 2 else value <= 35 for x, value in enumerate(pixels)), path.name


def verify_covers(bin_dir, scratch):
    for name, (codec, fixture) in COVERS.items():
        result = subprocess.run([
            bin_dir / "ffmpeg", "-hide_banner", "-loglevel", "error", "-nostdin",
            "-max_alloc", "33554432", "-threads", "1", "-max_pixels", "25000000",
            "-protocol_whitelist", "pipe", "-f", "image2pipe", "-c:v", codec, "-i", "pipe:0",
            "-an", "-frames:v", "1", "-filter_threads", "1",
            "-vf", "scale=32:32:force_original_aspect_ratio=decrease", "-threads", "1",
            "-c:v", "mjpeg", "-q:v", "4", "-f", "image2pipe", "pipe:1",
        ], input=base64.b64decode(fixture, validate=True), capture_output=True, timeout=5)
        if result.returncode:
            raise RuntimeError(f"{name}: {result.stderr.decode(errors='replace')}")
        verify_jpeg(bin_dir / "ffprobe", scratch / f"thumbnail-{name}.jpg", result.stdout)


def verify_cover_api(opener, url, credentials, item, media, scratch, ffprobe):
    salt = secrets.token_hex(12)
    auth = {"u": credentials["username"], "t": hashlib.md5((credentials["password"] + salt).encode()).hexdigest(),
            "s": salt, "v": "1.16.1", "c": "spk-test", "f": "json", "id": item}

    def fetch(**params):
        query = urlencode({**auth, **params})
        with opener.open(f"{url}/rest/getCoverArt.view?{query}", timeout=10) as response:
            return response.headers.get_content_type(), response.read()

    for name, (codec, fixture) in COVERS.items():
        extension = {"mjpeg": "jpg", "png": "png", "webp": "webp"}[codec]
        cover = media / f"cover.{extension}"
        image = base64.b64decode(fixture)
        cover.write_bytes(image)
        try:
            assert fetch() == (f"image/{'jpeg' if codec == 'mjpeg' else codec}", image), name
            mime, data = fetch(size=32)
            assert mime == "image/jpeg", (name, mime)
            response = scratch / f"response-{name}.jpg"
            verify_jpeg(ffprobe, response, data)
            verify_pixels(response)
            verify_pixels(scratch / f"thumbnail-{name}.jpg")
        finally:
            cover.unlink()
