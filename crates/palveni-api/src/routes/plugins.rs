//! Official Recipe & Widget Plugin Store and installation lifecycle
//! (`/api/v1/plugins*`).

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::plugins::{
    InstallPluginRequest, PluginCatalogEntry, PluginCatalogResponse, PluginList, PluginRiskLevel,
    PluginRuntimeKind, PluginSummary, UpdatePluginRequest, compute_risk_level,
};
use palveni_db::DbError;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::plugins_photo_ai::{PluginInstallationRecord, PluginInstallationUpsert};
use palveni_plugin_host::{RuntimeKind as HostRuntimeKind, skin_store::SkinStore};
use serde_json::{Map, Value, json};

use crate::auth::AdminUser;
use crate::json::ApiJson;
use crate::routes::audit;
use crate::{ApiError, AppState};

type IdPath = Result<Path<String>, PathRejection>;

#[derive(Clone)]
struct CatalogBlueprint {
    id: &'static str,
    name: &'static str,
    version: &'static str,
    previous_version: Option<&'static str>,
    runtime: PluginRuntimeKind,
    provides: Vec<&'static str>,
    permissions: Vec<&'static str>,
    source: &'static str,
    description_zh: &'static str,
    description_en: &'static str,
    settings_schema: Value,
    default_settings: Value,
}

impl CatalogBlueprint {
    fn description_map(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("zh-CN".to_owned(), self.description_zh.to_owned());
        map.insert("en".to_owned(), self.description_en.to_owned());
        map
    }

    fn permission_strings(&self) -> Vec<String> {
        self.permissions.iter().map(|p| (*p).to_owned()).collect()
    }

    fn provide_strings(&self) -> Vec<String> {
        self.provides.iter().map(|p| (*p).to_owned()).collect()
    }
}

fn official_catalog_blueprints() -> Vec<CatalogBlueprint> {
    vec![
        CatalogBlueprint {
            id: "palveni.recipe.tmdb",
            name: "TMDB Metadata Recipe",
            version: "1.2.0",
            previous_version: Some("1.1.0"),
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["metadata"],
            permissions: vec!["network:api.themoviedb.org", "library:write_metadata"],
            source: "official",
            description_zh: "声明式 TMDB 影视元数据刮削配方，自动补齐电影与剧集简介、演职表与海报。",
            description_en: "Declarative TMDB metadata recipe for movie and series synopses, cast and artwork.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "title": "Metadata Language",
                        "enum": ["zh-CN", "en-US", "ja-JP"],
                        "default": "zh-CN"
                    },
                    "includeAdult": {
                        "type": "boolean",
                        "title": "Include Adult Titles",
                        "default": false
                    },
                    "autoDownloadPoster": {
                        "type": "boolean",
                        "title": "Auto Download Poster",
                        "default": true
                    }
                }
            }),
            default_settings: json!({
                "language": "zh-CN",
                "includeAdult": false,
                "autoDownloadPoster": true
            }),
        },
        CatalogBlueprint {
            id: "palveni.recipe.bangumi",
            name: "Bangumi Anime & Book Recipe",
            version: "1.1.0",
            previous_version: Some("1.0.0"),
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["metadata"],
            permissions: vec!["network:api.bgm.tv", "library:write_metadata"],
            source: "official",
            description_zh: "面向番剧、漫画与轻小说的 Bangumi 刮削配方，支持中文译名优先与角色资料补全。",
            description_en: "Bangumi metadata recipe tailored for anime, manga and books with Chinese title preference.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "preferChineseTitle": {
                        "type": "boolean",
                        "title": "Prefer Chinese Title",
                        "default": true
                    },
                    "fetchCharacters": {
                        "type": "boolean",
                        "title": "Fetch Characters",
                        "default": true
                    }
                }
            }),
            default_settings: json!({
                "preferChineseTitle": true,
                "fetchCharacters": true
            }),
        },
        CatalogBlueprint {
            id: "palveni.recipe.lrclib",
            name: "LRCLIB Synced Lyrics Recipe",
            version: "1.0.2",
            previous_version: Some("1.0.0"),
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["lyrics"],
            permissions: vec!["network:lrclib.net"],
            source: "official",
            description_zh: "从 LRCLIB 拉取精准逐行同步 LRC 歌词，并在无时间轴歌词时平滑回退纯文本歌词。",
            description_en: "Fetches time-synced LRC lyrics from LRCLIB with optional plain-text fallback.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "preferSynced": {
                        "type": "boolean",
                        "title": "Prefer Synced LRC",
                        "default": true
                    },
                    "fallbackPlain": {
                        "type": "boolean",
                        "title": "Fallback to Plain Lyrics",
                        "default": true
                    }
                }
            }),
            default_settings: json!({
                "preferSynced": true,
                "fallbackPlain": true
            }),
        },
        CatalogBlueprint {
            id: "palveni.recipe.opensubtitles",
            name: "OpenSubtitles Subtitle Recipe",
            version: "1.0.1",
            previous_version: Some("1.0.0"),
            runtime: PluginRuntimeKind::Recipe,
            provides: vec!["subtitles"],
            permissions: vec!["network:api.opensubtitles.com", "library:write_subtitles"],
            source: "official",
            description_zh: "按文件哈希与片名自动匹配多语言外挂字幕，并保存至对应视频旁路目录。",
            description_en: "Matches multilingual subtitles via OpenSubtitles and saves sidecar tracks beside videos.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "preferredLanguages": {
                        "type": "string",
                        "title": "Preferred Languages",
                        "minLength": 2,
                        "maxLength": 64,
                        "default": "zh-CN,en"
                    },
                    "hearingImpaired": {
                        "type": "boolean",
                        "title": "Include Hearing Impaired (SDH)",
                        "default": false
                    }
                }
            }),
            default_settings: json!({
                "preferredLanguages": "zh-CN,en",
                "hearingImpaired": false
            }),
        },
        CatalogBlueprint {
            id: "palveni.widget.clock_weather",
            name: "Home Overview Clock & Media Summary Widget",
            version: "1.0.0",
            previous_version: None,
            runtime: PluginRuntimeKind::Widget,
            provides: vec!["widget"],
            permissions: vec![],
            source: "official",
            description_zh: "零权限概览小组件，提供本地时钟与家庭媒体库概览状态徽标。",
            description_en: "Zero-permission overview widget showing a local clock and media storage summary badge.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "use24Hour": {
                        "type": "boolean",
                        "title": "24-Hour Clock",
                        "default": true
                    },
                    "showStorageBadge": {
                        "type": "boolean",
                        "title": "Show Storage Badge",
                        "default": true
                    }
                }
            }),
            default_settings: json!({
                "use24Hour": true,
                "showStorageBadge": true
            }),
        },
        CatalogBlueprint {
            id: "palveni.widget.media_quote",
            name: "Daily Film & Book Quote Widget",
            version: "1.0.0",
            previous_version: None,
            runtime: PluginRuntimeKind::Widget,
            provides: vec!["widget"],
            permissions: vec!["library:read"],
            source: "official",
            description_zh: "只读概览小组件，每日从本地影视与书籍资料中精选一句台词或摘录。",
            description_en: "Read-only overview widget surfacing a daily quote from your film and book catalog.",
            settings_schema: json!({
                "type": "object",
                "properties": {
                    "sourceDomain": {
                        "type": "string",
                        "title": "Quote Source",
                        "enum": ["all", "video", "book"],
                        "default": "all"
                    }
                }
            }),
            default_settings: json!({
                "sourceDomain": "all"
            }),
        },
    ]
}

fn find_blueprint(plugin_id: &str) -> Option<CatalogBlueprint> {
    official_catalog_blueprints()
        .into_iter()
        .find(|entry| entry.id == plugin_id)
}

fn map_host_runtime(kind: HostRuntimeKind) -> PluginRuntimeKind {
    match kind {
        HostRuntimeKind::None => PluginRuntimeKind::None,
        HostRuntimeKind::Recipe => PluginRuntimeKind::Recipe,
        HostRuntimeKind::Wasm => PluginRuntimeKind::Wasm,
        HostRuntimeKind::Sidecar => PluginRuntimeKind::Sidecar,
    }
}

fn record_to_summary(record: PluginInstallationRecord) -> PluginSummary {
    let risk = compute_risk_level(&record.permissions);
    let description = find_blueprint(&record.plugin_id).map(|bp| bp.description_map());
    PluginSummary {
        id: record.plugin_id,
        name: record.name,
        version: record.version,
        runtime: record.runtime,
        enabled: record.enabled,
        provides: record.provides,
        previous_version: record.previous_version,
        permissions: Some(record.permissions),
        granted_permissions: Some(record.granted_permissions),
        settings_schema: Some(record.settings_schema),
        settings: Some(record.settings),
        source: Some(record.source),
        risk_level: Some(risk),
        description,
        managed: Some(true),
        installed_at: Some(record.installed_at),
        updated_at: Some(record.updated_at),
    }
}

/// Validates a settings JSON object against a JSON Schema `properties` definition,
/// filling defaults from `base` (defaults or existing settings) and rejecting unknown
/// keys, mismatched types, enum violations and length/numeric bounds.
pub fn validate_settings_against_schema(
    schema: &Value,
    incoming: Option<&Value>,
    base: &Value,
) -> Result<Value, ApiError> {
    let mut merged = match base.as_object() {
        Some(map) => map.clone(),
        None => Map::new(),
    };

    let props = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // Seed any missing schema defaults
    for (key, prop_schema) in &props {
        if !merged.contains_key(key)
            && let Some(def) = prop_schema.get("default")
        {
            merged.insert(key.clone(), def.clone());
        }
    }

    if let Some(patch_val) = incoming {
        let Some(patch_obj) = patch_val.as_object() else {
            return Err(
                ApiError::validation("Plugin settings must be a JSON object").field("settings"),
            );
        };
        for (key, val) in patch_obj {
            if !props.contains_key(key) {
                return Err(
                    ApiError::validation(format!("Unknown plugin setting `{key}`"))
                        .field("settings"),
                );
            }
            merged.insert(key.clone(), val.clone());
        }
    }

    for (key, prop_schema) in &props {
        let Some(val) = merged.get(key) else {
            continue;
        };
        if let Some(expected_type) = prop_schema.get("type").and_then(Value::as_str) {
            let type_ok = match expected_type {
                "boolean" => val.is_boolean(),
                "string" => val.is_string(),
                "integer" => val.is_i64() || val.is_u64(),
                "number" => val.is_number(),
                "array" => val.is_array(),
                "object" => val.is_object(),
                _ => true,
            };
            if !type_ok {
                return Err(ApiError::validation(format!(
                    "Setting `{key}` must be of type {expected_type}"
                ))
                .field("settings"));
            }
        }
        if let Some(allowed) = prop_schema.get("enum").and_then(Value::as_array)
            && !allowed.contains(val)
        {
            return Err(ApiError::validation(format!(
                "Setting `{key}` must be one of the allowed enum values"
            ))
            .field("settings"));
        }
        if let Some(s) = val.as_str() {
            let char_len = s.chars().count();
            if let Some(min_len) = prop_schema.get("minLength").and_then(Value::as_u64)
                && (char_len as u64) < min_len
            {
                return Err(ApiError::validation(format!(
                    "Setting `{key}` must be at least {min_len} characters"
                ))
                .field("settings"));
            }
            if let Some(max_len) = prop_schema.get("maxLength").and_then(Value::as_u64)
                && (char_len as u64) > max_len
            {
                return Err(ApiError::validation(format!(
                    "Setting `{key}` must be at most {max_len} characters"
                ))
                .field("settings"));
            }
        }
        if let Some(num) = val.as_f64() {
            if let Some(min_v) = prop_schema.get("minimum").and_then(Value::as_f64)
                && num < min_v
            {
                return Err(
                    ApiError::validation(format!("Setting `{key}` must be >= {min_v}"))
                        .field("settings"),
                );
            }
            if let Some(max_v) = prop_schema.get("maximum").and_then(Value::as_f64)
                && num > max_v
            {
                return Err(
                    ApiError::validation(format!("Setting `{key}` must be <= {max_v}"))
                        .field("settings"),
                );
            }
        }
    }

    Ok(Value::Object(merged))
}

/// `GET /api/v1/plugins` (Admin).
pub async fn list(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<PluginList>, ApiError> {
    let mut plugins: Vec<PluginSummary> = state
        .plugins
        .list()
        .into_iter()
        .map(|p| PluginSummary {
            id: p.id,
            name: p.name,
            version: p.version,
            runtime: map_host_runtime(p.runtime),
            enabled: p.enabled,
            provides: p.provides,
            previous_version: None,
            permissions: Some(Vec::new()),
            granted_permissions: Some(Vec::new()),
            settings_schema: Some(json!({})),
            settings: Some(json!({})),
            source: Some("host".to_owned()),
            risk_level: Some(PluginRiskLevel::Low),
            description: None,
            managed: Some(false),
            installed_at: None,
            updated_at: None,
        })
        .collect();

    let skins = super::skins::on_disk(state.skins, SkinStore::installed).await?;
    plugins.extend(skins.into_iter().map(|record| {
        let info = record.skin;
        let source = match record.source {
            palveni_plugin_host::skin_store::SkinSource::Upload => "local",
            _ => "official",
        };
        PluginSummary {
            name: info
                .name
                .get("en")
                .or_else(|| info.name.get("zh-CN"))
                .cloned()
                .unwrap_or(info.id),
            id: info.plugin_id,
            version: info.version,
            runtime: PluginRuntimeKind::None,
            enabled: true,
            provides: vec!["skin".to_owned()],
            previous_version: None,
            permissions: Some(Vec::new()),
            granted_permissions: Some(Vec::new()),
            settings_schema: Some(json!({})),
            settings: Some(json!({})),
            source: Some(source.to_owned()),
            risk_level: Some(PluginRiskLevel::Low),
            description: None,
            managed: Some(false),
            installed_at: None,
            updated_at: None,
        }
    }));

    let db_plugins = state.db.plugins_photo_ai().list_installed_plugins().await?;
    plugins.extend(db_plugins.into_iter().map(record_to_summary));

    Ok(Json(PluginList { plugins }))
}

/// `GET /api/v1/plugins/catalog` (Admin).
pub async fn catalog(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<PluginCatalogResponse>, ApiError> {
    let installed_rows = state.db.plugins_photo_ai().list_installed_plugins().await?;
    let installed_map: HashMap<String, PluginInstallationRecord> = installed_rows
        .into_iter()
        .map(|row| (row.plugin_id.clone(), row))
        .collect();

    let plugins = official_catalog_blueprints()
        .into_iter()
        .map(|bp| {
            let permissions = bp.permission_strings();
            let risk_level = compute_risk_level(&permissions);
            let installed_rec = installed_map.get(bp.id);
            PluginCatalogEntry {
                id: bp.id.to_owned(),
                name: bp.name.to_owned(),
                version: bp.version.to_owned(),
                previous_version: bp.previous_version.map(str::to_owned),
                runtime: bp.runtime,
                provides: bp.provide_strings(),
                permissions,
                risk_level,
                source: bp.source.to_owned(),
                description: bp.description_map(),
                settings_schema: bp.settings_schema,
                default_settings: bp.default_settings,
                installed: installed_rec.is_some(),
                installed_version: installed_rec.map(|r| r.version.clone()),
            }
        })
        .collect();

    Ok(Json(PluginCatalogResponse { plugins }))
}

/// `POST /api/v1/plugins/install` (Admin).
pub async fn install(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<InstallPluginRequest>, ApiError>,
) -> Result<(StatusCode, Json<PluginSummary>), ApiError> {
    let ApiJson(req) = body?;
    let plugin_id = req.plugin_id.trim();
    let Some(bp) = find_blueprint(plugin_id) else {
        return Err(ApiError::not_found(format!(
            "Plugin `{plugin_id}` is not in the official catalog"
        )));
    };

    let required_perms = bp.permission_strings();
    let granted_set: HashSet<&str> = req
        .granted_permissions
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let missing: Vec<&str> = required_perms
        .iter()
        .map(String::as_str)
        .filter(|perm| !granted_set.contains(perm))
        .collect();
    if !missing.is_empty() {
        return Err(ApiError::validation(format!(
            "Missing required plugin permissions: {}",
            missing.join(", ")
        ))
        .field("grantedPermissions"));
    }

    let validated_settings = validate_settings_against_schema(
        &bp.settings_schema,
        req.settings.as_ref(),
        &bp.default_settings,
    )?;

    let stored = state
        .db
        .plugins_photo_ai()
        .upsert_plugin_installation(PluginInstallationUpsert {
            plugin_id: bp.id.to_owned(),
            name: bp.name.to_owned(),
            version: bp.version.to_owned(),
            previous_version: bp.previous_version.map(str::to_owned),
            runtime: bp.runtime,
            provides: bp.provide_strings(),
            permissions: required_perms.clone(),
            granted_permissions: required_perms,
            settings_schema: bp.settings_schema,
            settings: validated_settings,
            enabled: true,
            source: bp.source.to_owned(),
        })
        .await?;

    let summary = record_to_summary(stored);
    audit(
        &state,
        AuditEntry {
            action: actions::PLUGIN_INSTALLED,
            actor: Some(admin.user.id),
            target: Some(summary.id.clone()),
            detail: Some(json!({
                "version": summary.version,
                "previousVersion": summary.previous_version,
                "riskLevel": summary.risk_level,
                "grantedPermissions": summary.granted_permissions,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(summary)))
}

/// `PATCH /api/v1/plugins/{id}` (Admin).
pub async fn update(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
    body: Result<ApiJson<UpdatePluginRequest>, ApiError>,
) -> Result<Json<PluginSummary>, ApiError> {
    let plugin_id = path?.0;
    let ApiJson(req) = body?;
    if req.enabled.is_none() && req.settings.is_none() {
        return Err(ApiError::validation(
            "Provide `enabled` or `settings` to update a plugin",
        ));
    }

    let existing = state
        .db
        .plugins_photo_ai()
        .get_installed_plugin(&plugin_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Installed plugin not found"))?;

    let next_settings = match req.settings.as_ref() {
        Some(incoming) => Some(validate_settings_against_schema(
            &existing.settings_schema,
            Some(incoming),
            &existing.settings,
        )?),
        None => None,
    };

    let updated = state
        .db
        .plugins_photo_ai()
        .update_plugin_installation(&plugin_id, req.enabled, next_settings)
        .await?;

    let summary = record_to_summary(updated);
    audit(
        &state,
        AuditEntry {
            action: actions::PLUGIN_UPDATED,
            actor: Some(admin.user.id),
            target: Some(summary.id.clone()),
            detail: Some(json!({
                "enabled": summary.enabled,
                "settingsUpdated": req.settings.is_some(),
            })),
        },
    )
    .await;

    Ok(Json(summary))
}

/// `POST /api/v1/plugins/{id}/rollback` (Admin).
pub async fn rollback(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<PluginSummary>, ApiError> {
    let plugin_id = path?.0;
    let rolled = state
        .db
        .plugins_photo_ai()
        .rollback_plugin_installation(&plugin_id)
        .await
        .map_err(|err| match err {
            DbError::NotFound(_) => ApiError::not_found("Installed plugin not found"),
            DbError::Invalid(msg) => ApiError::validation(msg),
            other => other.into(),
        })?;

    let summary = record_to_summary(rolled);
    audit(
        &state,
        AuditEntry {
            action: actions::PLUGIN_ROLLED_BACK,
            actor: Some(admin.user.id),
            target: Some(summary.id.clone()),
            detail: Some(json!({
                "rolledBackToVersion": summary.version,
            })),
        },
    )
    .await;

    Ok(Json(summary))
}

/// `DELETE /api/v1/plugins/{id}` (Admin).
pub async fn uninstall(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let plugin_id = path?.0;
    let deleted = state
        .db
        .plugins_photo_ai()
        .delete_plugin_installation(&plugin_id)
        .await?;
    if !deleted {
        return Err(ApiError::not_found("Installed plugin not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::PLUGIN_UNINSTALLED,
            actor: Some(admin.user.id),
            target: Some(plugin_id),
            detail: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
