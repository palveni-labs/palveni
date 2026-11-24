export interface paths {
    "/api/v1/artwork/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read cover artwork bytes for a catalog work or item. */
        get: operations["getArtwork"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the configured AI model provider preset, endpoint, model and available presets. */
        get: operations["getAssistantConfig"];
        /** @description Configure the AI model provider and encrypt its API key. */
        put: operations["updateAssistantConfig"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/conversation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read your active AI assistant conversation and its visual runs. */
        get: operations["getAssistantConversation"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/conversation/reset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Archive your current conversation and start a fresh one. */
        post: operations["resetAssistantConversation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/memories": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your visible assistant preference memories. */
        get: operations["listAssistantMemories"];
        put?: never;
        /** @description Save a visible assistant preference memory. */
        post: operations["createAssistantMemory"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/memories/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a saved assistant preference memory. */
        delete: operations["deleteAssistantMemory"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/messages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Send a prompt to the AI assistant and receive declarative result blocks or an approval preview. */
        post: operations["sendAssistantMessage"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/runs/{id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Approve and execute the selected rows of a pending assistant change preview. */
        post: operations["approveAssistantRun"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/runs/{id}/background": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Link an assistant run into the unified Tasks list. */
        post: operations["backgroundAssistantRun"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/runs/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Cancel a pending assistant run without applying changes. */
        post: operations["cancelAssistantRun"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assistant/suggestions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read contextual assistant prompt suggestions derived from live catalog state. */
        get: operations["listAssistantSuggestions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
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
    "/api/v1/catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read recently added works and per-library cover mosaic IDs. */
        get: operations["getCatalog"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/collections": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your cross-media collections. */
        get: operations["listCollections"];
        put?: never;
        /** @description Create a cross-media collection spanning video, music, books, comics and photos. */
        post: operations["createCollection"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/collections/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a cross-media collection with its manual items and automatically discovered related works. */
        get: operations["getCollection"];
        put?: never;
        post?: never;
        /** @description Delete a cross-media collection while keeping all underlying media items. */
        delete: operations["deleteCollection"];
        options?: never;
        head?: never;
        /** @description Update a cross-media collection title, description, cover or item order. */
        patch: operations["updateCollection"];
        trace?: never;
    };
    "/api/v1/collections/{id}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Add a video work, music album, track, book, comic or photo to a collection. */
        post: operations["addCollectionItem"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/collections/{id}/items/{entryId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Remove an entry from a cross-media collection. */
        delete: operations["removeCollectionItem"];
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
    "/api/v1/downloads": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read configured external downloader clients, live transfer queue and RSS media subscriptions. */
        get: operations["getDownloadsOverview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/clients": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Register or update a qBittorrent, Transmission or aria2 external downloader client with encrypted credentials. */
        post: operations["saveDownloaderClient"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/clients/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Remove an external downloader client and its encrypted credentials. */
        delete: operations["deleteDownloaderClient"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/subscriptions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Create an RSS or Atom media subscription rule. */
        post: operations["createMediaSubscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/subscriptions/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete one of your media subscriptions. */
        delete: operations["deleteMediaSubscription"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/subscriptions/{id}/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Fetch the subscription's RSS or Atom feed and match items against title and quality rules. */
        post: operations["checkMediaSubscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/downloads/tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Submit a Magnet link or HTTP(S) media URI to an enabled external downloader client. */
        post: operations["createDownloadTask"];
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
    "/api/v1/home": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read smart-home rooms, controllable devices, scenes and media-playback linkage rules. */
        get: operations["getHomeSnapshot"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/bridges": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List configured Home Assistant bridges. */
        get: operations["listHomeBridges"];
        put?: never;
        /** @description Register or update a Home Assistant bridge with an encrypted long-lived access token. */
        post: operations["saveHomeBridge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/bridges/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Remove a Home Assistant bridge and its synced devices. */
        delete: operations["deleteHomeBridge"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/bridges/{id}/sync": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Pull live entity states from Home Assistant and sync supported devices and scenes. */
        post: operations["syncHomeBridge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/devices": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Create a controllable local or virtual smart-home device. */
        post: operations["createHomeDevice"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/devices/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Update a smart-home device state and forward to its Home Assistant bridge when enabled. */
        post: operations["updateHomeDevice"];
        /** @description Remove a smart-home device. */
        delete: operations["deleteHomeDevice"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/mqtt": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List configured MQTT 3.1.1 bridges and ambient light bindings. */
        get: operations["listMqttBridges"];
        put?: never;
        /** @description Register or update an MQTT 3.1.1 broker bridge with encrypted password and optional ambient light binding. */
        post: operations["saveMqttBridge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/mqtt/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a configured MQTT bridge. */
        delete: operations["deleteMqttBridge"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/mqtt/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Connect to an MQTT 3.1.1 broker and publish a test status message. */
        post: operations["testMqttBridge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Create a media-playback smart-home automation rule. */
        post: operations["createHomeRule"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Enable or disable a media-playback automation rule. */
        post: operations["setHomeRule"];
        /** @description Remove a media-playback automation rule. */
        delete: operations["deleteHomeRule"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/home/scenes/{id}/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Activate a smart-home scene and apply its preset device actions. */
        post: operations["activateHomeScene"];
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
    "/api/v1/items/{id}/backdrop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a local fanart or backdrop image for an indexed film or series. */
        get: operations["getVideoBackdrop"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/book-download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Download an indexed ebook in its original format. */
        get: operations["downloadBook"];
        put?: never;
        /** @description Prepare a download of an original ebook. */
        post: operations["prepareBookDownload"];
        delete?: never;
        options?: never;
        /** @description Read the size and download name of an indexed ebook. */
        head: operations["headBook"];
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/chapters": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read embedded chapter markers for an indexed audiobook recording. */
        get: operations["listSpokenChapters"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
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
    "/api/v1/items/{id}/media-info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read codec, stream, HDR and chapter details for an indexed video. */
        get: operations["getVideoMediaInfo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/metadata-override": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the saved manual or scraped metadata override for an item. */
        get: operations["getItemMetadataOverride"];
        /** @description Save a metadata override for an item and sync display fields to the library. */
        put: operations["saveItemMetadataOverride"];
        post?: never;
        /** @description Delete a saved metadata override for an item. */
        delete: operations["deleteItemMetadataOverride"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/photo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an auto-oriented thumbnail, preview or original photo file. */
        get: operations["getPhotoImage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/photo-info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read full EXIF metadata, file details, favorite state and albums for a photo. */
        get: operations["getPhotoInfo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/photo-people": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your tagged people on a specific photo. */
        get: operations["listPhotoItemPeople"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/photo-people/{personId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Tag a person on a photo and automatically assign its cover if empty. */
        post: operations["tagPhotoPerson"];
        /** @description Remove a person tag from a photo. */
        delete: operations["untagPhotoPerson"];
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
    "/api/v1/items/{id}/reading-asset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a validated package-internal image asset from an EPUB. */
        get: operations["getReadingAsset"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/reading-chapter/{index}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a sanitized chapter from an ebook or text document. */
        get: operations["getReadingChapter"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/reading-cover": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the cover image of a book or comic. */
        get: operations["getReadingCover"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/reading-inline": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Stream an indexed PDF inline with HTTP byte-range support. */
        get: operations["streamReadingInline"];
        put?: never;
        /** @description Issue a short-lived inline PDF URL for browser rendering. */
        post: operations["prepareReadingInline"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/reading-manifest": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read book or comic metadata, chapters, reading progress and bookmarks. */
        get: operations["getReadingManifest"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/reading-page/{page}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a 0-based page image from a comic archive. */
        get: operations["getReadingPage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/segments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read intro, credits and recap skip markers for a video, falling back to chapter inference. */
        get: operations["listVideoSegments"];
        /** @description Save intro, credits and recap skip markers for a video. */
        put: operations["saveVideoSegments"];
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
        /** @description List sidecar and embedded text subtitles for an indexed video. */
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
        /** @description Read one sidecar or embedded text subtitle track as WebVTT. */
        get: operations["readVideoSubtitle"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/thumb": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a local episode still or a cached frame grabbed from the video. */
        get: operations["getVideoThumb"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/trickplay": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read an on-demand cached JPEG timeline scrubbing preview frame at a given millisecond offset. */
        get: operations["getVideoTrickplay"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/items/{id}/video-playback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Choose direct, remux or transcoded video delivery and start an HLS session when needed. */
        post: operations["startVideoPlayback"];
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
    "/api/v1/libraries/{id}/music/albums": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List albums in a music library with filtering, sorting and per-user state. */
        get: operations["listMusicAlbums"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/albums/{albumId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a music album and its tracks ordered by disc and track number. */
        get: operations["getMusicAlbum"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/albums/{albumId}/favorite": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Add a music album to your favorites. */
        put: operations["favoriteMusicAlbum"];
        post?: never;
        /** @description Remove a music album from your favorites. */
        delete: operations["unfavoriteMusicAlbum"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/artists": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List album artists in a music library with filtering, sorting and per-user state. */
        get: operations["listMusicArtists"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/artists/{artistId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a music artist, their albums by release year and top tracks. */
        get: operations["getMusicArtist"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/artists/{artistId}/favorite": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Add a music artist to your favorites. */
        put: operations["favoriteMusicArtist"];
        post?: never;
        /** @description Remove a music artist from your favorites. */
        delete: operations["unfavoriteMusicArtist"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/music/genres": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List music genres in a library with album and track counts. */
        get: operations["listMusicGenres"];
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
    "/api/v1/libraries/{id}/music/tracks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List tracks in a music library with filtering, sorting and per-user state. */
        get: operations["listMusicTracks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/photos/upload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Upload a photo into a writable photo library and index its EXIF metadata immediately. */
        post: operations["uploadPhoto"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/reading": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse books or comics in a library with filtering and sorting. */
        get: operations["listReadingItems"];
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
    "/api/v1/libraries/{id}/videos/genres": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List distinct genres and work counts in a video library. */
        get: operations["listVideoGenres"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/videos/people/{key}/image": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a local portrait image for a cast or crew member. */
        get: operations["getVideoPersonImage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/videos/works/{workId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read full metadata, cast, seasons, files, next-up and similar works for a film or series. */
        get: operations["getVideoWork"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/videos/works/{workId}/favorite": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Add a film or series to your favorites. */
        put: operations["favoriteVideoWork"];
        post?: never;
        /** @description Remove a film or series from your favorites. */
        delete: operations["unfavoriteVideoWork"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/libraries/{id}/videos/works/{workId}/played": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Mark an entire film or series as played. */
        put: operations["markVideoWorkPlayed"];
        post?: never;
        /** @description Mark an entire film or series as unplayed. */
        delete: operations["markVideoWorkUnplayed"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/link/devices": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read this account's paired devices and their current connection state. */
        get: operations["listLinkDevices"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/link/devices/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Disconnect a paired device and revoke its sessions. */
        delete: operations["revokeLinkDevice"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/link/pairing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Pair this authenticated device with its Ed25519 public key. */
        put: operations["pairLinkDevice"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/link/pairing/challenge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Issue a short-lived proof-of-possession challenge for this device. */
        post: operations["createLinkChallenge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/mcp": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Handle a JSON-RPC 2.0 Model Context Protocol Streamable HTTP request or notification. */
        post: operations["postMcp"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/mcp/info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the built-in MCP Streamable HTTP endpoint info, exposed tools and client configuration templates. */
        get: operations["getMcpInfo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/api-keys": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your active client API keys for Jellyfin, WebDAV and third-party apps. */
        get: operations["listClientApiKeys"];
        put?: never;
        /** @description Create a personal client API key and reveal its secret token once. */
        post: operations["createClientApiKey"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/api-keys/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Revoke a personal client API key immediately. */
        delete: operations["revokeClientApiKey"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/bookmarks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete one of your reading bookmarks. */
        delete: operations["deleteReadingBookmark"];
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
    "/api/v1/me/items/{id}/bookmarks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Create a personal bookmark on a book or comic. */
        post: operations["createReadingBookmark"];
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
    "/api/v1/me/items/{id}/played": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Mark a track, movie file or episode as played. */
        put: operations["markItemPlayed"];
        post?: never;
        /** @description Mark a track, movie file or episode as unplayed. */
        delete: operations["markItemUnplayed"];
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
    "/api/v1/me/items/{id}/reading-progress": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Save reading page, locator and completion state. */
        put: operations["saveReadingProgress"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/opds-access": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Get OPDS reader credentials scoped to the current session. */
        get: operations["getOpdsAccess"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Verify your current password, set a new password and optionally revoke your other sessions. */
        put: operations["changeMyPassword"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/photo-albums": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your personal photo albums. */
        get: operations["listPhotoAlbums"];
        put?: never;
        /** @description Create a personal photo album. */
        post: operations["createPhotoAlbum"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/photo-albums/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read a personal photo album summary. */
        get: operations["getPhotoAlbum"];
        put?: never;
        post?: never;
        /** @description Delete a personal photo album while keeping the underlying photos. */
        delete: operations["deletePhotoAlbum"];
        options?: never;
        head?: never;
        /** @description Rename a personal photo album or update its description and cover. */
        patch: operations["updatePhotoAlbum"];
        trace?: never;
    };
    "/api/v1/me/photo-albums/{id}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Add photos to a personal photo album. */
        post: operations["addPhotoAlbumItems"];
        /** @description Remove photos from a personal photo album. */
        delete: operations["removePhotoAlbumItems"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/photo-people": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your personal photo people profiles with photo counts and cover artwork. */
        get: operations["listPhotoPeople"];
        put?: never;
        /** @description Create a personal person profile for photo tagging. */
        post: operations["createPhotoPerson"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/photo-people/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a photo person profile and its tags while keeping all photos. */
        delete: operations["deletePhotoPerson"];
        options?: never;
        head?: never;
        /** @description Rename a photo person profile or change its cover photo. */
        patch: operations["updatePhotoPerson"];
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
    "/api/v1/me/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your active device login sessions. */
        get: operations["listMySessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/sessions/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Revoke one of your active login sessions. */
        delete: operations["revokeMySession"];
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
    "/api/v1/metadata/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read online metadata scraper configuration status without revealing secrets. */
        get: operations["getMetadataConfig"];
        /** @description Update encrypted TMDB/Bangumi credentials and metadata scraper preferences. */
        put: operations["updateMetadataConfig"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/metadata/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Search TMDB, Bangumi or OpenLibrary for normalized media metadata candidates. */
        post: operations["searchMetadata"];
        delete?: never;
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
    "/api/v1/music/tracks/lookup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Look up up to 500 music tracks by ID in the requested order. */
        post: operations["lookupMusicTracks"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/notifications": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your in-app notifications and recent channel delivery logs. */
        get: operations["listNotifications"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/notifications/channels": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your configured external notification push channels. */
        get: operations["listNotificationChannels"];
        put?: never;
        /** @description Register an external push channel with encrypted credentials and optional quiet hours. */
        post: operations["createNotificationChannel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/notifications/channels/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a notification push channel. */
        delete: operations["deleteNotificationChannel"];
        options?: never;
        head?: never;
        /** @description Update a notification channel's name, credentials, event filters, quiet hours or enabled state. */
        patch: operations["updateNotificationChannel"];
        trace?: never;
    };
    "/api/v1/notifications/channels/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Send an immediate test notification through a channel and record the delivery outcome. */
        post: operations["testNotificationChannel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/notifications/read-all": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Mark all unread in-app notifications as read. */
        post: operations["markAllNotificationsRead"];
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
    "/api/v1/organizer/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Rename or hardlink selected media files strictly within their library root and record an undoable batch. */
        post: operations["applyMediaOrganize"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizer/batches": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List recent media organize batches and naming presets. */
        get: operations["listOrganizeBatches"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizer/batches/{id}/undo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Atomically reverse a previously applied organize batch on disk and in the database. */
        post: operations["undoOrganizeBatch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizer/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Dry-run a naming template across a library's indexed files without modifying disk. */
        post: operations["previewMediaOrganize"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/photos/analyze": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Run pure-local perceptual dHash, dominant color and scene analysis on unanalyzed accessible photos. */
        post: operations["analyzePhotos"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/photos/duplicates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Group exact and perceptually similar duplicate photos with keep recommendations and reclaimable space. */
        get: operations["listPhotoDuplicates"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/photos/memories": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read on-this-day memories, recent highlights, place clusters and camera stats. */
        get: operations["getPhotoMemories"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/photos/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Search accessible photos with bilingual natural-language queries across time, season, color, camera, albums and people. */
        get: operations["searchPhotos"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/photos/timeline": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Browse photos chronologically across libraries or within an album. */
        get: operations["getPhotoTimeline"];
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
        /** @description List installed plugins including host modules, skins and recipe/widget store installations. */
        get: operations["listPlugins"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/plugins/catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List the curated official recipe and widget plugin catalog with risk levels and schemas. */
        get: operations["getPluginCatalog"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/plugins/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Review permissions, validate JSON Schema settings and install an official recipe or widget plugin. */
        post: operations["installPlugin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/plugins/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Uninstall a database-managed recipe or widget plugin. */
        delete: operations["uninstallPlugin"];
        options?: never;
        head?: never;
        /** @description Enable or disable an installed plugin and update its validated JSON Schema settings. */
        patch: operations["updatePlugin"];
        trace?: never;
    };
    "/api/v1/plugins/{id}/rollback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Roll back an installed plugin to its previous version. */
        post: operations["rollbackPlugin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/shares/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Resolve a public share link or inspect its password-locked prompt. */
        get: operations["getPublicShare"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/shares/{token}/cover/{itemId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read cover, poster or thumbnail artwork for a shared media item. */
        get: operations["getPublicShareCover"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/shares/{token}/media/{itemId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Stream or view a shared media item, stripping EXIF GPS data when configured. */
        get: operations["streamPublicShareMedia"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/shares/{token}/unlock": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Verify a share password and issue a short-lived signed proof ticket. */
        post: operations["unlockPublicShare"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reading/home": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read continue-reading, recent books, recent comics and series shelves. */
        get: operations["getReadingHome"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/related": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Discover matching works across film/series, soundtrack albums, books and comics by normalized title stem. */
        get: operations["listRelatedWorks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/routines": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your scheduled and event-driven automation routines. */
        get: operations["listRoutines"];
        put?: never;
        /** @description Create an hourly, daily, weekly or event-driven routine. */
        post: operations["createRoutine"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/routines/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a scheduled routine. */
        delete: operations["deleteRoutine"];
        options?: never;
        head?: never;
        /** @description Update a scheduled routine's schedule, action or enabled state. */
        patch: operations["updateRoutine"];
        trace?: never;
    };
    "/api/v1/routines/{id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Execute a scheduled routine immediately and record its run status. */
        post: operations["runRoutineNow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Search across video works, music albums and tracks, books, comics, photos, broadcast, collections and smart home. */
        get: operations["globalSearch"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sessions/playback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List active playback sessions across your devices (or all users for administrators with all=true). */
        get: operations["listPlaybackSessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sessions/playback/heartbeat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Report live playback state for a client session and dispatch MQTT and ambient light updates. */
        post: operations["heartbeatPlaybackSession"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sessions/playback/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Remove an active playback session. */
        delete: operations["deletePlaybackSession"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sessions/playback/{id}/command": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Send a remote playback command (play, pause, toggle, seek, next, prev, volume, play_item) to a live session over SSE. */
        post: operations["sendPlaybackCommand"];
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
    "/api/v1/shares": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List your share links or all server share links when signed in as an administrator. */
        get: operations["listShares"];
        put?: never;
        /** @description Create a public or password-protected share link with optional expiry, view cap and GPS stripping. */
        post: operations["createShare"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Revoke a share link immediately. */
        delete: operations["revokeShare"];
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
    "/api/v1/sync/changes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Fetch incremental library, item, personal state, playlist and bookmark deltas since a sync revision. */
        get: operations["getSyncChanges"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sync/push": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Apply idempotent offline client mutations with server revision conflict detection. */
        post: operations["pushSyncMutations"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/audit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List paginated security audit log entries enriched with actor usernames. */
        get: operations["listAuditLogs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/backups": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List portable database snapshot backups. */
        get: operations["listBackups"];
        put?: never;
        /** @description Create a portable SHA-256-verified database snapshot backup. */
        post: operations["createBackup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/backups/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a stored database snapshot backup. */
        delete: operations["deleteBackup"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/backups/{id}/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Download a portable database snapshot backup as a JSON attachment. */
        get: operations["downloadBackup"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/backups/{id}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Verify SHA-256 integrity and idempotently restore a portable snapshot backup. */
        post: operations["restoreBackup"];
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
    "/api/v1/system/doctor": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Run structured diagnostics across database, FFmpeg, storage, libraries and protocol surfaces. */
        get: operations["getSystemDoctor"];
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
    "/api/v1/system/import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Preview or import watch/listen history, favorites and playlists from Jellyfin, Emby, Navidrome or M3U. */
        post: operations["importExternalState"];
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
    "/api/v1/system/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read recent runtime logs merged with audit and background job events. */
        get: operations["getSystemLogs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read live CPU load, memory usage and connected client session count. */
        get: operations["getSystemMetrics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/prometheus": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Export live Palveni Server metrics in Prometheus 0.0.4 exposition text format. */
        get: operations["getSystemPrometheus"];
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
    "/api/v1/system/storage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read disk volumes, per-library byte totals and network mount reachability. */
        get: operations["getStorageReport"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/system/storage/mounts/{id}/reconnect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Re-probe a storage mount and refresh its online state. */
        post: operations["reconnectStorageMount"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List unified background jobs and assistant approval tasks. */
        get: operations["listTasks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tasks/{id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Approve and execute a task awaiting confirmation. */
        post: operations["approveTask"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tasks/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Cancel a queued, running or awaiting-approval task. */
        post: operations["cancelTask"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List family accounts with per-user policies, allowed libraries and active session counts. */
        get: operations["listUsers"];
        put?: never;
        /** @description Create a family member or administrator account with optional access policies. */
        post: operations["createUser"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a user account and revoke all of its sessions. */
        delete: operations["deleteUser"];
        options?: never;
        head?: never;
        /** @description Update an account's role, disabled state, password, capability policy or library access list. */
        patch: operations["updateUser"];
        trace?: never;
    };
    "/api/v1/video-sessions/{sessionId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Stop an active video HLS session and remove its temporary segments. */
        delete: operations["deleteVideoSession"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/video-sessions/{sessionId}/master.m3u8": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read the live HLS media playlist for an active video session. */
        get: operations["getVideoMasterPlaylist"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/video-sessions/{sessionId}/segment/{file}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read one initialization or media segment from an active video HLS session. */
        get: operations["getVideoSegment"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/videos/home": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Read continue-watching, next-up, latest movies and latest episodes rows. */
        get: operations["getVideoHome"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/virtual-channels": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List 24x7 virtual live television channels. */
        get: operations["listVirtualChannels"];
        put?: never;
        /** @description Create a 24x7 virtual television channel backed by a video library and optional genre filter. */
        post: operations["createVirtualChannel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/virtual-channels/epg.xml": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Export an XMLTV programme guide covering the live show and upcoming 12 hours. */
        get: operations["getVirtualTvEpg"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/virtual-channels/guide": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Compute the clock-synced live programme and 12-hour EPG schedule for all enabled virtual channels. */
        get: operations["getVirtualTvGuide"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/virtual-channels/playlist.m3u": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Export an M3U playlist with signed stream URLs for IPTV clients. */
        get: operations["getVirtualTvPlaylist"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/virtual-channels/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Delete a virtual television channel. */
        delete: operations["deleteVirtualChannel"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/webhooks/inbound": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description List registered inbound media automation webhooks. */
        get: operations["listInboundWebhooks"];
        put?: never;
        /** @description Create an inbound webhook for Sonarr, Radarr, MoviePilot or qBittorrent and return its token once. */
        post: operations["createInboundWebhook"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/webhooks/inbound/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Revoke and delete an inbound automation webhook. */
        delete: operations["deleteInboundWebhook"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/webhooks/inbound/{token}/trigger": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Receive an automation event from Sonarr, Radarr, MoviePilot or qBittorrent and queue library scans. */
        post: operations["triggerInboundWebhook"];
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
        AddMediaCollectionItemRequest: {
            artworkId?: string | null;
            entryKind: components["schemas"]["MediaCollectionEntryKind"];
            libraryId?: null | components["schemas"]["LibraryId"];
            subtitle?: string | null;
            targetId: string;
            title?: string | null;
        };
        ApiErrorBody: {
            error: {
                code: components["schemas"]["ApiErrorCode"];
                details?: components["schemas"]["ApiErrorDetails"];
                message: string;
            };
        };
        /** @enum {string} */
        ApiErrorCode: "validation_failed" | "path_not_found" | "unauthorized" | "invalid_credentials" | "forbidden" | "not_found" | "setup_already_completed" | "conflict" | "method_not_allowed" | "internal" | "too_many_requests" | "payload_too_large" | "unsupported_media_type" | "unavailable" | "busy";
        ApiErrorDetails: {
            field?: string;
            index?: number;
            path?: string;
        };
        ApprovalEffect: {
            icon?: null | components["schemas"]["BlockIcon"];
            path?: string | null;
            text: string;
        };
        ApprovalEffectItem: string | components["schemas"]["ApprovalEffect"];
        /** @enum {string} */
        ApprovalRisk: "low" | "medium" | "high";
        /** @enum {string} */
        ApprovalStatus: "pending" | "approved" | "cancelled";
        ApproveRequest: {
            selectedRowIds?: string[];
        };
        AssistantConfigResponse: {
            apiKeyConfigured: boolean;
            baseUrl: string;
            model: string;
            preset: string;
            presets: components["schemas"]["AssistantPresetInfo"][];
            redactCloudPaths?: boolean;
        };
        AssistantContext: {
            itemIds?: string[] | null;
            label?: string | null;
            page?: string;
        };
        AssistantMemory: {
            content: string;
            /** Format: date-time */
            createdAt: string;
            id: string;
        };
        AssistantMemoryList: {
            memories: components["schemas"]["AssistantMemory"][];
        };
        AssistantMessage: {
            /** Format: date-time */
            createdAt: string;
            id: string;
            role: components["schemas"]["MessageRole"];
            runId: string | null;
            text: string;
        };
        AssistantPresetInfo: {
            defaultBaseUrl: string;
            defaultModel: string;
            id: string;
            label: string;
            requiresApiKey: boolean;
        };
        AssistantRun: {
            blocks: components["schemas"]["Block"][];
            /** Format: date-time */
            createdAt: string;
            id: string;
            previews?: string[] | null;
            request?: string | null;
            state: components["schemas"]["RunState"];
            steps: components["schemas"]["RunStep"][];
            subject?: null | components["schemas"]["RunSubject"];
            taskId: string | null;
            title?: string | null;
        };
        AudioStreamInfo: {
            channelLayout: string | null;
            /** Format: int32 */
            channels: number | null;
            codec: string;
            default: boolean;
            /**
             * Format: int32
             * @description Stream index in the file; pass it back as `audioIndex`.
             */
            index: number;
            language: string | null;
            title: string | null;
        };
        AuditLogEntry: {
            action: string;
            actorId: null | components["schemas"]["UserId"];
            actorUsername: string | null;
            detail: string | null;
            id: string;
            /** Format: date-time */
            occurredAt: string;
            target: string | null;
        };
        AuditLogPage: {
            entries: components["schemas"]["AuditLogEntry"][];
            nextCursor: string | null;
        };
        AutomationRule: {
            action: string;
            enabled: boolean;
            id: string;
            /** Format: date-time */
            lastRunAt: string | null;
            trigger: string;
        };
        BackgroundRunResponse: {
            run: components["schemas"]["AssistantRun"];
            task: components["schemas"]["Task"];
        };
        /**
         * Format: uuid
         * @description A portable system backup snapshot.
         */
        BackupId: string;
        BackupList: {
            backups: components["schemas"]["BackupSummary"][];
        };
        BackupRestoreResult: {
            backupId: components["schemas"]["BackupId"];
            bookmarksRestored: number;
            homeEntitiesRestored: number;
            itemStatesRestored: number;
            librariesRestored: number;
            memoriesRestored: number;
            photoAlbumsRestored: number;
            playlistsRestored: number;
            settingsRestored: number;
            sha256Verified: boolean;
        };
        BackupSummary: {
            counts: components["schemas"]["BackupSummaryCounts"];
            /** Format: date-time */
            createdAt: string;
            creatorId: null | components["schemas"]["UserId"];
            creatorUsername: string | null;
            id: components["schemas"]["BackupId"];
            name: string;
            sha256: string;
            /** Format: int64 */
            sizeBytes: number;
        };
        BackupSummaryCounts: {
            bookmarks: number;
            homeDevices: number;
            homeRules: number;
            homeScenes: number;
            itemStates: number;
            libraries: number;
            memories: number;
            photoAlbums: number;
            playlists: number;
            settings: number;
        };
        Block: {
            id?: string | null;
            text: string;
            title?: string | null;
            tone?: null | components["schemas"]["BlockTone"];
            /** @enum {string} */
            type: "text";
        } | {
            hint?: string | null;
            icon?: null | components["schemas"]["BlockIcon"];
            id?: string | null;
            label: string;
            title?: string | null;
            tone?: null | components["schemas"]["BlockTone"];
            /** @enum {string} */
            type: "stat";
            unit?: string | null;
            value: components["schemas"]["StatValue"];
        } | {
            id?: string | null;
            items: components["schemas"]["CollectionItem"][];
            layout?: null | components["schemas"]["CollectionLayout"];
            /** Format: int32 */
            more?: number | null;
            title?: string | null;
            /** @enum {string} */
            type: "collection";
        } | {
            columns: components["schemas"]["TableColumn"][];
            id?: string | null;
            rows: {
                [key: string]: components["schemas"]["CellValue"];
            }[];
            title?: string | null;
            /** @enum {string} */
            type: "table";
        } | {
            columns?: null | components["schemas"]["ChangeColumns"];
            groups?: components["schemas"]["ChangeGroup"][] | null;
            id: string;
            rows: components["schemas"]["ChangeRow"][];
            summary?: string | null;
            title?: string | null;
            /** @enum {string} */
            type: "change-preview";
        } | {
            confirmIcon?: null | components["schemas"]["BlockIcon"];
            confirmLabel?: string | null;
            effects: components["schemas"]["ApprovalEffectItem"][];
            id: string;
            previewId?: string | null;
            risk: components["schemas"]["ApprovalRisk"];
            status?: null | components["schemas"]["ApprovalStatus"];
            title: string;
            /** @enum {string} */
            type: "approval";
        } | {
            detail?: string | null;
            id?: string | null;
            label: string;
            state?: null | components["schemas"]["ProgressBlockState"];
            title?: string | null;
            /** @enum {string} */
            type: "progress";
            /** Format: double */
            value: number | null;
        } | {
            id?: string | null;
            kind: components["schemas"]["ChartKind"];
            series: components["schemas"]["ChartPoint"][];
            title?: string | null;
            /** @enum {string} */
            type: "chart";
            unit?: null | components["schemas"]["ChartUnit"];
        } | {
            device: components["schemas"]["DeviceInfo"];
            id?: string | null;
            title?: string | null;
            /** @enum {string} */
            type: "device";
        };
        /** @enum {string} */
        BlockIcon: "library" | "video" | "music" | "photo" | "book" | "comic" | "folder" | "storage" | "duplicate" | "sparkles" | "clock" | "download" | "transcode" | "light" | "thermometer" | "scene" | "check" | "warning" | "move" | "restore" | "shield" | "trash";
        /** @enum {string} */
        BlockTone: "default" | "muted" | "success" | "warning" | "danger";
        BookDownload: {
            /** Format: date-time */
            expiresAt: string;
            url: string;
        };
        /**
         * Format: uuid
         * @description A personal reading bookmark.
         */
        BookmarkId: string;
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
        Catalog: {
            libraryCovers: {
                [key: string]: string[];
            };
            recent: components["schemas"]["CatalogWork"][];
        };
        CatalogWork: {
            /** Format: date-time */
            addedAt: string;
            artworkId: string | null;
            id: string;
            kind: components["schemas"]["CatalogWorkKind"];
            libraryId: string | null;
            meta: string | null;
            title: string;
        };
        /** @enum {string} */
        CatalogWorkKind: "movie" | "series" | "album" | "book" | "photo";
        /** @enum {string} */
        CellFormat: "text" | "number" | "bytes" | "date" | "duration";
        CellValue: number | string | null;
        ChangeColumns: {
            change?: string | null;
            item?: string | null;
            reason?: string | null;
        };
        ChangeGroup: {
            /** Format: int32 */
            collapseAfter?: number | null;
            id: string;
            label: string;
        };
        ChangePasswordRequest: {
            currentPassword: string;
            newPassword: string;
            revokeOtherSessions?: boolean;
        };
        /** @enum {string} */
        ChangeRelation: "identical" | "similar";
        ChangeRow: {
            after: string;
            before: string;
            detail?: string | null;
            group?: string | null;
            id: string;
            note?: string | null;
            relation?: null | components["schemas"]["ChangeRelation"];
            selected: boolean;
            /** Format: int64 */
            sizeBytes?: number | null;
            thumbs?: string[] | null;
        };
        ChapterInfo: {
            /** Format: int64 */
            startMs: number;
            title: string | null;
        };
        /** @enum {string} */
        ChartKind: "bar" | "donut";
        ChartPoint: {
            label: string;
            /** Format: double */
            value: number;
        };
        /** @enum {string} */
        ChartUnit: "bytes" | "count";
        /** @enum {string} */
        ClientApiKeyKind: "jellyfin" | "webdav" | "general";
        ClientApiKeyList: {
            keys: components["schemas"]["ClientApiKeySummary"][];
        };
        ClientApiKeySummary: {
            clientKind: components["schemas"]["ClientApiKeyKind"];
            /** Format: date-time */
            createdAt: string;
            id: string;
            keyPrefix: string;
            /** Format: date-time */
            lastUsedAt: string | null;
            name: string;
            /** Format: date-time */
            revokedAt: string | null;
        };
        ClientProfile: {
            audioCodecs?: string[];
            containers?: string[];
            hls?: components["schemas"]["HlsSupport"];
            videoCodecs?: string[];
        };
        /** @enum {string} */
        ClimateMode: "cool" | "heat" | "auto" | "dry" | "fan";
        /**
         * Format: uuid
         * @description An item membership entry inside a cross-media collection.
         */
        CollectionEntryId: string;
        /**
         * Format: uuid
         * @description A cross-media collection.
         */
        CollectionId: string;
        CollectionItem: {
            artworkId?: string | null;
            badge?: string | null;
            id: string;
            kind: components["schemas"]["CollectionItemKind"];
            subtitle?: string | null;
            title: string;
        };
        /** @enum {string} */
        CollectionItemKind: "video" | "music" | "photo" | "book" | "comic" | "folder";
        /** @enum {string} */
        CollectionLayout: "grid" | "row";
        /** @enum {string} */
        ColumnAlign: "start" | "end";
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
        Conversation: {
            id: string;
            messages: components["schemas"]["AssistantMessage"][];
            runs: components["schemas"]["AssistantRun"][];
        };
        CpuInfo: {
            arch: string;
            /** Format: int32 */
            cores: number;
            model: string;
        };
        CreateAssistantMemoryRequest: {
            content: string;
        };
        CreateBackupRequest: {
            name?: string | null;
        };
        CreateBookmarkRequest: {
            label: string;
            locator?: string | null;
            note?: string | null;
            /** Format: int32 */
            page: number;
        };
        CreateClientApiKeyRequest: {
            clientKind?: null | components["schemas"]["ClientApiKeyKind"];
            name: string;
        };
        CreateConnection: {
            address: string;
            kind: components["schemas"]["ConnectionKind"];
            name: string;
            secret?: string | null;
            username?: string | null;
        };
        CreateDownloadTaskRequest: {
            category?: string | null;
            clientId?: string | null;
            savePath?: string | null;
            uri: string;
        };
        CreateDownloadTaskResponse: {
            accepted: boolean;
            clientId: string;
            clientName: string;
            message: string;
        };
        CreateDownloaderClientRequest: {
            baseUrl: string;
            enabled?: boolean;
            id?: string | null;
            kind: components["schemas"]["DownloaderKind"];
            name: string;
            password?: string | null;
            savePath?: string | null;
            username?: string | null;
        };
        CreateHomeDeviceRequest: {
            kind: components["schemas"]["HomeDeviceKind"];
            name: string;
            roomName: string;
        };
        CreateHomeRuleRequest: {
            action: string;
            deviceId?: string | null;
            eventKind?: string | null;
            patch?: null | components["schemas"]["DevicePatch"];
            sceneId?: string | null;
            trigger: string;
        };
        CreateInboundWebhookRequest: {
            enabled?: boolean;
            libraryId?: null | components["schemas"]["LibraryId"];
            name: string;
            sourceKind: components["schemas"]["InboundWebhookSourceKind"];
        };
        CreateLibraryRequest: {
            kind: components["schemas"]["LibraryKind"];
            name: string;
            paths: string[];
        };
        CreateMediaCollectionRequest: {
            coverArtworkId?: string | null;
            description?: string | null;
            title: string;
        };
        CreateNotificationChannelRequest: {
            config: components["schemas"]["NotificationChannelConfig"];
            enabled?: boolean;
            events?: string[];
            kind: components["schemas"]["NotificationChannelKind"];
            name: string;
            quietHours?: string | null;
        };
        CreatePhotoAlbumRequest: {
            coverItemId?: null | components["schemas"]["ItemId"];
            description?: string | null;
            title: string;
        };
        CreatePhotoPersonRequest: {
            coverItemId?: null | components["schemas"]["ItemId"];
            name: string;
        };
        CreateRoutineRequest: {
            actionKind: components["schemas"]["RoutineActionKind"];
            actionPayload?: string;
            enabled?: boolean;
            name: string;
            scheduleExpr: string;
            scheduleKind: components["schemas"]["RoutineScheduleKind"];
        };
        CreateShareRequest: {
            allowDownload?: boolean | null;
            /** Format: date-time */
            expiresAt?: string | null;
            /** Format: int32 */
            expiresInHours?: number | null;
            libraryId?: null | components["schemas"]["LibraryId"];
            /** Format: int32 */
            maxViews?: number | null;
            password?: string | null;
            stripGps?: boolean | null;
            targetId: string;
            targetKind: components["schemas"]["ShareTargetKind"];
            title?: string | null;
        };
        CreateSubscriptionRequest: {
            enabled?: boolean;
            feedUrl?: string | null;
            mediaKind: components["schemas"]["SubscriptionMediaKind"];
            qualityRule?: string | null;
            targetLibraryId?: string | null;
            title: string;
        };
        CreateUserRequest: {
            allowedLibraryIds?: components["schemas"]["LibraryId"][];
            password: string;
            policy?: null | components["schemas"]["UserPolicy"];
            role?: components["schemas"]["Role"];
            username: string;
        };
        CreateVirtualChannelRequest: {
            enabled?: boolean;
            genreFilter?: string | null;
            libraryId: components["schemas"]["LibraryId"];
            name: string;
            /** Format: int32 */
            number: number;
        };
        CreatedClientApiKey: {
            key: components["schemas"]["ClientApiKeySummary"];
            token: string;
        };
        CreatedInboundWebhook: {
            token: string;
            triggerPath: string;
            webhook: components["schemas"]["InboundWebhook"];
        };
        /** @enum {string} */
        DeliveryMode: "direct" | "remux" | "transcode";
        /**
         * Format: uuid
         * @description A registered client installation.
         */
        DeviceId: string;
        DeviceInfo: {
            id: string;
            name: string;
            online: boolean;
            room?: string | null;
            state: components["schemas"]["DeviceState"];
        };
        DevicePatch: {
            /** Format: double */
            brightness?: number | null;
            on?: boolean | null;
            status?: null | components["schemas"]["MediaPlayerPatchStatus"];
            /** Format: double */
            target?: number | null;
            /** Format: double */
            volume?: number | null;
        };
        DeviceState: {
            /** Format: int32 */
            brightness: number;
            /** Format: int32 */
            colorTemp?: number | null;
            on: boolean;
            /** @enum {string} */
            type: "light";
        } | {
            on: boolean;
            /** Format: double */
            power?: number | null;
            /** @enum {string} */
            type: "switch";
        } | {
            /** Format: double */
            current: number;
            /** Format: double */
            humidity?: number | null;
            mode: components["schemas"]["ClimateMode"];
            on: boolean;
            /** Format: double */
            target: number;
            /** @enum {string} */
            type: "climate";
        } | {
            readings: components["schemas"]["SensorReading"][];
            /** @enum {string} */
            type: "sensor";
        } | {
            app: string | null;
            artworkId?: string | null;
            /** Format: double */
            progress?: number | null;
            status: components["schemas"]["MediaPlayerStatus"];
            subtitle: string | null;
            title: string | null;
            /** @enum {string} */
            type: "media_player";
            /** Format: int32 */
            volume: number;
        };
        DoctorCheck: {
            detail: string;
            id: string;
            name: string;
            remediation?: string | null;
            status: components["schemas"]["DoctorStatus"];
            summary: string;
        };
        /** @enum {string} */
        DoctorStatus: "healthy" | "degraded" | "unhealthy";
        DownloadTransfer: {
            clientId: string;
            clientKind: components["schemas"]["DownloaderKind"];
            clientName: string;
            /** Format: int64 */
            downloadSpeedBps: number;
            /** Format: int64 */
            downloadedBytes: number;
            /** Format: int64 */
            etaSeconds: number | null;
            id: string;
            name: string;
            /** Format: double */
            progress: number;
            savePath: string | null;
            /** Format: int64 */
            sizeBytes: number;
            state: components["schemas"]["TransferState"];
            /** Format: int64 */
            uploadSpeedBps: number;
        };
        /** @enum {string} */
        DownloaderClientStatus: "online" | "offline" | "disabled";
        DownloaderClientSummary: {
            baseUrl: string;
            /** Format: date-time */
            createdAt: string;
            credentialConfigured: boolean;
            enabled: boolean;
            id: string;
            kind: components["schemas"]["DownloaderKind"];
            name: string;
            savePath: string | null;
            status: components["schemas"]["DownloaderClientStatus"];
            statusMessage?: string | null;
        };
        /** @enum {string} */
        DownloaderKind: "qbittorrent" | "transmission" | "aria2";
        DownloadsOverviewResponse: {
            clients: components["schemas"]["DownloaderClientSummary"][];
            subscriptions: components["schemas"]["MediaSubscription"][];
            transfers: components["schemas"]["DownloadTransfer"][];
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
        ExternalImportRequest: {
            apiKey?: string | null;
            content?: string;
            dryRun?: boolean;
            password?: string | null;
            playlistName?: string | null;
            serverUrl?: string | null;
            source?: components["schemas"]["ExternalImportSource"];
            username?: string | null;
        };
        ExternalImportResult: {
            detectedSource: components["schemas"]["ExternalImportSource"];
            dryRun: boolean;
            favoritesImported: number;
            historyImported: number;
            matchedItems: number;
            playlistTracksImported: number;
            playlistsImported: number;
            samples: components["schemas"]["ExternalImportSample"][];
            totalEntries: number;
            unmatchedEntries: number;
        };
        ExternalImportSample: {
            kind: string;
            matched: boolean;
            matchedTitle: string | null;
            title: string;
        };
        /** @enum {string} */
        ExternalImportSource: "auto" | "jellyfin" | "emby" | "navidrome" | "m3u";
        FavoriteUpdate: {
            favorite: boolean;
        };
        FeedMatchItem: {
            link: string | null;
            matchedQuality: boolean;
            pubDate: string | null;
            title: string;
        };
        GlobalMusicSearchHit: components["schemas"]["MusicSearchHit"] & {
            libraryId: components["schemas"]["LibraryId"];
            libraryName: string;
        };
        GlobalMusicSearchPage: {
            entries: components["schemas"]["GlobalMusicSearchHit"][];
            nextCursor: string | null;
        };
        GlobalSearchHit: {
            artworkId?: string | null;
            id: string;
            kind: components["schemas"]["GlobalSearchKind"];
            libraryId?: null | components["schemas"]["LibraryId"];
            sourceId?: string | null;
            subtitle?: string | null;
            title: string;
        };
        /** @enum {string} */
        GlobalSearchKind: "video_work" | "music_album" | "track" | "book" | "comic" | "photo" | "broadcast" | "collection" | "home_device" | "home_scene";
        GlobalSearchResponse: {
            hits: components["schemas"]["GlobalSearchHit"][];
            query: string;
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
        /** @enum {string} */
        HdrFormat: "sdr" | "hdr10" | "hlg" | "dolby-vision";
        Health: {
            /** @constant */
            status: "ok";
        };
        /** @enum {string} */
        HlsSupport: "native" | "mse" | "none";
        HomeBridgeList: {
            bridges: components["schemas"]["HomeBridgeSummary"][];
        };
        HomeBridgeSummary: {
            baseUrl: string;
            enabled: boolean;
            id: string;
            /** Format: date-time */
            lastSyncAt: string | null;
            name: string;
            tokenConfigured: boolean;
        };
        HomeBridgeSyncResult: {
            bridge: components["schemas"]["HomeBridgeSummary"];
            snapshot: components["schemas"]["HomeSnapshot"];
            syncedDevices: number;
            syncedScenes: number;
        };
        HomeDevice: {
            id: string;
            name: string;
            online: boolean;
            room?: string | null;
            roomId: string;
            state: components["schemas"]["DeviceState"];
        };
        /** @enum {string} */
        HomeDeviceKind: "light" | "switch" | "climate" | "sensor" | "media_player";
        HomeSnapshot: {
            devices: components["schemas"]["HomeDevice"][];
            rooms: components["schemas"]["Room"][];
            rules: components["schemas"]["AutomationRule"][];
            scenes: components["schemas"]["Scene"][];
        };
        InboundWebhook: {
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            id: string;
            /** Format: date-time */
            lastTriggeredAt?: string | null;
            libraryId?: null | components["schemas"]["LibraryId"];
            name: string;
            sourceKind: components["schemas"]["InboundWebhookSourceKind"];
        };
        InboundWebhookList: {
            webhooks: components["schemas"]["InboundWebhook"][];
        };
        /** @description Open schema describing common fields sent by Sonarr, Radarr, MoviePilot, qBittorrent, or custom callers. */
        InboundWebhookPayload: {
            eventType?: string | null;
            name?: string | null;
            path?: string | null;
            title?: string | null;
            type?: string | null;
        };
        /**
         * @description Supported inbound media automation webhook sources.
         * @enum {string}
         */
        InboundWebhookSourceKind: "sonarr" | "radarr" | "moviepilot" | "qbittorrent" | "custom";
        InboundWebhookTriggerResponse: {
            accepted: boolean;
            queuedLibraryIds: components["schemas"]["LibraryId"][];
            sourceKind: components["schemas"]["InboundWebhookSourceKind"];
            summary: string;
        };
        InstallPluginRequest: {
            grantedPermissions?: string[];
            pluginId: string;
            settings?: unknown;
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
         * @description M0 only knows folders and one leaf kind per library kind; hierarchical
         *     kinds (series/season/episode, artist/album, …) arrive with their modules.
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
        LibraryBytes: {
            /** Format: int64 */
            bytes: number;
            libraryId: string;
            name: string;
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
        LinkChallenge: {
            challenge: string;
            deviceId: components["schemas"]["DeviceId"];
            serverNodeId: string;
        };
        LinkDevice: {
            connected: boolean;
            id: components["schemas"]["DeviceId"];
            name: string;
            nodeId: string;
        };
        LinkDeviceList: {
            devices: components["schemas"]["LinkDevice"][];
        };
        LinkPairRequest: {
            nodeId: string;
            signature: string;
        };
        LinkPairing: {
            deviceId: components["schemas"]["DeviceId"];
            serverNodeId: string;
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
        LyricsSource: "sidecar" | "embedded" | "online";
        /** @description Administrator view of an account including its policy, allowed libraries and active sessions. */
        ManagedUser: {
            /** Format: int64 */
            activeSessions: number;
            allowedLibraryIds: components["schemas"]["LibraryId"][];
            /** Format: date-time */
            createdAt: string;
            disabled: boolean;
            id: components["schemas"]["UserId"];
            policy: components["schemas"]["UserPolicy"];
            role: components["schemas"]["Role"];
            /** Format: date-time */
            updatedAt: string;
            username: string;
        };
        McpClientConfigs: {
            claudeDesktop: string;
            cursor: string;
            openclaw: string;
        };
        McpInfoResponse: {
            clientConfigs: components["schemas"]["McpClientConfigs"];
            endpointPath: string;
            protocolVersion: string;
            serverName: string;
            serverVersion: string;
            tools: components["schemas"]["McpToolDescriptor"][];
        };
        McpJsonRpcError: {
            /** Format: int32 */
            code: number;
            data?: Record<string, never> | null;
            message: string;
        };
        McpJsonRpcRequest: {
            id?: Record<string, never> | null;
            jsonrpc: string;
            method: string;
            params?: Record<string, never> | null;
        };
        McpJsonRpcResponse: {
            error?: null | components["schemas"]["McpJsonRpcError"];
            id: Record<string, never> | null;
            jsonrpc: string;
            result?: Record<string, never> | null;
        };
        McpToolDescriptor: {
            adminOnly: boolean;
            description: string;
            inputSchema: Record<string, never>;
            name: string;
            requiresApproval: boolean;
        };
        MediaCollectionDetail: {
            autoRelated: components["schemas"]["RelatedWork"][];
            collection: components["schemas"]["MediaCollectionSummary"];
            items: components["schemas"]["MediaCollectionEntry"][];
        };
        MediaCollectionEntry: {
            /** Format: date-time */
            addedAt: string;
            artworkId?: string | null;
            collectionId: components["schemas"]["CollectionId"];
            entryKind: components["schemas"]["MediaCollectionEntryKind"];
            id: components["schemas"]["CollectionEntryId"];
            libraryId?: null | components["schemas"]["LibraryId"];
            /** Format: int32 */
            position: number;
            subtitle?: string | null;
            targetId: string;
            title: string;
        };
        /** @enum {string} */
        MediaCollectionEntryKind: "video" | "music_album" | "track" | "book" | "comic" | "photo";
        MediaCollectionList: {
            collections: components["schemas"]["MediaCollectionSummary"][];
        };
        MediaCollectionSummary: {
            coverArtworkId?: string | null;
            /** Format: date-time */
            createdAt: string;
            description?: string | null;
            id: components["schemas"]["CollectionId"];
            /** Format: int32 */
            itemCount: number;
            kinds: components["schemas"]["MediaCollectionEntryKind"][];
            ownerId: components["schemas"]["UserId"];
            previewArtworkIds: string[];
            title: string;
            /** Format: date-time */
            updatedAt: string;
        };
        MediaInfo: {
            audio: components["schemas"]["AudioStreamInfo"][];
            /** Format: int32 */
            bitrateKbps: number | null;
            chapters: components["schemas"]["ChapterInfo"][];
            /** @description Normalized container: `mp4`, `mov`, `mkv`, `webm`, `ts`, `avi`, … */
            container: string | null;
            /** Format: int64 */
            durationMs: number | null;
            /** Format: int64 */
            sizeBytes: number | null;
            subtitles: components["schemas"]["SubtitleStreamInfo"][];
            video: null | components["schemas"]["VideoStreamInfo"];
        };
        /** @enum {string} */
        MediaPlayerPatchStatus: "playing" | "paused";
        /** @enum {string} */
        MediaPlayerStatus: "playing" | "paused" | "idle" | "off";
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
        MediaSubscription: {
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            feedUrl: string | null;
            id: string;
            /** Format: date-time */
            lastCheckedAt: string | null;
            mediaKind: components["schemas"]["SubscriptionMediaKind"];
            ownerId: string;
            qualityRule: string | null;
            targetLibraryId: string | null;
            title: string;
        };
        /** @enum {string} */
        MediaView: "favorites" | "recent" | "continue";
        /** @enum {string} */
        MessageRole: "user" | "assistant";
        MetadataConfigResponse: {
            autoFetchPosters: boolean;
            bangumiConfigured: boolean;
            preferredLanguage: string;
            tmdbConfigured: boolean;
        };
        MetadataOverride: {
            backdropUrl: string | null;
            externalId: string | null;
            genres: string[];
            itemId: components["schemas"]["ItemId"];
            originalTitle: string | null;
            overview: string | null;
            posterUrl: string | null;
            provider: components["schemas"]["MetadataProvider"];
            /** Format: double */
            rating: number | null;
            title: string | null;
            /** Format: date-time */
            updatedAt: string;
            workKey: string | null;
            /** Format: int32 */
            year: number | null;
        };
        MetadataOverrideResponse: {
            itemId: components["schemas"]["ItemId"];
            override: null | components["schemas"]["MetadataOverride"];
        };
        /** @enum {string} */
        MetadataProvider: "manual" | "tmdb" | "bangumi" | "openlibrary";
        MetadataSearchCandidate: {
            backdropUrl: string | null;
            externalId: string;
            genres: string[];
            mediaType: string;
            originalTitle: string | null;
            overview: string | null;
            posterUrl: string | null;
            provider: components["schemas"]["MetadataProvider"];
            /** Format: double */
            rating: number | null;
            title: string;
            /** Format: int32 */
            year: number | null;
        };
        /** @enum {string} */
        MetadataSearchMediaType: "movie" | "tv" | "book" | "anime";
        MetadataSearchRequest: {
            mediaType?: null | components["schemas"]["MetadataSearchMediaType"];
            provider: components["schemas"]["MetadataProvider"];
            query: string;
            /** Format: int32 */
            year?: number | null;
        };
        MetadataSearchResponse: {
            candidates: components["schemas"]["MetadataSearchCandidate"][];
            provider: components["schemas"]["MetadataProvider"];
            query: string;
        };
        MountList: {
            mounts: components["schemas"]["MountedDirectory"][];
        };
        /** @enum {string} */
        MountProtocol: "smb" | "nfs" | "webdav" | "sshfs" | "rclone" | "network";
        /** @enum {string} */
        MountState: "online" | "offline";
        MountedDirectory: {
            path: string;
            protocol: components["schemas"]["MountProtocol"];
            readOnly: boolean;
        };
        MqttBridgeList: {
            bridges: components["schemas"]["MqttBridgeSummary"][];
        };
        MqttBridgeSummary: {
            ambientLightDeviceId?: string | null;
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            host: string;
            id: string;
            /** Format: date-time */
            lastPublishedAt?: string | null;
            name: string;
            passwordConfigured: boolean;
            /** Format: int32 */
            port: number;
            publishPlayback: boolean;
            topicPrefix: string;
            username?: string | null;
        };
        MqttTestResponse: {
            error?: string | null;
            ok: boolean;
            topic: string;
        };
        MusicAlbum: {
            /** Format: date-time */
            addedAt: string;
            artist: string | null;
            artistId: string;
            artworkItemId: null | components["schemas"]["ItemId"];
            /** Format: int64 */
            discCount: number;
            /** Format: int64 */
            durationMs: number;
            favorite: boolean;
            genres: string[];
            id: string;
            /** Format: date-time */
            lastPlayedAt: string | null;
            libraryId: components["schemas"]["LibraryId"];
            name: string | null;
            /** Format: int64 */
            playCount: number;
            /** Format: int32 */
            stars: number;
            /** Format: int64 */
            trackCount: number;
            /** Format: int32 */
            year: number | null;
        };
        MusicAlbumDetail: {
            album: components["schemas"]["MusicAlbum"];
            tracks: components["schemas"]["MusicTrack"][];
        };
        MusicAlbumPage: {
            albums: components["schemas"]["MusicAlbum"][];
            nextCursor: string | null;
            /** Format: int64 */
            total: number;
        };
        /** @enum {string} */
        MusicAlbumSort: "title" | "artist" | "year" | "added" | "played" | "plays" | "rating" | "random";
        MusicArtist: {
            /** Format: int64 */
            albumCount: number;
            artworkItemId: null | components["schemas"]["ItemId"];
            favorite: boolean;
            id: string;
            libraryId: components["schemas"]["LibraryId"];
            name: string | null;
            /** Format: int64 */
            playCount: number;
            /** Format: int64 */
            trackCount: number;
        };
        MusicArtistDetail: {
            albums: components["schemas"]["MusicAlbum"][];
            artist: components["schemas"]["MusicArtist"];
            topTracks: components["schemas"]["MusicTrack"][];
        };
        MusicArtistPage: {
            artists: components["schemas"]["MusicArtist"][];
            nextCursor: string | null;
            /** Format: int64 */
            total: number;
        };
        /** @enum {string} */
        MusicArtistSort: "name" | "albums" | "tracks" | "plays" | "random";
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
        MusicGenre: {
            /** Format: int64 */
            albumCount: number;
            name: string;
            /** Format: int64 */
            trackCount: number;
        };
        MusicGenreList: {
            genres: components["schemas"]["MusicGenre"][];
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
        MusicSortOrder: "asc" | "desc";
        MusicTrack: {
            /** Format: date-time */
            addedAt: string;
            album: string | null;
            albumArtist: string | null;
            albumId: string | null;
            artist: string | null;
            artistId: string | null;
            artworkItemId: null | components["schemas"]["ItemId"];
            /** Format: int32 */
            bitDepth: number | null;
            /** Format: int32 */
            bitrateKbps: number | null;
            /** Format: int32 */
            channels: number | null;
            codec: string | null;
            /** Format: int32 */
            discNumber: number | null;
            /** Format: int64 */
            durationMs: number | null;
            genres: string[];
            id: components["schemas"]["ItemId"];
            libraryId: components["schemas"]["LibraryId"];
            /** Format: int32 */
            sampleRateHz: number | null;
            /** Format: int64 */
            sizeBytes: number | null;
            state: components["schemas"]["MusicTrackState"];
            title: string;
            /** Format: int32 */
            trackNumber: number | null;
            /** Format: int32 */
            year: number | null;
        };
        MusicTrackLookupRequest: {
            ids: components["schemas"]["ItemId"][];
        };
        MusicTrackLookupResponse: {
            tracks: components["schemas"]["MusicTrack"][];
        };
        MusicTrackPage: {
            nextCursor: string | null;
            /** Format: int64 */
            total: number;
            tracks: components["schemas"]["MusicTrack"][];
        };
        /** @enum {string} */
        MusicTrackSort: "title" | "artist" | "album" | "year" | "added" | "duration" | "plays" | "played" | "rating" | "random";
        MusicTrackState: {
            favorite: boolean;
            /** Format: date-time */
            lastPlayedAt: string | null;
            /** Format: int64 */
            playCount: number;
            /** Format: int32 */
            stars: number;
        };
        /** @description Configured push notification channel (secrets redacted). */
        NotificationChannel: {
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            events: string[];
            hasSecret: boolean;
            id: string;
            kind: components["schemas"]["NotificationChannelKind"];
            name: string;
            ownerId: components["schemas"]["UserId"];
            quietHours?: string | null;
            targetHint?: string | null;
            /** Format: date-time */
            updatedAt: string;
        };
        /** @description Secret channel configuration encrypted at rest via `AES-256-GCM`. */
        NotificationChannelConfig: {
            /** @description Provider target (Telegram `chat_id`, ntfy topic, etc.). */
            target?: string | null;
            /** @description Provider secret token (Telegram bot token, ntfy bearer token, Bark device key, or Webhook secret). */
            token?: string | null;
            /** @description Endpoint URL (Webhook / Bark / ntfy / WeCom / Feishu / Discord, or optional Telegram API base). */
            url?: string | null;
        };
        /**
         * @description Supported external notification delivery providers.
         * @enum {string}
         */
        NotificationChannelKind: "webhook" | "telegram" | "bark" | "ntfy" | "wecom" | "feishu" | "discord";
        NotificationChannelList: {
            channels: components["schemas"]["NotificationChannel"][];
        };
        NotificationDelivery: {
            body: string;
            channelId?: string | null;
            /** Format: date-time */
            createdAt: string;
            error?: string | null;
            eventType: string;
            id: string;
            ownerId?: null | components["schemas"]["UserId"];
            /** Format: date-time */
            readAt?: string | null;
            status: components["schemas"]["NotificationDeliveryStatus"];
            title: string;
        };
        /**
         * @description Delivery outcome for an in-app or external notification record.
         * @enum {string}
         */
        NotificationDeliveryStatus: "in_app" | "sent" | "failed" | "skipped_quiet";
        NotificationList: {
            notifications: components["schemas"]["NotificationDelivery"][];
            /** Format: int64 */
            unreadCount: number;
        };
        OpdsAccess: {
            password: string;
            username: string;
        };
        OrganizeApplyRequest: {
            libraryId: components["schemas"]["LibraryId"];
            mode?: components["schemas"]["OrganizeMode"];
            selectedItemIds?: components["schemas"]["ItemId"][];
            targetSubdir?: string | null;
            template: string;
        };
        OrganizeBatchList: {
            batches: components["schemas"]["OrganizeBatchSummary"][];
            presets: components["schemas"]["OrganizePreset"][];
        };
        OrganizeBatchSummary: {
            actorId: string;
            /** Format: date-time */
            createdAt: string;
            id: string;
            libraryId: components["schemas"]["LibraryId"];
            libraryName: string | null;
            mode: components["schemas"]["OrganizeMode"];
            operationCount: number;
            operations: components["schemas"]["OrganizeOperationRecord"][];
            template: string;
            /** Format: date-time */
            undoneAt: string | null;
        };
        OrganizeCandidate: {
            conflict?: string | null;
            itemId: components["schemas"]["ItemId"];
            relativeAfter: string;
            relativeBefore: string;
            rowId: string;
            selectable: boolean;
            /** Format: int64 */
            sizeBytes: number | null;
            sourcePath: string;
            targetPath: string;
        };
        /** @enum {string} */
        OrganizeMode: "rename" | "hardlink";
        OrganizeOperationRecord: {
            createdDirs?: string[];
            fromPath: string;
            itemId: components["schemas"]["ItemId"];
            toPath: string;
        };
        OrganizePreset: {
            example: string;
            id: string;
            label: string;
            mediaKind: string;
            template: string;
        };
        OrganizePreviewRequest: {
            libraryId: components["schemas"]["LibraryId"];
            /** Format: int32 */
            limit?: number | null;
            mode?: components["schemas"]["OrganizeMode"];
            targetSubdir?: string | null;
            template: string;
        };
        OrganizePreviewResponse: {
            candidates: components["schemas"]["OrganizeCandidate"][];
            collisionCount: number;
            libraryId: components["schemas"]["LibraryId"];
            libraryName: string;
            mode: components["schemas"]["OrganizeMode"];
            presets: components["schemas"]["OrganizePreset"][];
            previewBlock: components["schemas"]["Block"];
            readOnlyMount: boolean;
            readyCount: number;
            template: string;
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
        /**
         * Format: uuid
         * @description A personal photo album.
         */
        PhotoAlbumId: string;
        PhotoAlbumItemsRequest: {
            itemIds: components["schemas"]["ItemId"][];
        };
        PhotoAlbumList: {
            albums: components["schemas"]["PhotoAlbumSummary"][];
        };
        PhotoAlbumRef: {
            id: components["schemas"]["PhotoAlbumId"];
            title: string;
        };
        PhotoAlbumSummary: {
            coverItemId: null | components["schemas"]["ItemId"];
            /** Format: date-time */
            createdAt: string;
            description: string | null;
            id: components["schemas"]["PhotoAlbumId"];
            /** Format: int64 */
            itemCount: number;
            previewItemIds: components["schemas"]["ItemId"][];
            title: string;
            /** Format: date-time */
            updatedAt: string;
        };
        PhotoAnalyzeResponse: {
            /** Format: int64 */
            analyzedCount: number;
            /** Format: int64 */
            remaining: number;
            /** Format: int64 */
            totalTagged: number;
        };
        PhotoCameraBucket: {
            /** Format: int64 */
            count: number;
            name: string;
        };
        PhotoDuplicateCluster: {
            clusterId: string;
            items: components["schemas"]["PhotoDuplicateItem"][];
            keepItemId: components["schemas"]["ItemId"];
            kind: components["schemas"]["PhotoDuplicateKind"];
            /** Format: int32 */
            maxDistance: number;
            /** Format: int64 */
            reclaimableBytes: number;
        };
        PhotoDuplicateItem: {
            cameraMake: string | null;
            cameraModel: string | null;
            dayBucket: string;
            /** Format: int32 */
            distance: number;
            favorite: boolean;
            fileName: string;
            /** Format: int32 */
            height: number | null;
            id: components["schemas"]["ItemId"];
            /** Format: double */
            latitude: number | null;
            libraryId: components["schemas"]["LibraryId"];
            /** Format: double */
            longitude: number | null;
            monthBucket: string;
            /** Format: int32 */
            orientation: number;
            recommendedKeep: boolean;
            /** Format: int64 */
            sizeBytes: number | null;
            /** Format: date-time */
            takenAt: string;
            title: string;
            /** Format: int32 */
            width: number | null;
        };
        /** @enum {string} */
        PhotoDuplicateKind: "exact" | "similar";
        PhotoDuplicatesResponse: {
            clusters: components["schemas"]["PhotoDuplicateCluster"][];
            /** Format: int32 */
            threshold: number;
            /** Format: int64 */
            totalClusters: number;
            /** Format: int64 */
            totalDuplicatePhotos: number;
            /** Format: int64 */
            totalReclaimableBytes: number;
        };
        PhotoInfo: {
            albums: components["schemas"]["PhotoAlbumRef"][];
            cameraMake: string | null;
            cameraModel: string | null;
            contentHash: string | null;
            dayBucket: string;
            exposureTime: string | null;
            /** Format: double */
            fNumber: number | null;
            favorite: boolean;
            fileName: string;
            /** Format: double */
            focalLength: number | null;
            /** Format: int32 */
            height: number | null;
            id: components["schemas"]["ItemId"];
            /** Format: int32 */
            iso: number | null;
            /** Format: double */
            latitude: number | null;
            lensModel: string | null;
            libraryId: components["schemas"]["LibraryId"];
            /** Format: double */
            longitude: number | null;
            monthBucket: string;
            /** Format: int32 */
            orientation: number;
            path: string;
            /** Format: int64 */
            sizeBytes: number | null;
            /** Format: date-time */
            takenAt: string;
            title: string;
            /** Format: int32 */
            width: number | null;
        };
        PhotoItemPeopleResponse: {
            people: components["schemas"]["PhotoPersonRef"][];
        };
        PhotoMemories: {
            cameras: components["schemas"]["PhotoCameraBucket"][];
            /** Format: int64 */
            geotaggedCount: number;
            onThisDay: components["schemas"]["PhotoTimelineItem"][];
            places: components["schemas"]["PhotoPlaceCluster"][];
            recentHighlights: components["schemas"]["PhotoTimelineItem"][];
            /** Format: int64 */
            totalPhotos: number;
        };
        PhotoMonthBucket: {
            /** Format: int64 */
            count: number;
            month: string;
        };
        /**
         * Format: uuid
         * @description A user-managed person profile in the photo library.
         */
        PhotoPersonId: string;
        PhotoPersonList: {
            people: components["schemas"]["PhotoPersonSummary"][];
        };
        PhotoPersonRef: {
            id: components["schemas"]["PhotoPersonId"];
            name: string;
        };
        PhotoPersonSummary: {
            coverItemId: null | components["schemas"]["ItemId"];
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["PhotoPersonId"];
            name: string;
            /** Format: int64 */
            photoCount: number;
            previewItemIds: components["schemas"]["ItemId"][];
        };
        PhotoPlaceCluster: {
            /** Format: int64 */
            count: number;
            coverItemId: components["schemas"]["ItemId"];
            key: string;
            label: string;
            /** Format: double */
            latitude: number;
            /** Format: double */
            longitude: number;
        };
        PhotoQueryFilter: {
            cameraBrand: string | null;
            color: string | null;
            geotagged: boolean | null;
            keywords: string[];
            /** Format: int32 */
            month: number | null;
            orientation: string | null;
            season: string | null;
            timeOfDay: string | null;
            /** Format: int32 */
            year: number | null;
        };
        PhotoSearchHit: {
            cameraMake: string | null;
            cameraModel: string | null;
            dayBucket: string;
            dominantColor: string | null;
            dominantHex: string | null;
            favorite: boolean;
            /** Format: int32 */
            height: number | null;
            id: components["schemas"]["ItemId"];
            /** Format: double */
            latitude: number | null;
            libraryId: components["schemas"]["LibraryId"];
            /** Format: double */
            longitude: number | null;
            monthBucket: string;
            /** Format: int32 */
            orientation: number;
            people: components["schemas"]["PhotoPersonRef"][];
            sceneTags: string[];
            season: string | null;
            /** Format: date-time */
            takenAt: string;
            timeOfDay: string | null;
            title: string;
            /** Format: int32 */
            width: number | null;
        };
        PhotoSearchResponse: {
            items: components["schemas"]["PhotoSearchHit"][];
            parsedFilters: components["schemas"]["PhotoQueryFilter"];
            query: string;
        };
        PhotoTimelineItem: {
            cameraMake: string | null;
            cameraModel: string | null;
            dayBucket: string;
            favorite: boolean;
            /** Format: int32 */
            height: number | null;
            id: components["schemas"]["ItemId"];
            /** Format: double */
            latitude: number | null;
            libraryId: components["schemas"]["LibraryId"];
            /** Format: double */
            longitude: number | null;
            monthBucket: string;
            /** Format: int32 */
            orientation: number;
            /** Format: date-time */
            takenAt: string;
            title: string;
            /** Format: int32 */
            width: number | null;
        };
        PhotoTimelinePage: {
            items: components["schemas"]["PhotoTimelineItem"][];
            months: components["schemas"]["PhotoMonthBucket"][];
            nextCursor: string | null;
            /** Format: int64 */
            total: number;
        };
        PhotoUploadRequest: {
            dataBase64: string;
            fileName: string;
        };
        /** @enum {string} */
        PlaybackClientKind: "web" | "ios" | "tvos" | "macos" | "android" | "desktop";
        PlaybackCommandAck: {
            accepted: boolean;
            command: components["schemas"]["PlaybackCommandKind"];
            targetSessionId: string;
        };
        PlaybackCommandEvent: {
            command: string;
            issuedBy: string;
            itemId?: string | null;
            /** Format: int64 */
            positionMs?: number | null;
            targetSessionId: string;
            /** Format: int32 */
            volume?: number | null;
        };
        /** @enum {string} */
        PlaybackCommandKind: "play" | "pause" | "toggle" | "seek" | "next" | "prev" | "volume" | "play_item";
        PlaybackCommandRequest: {
            command: components["schemas"]["PlaybackCommandKind"];
            itemId?: string | null;
            /** Format: int64 */
            positionMs?: number | null;
            /** Format: int32 */
            volume?: number | null;
        };
        PlaybackHeartbeatRequest: {
            ambientHex?: string | null;
            artworkId?: string | null;
            clientKind: components["schemas"]["PlaybackClientKind"];
            deviceName: string;
            /** Format: int64 */
            durationMs?: number;
            itemId?: string | null;
            libraryId?: string | null;
            mediaKind: components["schemas"]["PlaybackMediaKind"];
            /** Format: int64 */
            positionMs?: number;
            sessionId: string;
            state: components["schemas"]["PlaybackSessionState"];
            subtitle?: string | null;
            title?: string | null;
            /** Format: int32 */
            volume?: number;
        };
        PlaybackInfo: {
            directUrl?: string | null;
            /** Format: date-time */
            expiresAt: string;
            mode: components["schemas"]["PlaybackMode"];
            url: string;
        };
        /** @enum {string} */
        PlaybackMediaKind: "audio" | "video" | "audiobook" | "none";
        /** @enum {string} */
        PlaybackMode: "direct" | "transcode";
        /** @enum {string} */
        PlaybackProtocol: "file" | "hls";
        PlaybackSession: {
            ambientHex?: string | null;
            artworkId?: string | null;
            clientKind: components["schemas"]["PlaybackClientKind"];
            deviceName: string;
            /** Format: int64 */
            durationMs: number;
            id: string;
            itemId?: string | null;
            libraryId?: string | null;
            mediaKind: components["schemas"]["PlaybackMediaKind"];
            /** Format: int64 */
            positionMs: number;
            state: components["schemas"]["PlaybackSessionState"];
            subtitle?: string | null;
            title?: string | null;
            /** Format: date-time */
            updatedAt: string;
            userId: components["schemas"]["UserId"];
            /** Format: int32 */
            volume: number;
        };
        PlaybackSessionList: {
            sessions: components["schemas"]["PlaybackSession"][];
        };
        /** @enum {string} */
        PlaybackSessionState: "playing" | "paused" | "stopped" | "idle";
        PlaybackSessionUpdatedEvent: {
            ambientHex?: string | null;
            deviceName: string;
            itemId?: string | null;
            /** Format: int64 */
            positionMs: number;
            sessionId: string;
            state: string;
            title?: string | null;
            /** Format: int32 */
            volume: number;
        };
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
        PluginCatalogEntry: {
            defaultSettings: unknown;
            description: {
                [key: string]: string;
            };
            id: string;
            installed: boolean;
            installedVersion: string | null;
            name: string;
            permissions: string[];
            previousVersion: string | null;
            provides: string[];
            riskLevel: components["schemas"]["PluginRiskLevel"];
            runtime: components["schemas"]["PluginRuntimeKind"];
            settingsSchema: unknown;
            source: string;
            version: string;
        };
        PluginCatalogResponse: {
            plugins: components["schemas"]["PluginCatalogEntry"][];
        };
        PluginList: {
            plugins: components["schemas"]["PluginSummary"][];
        };
        /** @enum {string} */
        PluginRiskLevel: "low" | "medium" | "high";
        /** @enum {string} */
        PluginRuntimeKind: "none" | "recipe" | "widget" | "wasm" | "sidecar";
        /**
         * @description Enriched installed plugin summary returned by `GET /api/v1/plugins`,
         *     `POST /api/v1/plugins/install`, `PATCH /api/v1/plugins/{id}` and
         *     `POST /api/v1/plugins/{id}/rollback`.
         *
         *     The first six fields match the original M0 contract (`id`, `name`, `version`,
         *     `runtime`, `enabled`, `provides`); the enrichment fields remain optional in
         *     the schema for wire backwards compatibility.
         */
        PluginSummary: {
            description?: {
                [key: string]: string;
            } | null;
            enabled: boolean;
            grantedPermissions?: string[] | null;
            id: string;
            /** Format: date-time */
            installedAt?: string | null;
            managed?: boolean | null;
            name: string;
            permissions?: string[] | null;
            previousVersion?: string | null;
            provides: string[];
            riskLevel?: null | components["schemas"]["PluginRiskLevel"];
            runtime: components["schemas"]["PluginRuntimeKind"];
            settings?: unknown;
            settingsSchema?: unknown;
            source?: string | null;
            /** Format: date-time */
            updatedAt?: string | null;
            version: string;
        };
        PresetDeviceAction: {
            deviceId: string;
            patch: components["schemas"]["DevicePatch"];
        };
        /** @enum {string} */
        ProgressBlockState: "running" | "succeeded" | "failed";
        PublicShareItem: {
            camera?: string | null;
            /** Format: int64 */
            durationMs?: number | null;
            format?: string | null;
            hasCover: boolean;
            /** Format: int32 */
            height?: number | null;
            itemId: components["schemas"]["ItemId"];
            /** Format: double */
            latitude?: number | null;
            /** Format: double */
            longitude?: number | null;
            mediaKind: components["schemas"]["ShareMediaKind"];
            /** Format: int64 */
            sizeBytes?: number | null;
            subtitle?: string | null;
            /** Format: date-time */
            takenAt?: string | null;
            title: string;
            /** Format: int32 */
            width?: number | null;
        };
        PublicShareView: {
            allowDownload: boolean;
            /** Format: date-time */
            expiresAt?: string | null;
            items: components["schemas"]["PublicShareItem"][];
            locked: boolean;
            proof?: string | null;
            stripGps: boolean;
            targetKind: components["schemas"]["ShareTargetKind"];
            title: string;
            token: string;
        };
        PutVideoSegmentsRequest: {
            segments: components["schemas"]["VideoSegmentInput"][];
        };
        RatingUpdate: {
            /** Format: int32 */
            stars: number;
        };
        ReadAllNotificationsResponse: {
            /** Format: int64 */
            markedRead: number;
        };
        ReadingBookmark: {
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["BookmarkId"];
            itemId: components["schemas"]["ItemId"];
            label: string;
            locator: string | null;
            note: string | null;
            /** Format: int32 */
            page: number;
        };
        ReadingChapter: {
            html: string;
            /** Format: int32 */
            index: number;
            plainText: string;
            title: string;
        };
        ReadingChapterSummary: {
            /** Format: int32 */
            index: number;
            title: string;
        };
        ReadingEntry: {
            /** Format: date-time */
            addedAt: string;
            authors: string[];
            format: string;
            hasCover: boolean;
            itemId: components["schemas"]["ItemId"];
            kind: components["schemas"]["ReadingKind"];
            language: string | null;
            libraryId: components["schemas"]["LibraryId"];
            /** Format: int32 */
            pageCount: number | null;
            publisher: string | null;
            /** Format: double */
            seriesIndex: number | null;
            seriesKey: string | null;
            seriesName: string | null;
            /** Format: int64 */
            sizeBytes: number | null;
            state: components["schemas"]["ReadingUserState"];
            summary: string | null;
            tags: string[];
            title: string;
        };
        ReadingHome: {
            continueReading: components["schemas"]["ReadingEntry"][];
            recentBooks: components["schemas"]["ReadingEntry"][];
            recentComics: components["schemas"]["ReadingEntry"][];
            series: components["schemas"]["ReadingSeries"][];
        };
        ReadingInline: {
            /** Format: date-time */
            expiresAt: string;
            url: string;
        };
        /** @enum {string} */
        ReadingKind: "book" | "comic";
        ReadingManifest: {
            bookmarks: components["schemas"]["ReadingBookmark"][];
            chapters: components["schemas"]["ReadingChapterSummary"][];
            entry: components["schemas"]["ReadingEntry"];
            readerMode: components["schemas"]["ReadingReaderMode"];
        };
        ReadingPage: {
            entries: components["schemas"]["ReadingEntry"][];
            nextCursor: string | null;
            /** Format: int64 */
            total: number;
        };
        ReadingProgressUpdate: {
            completed: boolean;
            locator?: string | null;
            /** Format: int32 */
            page: number;
            /** Format: int32 */
            totalPages?: number | null;
        };
        /** @enum {string} */
        ReadingReaderMode: "chapters" | "pages" | "pdf" | "download_only";
        ReadingSeries: {
            /** Format: int32 */
            completedCount: number;
            coverItemId: components["schemas"]["ItemId"];
            hasCover: boolean;
            /** Format: int32 */
            itemCount: number;
            kind: components["schemas"]["ReadingKind"];
            libraryId: components["schemas"]["LibraryId"];
            seriesKey: string;
            seriesName: string;
        };
        /** @enum {string} */
        ReadingSort: "added" | "title" | "series";
        /** @enum {string} */
        ReadingSortDir: "asc" | "desc";
        ReadingUserState: {
            completed: boolean;
            favorite: boolean;
            /** Format: date-time */
            lastReadAt: string | null;
            locator: string | null;
            /** Format: int32 */
            page: number;
            /** Format: int32 */
            totalPages: number | null;
        };
        RegistrySkin: components["schemas"]["SkinInfo"] & {
            downloadUrl: string;
            installed: boolean;
            publishedAt: string;
            sha256: string;
        };
        RelatedWork: {
            artworkId?: string | null;
            entryKind: components["schemas"]["MediaCollectionEntryKind"];
            libraryId: components["schemas"]["LibraryId"];
            subtitle?: string | null;
            targetId: string;
            title: string;
        };
        RelatedWorkList: {
            works: components["schemas"]["RelatedWork"][];
        };
        /** @enum {string} */
        RequestedMode: "auto" | "direct" | "transcode";
        /** @enum {string} */
        Risk: "read" | "write" | "destructive";
        /** @enum {string} */
        Role: "admin" | "user";
        Room: {
            id: string;
            name: string;
        };
        /**
         * @description Action executed when a scheduled routine runs.
         * @enum {string}
         */
        RoutineActionKind: "scan_all" | "scan_library" | "home_scene" | "assistant_prompt";
        /**
         * @description Schedule trigger type for automated routines.
         * @enum {string}
         */
        RoutineScheduleKind: "hourly" | "daily" | "weekly" | "on_event";
        RunRoutineResponse: {
            detail: string;
            routine: components["schemas"]["ScheduledRoutine"];
        };
        /** @enum {string} */
        RunState: "running" | "awaiting_approval" | "succeeded" | "failed" | "cancelled";
        RunStep: {
            detail?: string | null;
            id: string;
            state: components["schemas"]["RunStepState"];
            text: string;
        };
        /** @enum {string} */
        RunStepState: "running" | "done" | "failed";
        RunSubject: {
            artworkId: string | null;
            title: string;
        };
        /** @description Administrator overrides. Null restores boot configuration or host detection. */
        RuntimeSettings: {
            perfTier: null | components["schemas"]["PerfTier"];
            serverName: string | null;
        };
        SaveHomeBridgeRequest: {
            baseUrl: string;
            enabled?: boolean;
            id?: string | null;
            name: string;
            token?: string | null;
        };
        SaveMetadataOverrideRequest: {
            backdropUrl?: string | null;
            externalId?: string | null;
            genres?: string[] | null;
            originalTitle?: string | null;
            overview?: string | null;
            posterUrl?: string | null;
            provider?: null | components["schemas"]["MetadataProvider"];
            /** Format: double */
            rating?: number | null;
            title?: string | null;
            workKey?: string | null;
            /** Format: int32 */
            year?: number | null;
        };
        SaveMqttBridgeRequest: {
            ambientLightDeviceId?: string | null;
            enabled?: boolean;
            host: string;
            id?: string | null;
            name: string;
            password?: string | null;
            /** Format: int32 */
            port?: number;
            publishPlayback?: boolean;
            topicPrefix?: string;
            username?: string | null;
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
        Scene: {
            active: boolean;
            icon: components["schemas"]["SceneIcon"];
            id: string;
            name: string;
        };
        /** @enum {string} */
        SceneIcon: "movie" | "home" | "night" | "morning" | "away";
        ScheduledRoutine: {
            actionKind: components["schemas"]["RoutineActionKind"];
            actionPayload: string;
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            id: string;
            /** Format: date-time */
            lastRunAt?: string | null;
            lastStatus?: string | null;
            name: string;
            ownerId: components["schemas"]["UserId"];
            scheduleExpr: string;
            scheduleKind: components["schemas"]["RoutineScheduleKind"];
        };
        ScheduledRoutineList: {
            routines: components["schemas"]["ScheduledRoutine"][];
        };
        SendMessageRequest: {
            context?: null | components["schemas"]["AssistantContext"];
            text: string;
        };
        SendMessageResponse: {
            message: components["schemas"]["AssistantMessage"];
            reply: components["schemas"]["AssistantMessage"];
            run: components["schemas"]["AssistantRun"];
        };
        /** @enum {string} */
        SensorKind: "temperature" | "humidity" | "contact" | "motion" | "pm25";
        SensorReading: {
            kind: components["schemas"]["SensorKind"];
            unit?: string | null;
            value: components["schemas"]["SensorValue"];
        };
        SensorValue: boolean | number;
        ServerEventMap: {
            "assistant.run.updated": components["schemas"]["AssistantRun"];
            "home.device.updated": components["schemas"]["HomeDevice"];
            "job.updated": components["schemas"]["Job"];
            "library.deleted": {
                /** Format: uuid */
                id: string;
            };
            "library.updated": components["schemas"]["Library"];
            ping: Record<string, never>;
            "playback.command": components["schemas"]["PlaybackCommandEvent"];
            "playback.session.updated": components["schemas"]["PlaybackSessionUpdatedEvent"];
            "settings.updated": components["schemas"]["RuntimeSettings"];
            "stream.reset": Record<string, never>;
            "task.updated": components["schemas"]["Task"];
        };
        /**
         * Format: uuid
         * @description This server installation; generated on first start and never changed.
         */
        ServerId: string;
        ServerLogEntry: {
            id: string;
            level: components["schemas"]["ServerLogLevel"];
            message: string;
            source: string;
            target: string;
            /** Format: date-time */
            timestamp: string;
        };
        /** @enum {string} */
        ServerLogLevel: "info" | "warn" | "error";
        ServerLogList: {
            entries: components["schemas"]["ServerLogEntry"][];
            total: number;
        };
        /**
         * Format: uuid
         * @description A login session (one bearer token).
         */
        SessionId: string;
        SetHomeRuleRequest: {
            enabled: boolean;
        };
        SetupRequest: {
            password: string;
            username: string;
        };
        SetupResponse: {
            user: components["schemas"]["User"];
        };
        /**
         * Format: uuid
         * @description A public or password-protected share link.
         */
        ShareId: string;
        ShareLinkList: {
            shares: components["schemas"]["ShareLinkSummary"][];
        };
        ShareLinkSummary: {
            allowDownload: boolean;
            /** Format: date-time */
            createdAt: string;
            creatorId: components["schemas"]["UserId"];
            expired: boolean;
            /** Format: date-time */
            expiresAt?: string | null;
            hasPassword: boolean;
            id: components["schemas"]["ShareId"];
            libraryId?: null | components["schemas"]["LibraryId"];
            /** Format: int32 */
            maxViews?: number | null;
            /** Format: date-time */
            revokedAt?: string | null;
            sharePath: string;
            stripGps: boolean;
            targetId: string;
            targetKind: components["schemas"]["ShareTargetKind"];
            title: string;
            token: string;
            /** Format: int64 */
            viewCount: number;
        };
        /** @enum {string} */
        ShareMediaKind: "video" | "audio" | "photo" | "book" | "comic";
        /** @enum {string} */
        ShareTargetKind: "item" | "video_work" | "music_album" | "playlist" | "photo_album" | "collection";
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
        SpokenChapter: {
            /** Format: int64 */
            startMs: number;
            title: string | null;
        };
        SpokenChapterList: {
            chapters: components["schemas"]["SpokenChapter"][];
        };
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
        StatValue: number | string;
        /** @description A step parameter: the contract allows strings and numbers only. */
        StepValue: number | string;
        StorageFolders: {
            directories: string[];
            truncated: boolean;
        };
        StorageMount: {
            affectedLibraries: number;
            id: string;
            label: string;
            /** Format: date-time */
            since: string;
            state: components["schemas"]["MountState"];
        };
        StorageReport: {
            byLibrary: components["schemas"]["LibraryBytes"][];
            mounts: components["schemas"]["StorageMount"][];
            volumes: components["schemas"]["StorageVolume"][];
        };
        StorageVolume: {
            id: string;
            label: string;
            path: string;
            /** Format: int64 */
            totalBytes: number;
            /** Format: int64 */
            usedBytes: number;
        };
        SubscriptionCheckResult: {
            matches: components["schemas"]["FeedMatchItem"][];
            submittedCount: number;
            subscription: components["schemas"]["MediaSubscription"];
            totalFeedItems: number;
        };
        /** @enum {string} */
        SubscriptionMediaKind: "series" | "movie" | "music" | "book";
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
        SubtitleStreamInfo: {
            codec: string;
            default: boolean;
            forced: boolean;
            /** Format: int32 */
            index: number;
            language: string | null;
            /** @description Can be converted to WebVTT by `GET /items/{id}/subtitles/{track}`. */
            textBased: boolean;
            title: string | null;
        };
        SubtitleTrack: {
            forced: boolean;
            format: components["schemas"]["SubtitleFormat"];
            hearingImpaired: boolean;
            id: string;
            language: string | null;
        };
        Suggestion: {
            actionLabel?: string | null;
            icon: components["schemas"]["SuggestionIcon"];
            id: string;
            prompt: string;
            text: string;
        };
        /** @enum {string} */
        SuggestionIcon: "duplicate" | "storage" | "music" | "subtitle" | "scene";
        SuggestionList: {
            suggestions: components["schemas"]["Suggestion"][];
        };
        SyncChangesResponse: {
            hasMore: boolean;
            items: components["schemas"]["SyncItemDelta"][];
            /** Format: int64 */
            latestRevision: number;
            libraries: components["schemas"]["SyncLibraryDelta"][];
            nextCursor?: string | null;
            playlists: components["schemas"]["SyncPlaylistDelta"][];
            readingBookmarks: components["schemas"]["SyncReadingBookmarkDelta"][];
            /** Format: int64 */
            since: number;
            userStates: components["schemas"]["SyncUserStateDelta"][];
        };
        SyncConflict: {
            index: number;
            kind: string;
            reason: string;
            /** Format: int64 */
            serverRevision: number;
            targetId: string;
        };
        SyncItemDelta: {
            deleted: boolean;
            id: components["schemas"]["ItemId"];
            kind: components["schemas"]["ItemKind"];
            libraryId: components["schemas"]["LibraryId"];
            /** Format: date-time */
            modifiedAt?: string | null;
            parentId: null | components["schemas"]["ItemId"];
            /** Format: int64 */
            revision: number;
            /** Format: int64 */
            sizeBytes?: number | null;
            sortTitle: string;
            title: string;
            /** Format: date-time */
            updatedAt: string;
        };
        SyncLibraryDelta: {
            deleted: boolean;
            id: components["schemas"]["LibraryId"];
            kind: components["schemas"]["LibraryKind"];
            /** Format: date-time */
            lastScanAt?: string | null;
            name: string;
            /** Format: int64 */
            revision: number;
            /** Format: date-time */
            updatedAt: string;
        };
        SyncMutation: {
            /** Format: int64 */
            baseRevision?: number | null;
            /** Format: date-time */
            clientUpdatedAt?: string | null;
            favorite: boolean;
            itemId: components["schemas"]["ItemId"];
            /** @enum {string} */
            type: "set_favorite";
        } | {
            /** Format: int64 */
            baseRevision?: number | null;
            /** Format: date-time */
            clientUpdatedAt?: string | null;
            completed?: boolean;
            itemId: components["schemas"]["ItemId"];
            /** Format: int64 */
            positionMs: number;
            /** @enum {string} */
            type: "set_progress";
        } | {
            /** Format: int64 */
            baseRevision?: number | null;
            bookmarkId?: null | components["schemas"]["BookmarkId"];
            /** Format: date-time */
            clientUpdatedAt?: string | null;
            itemId: components["schemas"]["ItemId"];
            label: string;
            locator?: string | null;
            note?: string | null;
            /** Format: int32 */
            page: number;
            /** @enum {string} */
            type: "add_reading_bookmark";
        } | {
            /** Format: int64 */
            baseRevision?: number | null;
            bookmarkId: components["schemas"]["BookmarkId"];
            /** Format: date-time */
            clientUpdatedAt?: string | null;
            /** @enum {string} */
            type: "remove_reading_bookmark";
        };
        SyncPlaylistDelta: {
            /** Format: date-time */
            changedAt?: string | null;
            comment?: string | null;
            deleted: boolean;
            id: components["schemas"]["PlaylistId"];
            name: string;
            trackIds: components["schemas"]["ItemId"][];
            /** Format: int64 */
            version: number;
        };
        SyncPushRequest: {
            clientId: string;
            mutations?: components["schemas"]["SyncMutation"][];
        };
        SyncPushResponse: {
            applied: number;
            conflicts: components["schemas"]["SyncConflict"][];
            /** Format: int64 */
            latestRevision: number;
        };
        SyncReadingBookmarkDelta: {
            /** Format: date-time */
            createdAt: string;
            id: components["schemas"]["BookmarkId"];
            itemId: components["schemas"]["ItemId"];
            label: string;
            locator?: string | null;
            note?: string | null;
            /** Format: int32 */
            page: number;
        };
        SyncUserStateDelta: {
            completed: boolean;
            favorite: boolean;
            /** Format: int64 */
            favoriteOrder?: number | null;
            itemId: components["schemas"]["ItemId"];
            /** Format: date-time */
            lastPlayedAt?: string | null;
            /** Format: int64 */
            playCount: number;
            /** Format: int64 */
            playbackVersion: number;
            /** Format: int64 */
            positionMs: number;
            /** Format: int32 */
            stars: number;
        };
        SystemDoctorReport: {
            checks: components["schemas"]["DoctorCheck"][];
            /** Format: date-time */
            generatedAt: string;
            overallStatus: components["schemas"]["DoctorStatus"];
        };
        SystemInfo: {
            /** @description Capabilities registered at startup; clients hide missing modules. */
            features: string[];
            name: string;
            serverId: components["schemas"]["ServerId"];
            setupCompleted: boolean;
            version: string;
        };
        SystemMetrics: {
            /** Format: double */
            cpu: number;
            devicesOnline: number;
            /** Format: int64 */
            memoryTotalBytes: number;
            /** Format: int64 */
            memoryUsedBytes: number;
        };
        TableColumn: {
            align?: null | components["schemas"]["ColumnAlign"];
            format?: null | components["schemas"]["CellFormat"];
            key: string;
            label: string;
        };
        Task: {
            /** Format: date-time */
            createdAt: string;
            error: string | null;
            /** Format: int64 */
            etaSeconds?: number | null;
            /** Format: date-time */
            finishedAt: string | null;
            id: string;
            kind: string;
            libraryId?: string | null;
            message: string | null;
            origin: null | components["schemas"]["TaskOrigin"];
            previews?: string[] | null;
            /** Format: double */
            progress: number | null;
            rate?: string | null;
            result: components["schemas"]["Block"][];
            scope?: string | null;
            /** Format: date-time */
            startedAt: string | null;
            state: components["schemas"]["TaskState"];
            step?: null | components["schemas"]["JobStep"];
            steps: components["schemas"]["TaskStep"][];
            subject?: null | components["schemas"]["TaskSubject"];
            title: string;
        };
        TaskList: {
            tasks: components["schemas"]["Task"][];
        };
        TaskOrigin: {
            label: string;
            module: string;
        };
        /** @enum {string} */
        TaskState: "queued" | "running" | "awaiting_approval" | "succeeded" | "failed" | "cancelled";
        TaskStep: {
            /** Format: date-time */
            at: string | null;
            detail: string | null;
            id: string;
            label: string;
            state: components["schemas"]["TaskStepState"];
        };
        /** @enum {string} */
        TaskStepState: "pending" | "running" | "done" | "failed" | "skipped";
        TaskSubject: {
            artworkId: string | null;
            detail: string | null;
            title: string;
            verb?: string | null;
        };
        TestChannelResponse: {
            delivery: components["schemas"]["NotificationDelivery"];
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
        /** @enum {string} */
        TransferState: "downloading" | "seeding" | "paused" | "queued" | "completed" | "error";
        UnlockShareRequest: {
            password: string;
        };
        UnlockShareResponse: {
            /** Format: date-time */
            expiresAt: string;
            proof: string;
        };
        UpdateAssistantConfigRequest: {
            apiKey?: string | null;
            baseUrl?: string;
            clearApiKey?: boolean;
            model?: string;
            preset: string;
            redactCloudPaths?: boolean;
        };
        UpdateLibraryRequest: {
            addPaths?: string[];
            name?: string | null;
        };
        UpdateMediaCollectionRequest: {
            coverArtworkId?: string | null;
            description?: string | null;
            itemOrder?: components["schemas"]["CollectionEntryId"][] | null;
            title?: string | null;
        };
        UpdateMetadataConfigRequest: {
            autoFetchPosters?: boolean | null;
            bangumiToken?: string | null;
            clearBangumiToken?: boolean | null;
            clearTmdbKey?: boolean | null;
            preferredLanguage?: string | null;
            tmdbApiKey?: string | null;
        };
        UpdateNotificationChannelRequest: {
            config?: null | components["schemas"]["NotificationChannelConfig"];
            enabled?: boolean | null;
            events?: string[] | null;
            name?: string | null;
            /** @description Empty string clears quiet hours; `"HH:MM-HH:MM"` updates it. */
            quietHours?: string | null;
        };
        UpdatePhotoAlbumRequest: {
            coverItemId?: null | components["schemas"]["ItemId"];
            description?: string | null;
            title?: string | null;
        };
        UpdatePhotoPersonRequest: {
            coverItemId?: null | components["schemas"]["ItemId"];
            name?: string | null;
        };
        UpdatePluginRequest: {
            enabled?: boolean | null;
            settings?: unknown;
        };
        UpdateRoutineRequest: {
            actionKind?: null | components["schemas"]["RoutineActionKind"];
            actionPayload?: string | null;
            enabled?: boolean | null;
            name?: string | null;
            scheduleExpr?: string | null;
            scheduleKind?: null | components["schemas"]["RoutineScheduleKind"];
        };
        UpdateUserRequest: {
            allowedLibraryIds?: components["schemas"]["LibraryId"][] | null;
            disabled?: boolean | null;
            password?: string | null;
            policy?: null | components["schemas"]["UserPolicyPatch"];
            role?: null | components["schemas"]["Role"];
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
        UserList: {
            users: components["schemas"]["ManagedUser"][];
        };
        /** @description Per-account capability and quota policy. */
        UserPolicy: {
            allLibraries: boolean;
            allowDownload: boolean;
            allowTranscode: boolean;
            /** Format: int32 */
            dailyAiLimit: number;
        };
        /** @description Partial update to a [`UserPolicy`]. */
        UserPolicyPatch: {
            allLibraries?: boolean | null;
            allowDownload?: boolean | null;
            allowTranscode?: boolean | null;
            /** Format: int32 */
            dailyAiLimit?: number | null;
        };
        UserSession: {
            /** Format: date-time */
            createdAt: string;
            current: boolean;
            deviceId: null | components["schemas"]["DeviceId"];
            deviceName: string;
            /** Format: date-time */
            expiresAt: string | null;
            id: components["schemas"]["SessionId"];
            /** Format: date-time */
            lastUsedAt: string;
        };
        UserSessionList: {
            sessions: components["schemas"]["UserSession"][];
        };
        VideoEntry: {
            /** Format: date-time */
            addedAt: string;
            backdropItemId: null | components["schemas"]["ItemId"];
            /**
             * Format: double
             * @description 0–10.
             */
            communityRating: number | null;
            /** Format: int32 */
            episodeNumber: number | null;
            /** Format: int64 */
            fileCount: number;
            genres: string[];
            id: string;
            itemId: null | components["schemas"]["ItemId"];
            kind: components["schemas"]["VideoKind"];
            libraryId: components["schemas"]["LibraryId"];
            officialRating: string | null;
            originalTitle: string | null;
            overview: string | null;
            posterItemId: null | components["schemas"]["ItemId"];
            /** @description `YYYY-MM-DD`. */
            premiered: string | null;
            /** Format: int64 */
            runtimeMs: number | null;
            /** Format: int32 */
            seasonNumber: number | null;
            /** @description The series work for seasons and episodes. */
            seriesId: string | null;
            seriesTitle: string | null;
            state: null | components["schemas"]["VideoUserState"];
            tagline: string | null;
            thumbItemId: null | components["schemas"]["ItemId"];
            title: string;
            /** Format: int32 */
            year: number | null;
        };
        VideoFile: {
            itemId: components["schemas"]["ItemId"];
            /** Format: int64 */
            sizeBytes: number | null;
            title: string;
        };
        VideoGenre: {
            /** Format: int64 */
            count: number;
            name: string;
        };
        VideoGenreList: {
            genres: components["schemas"]["VideoGenre"][];
        };
        VideoHome: {
            continueWatching: components["schemas"]["VideoEntry"][];
            latestEpisodes: components["schemas"]["VideoEntry"][];
            latestMovies: components["schemas"]["VideoEntry"][];
            nextUp: components["schemas"]["VideoEntry"][];
        };
        /** @enum {string} */
        VideoKind: "movie" | "series" | "season" | "episode";
        VideoPage: {
            entries: components["schemas"]["VideoEntry"][];
            nextCursor: string | null;
            /**
             * Format: int64
             * @description Number of entries matching the filters.
             */
            total: number;
        };
        VideoPerson: {
            /** @description Relative URL of a local portrait. */
            imageUrl: string | null;
            kind: components["schemas"]["VideoPersonKind"];
            name: string;
            role: string | null;
        };
        /** @enum {string} */
        VideoPersonKind: "actor" | "director" | "writer" | "producer";
        VideoPlaybackRequest: {
            /** Format: int32 */
            audioIndex?: number | null;
            client: components["schemas"]["ClientProfile"];
            /** Format: int32 */
            maxBitrateKbps?: number | null;
            /** Format: int32 */
            maxHeight?: number | null;
            mode?: null | components["schemas"]["RequestedMode"];
            /** Format: int64 */
            startMs?: number | null;
        };
        VideoPlaybackResponse: {
            directUrl?: string | null;
            /** Format: date-time */
            expiresAt: string;
            mode: components["schemas"]["DeliveryMode"];
            protocol: components["schemas"]["PlaybackProtocol"];
            reason: string | null;
            sessionId: string;
            /** Format: int64 */
            startMs: number;
            url: string;
        };
        VideoSegment: {
            /** Format: int64 */
            endMs: number;
            id: string;
            itemId: components["schemas"]["ItemId"];
            kind: components["schemas"]["VideoSegmentKind"];
            source: components["schemas"]["VideoSegmentSource"];
            /** Format: int64 */
            startMs: number;
            /** Format: date-time */
            updatedAt: string;
        };
        VideoSegmentInput: {
            /** Format: int64 */
            endMs: number;
            kind: components["schemas"]["VideoSegmentKind"];
            source?: null | components["schemas"]["VideoSegmentSource"];
            /** Format: int64 */
            startMs: number;
        };
        /** @enum {string} */
        VideoSegmentKind: "intro" | "credits" | "recap";
        VideoSegmentList: {
            segments: components["schemas"]["VideoSegment"][];
        };
        /** @enum {string} */
        VideoSegmentSource: "manual" | "chapter" | "auto";
        VideoStreamInfo: {
            /** Format: int32 */
            bitDepth: number | null;
            codec: string;
            /** Format: double */
            frameRate: number | null;
            hdr: null | components["schemas"]["HdrFormat"];
            /** Format: int32 */
            height: number;
            profile: string | null;
            /** Format: int32 */
            width: number;
        };
        /** @description Per-account state. For series and seasons the values summarize episodes. */
        VideoUserState: {
            favorite: boolean;
            /** Format: date-time */
            lastPlayedAt: string | null;
            /** Format: int64 */
            playCount: number;
            played: boolean;
            /** Format: int64 */
            positionMs: number;
            /**
             * Format: int64
             * @description Series and seasons only.
             */
            unplayedCount: number | null;
        };
        VideoWorkDetail: {
            files: components["schemas"]["VideoFile"][];
            nextUp: null | components["schemas"]["VideoEntry"];
            people: components["schemas"]["VideoPerson"][];
            seasons: components["schemas"]["VideoEntry"][];
            similar: components["schemas"]["VideoEntry"][];
            studios: string[];
            work: components["schemas"]["VideoEntry"];
        };
        VirtualChannel: {
            /** Format: date-time */
            createdAt: string;
            enabled: boolean;
            genreFilter?: string | null;
            id: string;
            libraryId: components["schemas"]["LibraryId"];
            name: string;
            /** Format: int32 */
            number: number;
        };
        VirtualChannelGuide: {
            channel: components["schemas"]["VirtualChannel"];
            /** Format: int64 */
            cycleDurationMs: number;
            nowPlaying?: null | components["schemas"]["VirtualProgramme"];
            programmes: components["schemas"]["VirtualProgramme"][];
            totalItems: number;
        };
        VirtualChannelList: {
            channels: components["schemas"]["VirtualChannel"][];
        };
        VirtualProgramme: {
            /** Format: int64 */
            durationMs: number;
            /** Format: date-time */
            endsAt: string;
            /** Format: int32 */
            episode?: number | null;
            itemId: components["schemas"]["ItemId"];
            libraryId: components["schemas"]["LibraryId"];
            /** Format: int64 */
            offsetMs: number;
            /** Format: int32 */
            season?: number | null;
            /** Format: date-time */
            startedAt: string;
            title: string;
            workTitle: string;
        };
        VirtualTvGuide: {
            channels: components["schemas"]["VirtualChannelGuide"][];
            epgUrl: string;
            /** Format: date-time */
            generatedAt: string;
            playlistUrl: string;
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
    getArtwork: {
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
    getAssistantConfig: {
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
                    "application/json": components["schemas"]["AssistantConfigResponse"];
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
    updateAssistantConfig: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAssistantConfigRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AssistantConfigResponse"];
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
    getAssistantConversation: {
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
                    "application/json": components["schemas"]["Conversation"];
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
    resetAssistantConversation: {
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
                    "application/json": components["schemas"]["Conversation"];
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
    listAssistantMemories: {
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
                    "application/json": components["schemas"]["AssistantMemoryList"];
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
    createAssistantMemory: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAssistantMemoryRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AssistantMemory"];
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
    deleteAssistantMemory: {
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
    sendAssistantMessage: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SendMessageRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SendMessageResponse"];
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
    approveAssistantRun: {
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
                "application/json": components["schemas"]["ApproveRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AssistantRun"];
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
    backgroundAssistantRun: {
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
                    "application/json": components["schemas"]["BackgroundRunResponse"];
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
    cancelAssistantRun: {
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
                    "application/json": components["schemas"]["AssistantRun"];
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
    listAssistantSuggestions: {
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
                    "application/json": components["schemas"]["SuggestionList"];
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
    getCatalog: {
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
                    "application/json": components["schemas"]["Catalog"];
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
    listCollections: {
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
                    "application/json": components["schemas"]["MediaCollectionList"];
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
    createCollection: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateMediaCollectionRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaCollectionSummary"];
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
    getCollection: {
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
                    "application/json": components["schemas"]["MediaCollectionDetail"];
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
    deleteCollection: {
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
    updateCollection: {
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
                "application/json": components["schemas"]["UpdateMediaCollectionRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaCollectionSummary"];
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
    addCollectionItem: {
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
                "application/json": components["schemas"]["AddMediaCollectionItemRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaCollectionEntry"];
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
    removeCollectionItem: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                entryId: string;
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
    getDownloadsOverview: {
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
                    "application/json": components["schemas"]["DownloadsOverviewResponse"];
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
    saveDownloaderClient: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDownloaderClientRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DownloaderClientSummary"];
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
    deleteDownloaderClient: {
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
    createMediaSubscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSubscriptionRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MediaSubscription"];
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
    deleteMediaSubscription: {
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
    checkMediaSubscription: {
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
                    "application/json": components["schemas"]["SubscriptionCheckResult"];
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
    createDownloadTask: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDownloadTaskRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreateDownloadTaskResponse"];
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
    getHomeSnapshot: {
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
                    "application/json": components["schemas"]["HomeSnapshot"];
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
    listHomeBridges: {
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
                    "application/json": components["schemas"]["HomeBridgeList"];
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
    saveHomeBridge: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SaveHomeBridgeRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HomeBridgeSummary"];
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
    deleteHomeBridge: {
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
    syncHomeBridge: {
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
                    "application/json": components["schemas"]["HomeBridgeSyncResult"];
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
    createHomeDevice: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateHomeDeviceRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HomeDevice"];
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
    updateHomeDevice: {
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
                "application/json": components["schemas"]["DevicePatch"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HomeDevice"];
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
    deleteHomeDevice: {
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
    listMqttBridges: {
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
                    "application/json": components["schemas"]["MqttBridgeList"];
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
    saveMqttBridge: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SaveMqttBridgeRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MqttBridgeSummary"];
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
    deleteMqttBridge: {
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
    testMqttBridge: {
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
                    "application/json": components["schemas"]["MqttTestResponse"];
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
    createHomeRule: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateHomeRuleRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AutomationRule"];
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
    setHomeRule: {
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
                "application/json": components["schemas"]["SetHomeRuleRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AutomationRule"];
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
    deleteHomeRule: {
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
    activateHomeScene: {
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
                    "application/json": components["schemas"]["HomeSnapshot"];
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
            query?: {
                size?: number;
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
    getVideoBackdrop: {
        parameters: {
            query?: {
                size?: number;
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
    downloadBook: {
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
                    /** @description An attachment with the book's download filename. */
                    "Content-Disposition"?: string;
                    "Content-Length"?: number;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
                };
            };
            /** @description Partial content */
            206: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    /** @description An attachment with the book's download filename. */
                    "Content-Disposition"?: string;
                    "Content-Length"?: number;
                    "Content-Range"?: string;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
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
    prepareBookDownload: {
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
                    "application/json": components["schemas"]["BookDownload"];
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
    headBook: {
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
                    /** @description An attachment with the book's download filename. */
                    "Content-Disposition"?: string;
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
    listSpokenChapters: {
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
                    "application/json": components["schemas"]["SpokenChapterList"];
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
    getVideoMediaInfo: {
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
                    "application/json": components["schemas"]["MediaInfo"];
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
    getItemMetadataOverride: {
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
                    "application/json": components["schemas"]["MetadataOverrideResponse"];
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
    saveItemMetadataOverride: {
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
                "application/json": components["schemas"]["SaveMetadataOverrideRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MetadataOverride"];
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
    deleteItemMetadataOverride: {
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
    getPhotoImage: {
        parameters: {
            query?: {
                size?: number;
                original?: boolean;
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
    getPhotoInfo: {
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
                    "application/json": components["schemas"]["PhotoInfo"];
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
    listPhotoItemPeople: {
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
                    "application/json": components["schemas"]["PhotoItemPeopleResponse"];
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
    tagPhotoPerson: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                personId: string;
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
                    "application/json": components["schemas"]["PhotoPersonSummary"];
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
    untagPhotoPerson: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                personId: string;
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
                size?: number;
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
    getReadingAsset: {
        parameters: {
            query: {
                path: string;
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
                    "image/avif": string;
                    "image/gif": string;
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
    getReadingChapter: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                index: number;
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
                    "application/json": components["schemas"]["ReadingChapter"];
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
            /** @description Unsupported Media Type */
            415: {
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
    getReadingCover: {
        parameters: {
            query?: {
                size?: number;
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
                    "image/avif": string;
                    "image/gif": string;
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
    streamReadingInline: {
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
                    "Content-Disposition"?: "inline";
                    "Content-Length"?: number;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": string;
                };
            };
            /** @description Partial content */
            206: {
                headers: {
                    "Accept-Ranges"?: "bytes";
                    "Cache-Control"?: "private, no-store";
                    "Content-Disposition"?: "inline";
                    "Content-Length"?: number;
                    "Content-Range"?: string;
                    "Content-Type"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": string;
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
            /** @description Unsupported Media Type */
            415: {
                headers: {
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
    prepareReadingInline: {
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReadingInline"];
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
            /** @description Unsupported Media Type */
            415: {
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
    getReadingManifest: {
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
                    "application/json": components["schemas"]["ReadingManifest"];
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
    getReadingPage: {
        parameters: {
            query?: {
                size?: number;
            };
            header?: never;
            path: {
                id: string;
                page: number;
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
                    "image/avif": string;
                    "image/gif": string;
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
            /** @description Unsupported Media Type */
            415: {
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
    listVideoSegments: {
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
                    "application/json": components["schemas"]["VideoSegmentList"];
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
    saveVideoSegments: {
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
                "application/json": components["schemas"]["PutVideoSegmentsRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VideoSegmentList"];
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
    getVideoThumb: {
        parameters: {
            query?: {
                size?: number;
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
    getVideoTrickplay: {
        parameters: {
            query?: {
                atMs?: number;
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
    startVideoPlayback: {
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
                "application/json": components["schemas"]["VideoPlaybackRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VideoPlaybackResponse"];
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
    listMusicAlbums: {
        parameters: {
            query?: {
                sort?: components["schemas"]["MusicAlbumSort"];
                order?: components["schemas"]["MusicSortOrder"];
                genre?: string;
                artistId?: string;
                yearFrom?: number;
                yearTo?: number;
                favorite?: boolean;
                q?: string;
                seed?: string;
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
                    "application/json": components["schemas"]["MusicAlbumPage"];
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
    getMusicAlbum: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                albumId: string;
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
                    "application/json": components["schemas"]["MusicAlbumDetail"];
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
    favoriteMusicAlbum: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                albumId: string;
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
    unfavoriteMusicAlbum: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                albumId: string;
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
    listMusicArtists: {
        parameters: {
            query?: {
                sort?: components["schemas"]["MusicArtistSort"];
                order?: components["schemas"]["MusicSortOrder"];
                favorite?: boolean;
                q?: string;
                seed?: string;
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
                    "application/json": components["schemas"]["MusicArtistPage"];
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
    getMusicArtist: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                artistId: string;
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
                    "application/json": components["schemas"]["MusicArtistDetail"];
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
    favoriteMusicArtist: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                artistId: string;
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
    unfavoriteMusicArtist: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                artistId: string;
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
    listMusicGenres: {
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
                    "application/json": components["schemas"]["MusicGenreList"];
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
    listMusicTracks: {
        parameters: {
            query?: {
                sort?: components["schemas"]["MusicTrackSort"];
                order?: components["schemas"]["MusicSortOrder"];
                genre?: string;
                artistId?: string;
                albumId?: string;
                favorite?: boolean;
                q?: string;
                seed?: string;
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
                    "application/json": components["schemas"]["MusicTrackPage"];
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
    uploadPhoto: {
        parameters: {
            query?: {
                fileName?: string;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PhotoUploadRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoInfo"];
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
    listReadingItems: {
        parameters: {
            query?: {
                kind?: components["schemas"]["ReadingKind"];
                series?: string;
                author?: string;
                format?: string;
                favorite?: boolean;
                completed?: boolean;
                q?: string;
                sort?: components["schemas"]["ReadingSort"];
                dir?: components["schemas"]["ReadingSortDir"];
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
                    "application/json": components["schemas"]["ReadingPage"];
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
                genre?: string;
                yearFrom?: number;
                yearTo?: number;
                played?: boolean;
                favorite?: boolean;
                startsWith?: string;
                person?: string;
                order?: "title" | "poster" | "year" | "added" | "rating" | "runtime" | "random";
                dir?: "asc" | "desc";
                seed?: string;
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
    listVideoGenres: {
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
                    "application/json": components["schemas"]["VideoGenreList"];
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
    getVideoPersonImage: {
        parameters: {
            query?: {
                size?: number;
            };
            header?: never;
            path: {
                id: string;
                key: string;
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
    getVideoWork: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                workId: string;
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
                    "application/json": components["schemas"]["VideoWorkDetail"];
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
    favoriteVideoWork: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                workId: string;
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
    unfavoriteVideoWork: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                workId: string;
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
    markVideoWorkPlayed: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                workId: string;
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
    markVideoWorkUnplayed: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                workId: string;
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
    listLinkDevices: {
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LinkDeviceList"];
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
    revokeLinkDevice: {
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
    pairLinkDevice: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LinkPairRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LinkPairing"];
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
    createLinkChallenge: {
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LinkChallenge"];
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
    postMcp: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["McpJsonRpcRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["McpJsonRpcResponse"];
                };
            };
            /** @description Accepted JSON-RPC notification */
            202: {
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
    getMcpInfo: {
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
                    "application/json": components["schemas"]["McpInfoResponse"];
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
    listClientApiKeys: {
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
                    "application/json": components["schemas"]["ClientApiKeyList"];
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
    createClientApiKey: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateClientApiKeyRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedClientApiKey"];
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
    revokeClientApiKey: {
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
    deleteReadingBookmark: {
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
    createReadingBookmark: {
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
                "application/json": components["schemas"]["CreateBookmarkRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReadingBookmark"];
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
    markItemPlayed: {
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
    markItemUnplayed: {
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
    saveReadingProgress: {
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
                "application/json": components["schemas"]["ReadingProgressUpdate"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReadingUserState"];
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
    getOpdsAccess: {
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OpdsAccess"];
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
    changeMyPassword: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangePasswordRequest"];
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
    listPhotoAlbums: {
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
                    "application/json": components["schemas"]["PhotoAlbumList"];
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
    createPhotoAlbum: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePhotoAlbumRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoAlbumSummary"];
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
    getPhotoAlbum: {
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
                    "application/json": components["schemas"]["PhotoAlbumSummary"];
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
    deletePhotoAlbum: {
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
    updatePhotoAlbum: {
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
                "application/json": components["schemas"]["UpdatePhotoAlbumRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoAlbumSummary"];
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
    addPhotoAlbumItems: {
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
                "application/json": components["schemas"]["PhotoAlbumItemsRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoAlbumSummary"];
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
    removePhotoAlbumItems: {
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
                "application/json": components["schemas"]["PhotoAlbumItemsRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoAlbumSummary"];
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
    listPhotoPeople: {
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
                    "application/json": components["schemas"]["PhotoPersonList"];
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
    createPhotoPerson: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePhotoPersonRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoPersonSummary"];
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
    deletePhotoPerson: {
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
    updatePhotoPerson: {
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
                "application/json": components["schemas"]["UpdatePhotoPersonRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PhotoPersonSummary"];
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
    listMySessions: {
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
                    "application/json": components["schemas"]["UserSessionList"];
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
    revokeMySession: {
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
    getMetadataConfig: {
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
                    "application/json": components["schemas"]["MetadataConfigResponse"];
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
    updateMetadataConfig: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateMetadataConfigRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MetadataConfigResponse"];
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
    searchMetadata: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MetadataSearchRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MetadataSearchResponse"];
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
    lookupMusicTracks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MusicTrackLookupRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MusicTrackLookupResponse"];
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
    listNotifications: {
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
                    "application/json": components["schemas"]["NotificationList"];
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
    listNotificationChannels: {
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
                    "application/json": components["schemas"]["NotificationChannelList"];
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
    createNotificationChannel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateNotificationChannelRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NotificationChannel"];
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
    deleteNotificationChannel: {
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
    updateNotificationChannel: {
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
                "application/json": components["schemas"]["UpdateNotificationChannelRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NotificationChannel"];
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
    testNotificationChannel: {
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
                    "application/json": components["schemas"]["TestChannelResponse"];
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
    markAllNotificationsRead: {
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
                    "application/json": components["schemas"]["ReadAllNotificationsResponse"];
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
    applyMediaOrganize: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OrganizeApplyRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OrganizeBatchSummary"];
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
    listOrganizeBatches: {
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
                    "application/json": components["schemas"]["OrganizeBatchList"];
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
    undoOrganizeBatch: {
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
                    "application/json": components["schemas"]["OrganizeBatchSummary"];
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
    previewMediaOrganize: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OrganizePreviewRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OrganizePreviewResponse"];
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
    analyzePhotos: {
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
                    "application/json": components["schemas"]["PhotoAnalyzeResponse"];
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
    listPhotoDuplicates: {
        parameters: {
            query?: {
                threshold?: number;
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
                    "application/json": components["schemas"]["PhotoDuplicatesResponse"];
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
    getPhotoMemories: {
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
                    "application/json": components["schemas"]["PhotoMemories"];
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
    searchPhotos: {
        parameters: {
            query?: {
                q?: string;
                personId?: string;
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
                    "application/json": components["schemas"]["PhotoSearchResponse"];
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
    getPhotoTimeline: {
        parameters: {
            query?: {
                libraryId?: string;
                albumId?: string;
                favorite?: boolean;
                hasLocation?: boolean;
                camera?: string;
                month?: string;
                place?: string;
                q?: string;
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
                    "application/json": components["schemas"]["PhotoTimelinePage"];
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
    getPluginCatalog: {
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
                    "application/json": components["schemas"]["PluginCatalogResponse"];
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
    installPlugin: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallPluginRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginSummary"];
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
        };
    };
    uninstallPlugin: {
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
    updatePlugin: {
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
                "application/json": components["schemas"]["UpdatePluginRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginSummary"];
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
    rollbackPlugin: {
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
                    "application/json": components["schemas"]["PluginSummary"];
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
    getPublicShare: {
        parameters: {
            query?: {
                proof?: string;
            };
            header?: never;
            path: {
                token: string;
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
                    "application/json": components["schemas"]["PublicShareView"];
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
            /** @description Gone */
            410: {
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
    getPublicShareCover: {
        parameters: {
            query: {
                proof: string;
                size?: number;
            };
            header?: never;
            path: {
                token: string;
                itemId: string;
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
            /** @description Gone */
            410: {
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
    streamPublicShareMedia: {
        parameters: {
            query: {
                proof: string;
                size?: number;
                download?: boolean;
            };
            header?: {
                Range?: string;
            };
            path: {
                token: string;
                itemId: string;
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
            /** @description Gone */
            410: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description Range Not Satisfiable */
            416: {
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
    unlockPublicShare: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UnlockShareRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UnlockShareResponse"];
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
            /** @description Gone */
            410: {
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
    getReadingHome: {
        parameters: {
            query?: {
                libraryId?: string;
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
                    "application/json": components["schemas"]["ReadingHome"];
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
    listRelatedWorks: {
        parameters: {
            query: {
                title: string;
                excludeKind?: components["schemas"]["MediaCollectionEntryKind"];
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
                    "application/json": components["schemas"]["RelatedWorkList"];
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
    listRoutines: {
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
                    "application/json": components["schemas"]["ScheduledRoutineList"];
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
    createRoutine: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRoutineRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledRoutine"];
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
    deleteRoutine: {
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
    updateRoutine: {
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
                "application/json": components["schemas"]["UpdateRoutineRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledRoutine"];
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
    runRoutineNow: {
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
                    "application/json": components["schemas"]["RunRoutineResponse"];
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
    globalSearch: {
        parameters: {
            query?: {
                q?: string;
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
                    "application/json": components["schemas"]["GlobalSearchResponse"];
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
    listPlaybackSessions: {
        parameters: {
            query?: {
                all?: boolean;
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
                    "application/json": components["schemas"]["PlaybackSessionList"];
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
    heartbeatPlaybackSession: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PlaybackHeartbeatRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaybackSession"];
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
    deletePlaybackSession: {
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
    sendPlaybackCommand: {
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
                "application/json": components["schemas"]["PlaybackCommandRequest"];
            };
        };
        responses: {
            /** @description Success */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PlaybackCommandAck"];
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
    listShares: {
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
                    "application/json": components["schemas"]["ShareLinkList"];
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
    createShare: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateShareRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ShareLinkSummary"];
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
    revokeShare: {
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
    getSyncChanges: {
        parameters: {
            query?: {
                since?: number;
                limit?: number;
                cursor?: string;
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
                    "application/json": components["schemas"]["SyncChangesResponse"];
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
    pushSyncMutations: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SyncPushRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SyncPushResponse"];
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
    listAuditLogs: {
        parameters: {
            query?: {
                action?: string;
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
                    "application/json": components["schemas"]["AuditLogPage"];
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
    listBackups: {
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
                    "application/json": components["schemas"]["BackupList"];
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
    createBackup: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateBackupRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BackupSummary"];
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
    deleteBackup: {
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
    downloadBackup: {
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
                    "Cache-Control"?: "private, no-store";
                    "Content-Disposition"?: string;
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": string;
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
    restoreBackup: {
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
                    "application/json": components["schemas"]["BackupRestoreResult"];
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
    getSystemDoctor: {
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
                    "application/json": components["schemas"]["SystemDoctorReport"];
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
    importExternalState: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExternalImportRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExternalImportResult"];
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
    getSystemLogs: {
        parameters: {
            query?: {
                level?: "all" | "info" | "warn" | "error";
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
                    "application/json": components["schemas"]["ServerLogList"];
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
    getSystemMetrics: {
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
                    "application/json": components["schemas"]["SystemMetrics"];
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
    getSystemPrometheus: {
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
                    "text/plain; version=0.0.4; charset=utf-8": string;
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
    getStorageReport: {
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
                    "application/json": components["schemas"]["StorageReport"];
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
    reconnectStorageMount: {
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
                    "application/json": components["schemas"]["StorageMount"];
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
    listTasks: {
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
                    "application/json": components["schemas"]["TaskList"];
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
    approveTask: {
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
                "application/json": components["schemas"]["ApproveRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Task"];
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
    cancelTask: {
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
                    "application/json": components["schemas"]["Task"];
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
    listUsers: {
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
                    "application/json": components["schemas"]["UserList"];
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
    createUser: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUserRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ManagedUser"];
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
    deleteUser: {
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
    updateUser: {
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
                "application/json": components["schemas"]["UpdateUserRequest"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ManagedUser"];
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
    deleteVideoSession: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                sessionId: string;
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
    getVideoMasterPlaylist: {
        parameters: {
            query?: {
                ticket?: string;
            };
            header?: never;
            path: {
                sessionId: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/vnd.apple.mpegurl": string;
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
    getVideoSegment: {
        parameters: {
            query?: {
                ticket?: string;
            };
            header?: never;
            path: {
                sessionId: string;
                file: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "video/mp2t": string;
                    "video/mp4": string;
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
    getVideoHome: {
        parameters: {
            query?: {
                libraryId?: string;
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
                    "application/json": components["schemas"]["VideoHome"];
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
    listVirtualChannels: {
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
                    "application/json": components["schemas"]["VirtualChannelList"];
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
    createVirtualChannel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateVirtualChannelRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VirtualChannel"];
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
    getVirtualTvEpg: {
        parameters: {
            query?: {
                ticket?: string;
                at?: string;
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "application/xml": string;
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
    getVirtualTvGuide: {
        parameters: {
            query?: {
                at?: string;
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
                    "application/json": components["schemas"]["VirtualTvGuide"];
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
    getVirtualTvPlaylist: {
        parameters: {
            query?: {
                ticket?: string;
                at?: string;
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
                    "Cache-Control"?: "private, no-store";
                    [name: string]: unknown;
                };
                content: {
                    "audio/x-mpegurl": string;
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
    deleteVirtualChannel: {
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
    listInboundWebhooks: {
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
                    "application/json": components["schemas"]["InboundWebhookList"];
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
    createInboundWebhook: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateInboundWebhookRequest"];
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedInboundWebhook"];
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
    deleteInboundWebhook: {
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
    triggerInboundWebhook: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InboundWebhookPayload"];
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InboundWebhookTriggerResponse"];
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
}
