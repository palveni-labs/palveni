//! Online metadata scraper (TMDB / Bangumi / OpenLibrary) and manual item metadata overrides
//! (`module: metadata`).

use std::net::IpAddr;
use std::time::Duration;

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::ItemId;
use palveni_core::metadata::{
    MetadataConfigResponse, MetadataOverride, MetadataOverrideResponse, MetadataProvider,
    MetadataSearchCandidate, MetadataSearchMediaType, MetadataSearchRequest,
    MetadataSearchResponse, SaveMetadataOverrideRequest, UpdateMetadataConfigRequest,
};
use palveni_core::observability::ServerLogLevel;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::metadata_overrides::MetadataOverrideUpsert;
use palveni_db::settings::KEY_METADATA_CONFIG;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::home::{decrypt_secret, encrypt_secret};
use crate::routes::observability::record_log;
use crate::routes::{audit, path_id};
use crate::state::AppState;

const METADATA_SECRET_SCOPE: &[u8] = b"palveni:metadata-provider-key:v1";
const SCRAPER_TIMEOUT: Duration = Duration::from_secs(8);
const SCRAPER_USER_AGENT: &str = "Palveni-Server/0.1 (+https://palveni.local)";
const DEFAULT_LANGUAGE: &str = "zh-CN";
const MAX_URL_CHARS: usize = 2048;

type IdPath = Result<Path<String>, PathRejection>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredMetadataConfig {
    tmdb_api_key_cipher: Option<String>,
    bangumi_token_cipher: Option<String>,
    preferred_language: String,
    auto_fetch_posters: bool,
}

impl Default for StoredMetadataConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key_cipher: None,
            bangumi_token_cipher: None,
            preferred_language: DEFAULT_LANGUAGE.to_owned(),
            auto_fetch_posters: true,
        }
    }
}

async fn load_config(state: &AppState) -> Result<StoredMetadataConfig, ApiError> {
    let raw = state.db.settings().get(KEY_METADATA_CONFIG).await?;
    Ok(raw
        .as_deref()
        .and_then(|s| serde_json::from_str::<StoredMetadataConfig>(s).ok())
        .unwrap_or_default())
}

fn to_public_config(stored: &StoredMetadataConfig) -> MetadataConfigResponse {
    MetadataConfigResponse {
        tmdb_configured: stored
            .tmdb_api_key_cipher
            .as_deref()
            .is_some_and(|s| !s.is_empty()),
        bangumi_configured: stored
            .bangumi_token_cipher
            .as_deref()
            .is_some_and(|s| !s.is_empty()),
        preferred_language: stored.preferred_language.clone(),
        auto_fetch_posters: stored.auto_fetch_posters,
    }
}

/// `GET /api/v1/metadata/config` (Admin).
pub async fn get_config(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MetadataConfigResponse>, ApiError> {
    let stored = load_config(&state).await?;
    Ok(Json(to_public_config(&stored)))
}

/// `PUT /api/v1/metadata/config` (Admin).
pub async fn update_config(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<UpdateMetadataConfigRequest>, ApiError>,
) -> Result<Json<MetadataConfigResponse>, ApiError> {
    let ApiJson(req) = body?;
    let mut stored = load_config(&state).await?;

    if req.clear_tmdb_key == Some(true) {
        stored.tmdb_api_key_cipher = None;
    } else if let Some(key) = req.tmdb_api_key.as_deref().map(str::trim)
        && !key.is_empty()
    {
        if key.chars().count() > 512 {
            return Err(
                ApiError::validation("TMDB API key must be at most 512 characters")
                    .field("tmdbApiKey"),
            );
        }
        stored.tmdb_api_key_cipher = Some(encrypt_secret(
            &state.identity,
            METADATA_SECRET_SCOPE,
            "tmdb",
            key,
        )?);
    }

    if req.clear_bangumi_token == Some(true) {
        stored.bangumi_token_cipher = None;
    } else if let Some(token) = req.bangumi_token.as_deref().map(str::trim)
        && !token.is_empty()
    {
        if token.chars().count() > 512 {
            return Err(
                ApiError::validation("Bangumi token must be at most 512 characters")
                    .field("bangumiToken"),
            );
        }
        stored.bangumi_token_cipher = Some(encrypt_secret(
            &state.identity,
            METADATA_SECRET_SCOPE,
            "bangumi",
            token,
        )?);
    }

    if let Some(lang) = req.preferred_language.as_deref().map(str::trim) {
        if lang.is_empty()
            || lang.len() > 16
            || !lang
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(ApiError::validation(
                "preferredLanguage must be a valid BCP 47 tag (e.g. zh-CN or en-US)",
            )
            .field("preferredLanguage"));
        }
        stored.preferred_language = lang.to_owned();
    }

    if let Some(auto_fetch) = req.auto_fetch_posters {
        stored.auto_fetch_posters = auto_fetch;
    }

    let serialized = serde_json::to_string(&stored).map_err(ApiError::internal)?;
    state
        .db
        .settings()
        .set(KEY_METADATA_CONFIG, &serialized)
        .await?;

    let public = to_public_config(&stored);
    audit(
        &state,
        AuditEntry {
            action: actions::METADATA_CONFIG_UPDATED,
            actor: Some(admin.user.id),
            target: Some("metadata.config".to_owned()),
            detail: Some(json!({
                "tmdbConfigured": public.tmdb_configured,
                "bangumiConfigured": public.bangumi_configured,
                "preferredLanguage": public.preferred_language,
                "autoFetchPosters": public.auto_fetch_posters,
            })),
        },
    )
    .await;
    record_log(
        ServerLogLevel::Info,
        "metadata.config",
        format!(
            "Metadata config updated (tmdb={}, bangumi={}, lang={})",
            public.tmdb_configured, public.bangumi_configured, public.preferred_language
        ),
    );

    Ok(Json(public))
}

/// `POST /api/v1/metadata/search` (Admin).
pub async fn search(
    _admin: AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<MetadataSearchRequest>, ApiError>,
) -> Result<Json<MetadataSearchResponse>, ApiError> {
    let ApiJson(req) = body?;
    let query = req.query.trim();
    if query.is_empty() || query.chars().count() > 200 {
        return Err(
            ApiError::validation("Search query must be 1 to 200 characters").field("query"),
        );
    }
    if let Some(year) = req.year
        && !(1800..=2100).contains(&year)
    {
        return Err(ApiError::validation("year must be between 1800 and 2100").field("year"));
    }
    if req.provider == MetadataProvider::Manual {
        return Err(ApiError::validation(
            "Online search provider must be tmdb, bangumi, or openlibrary",
        )
        .field("provider"));
    }

    let stored = load_config(&state).await?;
    let client = reqwest::Client::builder()
        .timeout(SCRAPER_TIMEOUT)
        .user_agent(SCRAPER_USER_AGENT)
        .build()
        .map_err(ApiError::internal)?;

    let candidates = match req.provider {
        MetadataProvider::Tmdb => {
            let Some(cipher) = stored
                .tmdb_api_key_cipher
                .as_deref()
                .filter(|s| !s.is_empty())
            else {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_failed",
                    "TMDB API Key is not configured. Save a TMDB API Key in System Maintenance before searching TMDB.",
                )
                .field("provider"));
            };
            let api_key = decrypt_secret(&state.identity, METADATA_SECRET_SCOPE, "tmdb", cipher)?;
            let recipe_settings = state
                .db
                .plugins_photo_ai()
                .get_installed_plugin("palveni.recipe.tmdb")
                .await
                .ok()
                .flatten()
                .filter(|rec| rec.enabled)
                .map(|rec| rec.settings);
            let (effective_lang, include_adult) =
                resolve_tmdb_recipe_options(&stored.preferred_language, recipe_settings.as_ref());
            search_tmdb(
                &client,
                &api_key,
                query,
                req.media_type,
                req.year,
                &effective_lang,
                include_adult,
            )
            .await?
        }
        MetadataProvider::Bangumi => {
            let token = match stored
                .bangumi_token_cipher
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                Some(cipher) => Some(decrypt_secret(
                    &state.identity,
                    METADATA_SECRET_SCOPE,
                    "bangumi",
                    cipher,
                )?),
                None => None,
            };
            let prefer_chinese_title = state
                .db
                .plugins_photo_ai()
                .get_installed_plugin("palveni.recipe.bangumi")
                .await
                .ok()
                .flatten()
                .filter(|rec| rec.enabled)
                .and_then(|rec| {
                    rec.settings
                        .get("preferChineseTitle")
                        .and_then(Value::as_bool)
                })
                .unwrap_or(true);
            search_bangumi(
                &client,
                token.as_deref(),
                query,
                req.media_type,
                prefer_chinese_title,
            )
            .await?
        }
        MetadataProvider::Openlibrary => search_openlibrary(&client, query, req.year).await?,
        MetadataProvider::Manual => unreachable!(),
    };

    record_log(
        ServerLogLevel::Info,
        "metadata.search",
        format!(
            "Searched {} for {:?}: {} candidates",
            req.provider.as_str(),
            query,
            candidates.len()
        ),
    );

    Ok(Json(MetadataSearchResponse {
        provider: req.provider,
        query: query.to_owned(),
        candidates,
    }))
}

fn resolve_tmdb_recipe_options(
    default_language: &str,
    recipe_settings: Option<&Value>,
) -> (String, bool) {
    let language = recipe_settings
        .and_then(|v| v.get("language"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|lang| {
            !lang.is_empty()
                && lang.len() <= 16
                && lang
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
        .unwrap_or(default_language)
        .to_owned();
    let include_adult = recipe_settings
        .and_then(|v| v.get("includeAdult"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (language, include_adult)
}

async fn search_tmdb(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    media_type: Option<MetadataSearchMediaType>,
    year: Option<i32>,
    language: &str,
    include_adult: bool,
) -> Result<Vec<MetadataSearchCandidate>, ApiError> {
    let endpoint = match media_type {
        Some(MetadataSearchMediaType::Movie) => "https://api.themoviedb.org/3/search/movie",
        Some(MetadataSearchMediaType::Tv | MetadataSearchMediaType::Anime) => {
            "https://api.themoviedb.org/3/search/tv"
        }
        _ => "https://api.themoviedb.org/3/search/multi",
    };

    let mut params: Vec<(&str, String)> = vec![
        ("query", query.to_owned()),
        ("language", language.to_owned()),
        (
            "include_adult",
            if include_adult { "true" } else { "false" }.to_owned(),
        ),
    ];
    if let Some(yr) = year {
        let year_param = match media_type {
            Some(MetadataSearchMediaType::Tv | MetadataSearchMediaType::Anime) => {
                "first_air_date_year"
            }
            _ => "year",
        };
        params.push((year_param, yr.to_string()));
    }

    let mut builder = client.get(endpoint);
    if api_key.starts_with("eyJ") {
        builder = builder.bearer_auth(api_key);
    } else {
        params.push(("api_key", api_key.to_owned()));
    }

    let resp = builder
        .query(&params)
        .send()
        .await
        .map_err(|err| upstream_error("TMDB", err))?;

    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("TMDB returned HTTP {}", resp.status().as_u16()),
        ));
    }

    let payload: Value = resp
        .json()
        .await
        .map_err(|err| upstream_error("TMDB", err))?;
    Ok(parse_tmdb_results(&payload, media_type))
}

fn parse_tmdb_results(
    payload: &Value,
    requested_type: Option<MetadataSearchMediaType>,
) -> Vec<MetadataSearchCandidate> {
    let Some(items) = payload.get("results").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items.iter().take(12) {
        let raw_kind =
            item.get("media_type")
                .and_then(Value::as_str)
                .unwrap_or(match requested_type {
                    Some(MetadataSearchMediaType::Tv | MetadataSearchMediaType::Anime) => "tv",
                    _ => "movie",
                });
        if raw_kind == "person" {
            continue;
        }
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let title = item
            .get("title")
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let Some(title) = title else { continue };

        let original_title = item
            .get("original_title")
            .or_else(|| item.get("original_name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != title)
            .map(ToOwned::to_owned);

        let date_str = item
            .get("release_date")
            .or_else(|| item.get("first_air_date"))
            .and_then(Value::as_str);
        let year = date_str
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse::<i32>().ok());

        let overview = item
            .get("overview")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);

        let rating = item
            .get("vote_average")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| (v * 10.0).round() / 10.0);

        let genres = item
            .get("genre_ids")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_i64)
                    .filter_map(tmdb_genre_name)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let poster_url = item
            .get("poster_path")
            .and_then(Value::as_str)
            .filter(|p| p.starts_with('/'))
            .map(|p| format!("https://image.tmdb.org/t/p/w500{p}"));
        let backdrop_url = item
            .get("backdrop_path")
            .and_then(Value::as_str)
            .filter(|p| p.starts_with('/'))
            .map(|p| format!("https://image.tmdb.org/t/p/w1280{p}"));

        out.push(MetadataSearchCandidate {
            provider: MetadataProvider::Tmdb,
            external_id: id.to_string(),
            title: title.to_owned(),
            original_title,
            year,
            overview,
            rating,
            genres,
            poster_url,
            backdrop_url,
            media_type: if raw_kind == "tv" {
                "tv".to_owned()
            } else {
                "movie".to_owned()
            },
        });
    }
    out
}

fn tmdb_genre_name(id: i64) -> Option<&'static str> {
    match id {
        12 => Some("Adventure"),
        14 => Some("Fantasy"),
        16 => Some("Animation"),
        18 => Some("Drama"),
        27 => Some("Horror"),
        28 => Some("Action"),
        35 => Some("Comedy"),
        36 => Some("History"),
        37 => Some("Western"),
        53 => Some("Thriller"),
        80 => Some("Crime"),
        99 => Some("Documentary"),
        878 => Some("Science Fiction"),
        9648 => Some("Mystery"),
        10402 => Some("Music"),
        10749 => Some("Romance"),
        10751 => Some("Family"),
        10752 => Some("War"),
        10759 => Some("Action & Adventure"),
        10765 => Some("Sci-Fi & Fantasy"),
        _ => None,
    }
}

async fn search_bangumi(
    client: &reqwest::Client,
    token: Option<&str>,
    query: &str,
    media_type: Option<MetadataSearchMediaType>,
    prefer_chinese_title: bool,
) -> Result<Vec<MetadataSearchCandidate>, ApiError> {
    let type_filter: Vec<i32> = match media_type {
        Some(MetadataSearchMediaType::Book) => vec![1],
        Some(MetadataSearchMediaType::Anime) => vec![2],
        Some(MetadataSearchMediaType::Tv | MetadataSearchMediaType::Movie) => vec![2, 6],
        None => vec![1, 2, 6],
    };
    let body = json!({
        "keyword": query,
        "filter": {
            "type": type_filter,
        }
    });

    let mut builder = client
        .post("https://api.bgm.tv/v0/search/subjects?limit=12")
        .json(&body);
    if let Some(tok) = token {
        builder = builder.bearer_auth(tok);
    }

    let resp = builder
        .send()
        .await
        .map_err(|err| upstream_error("Bangumi", err))?;

    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("Bangumi returned HTTP {}", resp.status().as_u16()),
        ));
    }

    let payload: Value = resp
        .json()
        .await
        .map_err(|err| upstream_error("Bangumi", err))?;
    Ok(parse_bangumi_results(&payload, prefer_chinese_title))
}

fn parse_bangumi_results(
    payload: &Value,
    prefer_chinese_title: bool,
) -> Vec<MetadataSearchCandidate> {
    let Some(items) = payload.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items.iter().take(12) {
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let name_cn = item
            .get("name_cn")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let title = if prefer_chinese_title {
            if !name_cn.is_empty() { name_cn } else { name }
        } else if !name.is_empty() {
            name
        } else {
            name_cn
        };
        if title.is_empty() {
            continue;
        }
        let original_title = if prefer_chinese_title {
            (!name.is_empty() && name != title).then(|| name.to_owned())
        } else {
            (!name_cn.is_empty() && name_cn != title).then(|| name_cn.to_owned())
        };

        let year = item
            .get("date")
            .and_then(Value::as_str)
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse::<i32>().ok());
        let overview = item
            .get("summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        let rating = item
            .get("rating")
            .and_then(|r| r.get("score"))
            .or_else(|| item.get("score"))
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| (v * 10.0).round() / 10.0);

        let mut genres = Vec::new();
        if let Some(meta_tags) = item.get("meta_tags").and_then(Value::as_array) {
            for tag in meta_tags.iter().take(6) {
                if let Some(s) = tag.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    genres.push(s.to_owned());
                }
            }
        } else if let Some(tags) = item.get("tags").and_then(Value::as_array) {
            for tag in tags.iter().take(6) {
                if let Some(s) = tag
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    genres.push(s.to_owned());
                }
            }
        }

        let poster_url = item
            .get("images")
            .and_then(|imgs| {
                imgs.get("large")
                    .or_else(|| imgs.get("common"))
                    .or_else(|| imgs.get("medium"))
            })
            .and_then(Value::as_str)
            .map(normalize_https_url)
            .filter(|s| s.starts_with("https://"));

        let media_type = match item.get("type").and_then(Value::as_i64) {
            Some(1) => "book",
            Some(2) => "anime",
            _ => "tv",
        };

        out.push(MetadataSearchCandidate {
            provider: MetadataProvider::Bangumi,
            external_id: id.to_string(),
            title: title.to_owned(),
            original_title,
            year,
            overview,
            rating,
            genres,
            poster_url,
            backdrop_url: None,
            media_type: media_type.to_owned(),
        });
    }
    out
}

async fn search_openlibrary(
    client: &reqwest::Client,
    query: &str,
    year: Option<i32>,
) -> Result<Vec<MetadataSearchCandidate>, ApiError> {
    let q = match year {
        Some(yr) => format!("{query} {yr}"),
        None => query.to_owned(),
    };
    let resp = client
        .get("https://openlibrary.org/search.json")
        .query(&[
            ("q", q.as_str()),
            ("limit", "12"),
            (
                "fields",
                "key,title,author_name,first_publish_year,cover_i,subject,ratings_average,first_sentence",
            ),
        ])
        .send()
        .await
        .map_err(|err| upstream_error("OpenLibrary", err))?;

    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("OpenLibrary returned HTTP {}", resp.status().as_u16()),
        ));
    }

    let payload: Value = resp
        .json()
        .await
        .map_err(|err| upstream_error("OpenLibrary", err))?;
    Ok(parse_openlibrary_results(&payload))
}

fn parse_openlibrary_results(payload: &Value) -> Vec<MetadataSearchCandidate> {
    let Some(docs) = payload.get("docs").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for doc in docs.iter().take(12) {
        let Some(title) = doc
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let external_id = doc
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or(title)
            .to_owned();
        let year = doc
            .get("first_publish_year")
            .and_then(Value::as_i64)
            .and_then(|y| i32::try_from(y).ok());

        let authors: Vec<&str> = doc
            .get("author_name")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .take(4)
                    .collect()
            })
            .unwrap_or_default();

        let first_sentence = doc
            .get("first_sentence")
            .and_then(|v| {
                v.as_str()
                    .or_else(|| v.as_array().and_then(|a| a.first()).and_then(Value::as_str))
            })
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let overview = match (authors.is_empty(), first_sentence) {
            (false, Some(sentence)) => Some(format!("{} — {sentence}", authors.join(", "))),
            (false, None) => Some(authors.join(", ")),
            (true, Some(sentence)) => Some(sentence.to_owned()),
            (true, None) => None,
        };

        let rating = doc
            .get("ratings_average")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| ((v * 2.0).clamp(0.0, 10.0) * 10.0).round() / 10.0);

        let genres = doc
            .get("subject")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .take(6)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let poster_url = doc
            .get("cover_i")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"));

        out.push(MetadataSearchCandidate {
            provider: MetadataProvider::Openlibrary,
            external_id,
            title: title.to_owned(),
            original_title: None,
            year,
            overview,
            rating,
            genres,
            poster_url,
            backdrop_url: None,
            media_type: "book".to_owned(),
        });
    }
    out
}

fn normalize_https_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("https://{rest}")
    } else {
        trimmed.to_owned()
    }
}

fn upstream_error(provider: &str, err: reqwest::Error) -> ApiError {
    record_log(
        ServerLogLevel::Warn,
        "metadata.search",
        format!("{provider} search failed: {err}"),
    );
    ApiError::new(
        StatusCode::BAD_GATEWAY,
        "unavailable",
        format!("Could not reach {provider}: {err}"),
    )
}

/// `GET /api/v1/items/{id}/metadata-override` (Authenticated).
pub async fn get_override(
    current: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<MetadataOverrideResponse>, ApiError> {
    let id: ItemId = path_id(&path?.0, "Item")?;
    let item = state
        .db
        .items()
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    if !state
        .db
        .accounts()
        .can_access_library(current.user.id, current.user.role, item.library_id)
        .await?
    {
        return Err(ApiError::not_found("Item not found"));
    }
    let override_data = state.db.metadata_overrides().get(id).await?;
    Ok(Json(MetadataOverrideResponse {
        item_id: id,
        override_data,
    }))
}

/// `PUT /api/v1/items/{id}/metadata-override` (Admin).
pub async fn save_override(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
    body: Result<ApiJson<SaveMetadataOverrideRequest>, ApiError>,
) -> Result<Json<MetadataOverride>, ApiError> {
    let id: ItemId = path_id(&path?.0, "Item")?;
    let ApiJson(req) = body?;

    if let Some(ref title) = req.title
        && title.chars().count() > 255
    {
        return Err(ApiError::validation("title must be at most 255 characters").field("title"));
    }
    if let Some(ref orig) = req.original_title
        && orig.chars().count() > 255
    {
        return Err(
            ApiError::validation("originalTitle must be at most 255 characters")
                .field("originalTitle"),
        );
    }
    if let Some(ref overview) = req.overview
        && overview.chars().count() > 10_000
    {
        return Err(
            ApiError::validation("overview must be at most 10000 characters").field("overview"),
        );
    }
    if let Some(year) = req.year
        && !(1800..=2100).contains(&year)
    {
        return Err(ApiError::validation("year must be between 1800 and 2100").field("year"));
    }
    if let Some(rating) = req.rating
        && (!rating.is_finite() || !(0.0..=10.0).contains(&rating))
    {
        return Err(ApiError::validation("rating must be between 0.0 and 10.0").field("rating"));
    }

    let poster_url = validate_https_image_url(req.poster_url.as_deref(), "posterUrl")?;
    let backdrop_url = validate_https_image_url(req.backdrop_url.as_deref(), "backdropUrl")?;

    let saved = state
        .db
        .metadata_overrides()
        .upsert(MetadataOverrideUpsert {
            item_id: id,
            work_key: req.work_key,
            provider: req.provider.unwrap_or(MetadataProvider::Manual),
            external_id: req.external_id,
            title: req.title,
            original_title: req.original_title,
            year: req.year,
            overview: req.overview,
            rating: req.rating,
            genres: req.genres.unwrap_or_default(),
            poster_url,
            backdrop_url,
        })
        .await?;

    audit(
        &state,
        AuditEntry {
            action: actions::METADATA_OVERRIDE_SAVED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: Some(json!({
                "provider": saved.provider.as_str(),
                "title": saved.title,
                "externalId": saved.external_id,
            })),
        },
    )
    .await;
    record_log(
        ServerLogLevel::Info,
        "metadata.override",
        format!(
            "Saved metadata override for item {id} (provider={}, title={:?})",
            saved.provider.as_str(),
            saved.title
        ),
    );

    Ok(Json(saved))
}

/// `DELETE /api/v1/items/{id}/metadata-override` (Admin).
pub async fn delete_override(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let id: ItemId = path_id(&path?.0, "Item")?;
    let removed = state.db.metadata_overrides().delete(id).await?;
    if !removed {
        return Err(ApiError::not_found("Metadata override not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::METADATA_OVERRIDE_DELETED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: None,
        },
    )
    .await;
    record_log(
        ServerLogLevel::Info,
        "metadata.override",
        format!("Removed metadata override for item {id}"),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Validates optional artwork URLs strictly (`https://` public host only, preventing SSRF).
fn validate_https_image_url(raw: Option<&str>, field: &str) -> Result<Option<String>, ApiError> {
    let Some(trimmed) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if trimmed.chars().count() > MAX_URL_CHARS {
        return Err(ApiError::validation(format!(
            "{field} must be at most {MAX_URL_CHARS} characters"
        ))
        .field(field));
    }
    let parsed = Url::parse(trimmed).map_err(|_| {
        ApiError::validation(format!("{field} must be a valid https:// URL")).field(field)
    })?;
    if parsed.scheme() != "https" {
        return Err(
            ApiError::validation(format!("{field} must use the https:// scheme")).field(field),
        );
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(
            ApiError::validation(format!("{field} must not include credentials")).field(field),
        );
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| ApiError::validation(format!("{field} must have a host")).field(field))?;
    let bare_host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare_host.parse::<IpAddr>() {
        if is_disallowed_ip(ip) {
            return Err(ApiError::validation(format!(
                "{field} cannot target private or loopback IP addresses"
            ))
            .field(field));
        }
    } else {
        let lower = bare_host.trim_end_matches('.').to_ascii_lowercase();
        if lower.is_empty()
            || lower == "localhost"
            || lower.ends_with(".localhost")
            || lower.ends_with(".local")
            || lower.ends_with(".internal")
            || lower.ends_with(".home.arpa")
        {
            return Err(ApiError::validation(format!(
                "{field} cannot target local or internal hosts"
            ))
            .field(field));
        }
    }

    Ok(Some(trimmed.to_owned()))
}

fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            let is_cgnat = o[0] == 100 && (o[1] & 0b1100_0000) == 64;
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                || is_cgnat
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_disallowed_ip(IpAddr::V4(mapped));
            }
            let seg0 = v6.segments()[0];
            let is_ula = (seg0 & 0xfe00) == 0xfc00;
            let is_link_local = (seg0 & 0xffc0) == 0xfe80;
            v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() || is_ula || is_link_local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_validator_blocks_http_and_ssrf_hosts() {
        assert_eq!(
            validate_https_image_url(Some("https://image.tmdb.org/t/p/w500/abc.jpg"), "posterUrl")
                .unwrap()
                .as_deref(),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg")
        );
        assert_eq!(
            validate_https_image_url(Some("   "), "posterUrl").unwrap(),
            None
        );
        assert!(validate_https_image_url(Some("http://example.com/a.jpg"), "posterUrl").is_err());
        assert!(validate_https_image_url(Some("https://127.0.0.1/a.jpg"), "posterUrl").is_err());
        assert!(
            validate_https_image_url(Some("https://169.254.169.254/latest"), "posterUrl").is_err()
        );
        assert!(
            validate_https_image_url(Some("https://192.168.1.5/img.jpg"), "posterUrl").is_err()
        );
        assert!(validate_https_image_url(Some("https://localhost/a.jpg"), "posterUrl").is_err());
        assert!(validate_https_image_url(Some("https://nas.local/a.jpg"), "posterUrl").is_err());
        assert!(validate_https_image_url(Some("https://[::1]/a.jpg"), "posterUrl").is_err());
    }

    #[test]
    fn provider_parsers_normalize_candidates() {
        let tmdb = parse_tmdb_results(
            &json!({
                "results": [{
                    "id": 438631,
                    "media_type": "movie",
                    "title": "沙丘",
                    "original_title": "Dune",
                    "release_date": "2021-09-15",
                    "overview": "Arrakis.",
                    "vote_average": 7.82,
                    "genre_ids": [878, 12],
                    "poster_path": "/dune.jpg",
                    "backdrop_path": "/bg.jpg"
                }]
            }),
            Some(MetadataSearchMediaType::Movie),
        );
        assert_eq!(tmdb.len(), 1);
        assert_eq!(tmdb[0].external_id, "438631");
        assert_eq!(tmdb[0].year, Some(2021));
        assert_eq!(tmdb[0].rating, Some(7.8));
        assert_eq!(tmdb[0].genres, vec!["Science Fiction", "Adventure"]);
        assert_eq!(
            tmdb[0].poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/dune.jpg")
        );

        let bgm_payload = json!({
            "data": [{
                "id": 253,
                "type": 2,
                "name": "Cowboy Bebop",
                "name_cn": "星际牛仔",
                "date": "1998-04-03",
                "summary": "Space bounty hunters.",
                "rating": { "score": 9.1 },
                "meta_tags": ["科幻", "赏金猎人"],
                "images": { "large": "http://lain.bgm.tv/pic/cover/l/a.jpg" }
            }]
        });
        let bgm = parse_bangumi_results(&bgm_payload, true);
        assert_eq!(bgm.len(), 1);
        assert_eq!(bgm[0].title, "星际牛仔");
        assert_eq!(bgm[0].original_title.as_deref(), Some("Cowboy Bebop"));
        assert_eq!(bgm[0].media_type, "anime");
        assert_eq!(
            bgm[0].poster_url.as_deref(),
            Some("https://lain.bgm.tv/pic/cover/l/a.jpg")
        );

        let bgm_orig = parse_bangumi_results(&bgm_payload, false);
        assert_eq!(bgm_orig[0].title, "Cowboy Bebop");
        assert_eq!(bgm_orig[0].original_title.as_deref(), Some("星际牛仔"));

        assert_eq!(
            resolve_tmdb_recipe_options(
                "zh-CN",
                Some(&json!({ "language": "ja-JP", "includeAdult": true })),
            ),
            ("ja-JP".to_owned(), true)
        );
        assert_eq!(
            resolve_tmdb_recipe_options("zh-CN", Some(&json!({ "language": "bad lang!" }))),
            ("zh-CN".to_owned(), false)
        );

        let ol = parse_openlibrary_results(&json!({
            "docs": [{
                "key": "/works/OL1W",
                "title": "Dune",
                "author_name": ["Frank Herbert"],
                "first_publish_year": 1965,
                "cover_i": 8231856,
                "ratings_average": 4.3,
                "subject": ["Science fiction", "Desert"]
            }]
        }));
        assert_eq!(ol.len(), 1);
        assert_eq!(ol[0].external_id, "/works/OL1W");
        assert_eq!(ol[0].year, Some(1965));
        assert_eq!(ol[0].rating, Some(8.6));
        assert_eq!(
            ol[0].poster_url.as_deref(),
            Some("https://covers.openlibrary.org/b/id/8231856-L.jpg")
        );
    }
}
