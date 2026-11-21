use axum::Json;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use palveni_core::{Item, ItemId, ItemKind, LibraryId};
use palveni_db::items::{ItemCursor, ItemQuery};
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id, query_id};
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemsQuery {
    library_id: Option<String>,
    parent_id: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
}

#[derive(utoipa::ToSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemList {
    items: Vec<Item>,
    #[schema(required = true)]
    next_cursor: Option<String>,
}

/// Host paths are only shown to administrators.
fn visible_to(mut item: Item, current: &CurrentUser) -> Item {
    if !current.is_admin() {
        item.path = None;
    }
    item
}

pub async fn list(
    current: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<ItemsQuery>, QueryRejection>,
) -> Result<Json<ItemList>, ApiError> {
    let Query(query) = query?;
    let library_id: LibraryId = query_id(query.library_id.as_deref(), "libraryId")?
        .ok_or_else(|| ApiError::validation("libraryId is required").field("libraryId"))?;
    let parent_id: Option<ItemId> = query_id(query.parent_id.as_deref(), "parentId")?;
    let kind = match query
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty())
    {
        None => None,
        Some(text) => Some(text.parse::<ItemKind>().map_err(|_| {
            ApiError::validation(format!("Unknown item kind `{text}`")).field("kind")
        })?),
    };
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = match query.cursor.as_deref().filter(|c| !c.is_empty()) {
        None => None,
        Some(text) => Some(
            ItemCursor::decode(text)
                .ok_or_else(|| ApiError::validation("Invalid cursor").field("cursor"))?,
        ),
    };

    if state.db.libraries().get(library_id).await?.is_none() {
        return Err(ApiError::not_found("Library not found"));
    }
    if let Some(parent) = parent_id {
        let belongs = state
            .db
            .items()
            .get(parent)
            .await?
            .is_some_and(|item| item.library_id == library_id);
        if !belongs {
            return Err(ApiError::not_found("Parent item not found"));
        }
    }
    let page = state
        .db
        .items()
        .list(&ItemQuery {
            library_id,
            parent_id,
            kind,
            limit,
            cursor,
        })
        .await?;
    Ok(Json(ItemList {
        items: page
            .items
            .into_iter()
            .map(|item| visible_to(item, &current))
            .collect(),
        next_cursor: page.next_cursor.map(|c| c.encode()),
    }))
}

pub async fn get_one(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<Item>, ApiError> {
    let Path(raw) = path?;
    let id: ItemId = path_id(&raw, "Item")?;
    let item = state
        .db
        .items()
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    Ok(Json(visible_to(item, &current)))
}
