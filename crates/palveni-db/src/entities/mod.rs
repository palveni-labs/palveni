//! SeaORM entities.
//!
//! Portable column choices (identical behaviour on SQLite, PostgreSQL and
//! MySQL/MariaDB):
//! - IDs are UUIDv7 strings in `VARCHAR(36)`. Native UUID types differ per
//!   backend (PostgreSQL `uuid`, nothing on MySQL/SQLite) and sqlx encodes
//!   `Uuid` differently per driver; lowercase hyphenated text compares and
//!   sorts the same everywhere, and v7 strings sort by creation time.
//! - Timestamps are `BIGINT` milliseconds since the Unix epoch. This avoids
//!   `timestamptz` vs `DATETIME` vs text semantics, time-zone conversion and
//!   precision differences, and keeps range comparisons trivial.
//! - Indexed text columns are bounded `VARCHAR`s (MySQL cannot index `TEXT`
//!   without a prefix length). Unbounded text that needs uniqueness (file
//!   paths) is indexed through a SHA-256 hash column instead.
//! - Case-insensitive uniqueness uses an explicit lowercase `*_key` column
//!   instead of collations, which are backend-specific.

// Main store: the catalog and everything that joins with it.
pub mod broadcast_entries;
pub mod broadcast_favorites;
pub mod broadcast_sources;
pub mod devices;
pub mod items;
pub mod libraries;
pub mod library_mounts;
pub mod library_paths;
pub mod media_connections;
pub mod music_collection_favorites;
pub mod scan_review_items;
pub mod scan_reviews;
pub mod sessions;
pub mod settings;
pub mod subsonic_folders;
pub mod subsonic_plays;
pub mod sync_state;
pub mod track_loudness;
pub mod track_metadata;
pub mod users;
pub mod video_metadata;

// Ops store: high-churn operational data, never joined with the catalog.
pub mod audit_log;
pub mod jobs;
pub mod playlist_entries;
pub mod playlists;
pub mod plugin_kv;
pub mod user_item_state;
