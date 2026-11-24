# Palveni Server

Palveni is a self-hosted home media and smart-home hub for music, films, TV, photos, books, comics, audiobooks and podcasts. Its web dashboard brings together media libraries, smart-home controls, an approval-gated AI assistant, multi-user access control, sharing, cross-media collections, download automation and observability in a single binary. SQLite is the default database; PostgreSQL and MySQL are also supported.

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

Open the [dashboard](http://localhost:7258) and create an administrator account. Under **System → Libraries**, add `/media` and start a scan. The data volume stores accounts, the media index, device keys, backups and installed plugins. For a LAN deployment or hardware acceleration, see the [Compose example](packaging/docker/compose.yaml).

If a previously populated source becomes empty or loses most of its files, its missing entries stay in the library. Open the library as an administrator, choose **Review missing files**, and inspect the paths before confirming. The server checks the source again before removing entries from the index.

## Build from source

Install Node.js 24 and the [pinned Rust toolchain](rust-toolchain.toml). Install FFmpeg and FFprobe for media diagnostics and transcoding. The Docker image includes both tools. At startup, Palveni checks each encoder with a short test clip and limits the threads used by each probe.

```sh
npm ci --prefix web
npm run build --prefix web
cargo build --locked --manifest-path packaging/smb-helper/Cargo.toml --target-dir target
cargo run --locked -p palveni-server --features embed-web -- serve
```

Run `palveni doctor` to check FFmpeg, hardware acceleration and database connectivity. The running server publishes its OpenAPI specification at `/api/v1/openapi.json`. Use `palveni openapi` to export it offline.

## Pair Link devices

To accept direct Link connections, build with the `link` feature and start the server with `--link-bind <IP:UDP_PORT>` on a reachable interface. A paired client remembers the server’s public key and can reconnect after a restart. Under **System → Settings → Account → Paired Link devices**, you can see which devices are connected and revoke a device to close its connections and prevent it from reconnecting.

## Browse a music library

Choose **Music** when adding a library, then scan its folders. Open the library and choose **Album artists** to browse albums and tracks. Compilation albums stay together under their album artist. Tracks without tags appear under an unknown artist or album and use their filenames as titles.

To search across libraries, collections and smart-home devices, open the global command palette with ⌘K or Ctrl+K. Chinese names also match simplified and traditional characters, pinyin and initials.

To use an OpenSubsonic player, open **System → Settings → Account → OpenSubsonic music clients** and create a password. Enter the server’s base address, your username and this password in the player. The music catalog supports browsing, searching and streaming original files with seeking, personal favorites, ratings, playlists, listening history and synced/plain lyrics. Use HTTPS when connecting over the internet.

After the first scan, Palveni watches each library for file additions, edits, renames and deletions. Libraries are also checked on startup and periodically, including network mounts that do not deliver filesystem notifications.

Clients can play a scanned track in its original format via `GET /api/v1/items/{id}/stream` (with HTTP byte ranges) or short-lived signed links from `POST /api/v1/items/{id}/playback`. When needed, Palveni transcodes on the fly to MP3, AAC or Opus at 32–320 kbit/s, splits UTF-8 CUE sheets into individual playable tracks, runs background loudness analysis (LUFS / LU / dBTP) and serves `.lrc` / embedded lyrics.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/music-library-dark.png">
  <img src="docs/images/music-library-light.png" alt="Six tracks from Harbor Sessions, with the album artist, disc number and duration shown in the music library.">
</picture>

## Films, TV, virtual channels and spoken audio

Add a **Video** library and scan its folders to browse a poster wall with movie/series grouping, NFO metadata, pinyin initials, cast rails, multi-version selection and `.strm` remote stream files. The web player automatically chooses direct play, remux or HLS transcoding (including HEVC 10-bit handling and MSE HLS fallback for Chrome), supports external SRT/WebVTT and embedded text subtitles, chapter navigation, and intro/credits skip markers (inferred from chapters or saved manually).

Administrators can configure online metadata providers (TMDB, Bangumi and OpenLibrary) under **System → Maintenance** to search and save non-destructive metadata overrides directly from video and reading pages. Compatible Jellyfin clients can connect via the built-in Jellyfin API subset using dedicated API keys created in Settings.

Under **Radio & subscriptions**, you can also compose **24-hour virtual TV channels** from your local video libraries with deterministic schedules and standard M3U/XMLTV output, alongside radio/IPTV playlists and RSS/Atom podcast feeds. **Audiobooks** and **Podcasts** libraries support playback speed, sleep timers, M4B embedded chapters and cross-device resume.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/video-library-dark.png">
  <img src="docs/images/video-library-light.png" alt="A poster wall with seven fictional films and a two-episode series in the video library.">
</picture>

## Photos and local photo AI

Add a **Photos** library to browse a date-grouped timeline, EXIF metadata inspector, places map, custom albums, favorites and "On This Day" memories, with slideshow lightbox playback and browser uploads. On-device perceptual hashing (`dHash`) and local tagging power bilingual photo search, people grouping and duplicate/similar-photo cleanup without sending images to external services.

## Books, comics, OPDS and WebDAV

Add **Books** or **Comics** libraries to read EPUB, PDF and CBZ files directly in the browser with customizable typography, manga RTL/spread/webtoon modes, bookmarks and synced reading progress. Enable OPDS reader credentials in Account settings for external ebook/comic apps, or mount libraries read-only over RFC 4918 WebDAV at `/webdav`.

## Multi-user access, sharing and cross-media collections

Administrators manage family accounts under **System → Users** with per-library allowlists, download/transcode toggles, daily AI usage quotas, password resets and active-session visibility.

Members can create **cross-media collections** combining films, soundtracks, books, comics and photos—with automatic related-work discovery across libraries—and generate **public or password-protected share links** (`/s/{token}`) with expiration, view limits, optional downloads and automatic EXIF/GPS stripping on shared photos.

## Smart home, AI assistant and MCP

Under **Smart Home**, connect Home Assistant bridges or pure-Rust MQTT 3.1.1 brokers to monitor and control lights, switches, thermostats, sensors and media players, activate multi-device scenes, bind playback-driven lighting automations, and sync ambient lamp colors to playing album artwork.

The built-in **AI Visual Assistant** works both offline (via a deterministic local planner) and with user-configured OpenAI-compatible or Anthropic-compatible models. It renders structured interactive UI blocks, enforces server-side confirmation gates on write actions, applies configurable cloud path redaction and data-egress policies, and respects per-user daily quotas. External AI tools (such as Claude, Cursor or OpenClaw) can connect via the standard **Model Context Protocol** endpoint at `/api/v1/mcp` using the same permissions and audit trail.

## Downloads, media organizer and notifications

Under **Tasks**, bridge external **qBittorrent**, **Transmission** or **aria2** clients to monitor live transfers, submit Magnet/HTTP links, monitor RSS/Atom media subscriptions with quality rules and one-click downloader submission, and run the **Media Organizer** for dry-run template renaming or hardlinking with 1-click batch undo.

Under **Notifications & routines**, configure outbound push channels (Webhook, Telegram, WeCom, Bark), manage inbound webhook tokens for Sonarr/Radarr/MoviePilot, and schedule recurring automation routines.

## Sync, remote cast, observability and maintenance

Clients can consume incremental state changes via `/api/v1/sync/changes`, report active playback sessions and remotely control playback across devices from the topbar cast popover. Under **System → Observability & Maintenance**, inspect Prometheus metrics (`/api/v1/metrics`), live server logs, health diagnostics (`doctor`), audit logs, one-click SQLite backups, and the migration importer for Emby, Jellyfin and Navidrome libraries.

## Network sources

To index media on a NAS, mount its share on the host and expose it read-only to the container. For on-demand browsing, open **Network sources** and add an SMB3 or WebDAV directory, or connect a Jellyfin, Emby or Plex server using its access token. Credentials are encrypted with the installation’s node key. For a binary installation, place `palveni-smb-helper` beside `palveni`. The Docker image includes both.

## Skins and plugins

The default **Work Colour** skin pairs frosted-glass surfaces and cover-derived media accents with a 4-group sidebar on desktop and bottom navigation on phones. Under **System → Settings**, you can reposition the navigation bar, reorder entries and choose light, dark or system mode.

Open **Apps → Plugin store** to browse official skins, declarative metadata/lyrics/subtitle recipes and overview dashboard widgets (such as the live clock/storage badge and daily quote card). Recipe and widget plugins enforce explicit permission review before installation, schema-validated configuration drawers and one-click version rollback. Find official skin packages and contribution guidelines in the [plugins repository](https://github.com/palveni-labs/plugins).

## License

The server and dashboard use the [Mozilla Public License 2.0](LICENSE). Plugin interfaces in `wit/` use the [Apache License 2.0](wit/LICENSE). Bundled fonts retain their respective licenses.
