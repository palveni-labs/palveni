# Synology DSM package

Install the SPK for your NAS architecture in **Package Center → Manual Install**, then open **Palveni**. The dashboard listens on TCP 7258. First launch creates a separate Palveni administrator account. Give the **Palveni** system internal user read permission on just the media shares you want to index, then add their absolute folder paths under **System → Libraries**.

The package targets DSM 7.0-41890 or later. The x86_64 package contains x86-64 musl programs. The armv8 package requires AArch64 musl programs. Check the model in Synology's [architecture table](https://help.synology.com/developer-guide/appendix/platarchs.html).

The dashboard, three official skins, SMB client, FFmpeg and FFprobe are bundled for offline installation. This portable FFmpeg build supports local probing, decoding, remuxing and selected software encoders including AAC, FLAC and MPEG-4 Part 2. Encoder availability is shown in the dashboard. Hardware drivers and external codecs need a separate compatible FFmpeg build. The package contains the corresponding Jellyfin FFmpeg source, build script and license under `share/`.

DSM runs the service as the unprivileged package account. Accounts, device identity, installed skins, settings and indexes live in `/var/packages/Palveni/var/data`. Upgrades replace the application in `target` and retain that data directory. Stop the package before taking a filesystem copy of its data. The service writes its current startup log to `/var/packages/Palveni/tmp/service/server.log`. The package registers its firewall service without requesting automatic port forwarding. [DSM filesystem layout](https://help.synology.com/developer-guide/integrate_dsm/fhs.html) · [Package privileges](https://help.synology.com/developer-guide/privilege/preface.html)

## Build

Use the pinned Rust toolchain, Node 24 and Python 3.11 or newer. Build on Linux with a C compiler and linker for the target musl ABI, a native host C compiler, GNU make and tar. The commands below run from the repository root. For AArch64 use `aarch64-unknown-linux-musl`, set the Cargo C compiler and linker for that target, and pass `aarch64` to the media build.

```sh
target_arch=x86_64-unknown-linux-musl
musl_cc=musl-gcc
export CC_x86_64_unknown_linux_musl="$musl_cc"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$musl_cc"
export CARGO_TARGET_DIR="$PWD/target"
out="$PWD/target/spk"
mkdir -p "$out"
rustup target add "$target_arch"

npm ci --prefix web
npm run build --prefix web
cargo build --locked --release --target "$target_arch" -p palveni-server --features embed-web \
  --message-format=json-render-diagnostics > "$out/build.jsonl"
cargo build --locked --release --target "$target_arch" --manifest-path packaging/smb-helper/Cargo.toml
registry=$(python3 - "$out/build.jsonl" <<'PYTHON'
import json, sys
from pathlib import Path
rows = (json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines())
paths = [Path(row["out_dir"]) / "skin-registry.json" for row in rows
         if row.get("reason") == "build-script-executed" and "palveni-plugin-host" in row["package_id"]]
assert len(paths) == 1 and paths[0].is_file()
print(paths[0])
PYTHON
)

src="$out/jellyfin-ffmpeg.tar.gz"
curl --fail --location --output "$src" \
  https://codeload.github.com/jellyfin/jellyfin-ffmpeg/tar.gz/7250de8375a4959ac1c25c139ce9789fdac199fb
script="$PWD/packaging/synology/build-ffmpeg.sh"
mkdir "$out/ffmpeg-build"
(cd "$out/ffmpeg-build" && CC="$musl_cc" HOST_CC=cc JOBS=2 sh "$script" "$src" x86_64)

python3 packaging/synology/build.py --target "$target_arch" --revision 1 \
  --bin-dir "$CARGO_TARGET_DIR/$target_arch/release" --skin-registry "$registry" \
  --ffmpeg-source "$src" --media-dir "$out/ffmpeg-build/media-tools" --output "$out/dist"
```

The media build verifies the archive digest of the pinned Jellyfin 8.1.3-2 source. Packaging checks all four ELF binaries for the requested architecture and static linkage and verifies each official skin tar against its build-time registry. Increase the numeric revision for each new package release. The builder prints the SHA-256 of the resulting SPK. [Jellyfin FFmpeg source](https://github.com/jellyfin/jellyfin-ffmpeg/tree/7250de8375a4959ac1c25c139ce9789fdac199fb) · [SPK format](https://help.synology.com/developer-guide/synology_package/introduction.html)

## Verify the payload on Linux

Run as a regular user on the matching CPU architecture with Python 3.12 or newer, curl and flock. The verifier chooses a loopback port and works in a new temporary directory. It exercises the extracted package, preserves account data and installed skins across replacement of the program tree, and checks that a stale process file cannot stop an unrelated process.

```sh
python3 packaging/synology/verify-linux.py target/spk/dist/Palveni-x86_64-0.1.0-1.spk
```

Package Center installation, share ACLs and playback from a real DSM media share require an additional check on the target NAS.
