#!/usr/bin/env python3
"""Build an offline DSM 7 package from matching release binaries and skin assets."""
import argparse
import gzip
import hashlib
import io
import json
import re
import struct
import tarfile
import tempfile
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TARGETS = {"x86_64-unknown-linux-musl": ("x86_64", 62), "aarch64-unknown-linux-musl": ("armv8", 183)}
FFMPEG_SHA256 = "7367430190d99cd756470e24d9cb5f64e1f60977d226c3cbcf41c79151e20ab9"
ZLIB_ARCHIVE = "zlib-1.3.2.tar.gz"
ZLIB_SHA256 = "bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16"


def digest(path, algorithm="sha256"):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, algorithm).hexdigest()


def check_elf(path, machine):
    with path.open("rb") as stream:
        header = stream.read(64)
        if (header[:7] != b"\x7fELF\x02\x01\x01" or len(header) != 64
                or struct.unpack_from("<HH", header, 16) not in ((2, machine), (3, machine))):
            raise ValueError(f"{path} is not a target 64-bit Linux executable")
        phoff, = struct.unpack_from("<Q", header, 32)
        phsize, count = struct.unpack_from("<HH", header, 54)
        if phsize != 56 or not 0 < count < 1024:
            raise ValueError(f"{path} has an invalid program header")
        stream.seek(phoff)
        segments = list(struct.iter_unpack("<IIQQQQQQ", stream.read(phsize * count)))
        if len(segments) != count or not any(kind == 1 and flags & 1 for kind, flags, *_ in segments):
            raise ValueError(f"{path} has no executable segment")
        for kind, _, offset, _, _, size, _, _ in segments:
            if kind == 3:
                raise ValueError(f"{path} needs a dynamic interpreter")
            if kind == 2:  # Static PIE can legitimately contain a dynamic table.
                stream.seek(offset)
                for tag, _ in struct.iter_unpack("<qQ", stream.read(size)):
                    if tag == 1:
                        raise ValueError(f"{path} needs a shared library")
                    if tag == 0:
                        break


def add(archive, name, source, mode=0o644):
    info = tarfile.TarInfo(name)
    info.mode = mode
    if isinstance(source, bytes):
        info.size = len(source)
        archive.addfile(info, io.BytesIO(source))
    else:
        info.size = source.stat().st_size
        with source.open("rb") as stream:
            archive.addfile(info, stream)


def pack(args):
    arch, machine = TARGETS[args.target]
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    if args.revision <= 0:
        raise ValueError("package revision must be positive")
    version = f"{version}-{args.revision}"
    binaries = {"palveni": args.bin_dir / "palveni", "palveni-smb-helper": args.bin_dir / "palveni-smb-helper",
                "ffmpeg": args.media_dir / "ffmpeg", "ffprobe": args.media_dir / "ffprobe"}
    for path in binaries.values():
        check_elf(path, machine)
    if digest(args.ffmpeg_source) != FFMPEG_SHA256:
        raise ValueError("FFmpeg source archive differs from the pinned source")
    zlib_source = args.media_dir / ZLIB_ARCHIVE
    if digest(zlib_source) != ZLIB_SHA256:
        raise ValueError("zlib source archive differs from the pinned source")
    with tarfile.open(zlib_source) as sources:
        zlib_license = sources.extractfile("zlib-1.3.2/LICENSE").read()
    registry = json.loads(args.skin_registry.read_text())
    if len(registry) != 3 or {r["id"] for r in registry} != {"native", "ink", "gear"}:
        raise ValueError("expected all three official skins")
    skins = {}
    for row in registry:
        name = row["filename"]
        if not re.fullmatch(r"skin-" + row["id"] + r"-[0-9]+\.[0-9]+\.[0-9]+\.tar", name):
            raise ValueError("invalid skin package name")
        path = args.skin_registry.parent / "skin-packages" / name
        if digest(path) != row["sha256"]:
            raise ValueError(f"{name} differs from the compiled registry")
        skins[name] = path
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output / f"Palveni-{arch}-{version}.spk"
    with tempfile.TemporaryDirectory(dir=args.output) as scratch:
        temp = Path(scratch)
        payload = temp / "package.tgz"
        with payload.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=5) as zipped, tarfile.open(mode="w|", fileobj=zipped) as archive:
            for name, path in binaries.items():
                add(archive, "bin/" + name, path, 0o755)
            for name, path in sorted(skins.items()):
                add(archive, "bin/skin-packages/" + name, path)
            add(archive, "port_conf/Palveni.sc", HERE / "assets/Palveni.sc")
            add(archive, "ui/config", HERE / "assets/ui/config")
            menu_icons = {size: HERE / f"assets/ui/palveni_{size}.png" for size in (16, 24, 32, 48, 72)}
            menu_icons.update({64: HERE / "assets/PACKAGE_ICON.PNG", 256: HERE / "assets/PACKAGE_ICON_256.PNG"})
            for size, path in sorted(menu_icons.items()):
                add(archive, f"ui/images/palveni_{size}.png", path)
            add(archive, "share/licenses/Palveni-MPL-2.0", ROOT / "LICENSE")
            add(archive, "share/licenses/FFmpeg-LGPL-2.1", args.media_dir / "COPYING.LGPLv2.1")
            add(archive, "share/licenses/zlib", zlib_license)
            add(archive, "share/src/jellyfin-ffmpeg.tar.gz", args.ffmpeg_source)
            add(archive, "share/src/" + ZLIB_ARCHIVE, zlib_source)
            add(archive, "share/src/build-ffmpeg.sh", HERE / "build-ffmpeg.sh", 0o755)
        fields = {"package": "Palveni", "version": version, "displayname": "Palveni",
                  "description": "Self-hosted media server for music, films and TV.",
                  "maintainer": "palveni-labs", "arch": arch, "os_min_ver": "7.0-41890",
                  "adminprotocol": "http", "adminport": "7258", "adminurl": "/",
                  "dsmuidir": "ui", "dsmappname": "com.palveni.Server",
                  "checkport": "yes", "startable": "yes", "checksum": digest(payload, "md5")}
        with tarfile.open(temp / output.name, "w", format=tarfile.USTAR_FORMAT) as spk:
            add(spk, "INFO", "".join(f'{k}="{v}"\n' for k, v in fields.items()).encode())
            add(spk, "package.tgz", payload)
            for name in ("preinst", "postinst", "preuninst", "postuninst", "preupgrade", "postupgrade"):
                add(spk, "scripts/" + name, b"#!/bin/sh\nexit 0\n", 0o755)
            add(spk, "scripts/start-stop-status", HERE / "scripts/start-stop-status", 0o755)
            for name in ("privilege", "resource"):
                add(spk, "conf/" + name, HERE / "conf" / name)
            for name in ("PACKAGE_ICON.PNG", "PACKAGE_ICON_256.PNG"):
                add(spk, name, HERE / "assets" / name)
        (temp / output.name).replace(output)
    print(f"{digest(output)}  {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    for arg in ("bin-dir", "media-dir", "skin-registry", "ffmpeg-source", "output"):
        parser.add_argument("--" + arg, required=True, type=Path)
    parser.add_argument("--revision", required=True, type=int)
    try:
        pack(parser.parse_args())
    except (ValueError, OSError, KeyError, struct.error) as error:
        parser.exit(1, f"Cannot build package: {error}\n")
