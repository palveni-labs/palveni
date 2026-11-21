//! Native HTTP API, `/api/v1`.

mod auth;
mod endpoints;
mod error;
mod json;
mod login_limit;
mod openapi;
mod pairings;
pub use login_limit::LoginLimit;
pub use pairings::LinkPairings;
mod routes;
mod state;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderName, Request};
pub use openapi::openapi_document;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, Span};

pub use error::ApiError;
pub use state::{AppState, ServerInfo};

const REQUEST_ID_HEADER: &str = "x-request-id";

/// The `/api/v1` router with state applied. The caller adds a fallback for
/// everything outside `/api` (the embedded web UI, or [`not_found`]) and
/// then [`with_http_layers`].
pub fn router(state: AppState) -> Router {
    let mut v1 = Router::new();
    for endpoint in endpoints::endpoints() {
        if endpoint.capability.module == "link" && !state.link.enabled() {
            continue;
        }
        let path = endpoint
            .capability
            .path
            .trim_start_matches("/api/v1")
            .replace("{path}", "{*path}");
        let mut handler = (endpoint.handler)();
        if let endpoints::Input::LimitedJson(_, limit) = endpoint.input {
            handler = handler.layer(axum::extract::DefaultBodyLimit::max(limit));
        }
        v1 = v1.route(&path, handler);
        state.features.register(endpoint.capability);
    }
    let v1 = v1
        // Applies to the routes registered above; axum adds the `Allow`
        // header to whatever this returns.
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found);
    Router::new()
        .nest("/api/v1", v1)
        .merge(routes::opds::router())
        .route(
            "/rest/{method}",
            axum::routing::get(routes::subsonic::handle),
        )
        .with_state(state)
}

/// JSON 404 in the contract's error shape.
pub async fn not_found() -> ApiError {
    ApiError::not_found("No such endpoint")
}

/// JSON 405 for a known path with an unsupported method.
async fn method_not_allowed() -> ApiError {
    ApiError::method_not_allowed()
}

/// Request ID and tracing layers, applied outermost so they also cover the
/// fallback.
pub fn with_http_layers(router: Router) -> Router {
    let header = HeaderName::from_static(REQUEST_ID_HEADER);
    router
        .layer(PropagateRequestIdLayer::new(header.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        // Outermost: assigns the ID before tracing sees the request.
        .layer(SetRequestIdLayer::new(header, MakeRequestUuid))
}

fn make_span(request: &Request<Body>) -> Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    tracing::info_span!(
        "http",
        method = %request.method(),
        path = %request.uri().path(),
        request_id = %request_id,
    )
}
