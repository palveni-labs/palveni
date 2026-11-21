//! Embedded web UI (`embed-web` feature): serves `web/dist` from the
//! binary with an SPA fallback to `index.html`.

use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct WebAssets;

const INDEX: &str = "index.html";

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // Unknown API routes must stay JSON errors, never the SPA shell.
    if path == "api" || path.starts_with("api/") {
        return palveni_api::not_found().await.into_response();
    }
    let candidate = if path.is_empty() { INDEX } else { path };
    if let Some(file) = WebAssets::get(candidate) {
        return asset_response(candidate, file);
    }
    // Client-side routes have no file extension; missing assets do.
    let last_segment = candidate.rsplit('/').next().unwrap_or(candidate);
    if !last_segment.contains('.')
        && let Some(index) = WebAssets::get(INDEX)
    {
        return asset_response(INDEX, index);
    }
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

fn asset_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();
    // Vite puts content-hashed files under assets/; everything else (the
    // HTML shell in particular) must be revalidated so upgrades show up.
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache.to_owned()),
        ],
        file.data.into_owned(),
    )
        .into_response()
}
