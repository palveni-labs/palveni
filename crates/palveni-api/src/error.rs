//! Error responses:
//! `{"error": {"code": "...", "message": "...", "details"?: {...}}}`.
//!
//! `details` is optional and only present where the contract defines it
//! (`path_not_found`, `validation_failed`, `conflict`).

use std::fmt;

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use palveni_db::DbError;
use palveni_jobs::JobError;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Map<String, Value>>,
    retry_after: Option<u64>,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            status,
            code,
            message: message.into(),
            details: None,
            retry_after: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "validation_failed", message)
    }

    /// A library path that does not exist or is not a directory.
    /// `index` is its position in the request's `paths` array.
    pub fn path_not_found(message: impl Into<String>, index: usize, path: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "path_not_found", message)
            .with_detail("field", "paths")
            .with_detail("index", index)
            .with_detail("path", path)
    }

    pub fn throttled(seconds: u64) -> Self {
        let mut error = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_requests",
            "Too many login attempts",
        );
        error.retry_after = Some(seconds);
        error
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
        )
    }

    pub fn invalid_credentials() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Invalid username or password",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Administrator privileges required",
        )
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    /// The `Allow` header is added by the router.
    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "Method not allowed for this endpoint",
        )
    }

    pub fn setup_completed() -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "setup_already_completed",
            "Setup has already been completed",
        )
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// Logs the detail; the client only sees a generic message.
    pub fn internal(detail: impl fmt::Display) -> Self {
        tracing::error!(error = %detail, "internal error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Internal server error",
        )
    }

    /// Adds one `details` entry.
    #[must_use]
    pub fn with_detail(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.details
            .get_or_insert_with(Map::new)
            .insert(key.to_owned(), value.into());
        self
    }

    /// `details.field`: the request field the error is about.
    #[must_use]
    pub fn field(self, name: &str) -> Self {
        self.with_detail("field", name)
    }

    /// `details.field` plus `details.index` for an element of an array field.
    #[must_use]
    pub fn field_at(self, name: &str, index: usize) -> Self {
        self.with_detail("field", name).with_detail("index", index)
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn details(&self) -> Option<&Map<String, Value>> {
        self.details.as_ref()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.code, self.status, self.message)
    }
}

/// Wire shape; a struct so keys come out in the contract's order.
#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a Map<String, Value>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: &self.message,
                details: self.details.as_ref(),
            },
        });
        let mut response = (self.status, body).into_response();
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        if self.status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::Invalid(message) => Self::validation(message),
            DbError::NotFound(what) => {
                ApiError::not_found(format!("{} not found", capitalize(what)))
            }
            DbError::Conflict(message) => ApiError::conflict(message),
            other => ApiError::internal(other),
        }
    }
}

impl From<JobError> for ApiError {
    fn from(err: JobError) -> Self {
        match err {
            JobError::Db(db) => db.into(),
            other => ApiError::internal(other),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        ApiError::validation(rejection.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        ApiError::validation(rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        ApiError::validation(rejection.body_text())
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn keys_follow_the_contract_order() {
        let response = ApiError::conflict("dup").field("name").into_response();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            r#"{"error":{"code":"conflict","message":"dup","details":{"field":"name"}}}"#
        );
    }

    async fn body_of(error: ApiError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn details_are_omitted_unless_set() {
        let (status, body) = body_of(ApiError::not_found("Library not found")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            body,
            json!({ "error": { "code": "not_found", "message": "Library not found" } })
        );
    }

    #[tokio::test]
    async fn path_not_found_details() {
        let error = ApiError::path_not_found("Path does not exist: /m", 1, "/m");
        let (status, body) = body_of(error).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            body["error"]["details"],
            json!({ "field": "paths", "index": 1, "path": "/m" })
        );
    }

    #[tokio::test]
    async fn field_details() {
        let (_, body) = body_of(ApiError::validation("bad").field_at("paths", 2)).await;
        assert_eq!(
            body["error"]["details"],
            json!({ "field": "paths", "index": 2 })
        );
        let (status, body) = body_of(ApiError::conflict("dup").field("name")).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["details"], json!({ "field": "name" }));
    }
}
