//! Built-in Model Context Protocol (MCP) JSON-RPC 2.0 Streamable HTTP Server (`/api/v1/mcp`)
//! (`module: mcp`).

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use palveni_core::assistant::RunState;
use palveni_core::downloads::{OrganizeMode, OrganizePreviewRequest};
use palveni_core::home::DevicePatch;
use palveni_core::ids::LibraryId;
use palveni_core::mcp::{
    MCP_ENDPOINT_PATH, MCP_PROTOCOL_VERSION, McpClientConfigs, McpInfoResponse, McpJsonRpcRequest,
    McpJsonRpcResponse, McpToolDescriptor,
};
use palveni_db::audit::{AuditEntry, actions};
use serde_json::{Value, json};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::assistant::engine::{
    build_home_outcome, build_music_outcome, build_organizer_preview_outcome,
    build_scan_libraries_outcome, is_high_risk_device_name, sanitize_media_tag,
};
use crate::routes::assistant::{list_tasks_internal, persist_mcp_approval_run};
use crate::routes::audit;
use crate::routes::home::{activate_scene_internal, apply_device_patch_internal};
use crate::routes::organizer::build_preview_internal;
use crate::state::AppState;

const JSONRPC_INVALID_REQUEST: i32 = -32600;
const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
const JSONRPC_INVALID_PARAMS: i32 = -32602;
const JSONRPC_FORBIDDEN: i32 = -32003;

fn all_mcp_tools() -> Vec<McpToolDescriptor> {
    vec![
        McpToolDescriptor {
            name: "catalog_overview".into(),
            description: "Read Palveni media libraries, item counts, and recently added works.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "recentLimit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 12 }
                },
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "storage_report".into(),
            description: "Read storage volumes, per-library byte totals, and network mount status.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "search_music".into(),
            description: "Search indexed music tracks by title, artist, or album.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 15 }
                },
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "search_videos".into(),
            description: "Search movies and series by title, cast/crew person, or unwatched status.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "person": { "type": "string" },
                    "unwatchedOnly": { "type": "boolean", "default": false },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 15 }
                },
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "create_music_playlist".into(),
            description: "Create a selectable music playlist preview run in awaiting_approval state for user confirmation.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "query": { "type": "string" }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: true,
        },
        McpToolDescriptor {
            name: "control_home_device".into(),
            description: "Control a smart-home device. Locks, doors, security devices, or batch-off requests automatically pause in awaiting_approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deviceId": { "type": "string" },
                    "on": { "type": "boolean" },
                    "brightness": { "type": "number", "minimum": 0, "maximum": 100 },
                    "target": { "type": "number", "minimum": 16, "maximum": 30 },
                    "volume": { "type": "number", "minimum": 0, "maximum": 100 },
                    "allOff": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "activate_home_scene".into(),
            description: "Activate a smart-home preset scene (movie, home, night, morning; away pauses for confirmation).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "sceneId": { "type": "string" }
                },
                "required": ["sceneId"],
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
        McpToolDescriptor {
            name: "trigger_library_scan".into(),
            description: "Prepare an incremental media library scan run in awaiting_approval state (Admin only).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Optional library name or kind filter" }
                },
                "additionalProperties": false
            }),
            admin_only: true,
            requires_approval: true,
        },
        McpToolDescriptor {
            name: "preview_media_organize".into(),
            description: "Dry-run naming template preview on a media library without touching disk and create an awaiting_approval batch preview (Admin only).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "libraryId": { "type": "string", "format": "uuid" },
                    "mode": { "type": "string", "enum": ["rename", "hardlink"], "default": "rename" },
                    "template": { "type": "string", "default": "{title} ({year})" }
                },
                "required": ["libraryId"],
                "additionalProperties": false
            }),
            admin_only: true,
            requires_approval: true,
        },
        McpToolDescriptor {
            name: "list_pending_tasks".into(),
            description: "List active background jobs and awaiting_approval tasks.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 30 }
                },
                "additionalProperties": false
            }),
            admin_only: false,
            requires_approval: false,
        },
    ]
}

fn visible_tools(is_admin: bool) -> Vec<McpToolDescriptor> {
    all_mcp_tools()
        .into_iter()
        .filter(|t| is_admin || !t.admin_only)
        .collect()
}

fn infer_origin(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .filter(|s| matches!(*s, "http" | "https"))
        .unwrap_or("http");
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(axum::http::header::HOST))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
        .unwrap_or("127.0.0.1:3000");
    format!("{scheme}://{host}")
}

fn build_client_configs(endpoint_url: &str) -> McpClientConfigs {
    let claude = json!({
        "mcpServers": {
            "palveni": {
                "command": "npx",
                "args": [
                    "-y",
                    "mcp-remote",
                    endpoint_url,
                    "--header",
                    "Authorization: Bearer <YOUR_PALVENI_TOKEN>"
                ]
            }
        }
    });
    let cursor = json!({
        "mcpServers": {
            "palveni": {
                "type": "http",
                "url": endpoint_url,
                "headers": {
                    "Authorization": "Bearer <YOUR_PALVENI_TOKEN>"
                }
            }
        }
    });
    let openclaw = json!({
        "mcp": {
            "servers": {
                "palveni": {
                    "transport": "streamable-http",
                    "url": endpoint_url,
                    "headers": {
                        "Authorization": "Bearer <YOUR_PALVENI_TOKEN>"
                    }
                }
            }
        }
    });

    McpClientConfigs {
        claude_desktop: serde_json::to_string_pretty(&claude).unwrap_or_default(),
        cursor: serde_json::to_string_pretty(&cursor).unwrap_or_default(),
        openclaw: serde_json::to_string_pretty(&openclaw).unwrap_or_default(),
    }
}

/// `GET /api/v1/mcp/info`
pub async fn info(
    current: CurrentUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<McpInfoResponse>, ApiError> {
    let origin = infer_origin(&headers);
    let endpoint_url = format!("{origin}{MCP_ENDPOINT_PATH}");
    let runtime = state.settings.read().await;
    let server_name = runtime
        .server_name
        .clone()
        .unwrap_or_else(|| state.server.name.clone());
    drop(runtime);

    Ok(Json(McpInfoResponse {
        protocol_version: MCP_PROTOCOL_VERSION.into(),
        endpoint_path: MCP_ENDPOINT_PATH.into(),
        server_name,
        server_version: state.server.version.clone(),
        tools: visible_tools(current.is_admin()),
        client_configs: build_client_configs(&endpoint_url),
    }))
}

/// `POST /api/v1/mcp`
pub async fn handle_rpc(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<McpJsonRpcRequest>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(req) = body?;
    if req.jsonrpc != "2.0" {
        return Ok(Json(McpJsonRpcResponse::err(
            req.id.unwrap_or(Value::Null),
            JSONRPC_INVALID_REQUEST,
            "jsonrpc version must be \"2.0\"",
            None,
        ))
        .into_response());
    }

    // Streamable HTTP notifications (no `id`) return 202 Accepted with empty body.
    let Some(id) = req.id else {
        return Ok(StatusCode::ACCEPTED.into_response());
    };

    let params = req.params.unwrap_or_else(|| json!({}));
    let resp = dispatch_method(&state, &current, id, &req.method, &params).await?;
    Ok(Json(resp).into_response())
}

async fn dispatch_method(
    state: &AppState,
    current: &CurrentUser,
    id: Value,
    method: &str,
    params: &Value,
) -> Result<McpJsonRpcResponse, ApiError> {
    match method {
        "initialize" => {
            let runtime = state.settings.read().await;
            let server_name = runtime
                .server_name
                .clone()
                .unwrap_or_else(|| state.server.name.clone());
            drop(runtime);
            Ok(McpJsonRpcResponse::ok(
                id,
                json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": server_name,
                        "version": state.server.version
                    }
                }),
            ))
        }
        "notifications/initialized" | "ping" => Ok(McpJsonRpcResponse::ok(id, json!({}))),
        "tools/list" => {
            let tools: Vec<Value> = visible_tools(current.is_admin())
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                        "annotations": {
                            "readOnlyHint": !t.requires_approval && t.name != "control_home_device" && t.name != "activate_home_scene",
                            "destructiveHint": t.requires_approval
                        }
                    })
                })
                .collect();
            Ok(McpJsonRpcResponse::ok(id, json!({ "tools": tools })))
        }
        "tools/call" => {
            let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Missing required parameter `name`",
                    None,
                ));
            };
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            execute_mcp_tool(state, current, id, tool_name, &args).await
        }
        "resources/list" => Ok(McpJsonRpcResponse::ok(
            id,
            json!({
                "resources": [
                    {
                        "uri": "palveni://catalog/overview",
                        "name": "Catalog Overview",
                        "description": "Media libraries, item counts, and recent catalog additions",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "palveni://system/storage",
                        "name": "Storage Report",
                        "description": "Disk volumes, per-library bytes, and mount reachability",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "palveni://home/snapshot",
                        "name": "Smart Home Snapshot",
                        "description": "Rooms, controllable devices, scenes, and linkage rules",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "palveni://tasks/active",
                        "name": "Unified Tasks",
                        "description": "Pending approvals and running background tasks",
                        "mimeType": "application/json"
                    }
                ]
            }),
        )),
        "resources/read" => {
            let Some(uri) = params.get("uri").and_then(Value::as_str) else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Missing required parameter `uri`",
                    None,
                ));
            };
            let payload_val = match uri {
                "palveni://catalog/overview" => {
                    let libs = state.db.libraries().list().await?;
                    let cat = state.db.overview().catalog(12).await?;
                    json!({ "libraries": libs, "recent": cat.recent })
                }
                "palveni://system/storage" => {
                    let report = crate::routes::overview::storage_report(
                        current.clone(),
                        State(state.clone()),
                    )
                    .await?
                    .0;
                    json!(report)
                }
                "palveni://home/snapshot" => {
                    let snap = state.db.home().snapshot().await?;
                    json!(snap)
                }
                "palveni://tasks/active" => {
                    let tasks = list_tasks_internal(state, current, 30).await?;
                    json!({ "tasks": tasks })
                }
                _ => {
                    return Ok(McpJsonRpcResponse::err(
                        id,
                        JSONRPC_INVALID_PARAMS,
                        format!("Unknown resource URI: {uri}"),
                        None,
                    ));
                }
            };
            Ok(McpJsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&payload_val).unwrap_or_default()
                    }]
                }),
            ))
        }
        "prompts/list" => Ok(McpJsonRpcResponse::ok(
            id,
            json!({
                "prompts": [
                    {
                        "name": "curate_playlist",
                        "description": "Search tracks in Palveni and prepare an approval-gated music playlist",
                        "arguments": [
                            { "name": "mood", "description": "Genre, artist, or mood keyword", "required": true }
                        ]
                    },
                    {
                        "name": "evening_cinema",
                        "description": "Dim living-room lights and browse unwatched films",
                        "arguments": [
                            { "name": "room", "description": "Target room name", "required": false }
                        ]
                    },
                    {
                        "name": "organize_library",
                        "description": "Dry-run preview standardized naming for a media library",
                        "arguments": [
                            { "name": "template", "description": "Naming template", "required": false }
                        ]
                    }
                ]
            }),
        )),
        "prompts/get" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Missing required parameter `name`",
                    None,
                ));
            };
            let args = params.get("arguments");
            let (desc, prompt_text) = match name {
                "curate_playlist" => {
                    let mood = args
                        .and_then(|a| a.get("mood"))
                        .and_then(Value::as_str)
                        .unwrap_or("Jazz");
                    (
                        "Curate a Palveni playlist",
                        format!(
                            "Use `search_music` to find tracks matching \"{mood}\", then call `create_music_playlist` to prepare a confirmation preview."
                        ),
                    )
                }
                "evening_cinema" => {
                    let room = args
                        .and_then(|a| a.get("room"))
                        .and_then(Value::as_str)
                        .unwrap_or("客厅");
                    (
                        "Evening cinema mode",
                        format!(
                            "Activate the `movie` scene for {room} with `activate_home_scene`, then list unwatched films using `search_videos`."
                        ),
                    )
                }
                "organize_library" => {
                    let tpl = args
                        .and_then(|a| a.get("template"))
                        .and_then(Value::as_str)
                        .unwrap_or("{title} ({year})");
                    (
                        "Dry-run library organization",
                        format!(
                            "Inspect libraries with `catalog_overview`, then run `preview_media_organize` using template `{tpl}`."
                        ),
                    )
                }
                _ => {
                    return Ok(McpJsonRpcResponse::err(
                        id,
                        JSONRPC_INVALID_PARAMS,
                        format!("Unknown prompt: {name}"),
                        None,
                    ));
                }
            };
            Ok(McpJsonRpcResponse::ok(
                id,
                json!({
                    "description": desc,
                    "messages": [{
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": prompt_text
                        }
                    }]
                }),
            ))
        }
        _ => Ok(McpJsonRpcResponse::err(
            id,
            JSONRPC_METHOD_NOT_FOUND,
            format!("Method not found: {method}"),
            None,
        )),
    }
}

fn tool_success(
    id: Value,
    summary_text: impl Into<String>,
    structured: Value,
) -> McpJsonRpcResponse {
    McpJsonRpcResponse::ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": summary_text.into()
            }],
            "structuredContent": structured,
            "isError": false
        }),
    )
}

async fn execute_mcp_tool(
    state: &AppState,
    current: &CurrentUser,
    id: Value,
    tool_name: &str,
    args: &Value,
) -> Result<McpJsonRpcResponse, ApiError> {
    let Some(descriptor) = all_mcp_tools().into_iter().find(|t| t.name == tool_name) else {
        return Ok(McpJsonRpcResponse::err(
            id,
            JSONRPC_INVALID_PARAMS,
            format!("Unknown MCP tool: {tool_name}"),
            None,
        ));
    };
    if descriptor.admin_only && !current.is_admin() {
        return Ok(McpJsonRpcResponse::err(
            id,
            JSONRPC_FORBIDDEN,
            format!("Tool `{tool_name}` requires administrator permission"),
            None,
        ));
    }

    let resp = match tool_name {
        "catalog_overview" => {
            let recent_limit = args
                .get("recentLimit")
                .and_then(Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(12)
                .clamp(1, 50);
            let libs = state.db.libraries().list().await?;
            let cat = state.db.overview().catalog(recent_limit).await?;
            let safe_recent: Vec<Value> = cat
                .recent
                .iter()
                .map(|w| {
                    json!({
                        "id": w.id,
                        "title": sanitize_media_tag(&w.title),
                        "kind": w.kind,
                        "meta": w.meta.as_deref().map(sanitize_media_tag),
                        "addedAt": w.added_at,
                    })
                })
                .collect();
            let data = json!({
                "libraryCount": libs.len(),
                "libraries": libs,
                "recent": safe_recent,
            });
            tool_success(
                id,
                format!(
                    "共有 {} 个媒体库，最近入库 {} 项作品。",
                    libs.len(),
                    cat.recent.len()
                ),
                data,
            )
        }
        "storage_report" => {
            let report =
                crate::routes::overview::storage_report(current.clone(), State(state.clone()))
                    .await?
                    .0;
            let data = json!(report);
            tool_success(
                id,
                format!(
                    "存储卷 {} 个，媒体库空间统计 {} 项，网络挂载 {} 个。",
                    report.volumes.len(),
                    report.by_library.len(),
                    report.mounts.len()
                ),
                data,
            )
        }
        "search_music" => {
            let q = args.get("query").and_then(Value::as_str);
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(15)
                .clamp(1, 50);
            let tracks = state.db.overview().search_music_tracks(q, limit).await?;
            let safe: Vec<Value> = tracks
                .iter()
                .map(|t| {
                    json!({
                        "itemId": t.item_id,
                        "title": sanitize_media_tag(&t.title),
                        "artist": t.artist.as_deref().map(sanitize_media_tag),
                        "album": t.album.as_deref().map(sanitize_media_tag),
                    })
                })
                .collect();
            tool_success(
                id,
                format!("找到 {} 首匹配曲目。", safe.len()),
                json!({ "tracks": safe }),
            )
        }
        "search_videos" => {
            let q = args.get("query").and_then(Value::as_str);
            let p = args.get("person").and_then(Value::as_str);
            let unwatched = args
                .get("unwatchedOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(15)
                .clamp(1, 50);
            let works = state
                .db
                .overview()
                .search_video_works(current.user.id, q, p, unwatched, limit)
                .await?;
            let safe: Vec<Value> = works
                .iter()
                .map(|w| {
                    json!({
                        "itemId": w.item_id,
                        "title": sanitize_media_tag(&w.title),
                        "year": w.year,
                        "watched": w.watched,
                        "isSeries": w.is_series,
                    })
                })
                .collect();
            tool_success(
                id,
                format!("找到 {} 部匹配影视作品。", safe.len()),
                json!({ "works": safe }),
            )
        }
        "create_music_playlist" => {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("MCP 精选歌单");
            let q = args.get("query").and_then(Value::as_str).unwrap_or("");
            let synthetic = if q.trim().is_empty() {
                format!("生成歌单「{name}」")
            } else {
                format!("搜{}并生成歌单「{name}」", q.trim())
            };
            let outcome = build_music_outcome(state, &synthetic, &synthetic.to_lowercase()).await?;
            if outcome.state == RunState::AwaitingApproval {
                let (run, task) =
                    persist_mcp_approval_run(state, current, &synthetic, outcome).await?;
                tool_success(
                    id,
                    format!(
                        "歌单「{name}」已生成确认预览（任务 ID: {}），需用户在工作台确认后写入。",
                        task.id
                    ),
                    json!({
                        "status": "awaiting_approval",
                        "runId": run.id,
                        "taskId": task.id,
                        "title": run.title,
                    }),
                )
            } else {
                tool_success(id, outcome.reply, json!({ "status": "no_matching_tracks" }))
            }
        }
        "control_home_device" => {
            let all_off = args.get("allOff").and_then(Value::as_bool).unwrap_or(false);
            if all_off {
                let outcome =
                    build_home_outcome(state, "关闭全屋所有设备", "关闭全屋所有设备").await?;
                let (run, task) =
                    persist_mcp_approval_run(state, current, "关闭全屋所有设备", outcome).await?;
                tool_success(
                    id,
                    format!(
                        "全屋批量关闭已暂停在待确认状态（任务 ID: {}），请在工作台确认。",
                        task.id
                    ),
                    json!({
                        "status": "awaiting_approval",
                        "runId": run.id,
                        "taskId": task.id,
                    }),
                )
            } else {
                let Some(device_id) = args.get("deviceId").and_then(Value::as_str) else {
                    return Ok(McpJsonRpcResponse::err(
                        id,
                        JSONRPC_INVALID_PARAMS,
                        "Provide `deviceId` or `allOff: true`",
                        None,
                    ));
                };
                let snap = state.db.home().snapshot().await?;
                let target_dev = snap.devices.iter().find(|d| d.id == device_id);
                let high_risk = is_high_risk_device_name(device_id)
                    || target_dev.is_some_and(|d| is_high_risk_device_name(&d.name));
                if high_risk {
                    let label = target_dev.map_or(device_id, |d| d.name.as_str());
                    let prompt = format!("控制门锁/安防设备 {label}");
                    let outcome =
                        build_home_outcome(state, &prompt, &prompt.to_lowercase()).await?;
                    let (run, task) =
                        persist_mcp_approval_run(state, current, &prompt, outcome).await?;
                    tool_success(
                        id,
                        format!(
                            "高风险门锁/安防操作「{label}」已进入待确认队列（任务 ID: {}），未执行物理动作。",
                            task.id
                        ),
                        json!({
                            "status": "awaiting_approval",
                            "runId": run.id,
                            "taskId": task.id,
                            "risk": "high",
                        }),
                    )
                } else {
                    let patch = DevicePatch {
                        on: args.get("on").and_then(Value::as_bool),
                        brightness: args.get("brightness").and_then(Value::as_f64),
                        target: args.get("target").and_then(Value::as_f64),
                        volume: args.get("volume").and_then(Value::as_f64),
                        status: None,
                    };
                    if patch.is_empty() {
                        return Ok(McpJsonRpcResponse::err(
                            id,
                            JSONRPC_INVALID_PARAMS,
                            "Device patch must specify at least one field (`on`, `brightness`, `target`, or `volume`)",
                            None,
                        ));
                    }
                    let Some(updated) =
                        apply_device_patch_internal(state, device_id, &patch).await?
                    else {
                        return Ok(McpJsonRpcResponse::err(
                            id,
                            JSONRPC_INVALID_PARAMS,
                            format!("Device not found: {device_id}"),
                            None,
                        ));
                    };
                    tool_success(
                        id,
                        format!("已更新设备「{}」状态。", updated.name),
                        json!({ "status": "succeeded", "device": updated }),
                    )
                }
            }
        }
        "activate_home_scene" => {
            let Some(scene_id) = args.get("sceneId").and_then(Value::as_str) else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Missing `sceneId`",
                    None,
                ));
            };
            if scene_id == "away" || is_high_risk_device_name(scene_id) {
                let outcome = build_home_outcome(state, "离家模式", "离家模式").await?;
                let (run, task) =
                    persist_mcp_approval_run(state, current, "开启离家模式", outcome).await?;
                tool_success(
                    id,
                    format!(
                        "离家模式涉及全屋设备关闭，已生成待确认请求（任务 ID: {}）。",
                        task.id
                    ),
                    json!({
                        "status": "awaiting_approval",
                        "runId": run.id,
                        "taskId": task.id,
                    }),
                )
            } else {
                let Some(snap) = activate_scene_internal(state, scene_id).await? else {
                    return Ok(McpJsonRpcResponse::err(
                        id,
                        JSONRPC_INVALID_PARAMS,
                        format!("Unknown scene: {scene_id}"),
                        None,
                    ));
                };
                tool_success(
                    id,
                    format!("已激活家居场景「{scene_id}」。"),
                    json!({ "status": "succeeded", "sceneId": scene_id, "scenes": snap.scenes }),
                )
            }
        }
        "trigger_library_scan" => {
            let filt = args
                .get("filter")
                .and_then(Value::as_str)
                .unwrap_or("扫描全部媒体库");
            let outcome = build_scan_libraries_outcome(state, &filt.to_lowercase()).await?;
            if outcome.state == RunState::AwaitingApproval {
                let (run, task) = persist_mcp_approval_run(state, current, filt, outcome).await?;
                tool_success(
                    id,
                    format!(
                        "媒体库扫描计划已进入待确认队列（任务 ID: {}），确认后启动后台扫描。",
                        task.id
                    ),
                    json!({
                        "status": "awaiting_approval",
                        "runId": run.id,
                        "taskId": task.id,
                    }),
                )
            } else {
                tool_success(id, outcome.reply, json!({ "status": "empty" }))
            }
        }
        "preview_media_organize" => {
            let Some(raw_lib) = args.get("libraryId").and_then(Value::as_str) else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Missing `libraryId`",
                    None,
                ));
            };
            let Ok(library_id) = raw_lib.parse::<LibraryId>() else {
                return Ok(McpJsonRpcResponse::err(
                    id,
                    JSONRPC_INVALID_PARAMS,
                    "Invalid `libraryId` UUID",
                    None,
                ));
            };
            let mode = match args.get("mode").and_then(Value::as_str) {
                Some("hardlink") => OrganizeMode::Hardlink,
                _ => OrganizeMode::Rename,
            };
            let template = args
                .get("template")
                .and_then(Value::as_str)
                .unwrap_or("{title} ({year})");

            let preview = build_preview_internal(
                state,
                &OrganizePreviewRequest {
                    library_id,
                    mode,
                    template: template.to_owned(),
                    target_subdir: None,
                    limit: Some(100),
                },
            )
            .await?;

            let mode_str = mode.as_str();
            let outcome = build_organizer_preview_outcome(
                state,
                current,
                mode_str,
                mode_str,
                Some(library_id),
                Some(template),
            )
            .await?;

            if outcome.state == RunState::AwaitingApproval {
                let prompt = format!(
                    "预览整理媒体库 {} ({})",
                    preview.library_name, preview.template
                );
                let (run, task) =
                    persist_mcp_approval_run(state, current, &prompt, outcome).await?;
                tool_success(
                    id,
                    format!(
                        "已完成「{}」干跑预览（{} 项可整理，{} 项冲突），并创建待确认任务 {}。",
                        preview.library_name, preview.ready_count, preview.collision_count, task.id
                    ),
                    json!({
                        "status": "awaiting_approval",
                        "runId": run.id,
                        "taskId": task.id,
                        "readyCount": preview.ready_count,
                        "collisionCount": preview.collision_count,
                        "readOnlyMount": preview.read_only_mount,
                        "candidates": preview.candidates,
                    }),
                )
            } else {
                tool_success(
                    id,
                    outcome.reply,
                    json!({
                        "status": "dry_run_only",
                        "readyCount": preview.ready_count,
                        "collisionCount": preview.collision_count,
                        "readOnlyMount": preview.read_only_mount,
                        "candidates": preview.candidates,
                    }),
                )
            }
        }
        "list_pending_tasks" => {
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(30)
                .clamp(1, 100);
            let tasks = list_tasks_internal(state, current, limit).await?;
            let awaiting = tasks
                .iter()
                .filter(|t| t.state == palveni_core::assistant::TaskState::AwaitingApproval)
                .count();
            tool_success(
                id,
                format!(
                    "共 {} 项任务，其中 {} 项等待用户确认。",
                    tasks.len(),
                    awaiting
                ),
                json!({ "awaitingCount": awaiting, "tasks": tasks }),
            )
        }
        _ => McpJsonRpcResponse::err(
            id,
            JSONRPC_INVALID_PARAMS,
            format!("Unsupported tool: {tool_name}"),
            None,
        ),
    };

    audit(
        state,
        AuditEntry {
            action: actions::MCP_TOOL_CALLED,
            actor: Some(current.user.id),
            target: Some(tool_name.to_owned()),
            detail: Some(json!({ "admin": current.is_admin() })),
        },
    )
    .await;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use axum::extract::Path;
    use palveni_core::assistant::{ApproveRequest, TaskState};
    use palveni_core::downloads::{OrganizeApplyRequest, OrganizeMode};
    use palveni_core::music::{AudioProperties, TrackTags};
    use palveni_core::{FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId};
    use palveni_db::items::ItemUpsert;
    use palveni_db::libraries::NewLibrary;
    use palveni_db::{Db, DbOptions};
    use palveni_jobs::JobService;
    use palveni_media::{HardwareService, ProbeOptions};
    use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
    use tokio_util::sync::CancellationToken;

    use crate::auth::AdminUser;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mcp_jsonrpc_roundtrip_permission_gate_and_organizer_apply_undo() {
        let dir = std::env::temp_dir().join(format!(
            "palveni-mcp-organizer-test-{}",
            ServerId::generate()
        ));
        let music_dir = dir.join("music_lib");
        tokio::fs::create_dir_all(&music_dir).await.unwrap();

        let raw_file = music_dir.join("track01.flac");
        tokio::fs::write(&raw_file, b"fake-flac-audio-bytes")
            .await
            .unwrap();

        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let admin_user = db
            .users()
            .create("admin", "pw-hash", Role::Admin)
            .await
            .unwrap();
        let member_user = db
            .users()
            .create("member", "pw-hash", Role::User)
            .await
            .unwrap();
        let admin_sess = db
            .sessions()
            .create(admin_user.id, "ClaudeDesktop", None)
            .await
            .unwrap();
        let member_sess = db
            .sessions()
            .create(member_user.id, "CursorClient", None)
            .await
            .unwrap();

        let lib = db
            .libraries()
            .create(NewLibrary {
                name: "精选无损".into(),
                kind: LibraryKind::Music,
                paths: vec![music_dir.to_string_lossy().into_owned()],
            })
            .await
            .unwrap();

        let track_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                lib.id,
                &[ItemUpsert {
                    id: track_id,
                    parent_id: None,
                    kind: ItemKind::Track,
                    title: "七里香".into(),
                    sort_title: "qilixiang".into(),
                    search_text: "七里香 周杰伦".into(),
                    path: raw_file.to_string_lossy().into_owned(),
                    size_bytes: Some(21),
                    modified_at: Some(1_700_000_000_000),
                    music: Some(TrackTags {
                        title: Some("七里香".into()),
                        artist: Some("周杰伦".into()),
                        album_artist: Some("周杰伦".into()),
                        album: Some("七里香".into()),
                        year: Some(2004),
                        duration_ms: Some(299_000),
                        audio: AudioProperties::default(),
                        ..Default::default()
                    }),
                }],
                &[],
            )
            .await
            .unwrap();

        let bus = db.event_bus(32).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                dir.join("search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                dir.join("cache").join("video"),
                None,
                None,
            )),
            identity: Arc::new(palveni_link::NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                dir.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 2, false),
            skins: Arc::new(SkinStore::open(&dir.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "PalveniTest".into(),
                version: "0.1.0".into(),
            }),
            shutdown: CancellationToken::new(),
        };

        let admin_auth = CurrentUser {
            user: admin_user,
            session_id: admin_sess.session_id,
            device_id: Some(admin_sess.device_id),
        };
        let member_auth = CurrentUser {
            user: member_user,
            session_id: member_sess.session_id,
            device_id: Some(member_sess.device_id),
        };

        // 1. GET /mcp/info
        let info_resp = info(admin_auth.clone(), HeaderMap::new(), State(state.clone()))
            .await
            .unwrap()
            .0;
        assert_eq!(info_resp.protocol_version, "2025-03-26");
        assert_eq!(info_resp.endpoint_path, "/api/v1/mcp");
        assert_eq!(info_resp.tools.len(), 10);
        assert!(info_resp.client_configs.cursor.contains("/api/v1/mcp"));

        // Non-admin only sees non-admin tools (8 of 10)
        let member_info = info(member_auth.clone(), HeaderMap::new(), State(state.clone()))
            .await
            .unwrap()
            .0;
        assert_eq!(member_info.tools.len(), 8);

        // 2. MCP JSON-RPC `initialize` + `notifications/initialized` + `tools/list`
        let init = dispatch_method(
            &state,
            &admin_auth,
            json!(1),
            "initialize",
            &json!({ "protocolVersion": "2025-03-26" }),
        )
        .await
        .unwrap();
        assert_eq!(init.result.unwrap()["protocolVersion"], "2025-03-26");

        let notif_http = handle_rpc(
            admin_auth.clone(),
            State(state.clone()),
            Ok(ApiJson(McpJsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: None,
                method: "notifications/initialized".into(),
                params: None,
            })),
        )
        .await
        .unwrap();
        assert_eq!(notif_http.status(), StatusCode::ACCEPTED);

        // Non-admin calling `trigger_library_scan` is rejected with JSONRPC_FORBIDDEN
        let denied = dispatch_method(
            &state,
            &member_auth,
            json!(2),
            "tools/call",
            &json!({ "name": "trigger_library_scan", "arguments": {} }),
        )
        .await
        .unwrap();
        assert_eq!(denied.error.unwrap().code, JSONRPC_FORBIDDEN);

        // 3. MCP `create_music_playlist` creates an `awaiting_approval` run in `db.assistant()`!
        let pl_call = dispatch_method(
            &state,
            &member_auth,
            json!(3),
            "tools/call",
            &json!({
                "name": "create_music_playlist",
                "arguments": { "name": "周杰伦精选", "query": "周杰伦" }
            }),
        )
        .await
        .unwrap();
        let pl_struct = &pl_call.result.unwrap()["structuredContent"];
        assert_eq!(pl_struct["status"], "awaiting_approval");
        let task_id = pl_struct["taskId"].as_str().unwrap().to_owned();

        // Verify `list_pending_tasks` sees the awaiting task, then approve it via Tasks API!
        let pending_tasks = list_tasks_internal(&state, &member_auth, 20).await.unwrap();
        assert!(
            pending_tasks
                .iter()
                .any(|t| t.id == task_id && t.state == TaskState::AwaitingApproval)
        );
        let approved_task = crate::routes::assistant::approve_task(
            member_auth.clone(),
            State(state.clone()),
            Ok(Path(task_id)),
            Ok(ApiJson(ApproveRequest {
                selected_row_ids: vec!["pl-0".into()],
            })),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(approved_task.state, TaskState::Succeeded);

        // 4. MCP `control_home_device` on a door lock pauses in `awaiting_approval`!
        let lock_call = dispatch_method(
            &state,
            &admin_auth,
            json!(4),
            "tools/call",
            &json!({
                "name": "control_home_device",
                "arguments": { "deviceId": "lock.front_door", "on": true }
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            lock_call.result.unwrap()["structuredContent"]["status"],
            "awaiting_approval"
        );

        // 5. Organizer dry-run preview -> apply rename in tempdir -> 1-click undo restores original file & DB path!
        let preview = crate::routes::organizer::preview(
            AdminUser(admin_auth.clone()),
            State(state.clone()),
            Ok(ApiJson(OrganizePreviewRequest {
                library_id: lib.id,
                mode: OrganizeMode::Rename,
                template: "{artist} - {title} ({year})".into(),
                target_subdir: None,
                limit: Some(50),
            })),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(preview.ready_count, 1);
        assert_eq!(
            preview.candidates[0].relative_after,
            "周杰伦 - 七里香 (2004).flac"
        );
        assert!(raw_file.exists(), "dry-run must not touch disk");

        // Reject traversal (`..`)
        assert!(
            crate::routes::organizer::apply(
                AdminUser(admin_auth.clone()),
                State(state.clone()),
                Ok(ApiJson(OrganizeApplyRequest {
                    library_id: lib.id,
                    mode: OrganizeMode::Rename,
                    template: "../escape/{title}".into(),
                    target_subdir: None,
                    selected_item_ids: vec![track_id],
                })),
            )
            .await
            .is_err()
        );

        let applied_batch = crate::routes::organizer::apply(
            AdminUser(admin_auth.clone()),
            State(state.clone()),
            Ok(ApiJson(OrganizeApplyRequest {
                library_id: lib.id,
                mode: OrganizeMode::Rename,
                template: "{artist} - {title} ({year})".into(),
                target_subdir: None,
                selected_item_ids: vec![track_id],
            })),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(applied_batch.operation_count, 1);
        let organized_file = music_dir.join("周杰伦 - 七里香 (2004).flac");
        assert!(!raw_file.exists());
        assert!(organized_file.exists());
        assert_eq!(
            db.items()
                .get(track_id)
                .await
                .unwrap()
                .unwrap()
                .path
                .as_deref(),
            Some(organized_file.to_string_lossy().as_ref())
        );

        // 1-Click Undo reverses both filesystem rename and DB item path + logs organizer.undone!
        let undone_batch = crate::routes::organizer::undo_batch(
            AdminUser(admin_auth.clone()),
            State(state.clone()),
            Ok(Path(applied_batch.id.clone())),
        )
        .await
        .unwrap()
        .0;
        assert!(undone_batch.undone_at.is_some());
        assert!(raw_file.exists());
        assert!(!organized_file.exists());
        assert_eq!(
            db.items()
                .get(track_id)
                .await
                .unwrap()
                .unwrap()
                .path
                .as_deref(),
            Some(raw_file.to_string_lossy().as_ref())
        );

        let audits = db.audit().recent(20).await.unwrap();
        assert!(
            audits
                .iter()
                .any(|a| a.action == actions::ORGANIZER_APPLIED)
        );
        assert!(audits.iter().any(|a| a.action == actions::ORGANIZER_UNDONE));
        assert!(audits.iter().any(|a| a.action == actions::MCP_TOOL_CALLED));

        let _ = db.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
