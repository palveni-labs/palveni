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
pub mod assistant_conversations;
pub mod assistant_memories;
pub mod assistant_messages;
pub mod assistant_runs;
pub mod broadcast_entries;
pub mod broadcast_favorites;
pub mod broadcast_sources;
pub mod client_api_key;
pub mod devices;
pub mod downloader_clients;
pub mod home_bridges;
pub mod home_devices;
pub mod home_rooms;
pub mod home_rules;
pub mod home_scenes;
pub mod inbound_webhooks;
pub mod items;
pub mod libraries;
pub mod library_mounts;
pub mod library_paths;
pub mod media_collection_items;
pub mod media_collections;
pub mod media_connections;
pub mod media_subscriptions;
pub mod metadata_override;
pub mod mqtt_bridge;
pub mod music_album_pending;
pub mod music_albums;
pub mod music_collection_favorites;
pub mod notification_channels;
pub mod notification_deliveries;
pub mod organize_batches;
pub mod photo_ai_tag;
pub mod photo_album_items;
pub mod photo_albums;
pub mod photo_metadata;
pub mod photo_person;
pub mod photo_person_tag;
pub mod playback_session;
pub mod plugin_installation;
pub mod reading_bookmarks;
pub mod reading_metadata;
pub mod scan_review_items;
pub mod scan_reviews;
pub mod scheduled_routines;
pub mod sessions;
pub mod settings;
pub mod share_links;
pub mod subsonic_folders;
pub mod subsonic_plays;
pub mod sync_state;
pub mod system_backups;
pub mod track_genres;
pub mod track_loudness;
pub mod track_metadata;
pub mod user_library_access;
pub mod user_policies;
pub mod users;
pub mod video_favorites;
pub mod video_genres;
pub mod video_metadata;
pub mod video_people;
pub mod video_segment;
pub mod virtual_channels;

// Ops store: high-churn operational data, never joined with the catalog.
pub mod audit_log;
pub mod jobs;
pub mod playlist_entries;
pub mod playlists;
pub mod plugin_kv;
pub mod user_item_state;
