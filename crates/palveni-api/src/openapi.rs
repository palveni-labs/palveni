//! OpenAPI responses, parameters and operation metadata.
mod schemas;

use std::sync::OnceLock;

use axum::Json;
use axum::extract::State;
use palveni_core::features::{Capability, Permission};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::AppState;
use crate::auth::CurrentUser;
use crate::endpoints::{Endpoint, Input, Output, endpoints};

#[derive(Serialize)]
pub struct Capabilities {
    capabilities: Vec<Capability>,
}

pub async fn capabilities(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Json<Capabilities> {
    Json(Capabilities {
        capabilities: state.features.capabilities(current.is_admin()),
    })
}

pub async fn document() -> Json<&'static Value> {
    Json(openapi_document())
}

pub fn openapi_document() -> &'static Value {
    static DOCUMENT: OnceLock<Value> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        let mut paths = json!({});
        for endpoint in endpoints() {
            let cap = &endpoint.capability;
            paths[cap.path][cap.method.to_ascii_lowercase()] = operation(&endpoint);
        }
        json!({
            "openapi": "3.1.0", "info": {"title":"Palveni Server","version":palveni_core::VERSION},
            "paths": paths, "components":{
                "schemas":schemas::components(),
                "securitySchemes":{
                    "bearerAuth":{"type":"http","scheme":"bearer"},
                    "mediaTicket":{"type":"apiKey","in":"query","name":"ticket","description":"Time-limited signature for one media entry, operation and session."}
                }
            }
        })
    })
}

fn reference(name: &str) -> Value {
    json!({"$ref":format!("#/components/schemas/{name}")})
}

fn content(mime: &str, schema: Value) -> Value {
    json!({(mime):{"schema":schema}})
}

fn json_content(schema: Value) -> Value {
    content("application/json", schema)
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required})
}

fn binary(mimes: &[&str]) -> Value {
    let mut media = Map::new();
    for mime in mimes {
        media.insert(
            (*mime).into(),
            json!({"schema":{"type":"string","format":"binary"}}),
        );
    }
    Value::Object(media)
}

fn success(output: Output) -> Value {
    let mut response = json!({"description":"Success"});
    let mime = match output {
        Output::Json(name) => json_content(reference(name)),
        Output::List(key, element) => json_content(object(
            json!({(key):{"type":"array","items":reference(element)}}),
            &[key],
        )),
        Output::Asset => binary(&[
            "font/woff2",
            "image/png",
            "image/webp",
            "image/avif",
            "image/jpeg",
        ]),
        Output::Preview => binary(&["image/png"]),
        Output::Tar => binary(&["application/x-tar"]),
        Output::Audio => binary(&["audio/*", "video/*", "application/octet-stream"]),
        Output::Attachment => binary(&["application/octet-stream"]),
        Output::WebVtt => content("text/vtt", json!({"type":"string"})),
        Output::Poster => binary(&["image/jpeg", "image/png", "image/webp"]),
        Output::Document => json_content(json!({"type":"object","additionalProperties":true})),
        Output::SkinManifest => json_content(reference("SkinManifest")),
        Output::Events => content(
            "text/event-stream",
            json!({"type":"string","description":"SSE frames with decimal revision IDs. Heartbeats have no ID. A stream.reset frame requests a full state refresh."}),
        ),
        Output::Empty => return response,
    };
    response["content"] = mime;
    response
}

fn error_response(status: u16) -> Value {
    let reason = axum::http::StatusCode::from_u16(status)
        .ok()
        .and_then(|code| code.canonical_reason())
        .unwrap_or("Error");
    let mut result =
        json!({"description":reason,"content":json_content(reference("ApiErrorBody"))});
    match status {
        401 => {
            result["headers"] =
                json!({"WWW-Authenticate":{"schema":{"type":"string","const":"Bearer"}}})
        }
        405 => result["headers"] = json!({"Allow":{"schema":{"type":"string"}}}),
        429 => result["headers"] = json!({"Retry-After":{"schema":{"type":"integer","minimum":1}}}),
        _ => (),
    }
    result
}

fn parameter(name: &str, location: &str, required: bool, schema: Value) -> Value {
    json!({"name":name,"in":location,"required":required,"schema":schema})
}

fn parameters(cap: &Capability) -> Vec<Value> {
    let mut params = Vec::new();
    for part in cap.path.split('/') {
        if let Some(name) = part.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            let schema = if name == "id" && cap.module != "skins" {
                json!({"type":"string","format":"uuid"})
            } else {
                json!({"type":"string"})
            };
            params.push(parameter(name, "path", true, schema));
        }
    }
    if matches!(
        cap.id,
        "streamTrack"
            | "headTrack"
            | "streamConnection"
            | "streamAudio"
            | "headAudio"
            | "downloadBook"
            | "headBook"
    ) {
        params.push(parameter("ticket", "query", false, json!({"type":"string","description":"Short-lived credential issued for this file and session, as an alternative to a bearer header."})));
    }
    if matches!(cap.id, "preparePlayback" | "streamAudio" | "headAudio") {
        let direct = cap.id == "preparePlayback";
        let formats = if direct {
            json!(["original", "mp3", "aac", "opus"])
        } else {
            json!(["mp3", "aac", "opus"])
        };
        params.push(parameter(
            "format",
            "query",
            !direct,
            json!({"type":"string","enum":formats}),
        ));
        params.push(parameter(
            "bitrateKbps",
            "query",
            false,
            json!({"type":"integer","minimum":32,"maximum":320,"default":128}),
        ));
        params.push(parameter(
            "startMs",
            "query",
            false,
            json!({"type":"integer","minimum":0,"maximum":604800000,"default":0}),
        ));
    }
    match cap.id {
        "listScanReviewPaths" => {
            params.push(parameter(
                "offset",
                "query",
                false,
                json!({"type":"integer","minimum":0,"maximum":4294967295u64,"default":0}),
            ));
            params.push(parameter(
                "limit",
                "query",
                false,
                json!({"type":"integer","minimum":1,"maximum":100,"default":50}),
            ));
        }
        "issueSubsonicPassword" => params.push(parameter(
            "personal",
            "query",
            false,
            json!({"type":"boolean","default":false}),
        )),
        "browseSpoken" | "browseVideos" => {
            let fields: &[&str] = if cap.id == "browseSpoken" {
                &["collection", "cursor"]
            } else {
                &["series", "cursor"]
            };
            if cap.id == "browseVideos" {
                params.push(parameter(
                    "q",
                    "query",
                    false,
                    json!({"type":"string","maxLength":128,"description":"Literal substring in a film or series title. Applies only to the works list."}),
                ));
                params.push(parameter(
                    "kind",
                    "query",
                    false,
                    json!({"type":"string","enum":["movie","series"],"description":"Restrict the works list to films or series."}),
                ));
                params.push(parameter(
                    "order",
                    "query",
                    false,
                    json!({"type":"string","enum":["title","poster"],"description":"Works default to title order. Poster order puts works with a local poster first and is available only on the works list."}),
                ));
                params.push(parameter(
                    "season",
                    "query",
                    false,
                    json!({"type":"integer","minimum":0,"maximum":65535}),
                ));
            }
            for field in fields {
                params.push(parameter(field, "query", false, json!({"type":"string"})));
            }
            params.push(parameter(
                "limit",
                "query",
                false,
                json!({"type":"integer","minimum":1,"maximum":200}),
            ));
        }
        "getVideoPoster" => params.push(parameter(
            "level",
            "query",
            false,
            json!({"enum":["work","season","episode"]}),
        )),
        "browseConnection" => {
            params.push(parameter(
                "parent",
                "query",
                false,
                json!({"type":"string"}),
            ));
            params.push(parameter(
                "offset",
                "query",
                false,
                json!({"type":"integer","minimum":0}),
            ));
            params.push(parameter(
                "limit",
                "query",
                false,
                json!({"type":"integer","minimum":1,"maximum":200}),
            ));
        }
        "streamConnection" => {
            params.push(parameter("entry", "query", true, json!({"type":"string"})));
            params.push(parameter(
                "Range",
                "header",
                false,
                json!({"type":"string"}),
            ));
        }
        "listStorageFolders" => {
            params.push(parameter("mount", "query", true, json!({"type":"string"})));
            params.push(parameter(
                "folder",
                "query",
                false,
                json!({"type":"string"}),
            ));
        }
        "listBroadcastSources" => params.push(parameter(
            "kind",
            "query",
            false,
            reference("BroadcastKind"),
        )),
        "removeBroadcastSource" | "listBroadcastEntries" => {
            params.push(parameter(
                "version",
                "query",
                true,
                json!({"type":"integer","minimum":1}),
            ));
            if cap.id == "listBroadcastEntries" {
                params.push(parameter(
                    "offset",
                    "query",
                    false,
                    json!({"type":"integer","minimum":0}),
                ));
                params.push(parameter(
                    "limit",
                    "query",
                    false,
                    json!({"type":"integer","minimum":1,"maximum":200}),
                ));
            }
        }
        "listPersonalItems" | "listPlaylists" => {
            params.push(parameter(
                "before",
                "query",
                false,
                json!({"type":"integer","minimum":1}),
            ));
            if cap.id == "listPersonalItems" {
                params.push(parameter("view", "query", true, reference("MediaView")));
            }
        }
        "listPlaylistTracks" => {
            params.push(parameter(
                "version",
                "query",
                true,
                json!({"type":"integer","minimum":1}),
            ));
            params.push(parameter(
                "after",
                "query",
                false,
                json!({"type":"integer","minimum":0}),
            ));
        }
        "deletePlaylist" => params.push(parameter(
            "version",
            "query",
            true,
            json!({"type":"integer","minimum":1}),
        )),
        "searchMusic" | "searchAllMusic" => params.extend([
            parameter(
                "q",
                "query",
                true,
                json!({"type":"string","minLength":1,"maxLength":128}),
            ),
            parameter("kind", "query", false, reference("MusicSearchKind")),
            parameter("cursor", "query", false, json!({"type":"string"})),
            parameter(
                "limit",
                "query",
                false,
                json!({"type":"integer","minimum":1,"maximum":200,"default":30}),
            ),
        ]),
        "browseMusic" => {
            for name in ["artist", "album"] {
                params.push(parameter(
                    name,
                    "query",
                    false,
                    json!({"type":"string","pattern":"^[0-9a-f]{64}$"}),
                ));
            }
            params.push(parameter(
                "cursor",
                "query",
                false,
                json!({"type":"string"}),
            ));
        }
        "listItems" => {
            params.extend([
                parameter(
                    "libraryId",
                    "query",
                    true,
                    json!({"type":"string","format":"uuid"}),
                ),
                parameter(
                    "parentId",
                    "query",
                    false,
                    json!({"type":["string","null"],"format":"uuid"}),
                ),
                parameter("kind", "query", false, reference("ItemKind")),
                parameter("cursor", "query", false, json!({"type":["string","null"]})),
            ]);
        }
        "getHardware" => params.push(parameter(
            "refresh",
            "query",
            false,
            json!({"type":"boolean","default":false}),
        )),
        "streamEvents" => params.push(parameter(
            "Last-Event-ID",
            "header",
            false,
            json!({"type":"string","pattern":"^[0-9]+$"}),
        )),
        "streamTrack" | "downloadBook" => {
            params.push(parameter("Range", "header", false, json!({"type":"string","description":"One HTTP byte range, for example bytes=0-1."})));
            params.push(parameter("If-Range", "header", false, json!({"type":"string","description":"Returns the full representation because this endpoint supplies no entity validator."})));
        }
        _ => (),
    }
    if matches!(
        cap.id,
        "listItems"
            | "listJobs"
            | "browseMusic"
            | "listPersonalItems"
            | "listPlaylists"
            | "listPlaylistTracks"
    ) {
        params.push(parameter("limit","query",false,json!({"type":"integer","minimum":1,"default":50,"description":"Values above 200 are clamped to 200."})));
    }
    params
}

fn operation(endpoint: &Endpoint) -> Value {
    let cap = &endpoint.capability;
    let mut responses = json!({});
    responses[endpoint.status.to_string()] = success(endpoint.output);
    for status in endpoint.errors.iter().copied().chain([405, 500]) {
        responses[status.to_string()] = error_response(status);
    }
    if cap.permission != Permission::Public {
        responses["401"] = error_response(401);
    }
    if cap.permission == Permission::Admin {
        responses["403"] = error_response(403);
    }
    let mut op = json!({
        "operationId":cap.id, "tags":[cap.module], "description":cap.description,
        "x-permission":cap.permission, "x-risk":cap.risk, "x-reversible":cap.reversible,
        "parameters":parameters(cap), "responses":responses,
    });
    if cap.permission != Permission::Public {
        op["security"] = json!([{"bearerAuth":[]}]);
    }
    if cap.id == "getOpdsAccess" {
        op["responses"]["200"]["headers"]["Cache-Control"] =
            json!({"schema":{"type":"string","const":"private, no-store"}});
    }
    if matches!(
        cap.id,
        "streamTrack" | "headTrack" | "streamConnection" | "downloadBook" | "headBook"
    ) {
        op["security"] = json!([{"bearerAuth":[]},{"mediaTicket":[]}]);
        let mut headers = json!({
            "Accept-Ranges":{"schema":{"type":"string","const":"bytes"}},
            "Content-Length":{"schema":{"type":"integer","minimum":0}},
            "Content-Type":{"schema":{"type":"string"}},
            "Cache-Control":{"schema":{"type":"string","const":"private, no-store"}}
        });
        if matches!(cap.id, "downloadBook" | "headBook") {
            headers["Content-Disposition"] = json!({"description":"An attachment with the book's download filename.","schema":{"type":"string"}});
        }
        let responses = &mut op["responses"];
        responses["200"]["headers"] = headers;
        if matches!(cap.id, "streamTrack" | "streamConnection" | "downloadBook") {
            responses["206"] = responses["200"].clone();
            responses["206"]["description"] = json!("Partial content");
            let content_range = json!({"schema":{"type":"string"}});
            responses["206"]["headers"]["Content-Range"] = content_range.clone();
            responses["416"] = json!({"description":"Unsatisfiable byte range","headers":{"Content-Range":content_range}});
        } else if let Some(statuses) = responses.as_object_mut() {
            for response in statuses.values_mut().filter_map(Value::as_object_mut) {
                response.remove("content");
            }
        }
    }
    if matches!(cap.id, "streamAudio" | "headAudio") {
        op["security"] = json!([{"bearerAuth":[]},{"mediaTicket":[]}]);
        let responses = &mut op["responses"];
        responses["200"]["headers"] = json!({"Accept-Ranges":{"schema":{"type":"string","const":"none"}},
            "Cache-Control":{"schema":{"type":"string","const":"private, no-store"}}});
        if cap.id == "streamAudio" {
            responses["200"]["content"] = binary(&["audio/mpeg", "audio/mp4", "audio/ogg"]);
        } else if let Some(statuses) = responses.as_object_mut() {
            for entry in statuses.values_mut().filter_map(Value::as_object_mut) {
                entry.remove("content");
            }
        }
    }
    match endpoint.input {
        Input::None => (),
        Input::Json(name) | Input::LimitedJson(name, _) => {
            op["requestBody"] = json!({"required":true,"content":json_content(reference(name))})
        }
        Input::Tar { required } => {
            op["requestBody"] =
                json!({"required":required,"content":binary(&["application/x-tar"])})
        }
    }
    op
}

#[cfg(test)]
mod tests {
    use super::*;
    use palveni_core::FeatureRegistry;
    use std::collections::BTreeSet;

    #[test]
    fn video_order_is_an_optional_works_query() {
        let params =
            &openapi_document()["paths"]["/api/v1/libraries/{id}/videos"]["get"]["parameters"];
        let order = params
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "order")
            .unwrap();
        assert_eq!(order["in"], "query");
        assert_eq!(order["required"], false);
        assert_eq!(order["schema"]["enum"], json!(["title", "poster"]));
    }

    #[test]
    fn enabled_routes_share_operation_ids_and_role_visibility() {
        let registry = FeatureRegistry::new();
        let mut ids = BTreeSet::new();
        let doc = openapi_document();
        for endpoint in endpoints() {
            let cap = endpoint.capability;
            assert!(ids.insert(cap.id));
            let op = &doc["paths"][cap.path][cap.method.to_ascii_lowercase()];
            assert_eq!(op["operationId"], cap.id);
            assert_eq!(
                op.get("security").is_some(),
                cap.permission != Permission::Public
            );
            registry.register(cap);
        }
        assert_eq!(registry.capabilities(true).len(), ids.len());
        let normal = registry.capabilities(false);
        assert!(normal.iter().any(|c| c.id == "listLibraries"));
        assert!(normal.iter().all(|c| c.permission != Permission::Admin));
        assert!(!normal.iter().any(|c| c.id == "updateRuntimeSettings"));
        assert!(registry.list().iter().any(|name| name == "skins"));
        assert!(!registry.list().iter().any(|name| name == "assistant"));
    }

    #[test]
    fn all_schema_refs_resolve_and_wire_specifics_survive_generation() {
        let doc = openapi_document();
        fn visit(value: &Value, doc: &Value) {
            match value {
                Value::Object(map) => {
                    if let Some(target) = map.get("$ref").and_then(Value::as_str) {
                        assert!(
                            doc.pointer(target.strip_prefix('#').expect("local reference"))
                                .is_some(),
                            "{target}"
                        );
                    }
                    for child in map.values() {
                        visit(child, doc);
                    }
                }
                Value::Array(array) => {
                    for child in array {
                        visit(child, doc);
                    }
                }
                _ => (),
            }
        }
        visit(doc, doc);
        let schemas = &doc["components"]["schemas"];
        assert!(
            schemas["Item"]["required"]
                .as_array()
                .expect("item keys")
                .contains(&json!("parentId"))
        );
        assert!(
            schemas["RuntimeSettings"]["required"]
                .as_array()
                .expect("settings keys")
                .contains(&json!("perfTier"))
        );
        let paths = &doc["paths"];
        let stream = &paths["/api/v1/items/{id}/stream"];
        assert!(stream["get"]["responses"]["206"]["headers"]["Content-Range"].is_object());
        assert!(stream["get"]["responses"]["416"].get("content").is_none());
        assert!(stream["head"]["responses"]["200"].get("content").is_none());
        assert_eq!(
            stream["head"]["security"],
            json!([{"bearerAuth":[]},{"mediaTicket":[]}])
        );
        assert!(
            paths["/api/v1/auth/login"]["post"]["responses"]["429"]["headers"]["Retry-After"]
                .is_object()
        );
        assert!(
            paths["/api/v1/skins/upload"]["post"]["requestBody"]["content"]["application/x-tar"]
                .is_object()
        );
        assert!(
            paths["/api/v1/events"]["get"]["responses"]["200"]["content"]["text/event-stream"]
                .is_object()
        );
    }
}
