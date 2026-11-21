use super::{Failure, MISSING, NOT_FOUND, Params, UNSUPPORTED, auth::Client, integer, song};
use crate::AppState;
use palveni_core::{ItemId, clock, ids::PlaylistId, music::LyricsFormat, personal::MediaView};
use palveni_db::{music::catalog::CatalogSong, personal::MediaChange, playlists::ClientPlaylist};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn item(raw: &str) -> Result<ItemId, Failure> {
    raw.parse().map_err(|_| NOT_FOUND)
}
fn playlist(raw: &str) -> Result<PlaylistId, Failure> {
    raw.parse().map_err(|_| NOT_FOUND)
}

impl Params {
    fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

pub(super) async fn songs(
    state: &AppState,
    client: &Client,
    rows: Vec<CatalogSong>,
) -> Result<Vec<Value>, Failure> {
    let mut values = Vec::with_capacity(rows.len());
    let states = if client.personal {
        let ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        state.db.personal().states_for(client.id, &ids).await?
    } else {
        Default::default()
    };
    for row in rows {
        let personal = states.get(&row.id);
        let mut value = song(row);
        if let Some(s) = personal {
            value["userRating"] = json!(s.stars);
            value["playCount"] = json!(s.play_count);
            if let Some(at) = s.last_played_at {
                value["played"] = json!(at);
            }
        }
        values.push(value);
    }
    Ok(values)
}

fn header(row: ClientPlaylist, owner: &str) -> Value {
    let mut value = json!({"id":row.id,"name":row.name,"public":false,"owner":owner,"readonly":false,
        "songCount":integer(&row.count),"duration":row.duration_ms.as_deref().map(integer).unwrap_or(0)/1000});
    if let Some(text) = row.comment {
        value["comment"] = json!(text);
    }
    if let Some(at) = row.created_at.and_then(clock::from_millis) {
        value["created"] = json!(at);
    }
    if let Some(at) = row.changed_at.and_then(clock::from_millis) {
        value["changed"] = json!(at);
    }
    value
}

async fn detail(
    state: &AppState,
    client: &Client,
    owner: &str,
    id: PlaylistId,
) -> Result<Value, Failure> {
    let list = state.db.playlists().get(client.id, id).await?;
    let row = state
        .db
        .playlists()
        .client_headers(client.id, Some(id))
        .await?
        .pop()
        .ok_or(NOT_FOUND)?;
    let mut result = header(row, owner);
    let tracks = state.db.music().catalog_by_ids(&list.track_ids).await?;
    let entries = songs(state, client, tracks).await?;
    result["songCount"] = json!(entries.len());
    result["entry"] = json!(entries);
    Ok(json!({"playlist":result}))
}

async fn write_playlist(
    state: &AppState,
    client: &Client,
    owner: &str,
    name: &str,
    p: &Params,
) -> Result<Value, Failure> {
    if p.value("public").is_some_and(|s| s != "false") {
        return Err(UNSUPPORTED);
    }
    let lists = state.db.playlists();
    let (id, version, current_name, mut ids) =
        if let Some(id) = p.value("playlistId").or(p.value("id")) {
            let id = playlist(id)?;
            let list = lists.get(client.id, id).await?;
            (id, list.summary.version, list.summary.name, list.track_ids)
        } else if name == "createPlaylist" {
            (
                PlaylistId::generate(),
                0,
                p.required("name")?.to_owned(),
                Vec::new(),
            )
        } else {
            return Err(MISSING);
        };

    if name == "deletePlaylist" {
        lists.delete(client.id, id, version).await?;
        return Ok(json!({}));
    }
    if name == "createPlaylist" {
        ids = p.values("songId").map(item).collect::<Result<_, _>>()?;
    } else {
        let remove = p
            .values("songIndexToRemove")
            .map(|s| s.parse::<usize>().map_err(|_| MISSING))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if remove.last().is_some_and(|i| *i >= ids.len()) {
            return Err(MISSING);
        }
        ids = ids
            .into_iter()
            .enumerate()
            .filter_map(|(i, id)| (!remove.contains(&i)).then_some(id))
            .collect();
        for add in p.values("songIdToAdd") {
            ids.push(item(add)?);
        }
    }
    lists
        .save_with_comment(
            client.id,
            id,
            version,
            p.value("name").unwrap_or(&current_name),
            ids,
            p.value("comment"),
        )
        .await?;
    if name == "createPlaylist" {
        detail(state, client, owner, id).await
    } else {
        Ok(json!({}))
    }
}

async fn favorite_songs(
    state: &AppState,
    client: &Client,
    scope: &[i32],
) -> Result<Value, Failure> {
    let mut before = None;
    let mut favorites = Vec::new();
    loop {
        let page = state
            .db
            .personal()
            .list(client.id, MediaView::Favorites, before, 200)
            .await?;
        let ids = page.items.iter().map(|row| row.item_id).collect::<Vec<_>>();
        let tracks = state
            .db
            .music()
            .catalog_by_ids(&ids)
            .await?
            .into_iter()
            .filter(|row| scope.is_empty() || scope.contains(&row.folder))
            .collect();
        favorites.extend(songs(state, client, tracks).await?);
        before = page.next_before;
        if before.is_none() {
            break;
        }
    }
    let (artists, albums) = state
        .db
        .music()
        .favorite_collections(client.id, scope)
        .await?;
    Ok(json!({"starred2":{
        "artist":artists.into_iter().map(super::artist).collect::<Vec<_>>(),
        "album":albums.into_iter().map(super::album).collect::<Vec<_>>(),
        "song":favorites
    }}))
}

async fn star_collection(
    state: &AppState,
    client: &Client,
    p: &Params,
    favorite: bool,
) -> Result<Value, Failure> {
    if p.values("albumId").count() + p.values("artistId").count() + p.values("id").count() != 1 {
        return Err(MISSING);
    }
    let (folder, artist, album) = if let Some(raw) = p.value("albumId") {
        let key = super::album_key(raw)?;
        (key.folder, key.artist, Some(key.album))
    } else {
        let (folder, artist) = p.required("artistId")?.split_once('.').ok_or(NOT_FOUND)?;
        if !super::group(artist) {
            return Err(NOT_FOUND);
        }
        (folder.parse().map_err(|_| NOT_FOUND)?, artist, None)
    };
    state
        .db
        .music()
        .set_collection_favorite(
            client.id,
            folder,
            &artist.to_ascii_lowercase(),
            album.map(str::to_ascii_lowercase).as_deref(),
            favorite,
        )
        .await?;
    Ok(json!({}))
}

pub(super) async fn dispatch(
    state: &AppState,
    name: &str,
    p: &Params,
    client: &Client,
) -> Result<Value, Failure> {
    if !matches!(
        name,
        "getStarred2"
            | "star"
            | "unstar"
            | "setRating"
            | "scrobble"
            | "getPlaylists"
            | "getPlaylist"
            | "updatePlaylist"
            | "createPlaylist"
            | "deletePlaylist"
    ) {
        return Err(UNSUPPORTED);
    }
    if !client.personal {
        return Err(Failure(50, "Music client personal access is disabled"));
    }
    let owner = p.required("u")?;
    match name {
        "getStarred2" => favorite_songs(state, client, &p.scope()?).await,
        "getPlaylists" => {
            if p.value("username")
                .is_some_and(|u| !u.eq_ignore_ascii_case(owner))
            {
                return Err(Failure(50, "Access denied"));
            }
            let headers = state.db.playlists().client_headers(client.id, None).await?;
            Ok(
                json!({"playlists":{"playlist":headers.into_iter().map(|row|header(row,owner)).collect::<Vec<_>>()}}),
            )
        }
        "getPlaylist" => detail(state, client, owner, playlist(p.required("id")?)?).await,
        "updatePlaylist" | "createPlaylist" | "deletePlaylist" => {
            write_playlist(state, client, owner, name, p).await
        }
        "star" | "unstar" if p.value("albumId").is_some() || p.value("artistId").is_some() => {
            star_collection(state, client, p, name == "star").await
        }
        _ => {
            if p.value("albumId").is_some() || p.value("artistId").is_some() {
                return Err(UNSUPPORTED);
            }
            if p.values("id").count() != 1 || p.values("time").count() > 1 {
                return Err(MISSING);
            }
            let id = item(p.required("id")?)?;
            state
                .db
                .music()
                .catalog_song(&id.to_string())
                .await?
                .ok_or(NOT_FOUND)?;
            let change = match name {
                "star" => MediaChange::Favorite(true),
                "unstar" => MediaChange::Favorite(false),
                "setRating" => {
                    MediaChange::Rating(p.required("rating")?.parse().map_err(|_| MISSING)?)
                }
                "scrobble" => {
                    let now = clock::now().timestamp_millis();
                    let at = p
                        .value("time")
                        .map(str::parse::<i64>)
                        .transpose()
                        .map_err(|_| MISSING)?
                        .unwrap_or(now);
                    if !(0..=now + 300_000).contains(&at) {
                        return Err(MISSING);
                    }
                    let submitted = match p.value("submission") {
                        None | Some("true") => true,
                        Some("false") => false,
                        _ => return Err(MISSING),
                    };
                    MediaChange::Scrobble { at, submitted }
                }
                _ => return Err(UNSUPPORTED),
            };
            state.db.personal().update(client.id, id, change).await?;
            Ok(json!({}))
        }
    }
}

pub(super) async fn lyrics(state: &AppState, p: &Params) -> Result<Value, Failure> {
    let id = item(p.required("id")?)?;
    let location = state
        .db
        .music()
        .track_location(id)
        .await?
        .ok_or(NOT_FOUND)?;
    let lyrics = palveni_library::lyrics::read(location)
        .await
        .map_err(|_| NOT_FOUND)?;
    let tracks: Vec<_> = lyrics.as_ref().map(structured).into_iter().collect();
    Ok(json!({"lyricsList":{"structuredLyrics":tracks}}))
}

fn structured(doc: &palveni_core::music::Lyrics) -> Value {
    let mut lines = Vec::new();
    if doc.format == LyricsFormat::Lrc {
        for row in doc.text.lines() {
            let mut rest = row.trim();
            let mut times = Vec::new();
            while let Some((tag, tail)) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
                let Some(ms) = timestamp(tag) else { break };
                times.push(ms);
                rest = tail;
            }
            lines.extend(times.into_iter().map(|ms| (ms, rest)));
        }
    }
    let synced = !lines.is_empty();
    lines.sort_by_key(|line| line.0);
    let entries: Vec<_> = if synced {
        lines
            .into_iter()
            .map(|(ms, text)| json!({"start":ms,"value":text}))
            .collect()
    } else {
        doc.text.lines().map(|text| json!({"value":text})).collect()
    };
    let mut result = json!({"lang":"und","synced":synced,"line":entries});
    if synced
        && let Some(offset) = doc.text.lines().find_map(|text| {
            text.trim()
                .strip_prefix("[offset:")?
                .strip_suffix(']')?
                .trim()
                .parse::<i32>()
                .ok()
        })
    {
        result["offset"] = json!(offset);
    }
    result
}

fn timestamp(tag: &str) -> Option<i64> {
    let (m, s) = tag.split_once(':')?;
    let (s, f) = s.split_once('.').unwrap_or((s, ""));
    if !m.bytes().all(|b| b.is_ascii_digit())
        || s.len() != 2
        || !s.bytes().all(|b| b.is_ascii_digit())
        || f.len() > 3
        || !f.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let sec = s.parse::<i64>().ok()?;
    if sec >= 60 {
        return None;
    }
    let fraction = format!("{f:0<3}").parse::<i64>().ok()?;
    m.parse::<i64>()
        .ok()?
        .checked_mul(60_000)?
        .checked_add(sec * 1000 + fraction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use palveni_core::music::{Lyrics, LyricsSource};
    #[test]
    fn timed_lyrics_keep_repeated_lines_order_offset_and_literal_text() {
        let doc = Lyrics {
            source: LyricsSource::Sidecar,
            format: LyricsFormat::Lrc,
            text: "[offset:-150]\n[01:02.3][00:04.00]One & two\n[01:20.045]Two".into(),
        };
        let value = structured(&doc);
        assert_eq!(value["synced"], true);
        assert_eq!(value["offset"], -150);
        assert_eq!(
            value["line"],
            json!([{"start":4000,"value":"One & two"},{"start":62300,"value":"One & two"},{"start":80045,"value":"Two"}])
        );
    }
}
