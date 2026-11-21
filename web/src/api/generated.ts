export interface paths {
    "/api/v1/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Authenticate a device and issue its access token. */
        post: operations["login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Revoke the current device session. */
        post: operations["logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the authenticated user. */
        get: operations["getCurrentUser"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/broadcast/sources": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List podcast feeds, radio and live television sources. */
        get: operations["listBroadcastSources"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/broadcast/sources/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Import or refresh a feed, station list or live television playlist. */
        put: operations["saveBroadcastSource"];
        post?: never;
        /** @description Remove a subscription and its catalog. */
        delete: operations["removeBroadcastSource"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/broadcast/sources/{id}/entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse channels and episodes in one subscription. */
        get: operations["listBroadcastEntries"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/broadcast/sources/{id}/favorites/{entryId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Save or remove a channel or episode in your favorites. */
        put: operations["setBroadcastFavorite"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/connections": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List registered network sources. */
        get: operations["listConnections"];
        put?: never;
        /** @description Connect to and register a read-only network source. */
        post: operations["createConnection"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/connections/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Remove a saved connection and its stored credentials. */
        delete: operations["removeConnection"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/connections/{id}/entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse media on the connected server. */
        get: operations["browseConnection"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/connections/{id}/playback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Issue a session-bound playback URL for a remote media entry. */
        post: operations["prepareConnectionPlayback"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/connections/{id}/stream": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an original file from the connected server. */
        get: operations["streamConnection"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Stream visible changes and resume from the Last-Event-ID header. */
        get: operations["streamEvents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Page through one library folder. */
        get: operations["listItems"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an indexed media item. */
        get: operations["getItem"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/artwork": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read local album artwork for a music track. */
        get: operations["getMusicArtwork"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/audio": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a live audio stream in the requested format. */
        get: operations["streamAudio"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        /** @description Read the selected audio representation headers without encoding. */
        head: operations["headAudio"];
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/loudness": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the last EBU R128 measurement and whether it matches the indexed track. */
        get: operations["getTrackLoudness"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/lyrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read synchronized or plain lyrics from a track's sidecar or embedded tags. */
        get: operations["getTrackLyrics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/playback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Issue a short-lived original or transcoded playback URL. */
        post: operations["preparePlayback"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/poster": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a local artwork image for an indexed work, season or episode. */
        get: operations["getVideoPoster"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/stream": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the original media file, optionally requesting a single byte range. */
        get: operations["streamTrack"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        /** @description Read the media type and total size of the original track. */
        head: operations["headTrack"];
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/subtitles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List external subtitles for an indexed video. */
        get: operations["listVideoSubtitles"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/subtitles/{track}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read one external subtitle track as WebVTT. */
        get: operations["readVideoSubtitle"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/jobs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List recent background jobs. */
        get: operations["listJobs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/jobs/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Cancel queued work or request a running job to stop. */
        post: operations["cancelJob"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List media libraries. */
        get: operations["listLibraries"];
        put?: never;
        /** @description Register a media library and its local paths. */
        post: operations["createLibrary"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a media library. */
        get: operations["getLibrary"];
        put?: never;
        post?: never;
        /** @description Delete a library and its index while preserving the media files. */
        delete: operations["deleteLibrary"];
        options?: never;
        head?: never;
        /** @description Rename a library or add media folders. */
        patch: operations["updateLibrary"];
        trace?: never;
    };
    "/api/v1/libraries/{id}/music": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse the album artists, albums and tracks in a music library. */
        get: operations["browseMusic"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/loudness": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Measure the loudness of new and changed music files in the background. */
        post: operations["analyzeLoudness"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Search tracks, albums or artists in this music library. */
        get: operations["searchMusic"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/scan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Queue a file scan or return the library's active scan. */
        post: operations["scanLibrary"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/scan/reviews": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List scopes with missing media awaiting administrator review. */
        get: operations["listScanReviews"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/scan/reviews/{revision}/confirm": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Rescan and remove reviewed index entries only if the same media is still missing. */
        post: operations["confirmScanReview"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/scan/reviews/{revision}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a page of paths from an exact missing-media review. */
        get: operations["listScanReviewPaths"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/spoken": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse books, shows, chapters and episodes. */
        get: operations["browseSpoken"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/videos": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse works, seasons and episodes in a video library. */
        get: operations["browseVideos"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse your favorites, recent media or unfinished playback. */
        get: operations["listPersonalItems"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/items/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read your favorite, rating and playback state for a track or video. */
        get: operations["getPersonalItem"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/items/{id}/favorite": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Add or remove a track or video from your favorites. */
        put: operations["setFavorite"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/items/{id}/progress": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Save a playback position or completion against its last known version. */
        put: operations["reportPlayback"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/items/{id}/rating": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Rate a track or video from one to five stars, or clear its rating with zero. */
        put: operations["setRating"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/playlists": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your music playlists in order of last change. */
        get: operations["listPlaylists"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/playlists/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an ordered playlist, including repeated tracks. */
        get: operations["getPlaylist"];
        /** @description Create, rename or replace your playlist in one version-checked operation. */
        put: operations["savePlaylist"];
        post?: never;
        /** @description Delete your playlist at its current version. */
        delete: operations["deletePlaylist"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/playlists/{id}/tracks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a page of playlist positions with track titles, artists, albums and durations. */
        get: operations["listPlaylistTracks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/subsonic-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Check whether your music client password is enabled. */
        get: operations["getSubsonicPasswordStatus"];
        put?: never;
        /** @description Generate or replace your dedicated music client password. */
        post: operations["issueSubsonicPassword"];
        /** @description Disable access with your music client password. */
        delete: operations["revokeSubsonicPassword"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/music/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Search tracks, albums or artists across music libraries and identify their sources. */
        get: operations["searchAllMusic"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/openapi.json": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the OpenAPI 3.1 description. */
        get: operations["getOpenApi"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/plugins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List installed plugins. */
        get: operations["listPlugins"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/setup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Create the first administrator on an unconfigured server. */
        post: operations["createAdministrator"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/installed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List installed skin packages. */
        get: operations["listInstalledSkins"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/installed/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Uninstall a skin and its assets. */
        delete: operations["removeSkin"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/installed/{id}/assets/{path}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a validated font or bitmap by its package-relative path. */
        get: operations["getSkinAsset"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/installed/{id}/skin.json": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an installed skin manifest. */
        get: operations["getSkinManifest"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/registry": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List official skins pinned to this release. */
        get: operations["listSkinRegistry"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/registry/{id}/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Install an official skin from its pinned package or the bundled copy. */
        post: operations["installOfficialSkin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/registry/{id}/package": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Download the verified bundled archive of an official skin. */
        get: operations["downloadSkinPackage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/registry/{id}/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the preview bitmap of an official skin. */
        get: operations["getSkinPreview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/skins/upload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Validate and install a third-party data-only skin archive. */
        post: operations["uploadSkin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/storage/folders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse directories within a mounted network filesystem. */
        get: operations["listStorageFolders"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/storage/mounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List network filesystems mounted on the server. */
        get: operations["listStorageMounts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List the operations available to this caller. */
        get: operations["listCapabilities"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/hardware": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the hardware report, optionally probing encoders again. */
        get: operations["getHardware"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Check server liveness. */
        get: operations["getHealth"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the server identity, setup status and enabled modules. */
        get: operations["getSystemInfo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the active runtime setting overrides. */
        get: operations["getRuntimeSettings"];
        /** @description Replace server name and performance tier overrides. */
        put: operations["updateRuntimeSettings"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        ApiErrorBody: {
            error: {
                code: components["schemas"]["ApiErrorCode"];
                details?: components["schemas"]["ApiErrorDetails"];
                message: string;
            };
        };
        /** @enum {string} */
        ApiErrorCode: "validation_failed" | "path_not_found" | "unauthorized" | "invalid_credentials" | "forbidden" | "not_found" | "setup_already_completed" | "conflict" | "method_not_allowed" | "internal" | "too_many_requests" | "payload_too_large" | "unavailable" | "busy";
        ApiErrorDetails: {
            field?: string;
            index?: number;
            path?: string;
        };
        BroadcastEntry: {
            current?: null | components["schemas"]["BroadcastProgramme"];
            favorite: boolean;
            group?: string | null;
            id: string;
            next?: null | components["schemas"]["BroadcastProgramme"];
            /** Format: date-time */
            publishedAt?: string | null;
            streams: string[];
            title: string;
        };
        BroadcastEntryPage: {
            entries: components["schemas"]["BroadcastEntry"][];
            /** Format: int32 */
            nextOffset?: number | null;
        };
        BroadcastFavorite: {
            favorite: boolean;
        };
        BroadcastImport: {
            content?: string | null;
            /** Format: int64 */
            expectedVersion: number;
            feedUrl?: string | null;
            guide?: string | null;
            guideUrl?: string | null;
            kind: components["schemas"]["BroadcastKind"];
            name: string;
        };
        /** @enum {string} */
        BroadcastKind: "radio" | "iptv" | "podcast";
        BroadcastProgramme: {
            /** Format: date-time */
            end: string;
            /** Format: date-time */
            start: string;
            title: string;
        };
        BroadcastSource: {
            /** Format: int32 */
            entryCount: number;
            feedUrl?: string | null;
            guideUrl?: string | null;
            id: string;
            kind: components["schemas"]["BroadcastKind"];
            name: string;
            /** Format: int64 */
            version: number;
        };
        BroadcastSourceList: {
            sources: components["schemas"]["BroadcastSource"][];
        };
        Capability: {
            description: string;
            id: string;
            method: string;
            module: string;
            path: string;
            permission: components["schemas"]["Permission"];
            reversible: boolean;
            risk: components["schemas"]["Risk"];
        };
        ConnectionEntry: {
            id: string;
            kind: components["schemas"]["ConnectionEntryKind"];
            name: string;
        };
        /** @enum {string} */
        ConnectionEntryKind: "folder" | "audio" | "video" | "file";
        /** @enum {string} */
        ConnectionKind: "smb" | "webdav" | "jellyfin" | "emby" | "plex";
        ConnectionList: {
            connections: components["schemas"]["ConnectionSummary"][];
        };
        ConnectionPage: {
            entries: components["schemas"]["ConnectionEntry"][];
            /** Format: int32 */
            nextOffset?: number | null;
        };
        ConnectionPlayback: {
            entry: string;
        };
        ConnectionSummary: {
            id: string;
            kind: components["schemas"]["ConnectionKind"];
            name: string;
        };
        CpuInfo: {
            arch: string;
            /** Format: int32 */
            cores: number;
            model: string;
        };
        CreateConnection: {
            address: string;
            kind: components["schemas"]["ConnectionKind"];
            name: string;
            secret?: string | null;
            username?: string | null;
        };
        CreateLibraryRequest: {
            kind: components["schemas"]["LibraryKind"];
            name: string;
            paths: string[];
        };
        /** @enum {string} */
        EncodeMethod: "software" | "vaapi" | "qsv" | "nvenc" | "amf" | "rkmpp" | "v4l2m2m" | "videotoolbox";
        EncoderCapability: {
            /** @description `h264`, `hevc`, `av1`. */
            codec: string;
            /** @description Why the test encode failed or was skipped. */
            error: string | null;
            /** @description ffmpeg encoder name, e.g. `h264_vaapi`, `libx264`. */
            id: string;
            /** @description Present in `ffmpeg -encoders`. */
            listed: boolean;
            method: components["schemas"]["EncodeMethod"];
            /** @description A real test encode succeeded. */
            verified: boolean;
        };
        FavoriteUpdate: {
            favorite: boolean;
        };
        GlobalMusicSearchHit: components["schemas"]["MusicSearchHit"] & {
            libraryId: components["schemas"]["LibraryId"];
            libraryName: string;
        };
        GlobalMusicSearchPage: {
            entries: components["schemas"]["GlobalMusicSearchHit"][];
            nextCursor: string | null;
        };
        HardwareReport: {
            cpu: components["schemas"]["CpuInfo"];
            encoders: components["schemas"]["EncoderCapability"][];
            ffmpeg: components["schemas"]["ToolInfo"];
            ffprobe: components["schemas"]["ToolInfo"];
            hwaccels: string[];
            /** Format: int64 */
            memoryBytes: number;
            perfTier: components["schemas"]["PerfTier"];
            /** Format: date-time */
            probedAt: string;
        };
        Health: {
            /** @constant */
            status: "ok";
        };
        InstalledSkin: components["schemas"]["SkinInfo"] & {
            installedAt: string;
            source: components["schemas"]["SkinSource"];
        };
        Item: {
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["ItemId"];
            kind: components["schemas"]["ItemKind"];
            libraryId: components["schemas"]["LibraryId"];
            /** Format: date-time */
            modifiedAt: string | null;
            parentId: null | components["schemas"]["ItemId"];
            /** @description Absolute path; the API layer clears it for non-admin callers. */
            path: string | null;
            /**
             * Format: int64
             * @description `None` for folders.
             */
            sizeBytes: number | null;
            /** @description File name without extension, or the folder name. */
            title: string;
            /** Format: date-time */
            updatedAt: string;
        };
        /**
         * Format: uuid
         * @description A node in a library's item tree.
         */
        ItemId: string;
        /**
         * @description A folder or media item in a library.
         * @enum {string}
         */
        ItemKind: "folder" | "video" | "track" | "chapter" | "episode" | "photo" | "book" | "comic";
        ItemList: {
            items: components["schemas"]["Item"][];
            nextCursor: string | null;
        };
        Job: {
            /** Format: date-time */
            createdAt: string;
            /** @description Failure reason; only set when `state` is `failed`. */
            error: string | null;
            /** Format: date-time */
            finishedAt: string | null;
            id: components["schemas"]["JobId"];
            /** @description Handler key, e.g. `"library.scan"`. */
            kind: string;
            /** @description English rendering of `step`. */
            message: string | null;
            /**
             * Format: double
             * @description 0.0 to 1.0, or `None` when it cannot be estimated.
             */
            progress: number | null;
            /** Format: date-time */
            startedAt: string | null;
            state: components["schemas"]["JobState"];
            step: null | components["schemas"]["JobStep"];
            subject: null | components["schemas"]["JobSubject"];
        };
        /**
         * Format: uuid
         * @description A background job.
         */
        JobId: string;
        JobList: {
            jobs: components["schemas"]["Job"][];
        };
        /** @enum {string} */
        JobState: "queued" | "running" | "succeeded" | "failed" | "cancelled";
        /**
         * @description Machine-readable description of what a job is doing, for clients to
         *     localise; `Job::message` carries the English rendering of the same step.
         */
        JobStep: {
            code: string;
            params: {
                [key: string]: components["schemas"]["StepValue"];
            };
        };
        /**
         * @description The object a job is about. Fixed when the job is created; `name` is a
         *     snapshot, so it survives deletion of the object.
         */
        JobSubject: {
            id: string;
            name: string | null;
            type: string;
        };
        Library: {
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["LibraryId"];
            /**
             * Format: int64
             * @description Number of live leaf items (folders excluded).
             */
            itemCount: number;
            kind: components["schemas"]["LibraryKind"];
            /** Format: date-time */
            lastScanAt: string | null;
            name: string;
            /** @description Absolute paths on the server's filesystem. */
            paths: string[];
            /** Format: date-time */
            updatedAt: string;
        };
        /**
         * Format: uuid
         * @description A media library.
         */
        LibraryId: string;
        /** @enum {string} */
        LibraryKind: "video" | "music" | "audiobook" | "podcast" | "photo" | "book" | "comic";
        LibraryList: {
            libraries: components["schemas"]["Library"][];
        };
        LoginRequest: {
            deviceName?: string | null;
            password: string;
            username: string;
        };
        LoginResponse: {
            accessToken: string;
            /** Format: date-time */
            expiresAt: string | null;
            /** @constant */
            tokenType: "Bearer";
            user: components["schemas"]["User"];
        };
        LoudnessMeasurement: {
            current: boolean;
            /** Format: double */
            integratedLufs: number | null;
            /** Format: date-time */
            measuredAt: string;
            /** Format: double */
            rangeLu: number | null;
            /** Format: double */
            truePeakDbtp: number | null;
        };
        Lyrics: {
            format: components["schemas"]["LyricsFormat"];
            source: components["schemas"]["LyricsSource"];
            text: string;
        };
        /** @enum {string} */
        LyricsFormat: "lrc" | "plain";
        /** @enum {string} */
        LyricsSource: "sidecar" | "embedded";
        MediaState: {
            completed: boolean;
            favorite: boolean;
            /** Format: date-time */
            lastPlayedAt: string | null;
            /** Format: int64 */
            playCount: number;
            /** Format: int64 */
            playbackVersion: number;
            /** Format: int64 */
            positionMs: number;
            /** Format: int32 */
            stars: number;
        };
        /** @enum {string} */
        MediaView: "favorites" | "recent" | "continue";
        MountList: {
            mounts: components["schemas"]["MountedDirectory"][];
        };
        /** @enum {string} */
        MountProtocol: "smb" | "nfs" | "webdav" | "sshfs" | "rclone" | "network";
        MountedDirectory: {
            path: string;
            protocol: components["schemas"]["MountProtocol"];
            readOnly: boolean;
        };
        MusicEntry: {
            id: string;
            /** @enum {string} */
            kind: "artist";
            name: string | null;
        } | {
            artworkItemId: components["schemas"]["ItemId"];
            id: string;
            /** @enum {string} */
            kind: "album";
            name: string | null;
        } | {
            artist: string | null;
            /** Format: int32 */
            discNumber: number | null;
            /** Format: int64 */
            durationMs: number | null;
            id: components["schemas"]["ItemId"];
            /** @enum {string} */
            kind: "track";
            title: string;
            /** Format: int32 */
            trackNumber: number | null;
        };
        MusicPage: {
            entries: components["schemas"]["MusicEntry"][];
            nextCursor: string | null;
        };
        MusicSearchHit: {
            album: string | null;
            artist: string | null;
            artistId: string | null;
            id: string;
            title: string;
        };
        /** @enum {string} */
        MusicSearchKind: "artist" | "album" | "track";
        MusicSearchPage: {
            entries: components["schemas"]["MusicSearchHit"][];
            nextCursor: string | null;
        };
        /** @enum {string} */
        PerfTier: "tiny" | "lite" | "standard" | "performance";
        /**
         * @description Access level required by the operation's HTTP handler.
         * @enum {string}
         */
        Permission: "public" | "authenticated" | "admin";
        PersonalItem: {
            itemId: components["schemas"]["ItemId"];
            kind: components["schemas"]["ItemKind"];
            libraryId: components["schemas"]["LibraryId"];
            state: components["schemas"]["MediaState"];
            title: string;
        };
        PersonalItemPage: {
            items: components["schemas"]["PersonalItem"][];
            /** Format: int64 */
            nextBefore: number | null;
        };
        PlaybackInfo: {
            /** Format: date-time */
            expiresAt: string;
            mode: components["schemas"]["PlaybackMode"];
            url: string;
        };
        /** @enum {string} */
        PlaybackMode: "direct" | "transcode";
        PlaybackUpdate: {
            completed: boolean;
            /** Format: int64 */
            expectedVersion: number;
            /** Format: int64 */
            positionMs: number;
        };
        Playlist: components["schemas"]["PlaylistSummary"] & {
            trackIds: components["schemas"]["ItemId"][];
        };
        /**
         * Format: uuid
         * @description An ordered personal music playlist.
         */
        PlaylistId: string;
        PlaylistPage: {
            /** Format: int64 */
            nextBefore: number | null;
            playlists: components["schemas"]["PlaylistSummary"][];
        };
        PlaylistSummary: {
            id: components["schemas"]["PlaylistId"];
            name: string;
            /** Format: int64 */
            version: number;
        };
        PlaylistTrack: {
            album: string | null;
            artist: string | null;
            /** Format: int64 */
            durationMs: number | null;
            itemId: components["schemas"]["ItemId"];
            libraryId: components["schemas"]["LibraryId"];
            /** Format: int32 */
            position: number;
            title: string;
        };
        PlaylistTrackPage: {
            /** Format: int32 */
            nextAfter: number | null;
            tracks: components["schemas"]["PlaylistTrack"][];
        };
        PlaylistUpdate: {
            /** Format: int64 */
            expectedVersion: number;
            name: string;
            trackIds: components["schemas"]["ItemId"][];
        };
        PluginList: {
            plugins: components["schemas"]["PluginSummary"][];
        };
        /** @description The contract's `Plugin` shape. */
        PluginSummary: {
            enabled: boolean;
            id: string;
            name: string;
            /** @description Implemented extension points. */
            provides: string[];
            runtime: components["schemas"]["RuntimeKind"];
            version: string;
        };
        RatingUpdate: {
            /** Format: int32 */
            stars: number;
        };
        RegistrySkin: components["schemas"]["SkinInfo"] & {
            downloadUrl: string;
            installed: boolean;
            publishedAt: string;
            sha256: string;
        };
        /** @enum {string} */
        Risk: "read" | "write" | "destructive";
        /** @enum {string} */
        Role: "admin" | "user";
        /** @enum {string} */
        RuntimeKind: "none" | "recipe" | "wasm" | "sidecar";
        /** @description Administrator overrides. Null restores boot configuration or host detection. */
        RuntimeSettings: {
            perfTier: null | components["schemas"]["PerfTier"];
            serverName: string | null;
        };
        ScanAccepted: {
            jobId: components["schemas"]["JobId"];
        };
        ScanReview: {
            /** Format: int64 */
            missingCount: number;
            revision: string;
            samplePaths: string[];
            scope: string;
        };
        ScanReviewList: {
            reviews: components["schemas"]["ScanReview"][];
        };
        ScanReviewPaths: {
            paths: string[];
            /** Format: int64 */
            total: number;
        };
        ServerEventMap: {
            "job.updated": components["schemas"]["Job"];
            "library.deleted": {
                /** Format: uuid */
                id: string;
            };
            "library.updated": components["schemas"]["Library"];
            ping: Record<string, never>;
            "settings.updated": components["schemas"]["RuntimeSettings"];
            "stream.reset": Record<string, never>;
        };
        /**
         * Format: uuid
         * @description This server installation; generated on first start and never changed.
         */
        ServerId: string;
        SetupRequest: {
            password: string;
            username: string;
        };
        SetupResponse: {
            user: components["schemas"]["User"];
        };
        SkinInfo: {
            description: {
                [key: string]: string;
            };
            id: string;
            name: {
                [key: string]: string;
            };
            pluginId: string;
            previewUrl: string | null;
            sizeBytes: number;
            version: string;
        };
        SkinManifest: {
            author?: string;
            colors: Record<string, never>;
            description?: {
                [key: string]: string;
            };
            elevation: Record<string, never>;
            fonts?: Record<string, never>[];
            /** @constant */
            format: 1;
            id: string;
            motion: Record<string, never>;
            name: {
                [key: string]: string;
            };
            shape: Record<string, never>;
            slots: Record<string, never>;
            type: Record<string, never>;
            work: Record<string, never>;
        };
        /** @enum {string} */
        SkinSource: "official" | "upload";
        SpokenEntry: {
            id: string;
            /** @enum {string} */
            kind: "book";
            title: string;
        } | {
            id: string;
            /** @enum {string} */
            kind: "show";
            title: string;
        } | (components["schemas"]["SpokenRecording"] & {
            /** @enum {string} */
            kind: "chapter";
        }) | (components["schemas"]["SpokenRecording"] & {
            /** @enum {string} */
            kind: "episode";
        });
        SpokenPage: {
            entries: components["schemas"]["SpokenEntry"][];
            nextCursor: string | null;
        };
        SpokenRecording: {
            artist: string | null;
            /** Format: int32 */
            discNumber: number | null;
            /** Format: int64 */
            durationMs: number | null;
            id: components["schemas"]["ItemId"];
            /** Format: int32 */
            number: number | null;
            title: string;
        };
        /** @description A step parameter: the contract allows strings and numbers only. */
        StepValue: number | string;
        StorageFolders: {
            directories: string[];
            truncated: boolean;
        };
        SubsonicPassword: {
            password: string;
            username: string;
        };
        SubsonicPasswordStatus: {
            enabled: boolean;
            personal: boolean;
        };
        /** @enum {string} */
        SubtitleFormat: "srt" | "vtt";
        SubtitleList: {
            tracks: components["schemas"]["SubtitleTrack"][];
            truncated: boolean;
        };
        SubtitleTrack: {
            forced: boolean;
            format: components["schemas"]["SubtitleFormat"];
            hearingImpaired: boolean;
            id: string;
            language: string | null;
        };
        SystemInfo: {
            /** @description Capabilities registered at startup; clients hide missing modules. */
            features: string[];
            name: string;
            serverId: components["schemas"]["ServerId"];
            setupCompleted: boolean;
            version: string;
        };
        ToolInfo: {
            found: boolean;
            path: string | null;
            source: null | components["schemas"]["ToolSource"];
            version: string | null;
        };
        /** @enum {string} */
        ToolSource: "config" | "bundled" | "path";
        TrackLoudness: {
            measurement: null | components["schemas"]["LoudnessMeasurement"];
        };
        TrackLyrics: {
            lyrics: null | components["schemas"]["Lyrics"];
        };
        UpdateLibraryRequest: {
            addPaths?: string[];
            name?: string | null;
        };
        User: {
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["UserId"];
            role: components["schemas"]["Role"];
            username: string;
        };
        /**
         * Format: uuid
         * @description A user account.
         */
        UserId: string;
        VideoEntry: {
            /** Format: int32 */
            episodeNumber: number | null;
            /** Format: int64 */
            fileCount: number;
            id: string;
            itemId: null | components["schemas"]["ItemId"];
            kind: components["schemas"]["VideoKind"];
            overview: string | null;
            posterItemId: null | components["schemas"]["ItemId"];
            /** Format: int32 */
            seasonNumber: number | null;
            title: string;
            /** Format: int32 */
            year: number | null;
        };
        /** @enum {string} */
        VideoKind: "movie" | "series" | "season" | "episode";
        VideoPage: {
            entries: components["schemas"]["VideoEntry"][];
            nextCursor: string | null;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginResponse"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Too Many Requests */
            429: {
                headers: {
                    "Retry-After"?: number;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getCurrentUser: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listBroadcastSources: {
        parameters: {
            query?: {
                kind?: components["schemas"]["BroadcastKind"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BroadcastSourceList"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    saveBroadcastSource: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BroadcastImport"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BroadcastSource"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Payload Too Large */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    removeBroadcastSource: {
        parameters: {
            query: {
                version: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listBroadcastEntries: {
        parameters: {
            query: {
                version: number;
                offset?: number;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BroadcastEntryPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    setBroadcastFavorite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                entryId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BroadcastFavorite"];
            };
        };
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listConnections: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConnectionList"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    createConnection: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateConnection"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConnectionSummary"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Bad Gateway */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    removeConnection: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    browseConnection: {
        parameters: {
            query?: {
                parent?: string;
                offset?: number;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConnectionPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Bad Gateway */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    prepareConnectionPlayback: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConnectionPlayback"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaybackInfo"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Bad Gateway */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    streamConnection: {
        parameters: {
            query: {
                ticket?: string;
                entry: string;
            };
            header?: {
                Range?: string;
            };
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Length"?: number;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
                    "audio/*": string;
                    "video/*": string;
                };
            };
            /** @description Partial content */
            206: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Length"?: number;
                    "Content-Range"?: string;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
                    "audio/*": string;
                    "video/*": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unsatisfiable byte range */
            416: {
                headers: {
                    "Content-Range"?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Bad Gateway */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    streamEvents: {
        parameters: {
            query?: never;
            header?: {
                "Last-Event-ID"?: string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/event-stream": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listItems: {
        parameters: {
            query: {
                libraryId: string;
                parentId?: string | null;
                kind?: components["schemas"]["ItemKind"];
                cursor?: string | null;
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ItemList"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getItem: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Item"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getMusicArtwork: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "image/jpeg": string;
                    "image/png": string;
                    "image/webp": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    streamAudio: {
        parameters: {
            query: {
                ticket?: string;
                format: "mp3" | "aac" | "opus";
                bitrateKbps?: number;
                startMs?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Accept-Ranges"?: "none";
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "audio/mp4": string;
                    "audio/mpeg": string;
                    "audio/ogg": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unprocessable Entity */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    headAudio: {
        parameters: {
            query: {
                ticket?: string;
                format: "mp3" | "aac" | "opus";
                bitrateKbps?: number;
                startMs?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Accept-Ranges"?: "none";
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    getTrackLoudness: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TrackLoudness"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getTrackLyrics: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TrackLyrics"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    preparePlayback: {
        parameters: {
            query?: {
                format?: "original" | "mp3" | "aac" | "opus";
                bitrateKbps?: number;
                startMs?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaybackInfo"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getVideoPoster: {
        parameters: {
            query?: {
                level?: "work" | "season" | "episode";
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "image/jpeg": string;
                    "image/png": string;
                    "image/webp": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    streamTrack: {
        parameters: {
            query?: {
                ticket?: string;
            };
            header?: {
                Range?: string;
                "If-Range"?: string;
            };
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Length"?: number;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
                    "audio/*": string;
                    "video/*": string;
                };
            };
            /** @description Partial content */
            206: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Length"?: number;
                    "Content-Range"?: string;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
                    "audio/*": string;
                    "video/*": string;
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unsatisfiable byte range */
            416: {
                headers: {
                    "Content-Range"?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    headTrack: {
        parameters: {
            query?: {
                ticket?: string;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Length"?: number;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    listVideoSubtitles: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SubtitleList"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    readVideoSubtitle: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                track: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/vtt": string;
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listJobs: {
        parameters: {
            query?: {
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobList"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    cancelJob: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listLibraries: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryList"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    createLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateLibraryRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Library"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Library"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    deleteLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    updateLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateLibraryRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Library"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    browseMusic: {
        parameters: {
            query?: {
                artist?: string;
                album?: string;
                cursor?: string;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MusicPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    analyzeLoudness: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanAccepted"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    searchMusic: {
        parameters: {
            query: {
                q: string;
                kind?: components["schemas"]["MusicSearchKind"];
                cursor?: string;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MusicSearchPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    scanLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanAccepted"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listScanReviews: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanReviewList"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    confirmScanReview: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                revision: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanAccepted"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listScanReviewPaths: {
        parameters: {
            query?: {
                offset?: number;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
                revision: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanReviewPaths"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    browseSpoken: {
        parameters: {
            query?: {
                collection?: string;
                cursor?: string;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SpokenPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    browseVideos: {
        parameters: {
            query?: {
                q?: string;
                kind?: "movie" | "series";
                order?: "title" | "poster";
                season?: number;
                series?: string;
                cursor?: string;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VideoPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listPersonalItems: {
        parameters: {
            query: {
                before?: number;
                view: components["schemas"]["MediaView"];
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PersonalItemPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getPersonalItem: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaState"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    setFavorite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FavoriteUpdate"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaState"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    reportPlayback: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PlaybackUpdate"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaState"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    setRating: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RatingUpdate"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaState"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listPlaylists: {
        parameters: {
            query?: {
                before?: number;
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaylistPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getPlaylist: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Playlist"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    savePlaylist: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PlaylistUpdate"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Playlist"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    deletePlaylist: {
        parameters: {
            query: {
                version: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listPlaylistTracks: {
        parameters: {
            query: {
                version: number;
                after?: number;
                limit?: number;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaylistTrackPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getSubsonicPasswordStatus: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SubsonicPasswordStatus"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    issueSubsonicPassword: {
        parameters: {
            query?: {
                personal?: boolean;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SubsonicPassword"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    revokeSubsonicPassword: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    searchAllMusic: {
        parameters: {
            query: {
                q: string;
                kind?: components["schemas"]["MusicSearchKind"];
                cursor?: string;
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GlobalMusicSearchPage"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getOpenApi: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        [key: string]: unknown;
                    };
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listPlugins: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginList"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    createAdministrator: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetupRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SetupResponse"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listInstalledSkins: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        skins: components["schemas"]["InstalledSkin"][];
                    };
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    removeSkin: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getSkinAsset: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                path: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "font/woff2": string;
                    "image/avif": string;
                    "image/jpeg": string;
                    "image/png": string;
                    "image/webp": string;
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getSkinManifest: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SkinManifest"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listSkinRegistry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        skins: components["schemas"]["RegistrySkin"][];
                    };
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    installOfficialSkin: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/x-tar": string;
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledSkin"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Payload Too Large */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    downloadSkinPackage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/x-tar": string;
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getSkinPreview: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "image/png": string;
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    uploadSkin: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/x-tar": string;
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledSkin"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Conflict */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Payload Too Large */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Service Unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listStorageFolders: {
        parameters: {
            query: {
                mount: string;
                folder?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageFolders"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Not Found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listStorageMounts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MountList"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    listCapabilities: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        capabilities: components["schemas"]["Capability"][];
                    };
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getHardware: {
        parameters: {
            query?: {
                refresh?: boolean;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HardwareReport"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getHealth: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Health"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getSystemInfo: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SystemInfo"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    getRuntimeSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeSettings"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    updateRuntimeSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RuntimeSettings"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeSettings"];
                };
            };
            /** @description Bad Request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Unauthorized */
            401: {
                headers: {
                    "WWW-Authenticate"?: "Bearer";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Method Not Allowed */
            405: {
                headers: {
                    Allow?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Internal Server Error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
}
