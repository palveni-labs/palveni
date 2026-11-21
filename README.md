# Palveni Server

Palveni is a self-hosted media library server with a web dashboard. Add local media folders, scan and browse their contents, monitor background jobs, and check which hardware encoders work on your machine. SQLite is the default database. PostgreSQL and MySQL are also supported.

## Run with Docker

Build the image from the repository root. Replace `/path/to/media` with a directory containing your own media.

```sh
docker build -f packaging/docker/Dockerfile -t palveni-server .
docker run -d --name palveni \
  -p 127.0.0.1:7258:7258 \
  -v palveni-data:/data \
  -v /path/to/media:/media:ro \
  palveni-server
```

Open the [dashboard](http://localhost:7258) and create an administrator account. Under **System → Libraries**, add `/media` and start a scan. The data volume stores accounts, the media index, device keys and installed skins. For a LAN deployment or hardware acceleration, see the [Compose example](packaging/docker/compose.yaml).

## Build from source

Install Node.js 24 and the [pinned Rust toolchain](rust-toolchain.toml). Install FFmpeg and FFprobe for media diagnostics. The Docker image includes both tools.

```sh
npm ci --prefix web
npm run build --prefix web
cargo run --locked -p palveni-server --features embed-web -- serve
```

Run `palveni doctor` to check FFmpeg, hardware acceleration and database connectivity. The running server publishes its OpenAPI specification at `/api/v1/openapi.json`. Use `palveni openapi` to export it offline.

## Browse a music library

Choose **Music** when adding a library, then scan its folders. Open the library and choose **Album artists** to browse albums and tracks. Compilation albums stay together under their album artist. Tracks without tags appear under an unknown artist or album and use their filenames as titles. Run a scan after upgrading to index music that is already in your library.

Clients can play a scanned track in its original format through the native API. Send an authenticated `GET /api/v1/items/{id}/stream` to read the file or seek with an HTTP byte range. For a player that opens URLs, send an authenticated `POST /api/v1/items/{id}/playback` and resolve the returned `url` against the server address. The link is valid for 15 minutes and can be renewed for later seeks.

Signed-in clients can favorite and rate tracks and videos, browse recently played items, and continue from the position saved on another device. Each account has its own history and ordered music playlists. The native API exposes these features under `/api/v1/me/items` and `/api/v1/me/playlists`. Playback updates and playlist edits use a version so one device can detect changes made on another. To display a playlist, use `GET /api/v1/me/playlists/{id}/tracks?version=N` to load titles, artists, albums and durations in order.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/music-library-dark.png">
  <img src="docs/images/music-library-light.png" alt="Six tracks from Harbor Sessions, with the album artist, disc number and duration shown in the music library.">
</picture>

## Skins and plugins

Install an official skin under **Apps → Appearance**, or upload a community skin as a tar archive. Palveni validates the archive before applying it. Find official packages and contribution guidelines in the [plugins repository](https://github.com/palveni-labs/plugins).

## License

The server and dashboard use the [Mozilla Public License 2.0](LICENSE). Plugin interfaces in `wit/` use the [Apache License 2.0](wit/LICENSE). Bundled fonts retain their respective licenses.
