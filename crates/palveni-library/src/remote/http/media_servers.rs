use super::*;
use serde_json::Value;

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn id_of(row: &Value, key: &str) -> Option<String> {
    let id = &row[key];
    if let Some(s) = id.as_str() {
        return valid_id(s).then(|| s.to_owned());
    }
    id.as_u64().map(|n| n.to_string())
}
fn typed(value: &str) -> Result<(&str, &str)> {
    let (kind, id) = value.split_once(':').ok_or(RemoteError::Invalid)?;
    valid_id(id)
        .then_some((kind, id))
        .ok_or(RemoteError::Invalid)
}
fn prefix(kind: Kind) -> &'static str {
    match kind {
        Kind::Folder => "folder",
        Kind::Audio => "audio",
        Kind::Video => "video",
        Kind::File => "file",
    }
}

async fn get(source: &Source<'_>, url: Url) -> Result<Response> {
    source
        .send(
            source
                .request(Method::GET, url)?
                .header(header::ACCEPT, "application/json"),
        )
        .await
}
async fn json(response: Response) -> Result<Value> {
    let bytes = small_body(response).await?;
    tokio::task::spawn_blocking(move || serde_json::from_slice(&bytes))
        .await
        .map_err(|_| RemoteError::Unavailable)?
        .map_err(|_| RemoteError::Unavailable)
}
fn array<'a>(object: &'a Value, key: &str) -> Result<&'a [Value]> {
    let rows = object[key].as_array().ok_or(RemoteError::Unavailable)?;
    if rows.len() > MAX_ROWS {
        return Err(RemoteError::Unavailable);
    }
    Ok(rows)
}
fn paged(
    entries: Vec<ConnectionEntry>,
    source_count: usize,
    total: Option<u64>,
    offset: u32,
    limit: u32,
) -> ConnectionPage {
    let consumed = u32::try_from(source_count).unwrap_or(u32::MAX).min(limit);
    let next = offset.checked_add(consumed);
    let more = next.is_some_and(|n| {
        consumed != 0 && (source_count > limit as usize || total.is_some_and(|t| u64::from(n) < t))
    });
    ConnectionPage {
        entries,
        next_offset: next.filter(|_| more),
    }
}

pub(super) async fn entries(
    source: &Source<'_>,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage> {
    if source.kind == ConnectionKind::Plex {
        return plex_entries(source, parent, offset, limit).await;
    }
    let root = parent.is_empty();
    let list = if root {
        views(source).await?
    } else {
        let (kind, id) = typed(parent)?;
        if kind != "folder" {
            return Err(RemoteError::Invalid);
        }
        let mut url = source.endpoint(&["Items"])?;
        url.query_pairs_mut()
            .append_pair("ParentId", id)
            .append_pair("Recursive", "false")
            .append_pair("SortBy", "SortName")
            .append_pair("SortOrder", "Ascending")
            .append_pair("StartIndex", &offset.to_string())
            .append_pair("Limit", &(limit + 1).to_string());
        json(get(source, url).await?).await?
    };
    let rows = array(&list, "Items")?;
    let entries: Vec<_> = rows
        .iter()
        .take(if root { MAX_ROWS } else { limit as usize })
        .filter_map(|r| {
            let id = id_of(r, "Id")?;
            let name = named(r["Name"].as_str()?)?;
            let ty = r["Type"].as_str().unwrap_or_default();
            let kind = if root
                || r["IsFolder"] == true
                || matches!(
                    ty,
                    "Folder"
                        | "CollectionFolder"
                        | "UserView"
                        | "Series"
                        | "Season"
                        | "MusicAlbum"
                        | "MusicArtist"
                        | "BoxSet"
                        | "Playlist"
                ) {
                Kind::Folder
            } else {
                match r["MediaType"].as_str().unwrap_or(ty) {
                    "Audio" | "AudioBook" => Kind::Audio,
                    "Video" | "Movie" | "Episode" | "MusicVideo" | "Trailer" => Kind::Video,
                    _ => Kind::File,
                }
            };
            Some(ConnectionEntry {
                id: format!("{}:{id}", prefix(kind)),
                name,
                kind,
            })
        })
        .collect();
    Ok(if root {
        page(entries, offset, limit)
    } else {
        paged(
            entries,
            rows.len(),
            list["TotalRecordCount"].as_u64(),
            offset,
            limit,
        )
    })
}

async fn current_user(source: &Source<'_>) -> Result<Option<String>> {
    let response = get(source, source.endpoint(&["Users", "Me"])?).await?;
    if matches!(response.status().as_u16(), 400 | 401 | 403 | 404) {
        return Ok(None);
    }
    id_of(&json(response).await?, "Id")
        .map(Some)
        .ok_or(RemoteError::Unavailable)
}
async fn views(source: &Source<'_>) -> Result<Value> {
    if source.kind == ConnectionKind::Emby {
        return json(get(source, source.endpoint(&["Library", "MediaFolders"])?).await?).await;
    }
    let response = get(source, source.endpoint(&["UserViews"])?).await?;
    if !matches!(response.status().as_u16(), 400 | 401 | 403 | 404) {
        return json(response).await;
    }
    let url = if let Some(user) = current_user(source).await? {
        source.endpoint(&["Users", &user, "Views"])?
    } else {
        source.endpoint(&["Library", "MediaFolders"])?
    };
    json(get(source, url).await?).await
}

fn plex_kind(row: &Value) -> Kind {
    match row["type"].as_str().unwrap_or_default() {
        "artist" | "album" | "show" | "season" | "collection" | "playlist" | "folder"
        | "photoalbum" => Kind::Folder,
        "track" => Kind::Audio,
        "movie" | "episode" | "clip" | "extra" => Kind::Video,
        _ => Kind::File,
    }
}
fn parts(row: &Value) -> Option<&[Value]> {
    row.get("Media")?
        .as_array()?
        .first()?
        .get("Part")?
        .as_array()
        .map(|a| a.as_slice())
}
async fn metadata(source: &Source<'_>, id: &str) -> Result<Value> {
    let value = json(get(source, source.endpoint(&["library", "metadata", id])?).await?).await?;
    array(&value["MediaContainer"], "Metadata")?
        .iter()
        .find(|r| id_of(r, "ratingKey").as_deref() == Some(id))
        .cloned()
        .ok_or(RemoteError::Unavailable)
}

async fn plex_entries(
    source: &Source<'_>,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage> {
    let root = parent.is_empty();
    let mut url = if root {
        source.endpoint(&["library", "sections"])?
    } else {
        let (ty, id) = typed(parent)?;
        match ty {
            "section" => source.endpoint(&["library", "sections", id, "all"])?,
            "folder" => source.endpoint(&["library", "metadata", id, "children"])?,
            "parts" => {
                let row = metadata(source, id).await?;
                let kind = plex_kind(&row);
                if !matches!(kind, Kind::Video | Kind::Audio) {
                    return Err(RemoteError::Invalid);
                }
                let title = named(row["title"].as_str().unwrap_or_default())
                    .ok_or(RemoteError::Unavailable)?;
                let parts = parts(&row).ok_or(RemoteError::Unavailable)?;
                if parts.len() > MAX_ROWS {
                    return Err(RemoteError::Unavailable);
                }
                let entries = parts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        Some(ConnectionEntry {
                            id: format!("{}:{id}:{}", prefix(kind), id_of(p, "id")?),
                            kind,
                            name: format!("{title} · {}/{}", i + 1, parts.len()),
                        })
                    })
                    .collect();
                return Ok(page(entries, offset, limit));
            }
            _ => return Err(RemoteError::Invalid),
        }
    };
    if !root {
        url.query_pairs_mut()
            .append_pair("X-Plex-Container-Start", &offset.to_string())
            .append_pair("X-Plex-Container-Size", &(limit + 1).to_string());
    }
    let value = json(get(source, url).await?).await?;
    let container = value
        .get("MediaContainer")
        .filter(|r| r.is_object())
        .ok_or(RemoteError::Unavailable)?;
    let key = if root { "Directory" } else { "Metadata" };
    let rows = if container[key].is_null() && container["size"] == 0 {
        &[][..]
    } else {
        array(container, key)?
    };
    if !root
        && container["offset"]
            .as_u64()
            .is_some_and(|v| v != u64::from(offset))
    {
        return Err(RemoteError::Unavailable);
    }
    let entries = rows
        .iter()
        .take(if root { MAX_ROWS } else { limit as usize })
        .filter_map(|r| {
            let name = named(r["title"].as_str()?)?;
            let id = id_of(r, if root { "key" } else { "ratingKey" })?;
            let kind = if root { Kind::Folder } else { plex_kind(r) };
            let multiple =
                matches!(kind, Kind::Audio | Kind::Video) && parts(r).is_some_and(|p| p.len() > 1);
            let category = if root {
                "section"
            } else if multiple {
                "parts"
            } else {
                prefix(kind)
            };
            Some(ConnectionEntry {
                id: format!("{category}:{id}"),
                name,
                kind: if multiple { Kind::Folder } else { kind },
            })
        })
        .collect();
    Ok(if root {
        page(entries, offset, limit)
    } else {
        paged(
            entries,
            rows.len(),
            container["totalSize"].as_u64(),
            offset,
            limit,
        )
    })
}

pub(super) async fn file(source: &Source<'_>, value: &str) -> Result<Url> {
    let mut pieces = value.split(':');
    let kind = pieces.next().unwrap_or_default();
    let id = pieces
        .next()
        .filter(|s| valid_id(s))
        .ok_or(RemoteError::Invalid)?;
    let pid = pieces.next();
    if pieces.next().is_some() || !matches!(kind, "audio" | "video") {
        return Err(RemoteError::Invalid);
    }
    if source.kind != ConnectionKind::Plex {
        if pid.is_some() {
            return Err(RemoteError::Invalid);
        }
        let mut url = source.endpoint(&[
            if kind == "audio" { "Audio" } else { "Videos" },
            id,
            "stream",
        ])?;
        url.query_pairs_mut().append_pair("Static", "true");
        return Ok(url);
    }
    if pid.is_some_and(|p| !valid_id(p)) {
        return Err(RemoteError::Invalid);
    }
    let row = metadata(source, id).await?;
    if prefix(plex_kind(&row)) != kind {
        return Err(RemoteError::Invalid);
    }
    let parts = parts(&row).ok_or(RemoteError::Unavailable)?;
    let part = if let Some(pid) = pid {
        parts
            .iter()
            .find(|p| id_of(p, "id").as_deref() == Some(pid))
    } else if parts.len() == 1 {
        parts.first()
    } else {
        None
    };
    let part = part.ok_or(RemoteError::Invalid)?;
    let part_id = id_of(part, "id").ok_or(RemoteError::Unavailable)?;
    let Some(key) = part["key"].as_str() else {
        return source.endpoint(&["library", "parts", &part_id]);
    };
    let url = if let Some(path) = key.strip_prefix("/library/parts/") {
        if path.contains(['?', '#']) {
            return Err(RemoteError::Unavailable);
        }
        let segment: Vec<_> = path
            .split('/')
            .map(decode)
            .collect::<Option<_>>()
            .ok_or(RemoteError::Unavailable)?;
        let mut path = vec!["library", "parts"];
        path.extend(segment.iter().map(String::as_str));
        source.endpoint(&path)?
    } else {
        scoped_href(source.address, source.address, key).ok_or(RemoteError::Unavailable)?
    };
    let expected = source.endpoint(&["library", "parts", &part_id])?;
    let required = components(&expected).ok_or(RemoteError::Unavailable)?;
    let actual = components(&url).ok_or(RemoteError::Unavailable)?;
    if !actual.starts_with(&required) || actual.len() > required.len() + 2 {
        return Err(RemoteError::Unavailable);
    }
    Ok(url)
}
