mod auth;
mod personal;
mod wire;

use super::playback::{open_audio, playback_key, serve_track};
use crate::AppState;
pub use auth::{SubsonicPassword, SubsonicPasswordStatus, issue, revoke, status};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, Method},
    response::{IntoResponse, Response},
};
use palveni_core::{ItemId, clock};
use palveni_db::music::catalog::{AlbumKey, CatalogAlbum, CatalogArtist, CatalogSong, Page};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::Path as FilePath;
use wire::{DENIED, Failure, MISSING, NOT_FOUND, UNSUPPORTED};

struct Params(Vec<(String, String)>);

impl Params {
    fn value(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    fn required(&self, key: &str) -> Result<&str, Failure> {
        self.value(key).filter(|x| !x.is_empty()).ok_or(MISSING)
    }
    fn page(&self, count: &str, offset: &str, default: u64, max: u64) -> Result<Page, Failure> {
        let parse = |key, default| {
            self.value(key)
                .map_or(Ok(default), |s| s.parse::<u64>().map_err(|_| MISSING))
        };
        Ok(Page {
            count: parse(count, default)?.min(max),
            offset: parse(offset, 0)?.min(i64::MAX as u64),
        })
    }
    fn scope(&self) -> Result<Vec<i32>, Failure> {
        self.0
            .iter()
            .filter(|(k, _)| k == "musicFolderId")
            .map(|(_, v)| v.parse().map_err(|_| MISSING))
            .collect()
    }
}

fn album_id(folder: i32, artist: &str, album: &str) -> String {
    format!("{folder}.{artist}.{album}")
}
fn group(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|c| c.is_ascii_hexdigit())
}

fn album_key(id: &str) -> Result<AlbumKey<'_>, Failure> {
    let mut parts = id.split('.');
    let folder = parts.next().and_then(|v| v.parse().ok()).ok_or(NOT_FOUND)?;
    let artist = parts.next().filter(|v| group(v)).ok_or(NOT_FOUND)?;
    let album = parts.next().filter(|v| group(v)).ok_or(NOT_FOUND)?;
    if parts.next().is_some() {
        return Err(NOT_FOUND);
    }
    Ok(AlbumKey {
        folder,
        artist,
        album,
    })
}

fn song(row: CatalogSong) -> Value {
    let name = row
        .path
        .as_deref()
        .and_then(|path| FilePath::new(path).file_name()?.to_str());
    let suffix = name
        .and_then(|n| FilePath::new(n).extension()?.to_str())
        .map(str::to_lowercase);
    let mut value = json!({"id":row.id,"coverArt":row.id,"isDir":false,"type":"music","isVideo":false,"title":row.title,
        "artist":row.artist,"album":row.album,"displayAlbumArtist":row.album_artist,"albumId":album_id(row.folder,&row.artist_key,&row.album_key),
        "track":row.track,"discNumber":row.disc,"duration":row.duration_ms.map(|ms|ms/1000),"size":row.bytes,
        "created":clock::from_millis(row.created),"suffix":suffix,"contentType":name.map(palveni_library::classify::audio_content_type)});
    if let (Some(start), Some(end)) = (row.cue_start, row.cue_end) {
        value["suffix"] = json!("wav");
        value["contentType"] = json!("audio/wav");
        value["size"] = json!(44 + (end - start) * 2352);
    }
    if let Some(artist) = row.album_artist.as_ref() {
        value["albumArtists"] =
            json!([{"id":format!("{}.{}",row.folder,row.artist_key),"name":artist}]);
    }
    if let Value::Object(fields) = &mut value {
        fields.retain(|_, v| !v.is_null());
    }
    value
}

fn integer(text: &str) -> i64 {
    text.trim().parse::<i64>().unwrap_or_default()
}

fn album(row: CatalogAlbum) -> Value {
    json!({"id":album_id(row.folder,&row.artist_key,&row.album_key),"coverArt":row.artwork_id,"name":row.name.unwrap_or_else(||"Unknown Album".into()),
        "artist":row.artist.unwrap_or_else(||"Unknown Artist".into()), "artistId":format!("{}.{}",row.folder,row.artist_key),
        "songCount":integer(&row.count),"duration":row.duration_ms.as_deref().map(integer).unwrap_or(0)/1000,
        "created":clock::from_millis(row.created)})
}

fn artist(row: CatalogArtist) -> Value {
    json!({"id":format!("{}.{}",row.folder,row.artist_key),"name":row.name.unwrap_or_else(||"Unknown Artist".into()),
        "albumCount":integer(&row.album_count)})
}

async fn browse(
    state: &AppState,
    name: &str,
    p: &Params,
    client: &auth::Client,
) -> Result<Value, Failure> {
    let music = state.db.music();
    match name {
        "ping" => Ok(json!({})),
        "getScanStatus" => {
            let (scanning, count, last) = music.catalog_scan_status().await?;
            let mut status = json!({"scanning":scanning,"count":count});
            if let Some(at) = last.and_then(clock::from_millis) {
                status["lastScan"] = json!(at);
            }
            Ok(json!({"scanStatus":status}))
        }
        "startScan" => Err(Failure(
            50,
            "Use the administrator dashboard to scan libraries",
        )),
        "getOpenSubsonicExtensions" => {
            Ok(json!({"openSubsonicExtensions":[{"name":"songLyrics","versions":[1]}]}))
        }
        "getLyricsBySongId" => personal::lyrics(state, p).await,
        "getMusicFolders" => Ok(
            json!({"musicFolders":{"musicFolder":music.catalog_folders().await?.into_iter().map(|(id,name)|json!({"id":id,"name":name})).collect::<Vec<_>>()}}),
        ),
        "getSong" => {
            let row = music
                .catalog_song(p.required("id")?)
                .await?
                .ok_or(NOT_FOUND)?;
            let entry = personal::songs(state, client, vec![row])
                .await?
                .pop()
                .ok_or(NOT_FOUND)?;
            Ok(json!({"song":entry}))
        }
        "getAlbumList2" => {
            let by_artist = match p.required("type")? {
                "alphabeticalByArtist" => true,
                "alphabeticalByName" => false,
                _ => return Err(UNSUPPORTED),
            };
            let list = music
                .catalog_albums(
                    &p.scope()?,
                    "",
                    p.page("size", "offset", 10, 500)?,
                    by_artist,
                )
                .await?;
            Ok(json!({"albumList2":{"album":list.into_iter().map(album).collect::<Vec<_>>()}}))
        }
        "getAlbum" => {
            let key = album_key(p.required("id")?)?;
            let mut value = album(music.catalog_album(&key).await?.ok_or(NOT_FOUND)?);
            let songs = music.catalog_album_songs(&key).await?;
            if songs.len() > palveni_db::music::catalog::MAX_ALBUM_TRACKS {
                return Err(Failure(
                    0,
                    "Use a paged catalog request for albums over 10000 tracks",
                ));
            }
            let songs = personal::songs(state, client, songs).await?;
            value["songCount"] = json!(songs.len());
            value["song"] = json!(songs);
            Ok(json!({"album":value}))
        }
        "search3" => {
            let query = p.value("query").ok_or(MISSING)?.trim();
            if query.chars().count() > 256 {
                return Err(MISSING);
            }
            let scope = p.scope()?;
            let artists = p.page("artistCount", "artistOffset", 20, 2000)?;
            let albums = p.page("albumCount", "albumOffset", 20, 2000)?;
            let songs = p.page("songCount", "songOffset", 20, 2000)?;
            let mut found = json!({});
            if artists.count > 0 {
                found["artist"] = json!(
                    music
                        .catalog_artists(&scope, query, artists)
                        .await?
                        .into_iter()
                        .map(artist)
                        .collect::<Vec<_>>()
                );
            }
            if albums.count > 0 {
                found["album"] = json!(
                    music
                        .catalog_albums(&scope, query, albums, false)
                        .await?
                        .into_iter()
                        .map(album)
                        .collect::<Vec<_>>()
                );
            }
            if songs.count > 0 {
                let rows = music.catalog_songs(&scope, query, songs).await?;
                found["song"] = json!(personal::songs(state, client, rows).await?);
            }
            Ok(json!({"searchResult3":found}))
        }
        _ => personal::dispatch(state, name, p, client).await,
    }
}

async fn cover_art(state: &AppState, method: Method, p: &Params) -> Result<Response, Failure> {
    use axum::http::header;
    static SCALER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    let id: ItemId = p.required("id")?.parse().map_err(|_| NOT_FOUND)?;
    let size = p
        .value("size")
        .map(|s| {
            s.parse::<u16>()
                .ok()
                .filter(|n| (1..=1024).contains(n))
                .ok_or(MISSING)
        })
        .transpose()?;
    let _permit = if size.is_some() {
        Some(
            SCALER
                .acquire()
                .await
                .map_err(|_| Failure(0, "Thumbnail unavailable"))?,
        )
    } else {
        None
    };
    let (mut mime, mut image) = palveni_library::artwork::open(&state.db, id)
        .await
        .map_err(|_| NOT_FOUND)?;
    if let Some(size) = size {
        image = state
            .hardware
            .cover_thumbnail(&image, mime, size)
            .await
            .map_err(|_| Failure(0, "Thumbnail unavailable"))?;
        mime = "image/jpeg";
    }
    let length = image.len();
    let body = if method == Method::HEAD {
        Vec::new()
    } else {
        image
    };
    Ok((
        [
            (header::CONTENT_TYPE, mime.to_owned()),
            (header::CONTENT_LENGTH, length.to_string()),
            (header::CACHE_CONTROL, "private, no-cache".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        body,
    )
        .into_response())
}

async fn media(
    state: &AppState,
    method: Method,
    headers: &HeaderMap,
    name: &str,
    p: &Params,
) -> Result<Response, Failure> {
    if name == "stream"
        && (p.value("format").is_some_and(|v| v != "raw")
            || p.value("timeOffset").is_some_and(|v| v != "0"))
    {
        return Err(UNSUPPORTED);
    }
    let id: ItemId = p.required("id")?.parse().map_err(|_| NOT_FOUND)?;
    let loc = state
        .db
        .music()
        .track_location(id)
        .await?
        .ok_or(NOT_FOUND)?;
    let file = open_audio(loc).await.map_err(|_| NOT_FOUND)?;
    // Repeated ranges of one authenticated client URL share its proof; do not store the proof.
    let key = playback_key(id, b"subsonic", p.required("t")?.as_bytes());
    serve_track(state, file, method, headers, key)
        .await
        .map_err(|_| Failure(0, "Media unavailable"))
}

pub async fn handle(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(name): Path<String>,
    Query(params): Query<Vec<(String, String)>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let p = Params(params);
    let as_json = p.value("f") == Some("json");
    let fail = |failure| wire::reply(as_json, Err(failure));
    if !matches!(p.value("f"), None | Some("json" | "xml")) || p.required("c").is_err() {
        return fail(MISSING);
    }
    let Ok(version) = p.required("v") else {
        return fail(MISSING);
    };
    let parts = version
        .split('.')
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>();
    match parts.as_deref() {
        Ok([1, minor, ..]) if *minor <= 16 => {}
        Ok([major, ..]) if *major < 1 => {
            return fail(Failure(20, "Client protocol version is too old"));
        }
        Ok(_) => return fail(Failure(30, "Server does not support this protocol version")),
        Err(_) => return fail(MISSING),
    }
    let attempt = match state.subsonic_limit.begin(peer.ip()) {
        Ok(a) => a,
        Err(err) => return err.into_response(),
    };
    let token_auth = p.value("s").is_some() || p.value("t").is_some();
    let password_auth = p.value("p").is_some();
    let key_auth = p.value("apiKey").is_some();
    if [token_auth, password_auth, key_auth]
        .into_iter()
        .filter(|v| *v)
        .count()
        > 1
        || key_auth && p.value("u").is_some()
    {
        attempt.finish(false);
        return fail(Failure(43, "Conflicting authentication methods"));
    }
    if password_auth || key_auth {
        attempt.finish(false);
        return fail(Failure(42, "Unsupported authentication method"));
    }
    let (Ok(user), Ok(salt), Ok(token)) = (p.required("u"), p.required("s"), p.required("t"))
    else {
        attempt.finish(false);
        return fail(MISSING);
    };
    let verified = auth::verify(&state, user, salt, token).await;
    let client = match verified {
        Ok(Some(client)) => {
            attempt.finish(true);
            client
        }
        Ok(None) => {
            attempt.finish(false);
            return fail(DENIED);
        }
        Err(_) => return fail(Failure(0, "Internal server error")),
    };
    let name = name.strip_suffix(".view").unwrap_or(&name);
    if method == Method::HEAD && !matches!(name, "stream" | "download" | "getCoverArt") {
        return fail(UNSUPPORTED);
    }
    if name == "getCoverArt" {
        cover_art(&state, method, &p).await.unwrap_or_else(fail)
    } else if matches!(name, "stream" | "download") {
        media(&state, method, &headers, name, &p)
            .await
            .unwrap_or_else(fail)
    } else {
        wire::reply(as_json, browse(&state, name, &p, &client).await)
    }
}
