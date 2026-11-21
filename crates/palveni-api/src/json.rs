//! JSON request bodies with field-level validation errors.
//!
//! Like `axum::Json`, but every failure becomes `400 validation_failed`
//! (contract: any body error) and, where the failing field can be
//! determined, carries `details.field` / `details.index`.

use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, header};
use serde::de::DeserializeOwned;
use serde_path_to_error::Segment;

use crate::error::ApiError;

pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !is_json(req.headers()) {
            return Err(ApiError::validation(
                "Expected request with `Content-Type: application/json`",
            ));
        }
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|rejection| ApiError::validation(rejection.body_text()))?;
        parse(&bytes).map(ApiJson)
    }
}

fn is_json(headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let mime = value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))
}

/// Deserialises a body, mapping errors to `validation_failed` with the
/// failing field when it is known.
pub fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = serde_path_to_error::deserialize::<_, T>(&mut deserializer)
        .map_err(|err| from_path_error(&err))?;
    deserializer
        .end()
        .map_err(|e| ApiError::validation(format!("Request body is not valid JSON: {e}")))?;
    Ok(value)
}

fn from_path_error(err: &serde_path_to_error::Error<serde_json::Error>) -> ApiError {
    let inner = err.inner();
    if inner.is_syntax() || inner.is_eof() {
        return ApiError::validation(format!("Request body is not valid JSON: {inner}"));
    }
    let mut segments = err.path().iter();
    let field = match segments.next() {
        Some(Segment::Map { key }) => Some(key.clone()),
        _ => None,
    };
    let index = match segments.next() {
        Some(Segment::Seq { index }) => Some(*index),
        _ => None,
    };
    // A missing field is reported at the enclosing object, so its name is
    // only in serde's message: "missing field `name`".
    let field = field.or_else(|| missing_field_name(&inner.to_string()));
    let message = match &field {
        Some(_) if err.path().iter().next().is_some() => {
            format!("Invalid value for `{}`: {inner}", err.path())
        }
        _ => format!("Invalid request body: {inner}"),
    };
    let error = ApiError::validation(message);
    match (field, index) {
        (Some(field), Some(index)) => error.field_at(&field, index),
        (Some(field), None) => error.field(&field),
        _ => error,
    }
}

fn missing_field_name(message: &str) -> Option<String> {
    let rest = message.strip_prefix("missing field `")?;
    let end = rest.find('`')?;
    Some(rest[..end].to_owned())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Body {
        name: String,
        kind: Kind,
        paths: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum Kind {
        Music,
    }

    fn details(body: &str) -> Option<Value> {
        let err = parse::<Body>(body.as_bytes()).unwrap_err();
        assert_eq!(err.code(), "validation_failed");
        err.details().map(|d| Value::Object(d.clone()))
    }

    #[test]
    fn valid_body() {
        let body: Body = parse(br#"{"name":"a","kind":"music","paths":["/x"]}"#).unwrap();
        assert_eq!(body.paths, vec!["/x"]);
    }

    #[test]
    fn field_of_wrong_value() {
        assert_eq!(
            details(r#"{"name":"a","kind":"podcast","paths":[]}"#),
            Some(json!({ "field": "kind" }))
        );
        assert_eq!(
            details(r#"{"name":"a","kind":"music","paths":["/x", 5]}"#),
            Some(json!({ "field": "paths", "index": 1 }))
        );
        assert_eq!(
            details(r#"{"name":7,"kind":"music","paths":[]}"#),
            Some(json!({ "field": "name" }))
        );
    }

    #[test]
    fn field_of_missing_value() {
        assert_eq!(
            details(r#"{"name":"a","kind":"music"}"#),
            Some(json!({ "field": "paths" }))
        );
    }

    #[test]
    fn syntax_errors_have_no_field() {
        assert_eq!(details(r#"{"name":"#), None);
        assert_eq!(
            details(r#"{"name":"a","kind":"music","paths":[]} trailing"#),
            None
        );
        assert_eq!(details("[]"), None);
    }

    #[test]
    fn content_type_check() {
        let mut headers = HeaderMap::new();
        assert!(!is_json(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            "application/json; charset=utf-8".parse().unwrap(),
        );
        assert!(is_json(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            "application/vnd.api+json".parse().unwrap(),
        );
        assert!(is_json(&headers));
        headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
        assert!(!is_json(&headers));
    }
}
