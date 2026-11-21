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

## Skins and plugins

Install an official skin under **Apps → Appearance**, or upload a community skin as a tar archive. Palveni validates the archive before applying it. Find official packages and contribution guidelines in the [plugins repository](https://github.com/palveni-labs/plugins).

## License

The server and dashboard use the [Mozilla Public License 2.0](LICENSE). Plugin interfaces in `wit/` use the [Apache License 2.0](wit/LICENSE). Bundled fonts retain their respective licenses.
