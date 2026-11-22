use serde_json::{Value, json};

use super::{object, reference};

// These envelopes describe status-dependent fields and the validated JSON
// skin format, which are not represented by a single serializable Rust DTO.
fn additional_schemas() -> Value {
    let error_codes = [
        "validation_failed",
        "path_not_found",
        "unauthorized",
        "invalid_credentials",
        "forbidden",
        "not_found",
        "setup_already_completed",
        "conflict",
        "method_not_allowed",
        "internal",
        "too_many_requests",
        "payload_too_large",
        "unavailable",
        "busy",
    ];
    let mut schemas = json!({});
    schemas["ApiErrorCode"] = json!({"type":"string","enum":error_codes});
    schemas["ApiErrorDetails"] = json!({"type":"object","properties":{"field":{"type":"string"},"index":{"type":"integer","minimum":0},"path":{"type":"string"}}});
    schemas["ApiErrorBody"] = object(
        json!({"error":object(json!({"code":reference("ApiErrorCode"),"message":{"type":"string"},"details":reference("ApiErrorDetails")}), &["code","message"])}),
        &["error"],
    );
    let localized = json!({"type":"object","additionalProperties":{"type":"string"}});
    let mut manifest = json!({"format":{"type":"integer","const":1},"id":{"type":"string"},"name":localized,"description":localized,"author":{"type":"string"}});
    for name in [
        "slots",
        "colors",
        "type",
        "shape",
        "motion",
        "elevation",
        "work",
    ] {
        manifest[name] = json!({"type":"object"});
    }
    manifest["fonts"] = json!({"type":"array","items":{"type":"object"}});
    schemas["SkinManifest"] = object(
        manifest,
        &[
            "format",
            "id",
            "name",
            "slots",
            "colors",
            "type",
            "shape",
            "motion",
            "elevation",
            "work",
        ],
    );
    let events = json!({
        "job.updated":reference("Job"),"library.updated":reference("Library"),
        "settings.updated":reference("RuntimeSettings"),
        "library.deleted":object(json!({"id":{"type":"string","format":"uuid"}}), &["id"]),
        "ping":{"type":"object","additionalProperties":false},
        "stream.reset":{"type":"object","additionalProperties":false}
    });
    schemas["ServerEventMap"] = object(
        events,
        &[
            "job.updated",
            "library.updated",
            "settings.updated",
            "library.deleted",
            "ping",
            "stream.reset",
        ],
    );
    schemas
}

use crate::routes::connections::{ConnectionList, ConnectionPlayback};
use crate::routes::storage::{MountList, MountProtocol, MountedDirectory, StorageFolders};
use crate::routes::{
    auth::{LoginRequest, LoginResponse, SetupRequest, SetupResponse},
    books::BookDownload,
    items::ItemList,
    jobs::JobList,
    libraries::{CreateLibraryRequest, LibraryList, ScanAccepted, UpdateLibraryRequest},
    link::{LinkChallenge, LinkDevice, LinkDeviceList, LinkPairRequest, LinkPairing},
    opds::OpdsAccess,
    personal::{FavoriteUpdate, PlaylistUpdate, RatingUpdate},
    playback::{PlaybackInfo, PlaybackMode},
    plugins::PluginList,
    scan_reviews::{ScanReview, ScanReviewList, ScanReviewPaths},
    subsonic::{SubsonicPassword, SubsonicPasswordStatus},
    system::{Health, SystemInfo},
};
use palveni_core::broadcast::{
    BroadcastEntry, BroadcastEntryPage, BroadcastFavorite, BroadcastImport, BroadcastKind,
    BroadcastProgramme, BroadcastSource, BroadcastSourceList,
};
use palveni_core::connections::{
    ConnectionEntry, ConnectionEntryKind, ConnectionKind, ConnectionPage, ConnectionSummary,
    CreateConnection,
};
use palveni_core::music::{
    GlobalMusicSearchHit, GlobalMusicSearchPage, LoudnessMeasurement, Lyrics, LyricsFormat,
    LyricsSource, MusicAlbum, MusicAlbumDetail, MusicAlbumPage, MusicAlbumSort, MusicArtist,
    MusicArtistDetail, MusicArtistPage, MusicArtistSort, MusicEntry, MusicGenre, MusicGenreList,
    MusicPage, MusicSearchHit, MusicSearchKind, MusicSearchPage, MusicSortOrder, MusicTrack,
    MusicTrackLookupRequest, MusicTrackLookupResponse, MusicTrackPage, MusicTrackSort,
    MusicTrackState, TrackLoudness, TrackLyrics,
};
use palveni_core::personal::{
    MediaState, MediaView, PersonalItem, PersonalItemPage, PlaybackUpdate, Playlist, PlaylistPage,
    PlaylistSummary, PlaylistTrack, PlaylistTrackPage,
};
use palveni_core::spoken::{
    SpokenChapter, SpokenChapterList, SpokenEntry, SpokenPage, SpokenRecording,
};
use palveni_core::video::{
    SubtitleFormat, SubtitleList, SubtitleTrack, VideoEntry, VideoFile, VideoGenre, VideoGenreList,
    VideoHome, VideoKind, VideoPage, VideoPerson, VideoPersonKind, VideoUserState, VideoWorkDetail,
};
use palveni_core::{
    Item, ItemKind, Job, Library, User, features::Capability, settings::RuntimeSettings,
};
use palveni_media::HardwareReport;
use palveni_plugin_host::skin_store::{InstalledSkin, RegistrySkin};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(components(schemas(
    SystemInfo,
    ConnectionEntry,
    ConnectionEntryKind,
    ConnectionKind,
    ConnectionList,
    ConnectionPage,
    ConnectionPlayback,
    ConnectionSummary,
    CreateConnection,
    BookDownload,
    SpokenChapter,
    SpokenChapterList,
    SpokenEntry,
    SpokenPage,
    SpokenRecording,
    VideoEntry,
    VideoFile,
    VideoGenre,
    VideoGenreList,
    VideoHome,
    VideoKind,
    VideoPage,
    VideoPerson,
    VideoPersonKind,
    VideoUserState,
    VideoWorkDetail,
    SubtitleList,
    SubtitleTrack,
    SubtitleFormat,
    MountedDirectory,
    MountList,
    MountProtocol,
    StorageFolders,
    BroadcastEntry,
    BroadcastEntryPage,
    BroadcastFavorite,
    BroadcastImport,
    BroadcastKind,
    BroadcastProgramme,
    BroadcastSource,
    BroadcastSourceList,
    Health,
    SetupRequest,
    SetupResponse,
    LoginRequest,
    LoginResponse,
    LinkChallenge,
    LinkDevice,
    LinkDeviceList,
    LinkPairRequest,
    LinkPairing,
    OpdsAccess,
    SubsonicPassword,
    SubsonicPasswordStatus,
    User,
    LibraryList,
    CreateLibraryRequest,
    UpdateLibraryRequest,
    Library,
    ScanAccepted,
    ScanReview,
    ScanReviewList,
    ScanReviewPaths,
    ItemList,
    Lyrics,
    LyricsFormat,
    LyricsSource,
    TrackLyrics,
    TrackLoudness,
    LoudnessMeasurement,
    MusicEntry,
    MusicPage,
    MusicSortOrder,
    MusicAlbumSort,
    MusicArtistSort,
    MusicTrackSort,
    MusicTrackState,
    MusicTrack,
    MusicAlbum,
    MusicArtist,
    MusicGenre,
    MusicAlbumPage,
    MusicAlbumDetail,
    MusicArtistPage,
    MusicArtistDetail,
    MusicGenreList,
    MusicTrackPage,
    MusicTrackLookupRequest,
    MusicTrackLookupResponse,
    MusicSearchPage,
    MusicSearchKind,
    MusicSearchHit,
    GlobalMusicSearchHit,
    GlobalMusicSearchPage,
    MediaState,
    MediaView,
    PersonalItem,
    PersonalItemPage,
    PlaybackUpdate,
    Playlist,
    PlaylistPage,
    PlaylistSummary,
    PlaylistTrack,
    PlaylistTrackPage,
    FavoriteUpdate,
    RatingUpdate,
    PlaylistUpdate,
    PlaybackInfo,
    PlaybackMode,
    Item,
    ItemKind,
    JobList,
    Job,
    HardwareReport,
    PluginList,
    InstalledSkin,
    RegistrySkin,
    RuntimeSettings,
    Capability
)))]
struct WireSchemas;

pub(super) fn components() -> Value {
    let mut schemas = additional_schemas();
    for (name, schema) in WireSchemas::openapi()
        .components
        .into_iter()
        .flat_map(|c| c.schemas)
    {
        schemas[name] = json!(schema);
    }
    schemas["Health"]["properties"]["status"] = json!({"type":"string","const":"ok"});
    schemas["LoginResponse"]["properties"]["tokenType"] = json!({"type":"string","const":"Bearer"});
    schemas
}
