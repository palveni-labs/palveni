//! Books and comics catalog, reading state, chapters and bookmarks.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{BookmarkId, ItemId, LibraryId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReadingKind {
    Book,
    Comic,
}

impl ReadingKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Book => "book",
            Self::Comic => "comic",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "book" => Some(Self::Book),
            "comic" => Some(Self::Comic),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReadingSort {
    #[default]
    Added,
    Title,
    Series,
}

impl ReadingSort {
    pub const fn default_dir(self) -> ReadingSortDir {
        match self {
            Self::Added => ReadingSortDir::Desc,
            Self::Title | Self::Series => ReadingSortDir::Asc,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReadingSortDir {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadingReaderMode {
    Chapters,
    Pages,
    Pdf,
    DownloadOnly,
}

impl ReadingReaderMode {
    pub fn from_kind_and_format(kind: ReadingKind, format: &str) -> Self {
        match (kind, format) {
            (_, "pdf") => Self::Pdf,
            (ReadingKind::Book, "epub" | "txt" | "md") => Self::Chapters,
            (ReadingKind::Comic, "cbz" | "zip" | "cbt" | "tar") => Self::Pages,
            _ => Self::DownloadOnly,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingUserState {
    pub page: i32,
    #[schema(required = true)]
    pub total_pages: Option<i32>,
    #[schema(required = true)]
    pub locator: Option<String>,
    pub completed: bool,
    pub favorite: bool,
    #[schema(required = true)]
    pub last_read_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingEntry {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub kind: ReadingKind,
    pub format: String,
    pub title: String,
    pub authors: Vec<String>,
    #[schema(required = true)]
    pub series_name: Option<String>,
    #[schema(required = true)]
    pub series_key: Option<String>,
    #[schema(required = true)]
    pub series_index: Option<f64>,
    #[schema(required = true)]
    pub summary: Option<String>,
    #[schema(required = true)]
    pub publisher: Option<String>,
    #[schema(required = true)]
    pub language: Option<String>,
    #[schema(required = true)]
    pub page_count: Option<i32>,
    pub has_cover: bool,
    pub tags: Vec<String>,
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
    pub added_at: DateTime<Utc>,
    pub state: ReadingUserState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSeries {
    pub series_key: String,
    pub series_name: String,
    pub library_id: LibraryId,
    pub kind: ReadingKind,
    pub item_count: u32,
    pub completed_count: u32,
    pub cover_item_id: ItemId,
    pub has_cover: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingHome {
    pub continue_reading: Vec<ReadingEntry>,
    pub recent_books: Vec<ReadingEntry>,
    pub recent_comics: Vec<ReadingEntry>,
    pub series: Vec<ReadingSeries>,
}

#[derive(Clone, Debug, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingPage {
    pub entries: Vec<ReadingEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingChapterSummary {
    pub index: u32,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingBookmark {
    pub id: BookmarkId,
    pub item_id: ItemId,
    pub page: i32,
    #[schema(required = true)]
    pub locator: Option<String>,
    pub label: String,
    #[schema(required = true)]
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingManifest {
    pub entry: ReadingEntry,
    pub reader_mode: ReadingReaderMode,
    pub chapters: Vec<ReadingChapterSummary>,
    pub bookmarks: Vec<ReadingBookmark>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingChapter {
    pub index: u32,
    pub title: String,
    pub html: String,
    pub plain_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadingInline {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingProgressUpdate {
    #[schema(minimum = 0)]
    pub page: i32,
    #[schema(minimum = 1)]
    pub total_pages: Option<i32>,
    #[schema(max_length = 512)]
    pub locator: Option<String>,
    pub completed: bool,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateBookmarkRequest {
    #[schema(minimum = 0)]
    pub page: i32,
    #[schema(max_length = 512)]
    pub locator: Option<String>,
    #[schema(min_length = 1, max_length = 200)]
    pub label: String,
    #[schema(max_length = 1000)]
    pub note: Option<String>,
}
