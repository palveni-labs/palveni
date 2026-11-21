use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
};
use palveni_core::{
    Error, LibraryId,
    music::{GlobalMusicSearchPage, MusicSearchKind, MusicSearchPage},
};
use palveni_library::music_search::{GlobalCursor, terms};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{AppState, auth::CurrentUser, error::ApiError, routes::path_id};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    kind: Option<MusicSearchKind>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct ScopedCursor<T> {
    scope: String,
    #[serde(flatten)]
    state: T,
}

#[derive(Serialize, Deserialize)]
struct LibraryCursor {
    revision: i64,
    position: usize,
}

struct Search<T> {
    kind: MusicSearchKind,
    terms: Vec<String>,
    limit: usize,
    scope: String,
    cursor: Option<T>,
}

impl SearchQuery {
    fn parse<T: DeserializeOwned>(self, source: &str) -> Result<Search<T>, ApiError> {
        let kind = self.kind.unwrap_or(MusicSearchKind::Track);
        let terms = terms(&self.q)
            .map_err(|_| ApiError::validation("Invalid music search query").field("q"))?;
        let limit = self.limit.unwrap_or(30);
        if !(1..=200).contains(&limit) {
            return Err(ApiError::validation("Invalid limit").field("limit"));
        }
        let scope = hex::encode(ring::digest::digest(
            &ring::digest::SHA256,
            format!("{source}/{kind:?}/{}", terms.join("\0")).as_bytes(),
        ));
        let cursor = self
            .cursor
            .as_deref()
            .map(|text| {
                (text.len() <= 1024)
                    .then(|| hex::decode(text).ok())
                    .flatten()
                    .and_then(|bytes| serde_json::from_slice::<ScopedCursor<T>>(&bytes).ok())
                    .filter(|cursor| cursor.scope == scope)
                    .map(|cursor| cursor.state)
                    .ok_or_else(|| {
                        ApiError::validation("Invalid music search cursor").field("cursor")
                    })
            })
            .transpose()?;
        Ok(Search {
            kind,
            terms,
            limit: limit as usize,
            scope,
            cursor,
        })
    }
}

fn next_cursor<T: Serialize>(scope: String, next: Option<T>) -> Result<Option<String>, ApiError> {
    next.map(|state| {
        serde_json::to_vec(&ScopedCursor { scope, state })
            .map(hex::encode)
            .map_err(ApiError::internal)
    })
    .transpose()
}

fn search_error(error: Error) -> ApiError {
    match error {
        Error::NotFound(_) => ApiError::not_found("Music library not found"),
        Error::Invalid(message) => ApiError::validation(message),
        Error::Conflict(message) => ApiError::conflict(message),
        other => ApiError::internal(other),
    }
}

pub async fn library(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<SearchQuery>, QueryRejection>,
) -> Result<Json<MusicSearchPage>, ApiError> {
    let id: LibraryId = path_id(&path?.0, "Library")?;
    let search = query?.0.parse::<LibraryCursor>(&id.to_string())?;
    let matches = state
        .music_search
        .search(
            &state.db,
            id,
            search.kind,
            search.terms,
            search.limit,
            search
                .cursor
                .map(|cursor| (cursor.revision, cursor.position)),
        )
        .await
        .map_err(search_error)?;
    let next = matches.next.map(|position| LibraryCursor {
        revision: matches.revision,
        position,
    });
    Ok(Json(MusicSearchPage {
        entries: matches.entries,
        next_cursor: next_cursor(search.scope, next)?,
    }))
}

pub async fn all(
    _: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<SearchQuery>, QueryRejection>,
) -> Result<Json<GlobalMusicSearchPage>, ApiError> {
    let search = query?.0.parse::<GlobalCursor>("all")?;
    let matches = state
        .music_search
        .search_all(
            &state.db,
            search.kind,
            search.terms,
            search.limit,
            search.cursor,
        )
        .await
        .map_err(search_error)?;
    Ok(Json(GlobalMusicSearchPage {
        entries: matches.entries,
        next_cursor: next_cursor(search.scope, matches.next)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_page_can_resume_a_preexisting_cursor() {
        let id = "11111111-1111-4111-8111-111111111111";
        let query = |cursor| SearchQuery {
            q: "Shore".into(),
            kind: None,
            cursor: Some(cursor),
            limit: None,
        };
        // The original library endpoint stores the normalized query scope and row directly.
        let scope = hex::encode(ring::digest::digest(
            &ring::digest::SHA256,
            format!("{id}/Track/shore").as_bytes(),
        ));
        let wire = serde_json::json!({"revision": 16, "position": 5, "scope": scope});
        let encoded = hex::encode(serde_json::to_vec(&wire).unwrap());
        let old = query(encoded).parse::<LibraryCursor>(id).unwrap();
        let cursor = old.cursor.unwrap();
        assert_eq!((cursor.revision, cursor.position), (16, 5));
        let next = next_cursor(old.scope, Some(cursor)).unwrap().unwrap();
        let decoded: serde_json::Value =
            serde_json::from_slice(&hex::decode(next).unwrap()).unwrap();
        assert_eq!(decoded, wire);
    }
}
