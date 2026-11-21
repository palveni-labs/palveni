use crate::routes::xml::escape;
use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::fmt::Write;

pub(super) struct Failure(pub u16, pub &'static str);
pub(super) const MISSING: Failure = Failure(10, "Missing or invalid parameter");
pub(super) const DENIED: Failure = Failure(40, "Wrong username or password");
pub(super) const NOT_FOUND: Failure = Failure(70, "Requested data not found");
pub(super) const UNSUPPORTED: Failure = Failure(0, "This operation is not supported");

impl From<palveni_db::DbError> for Failure {
    fn from(err: palveni_db::DbError) -> Self {
        match err {
            palveni_db::DbError::NotFound(_) => return NOT_FOUND,
            palveni_db::DbError::Invalid(_) => return MISSING,
            palveni_db::DbError::Conflict(_) => {
                return Failure(0, "Data changed; refresh and retry");
            }
            _ => {}
        }
        tracing::error!(error=%err, "Music catalog query failed");
        Failure(0, "Internal server error")
    }
}

fn element(output: &mut String, name: &str, value: &Value) {
    if let Some(entries) = value.as_array() {
        for entry in entries {
            element(output, name, entry);
        }
        return;
    }
    let _ = write!(output, "<{name}");
    if name == "subsonic-response" {
        output.push_str(" xmlns=\"http://subsonic.org/restapi\"");
    }
    if let Some(fields) = value.as_object() {
        for (key, value) in fields {
            if value.is_null()
                || value.is_object()
                || value.is_array()
                || (name == "line" && key == "value")
            {
                continue;
            }
            let value = value
                .as_str()
                .map(escape)
                .unwrap_or_else(|| value.to_string());
            let _ = write!(output, " {key}=\"{value}\"");
        }
        output.push('>');
        if name == "line"
            && let Some(text) = fields.get("value").and_then(Value::as_str)
        {
            output.push_str(&escape(text));
        }
        for (key, value) in fields {
            if value.is_object() || value.is_array() {
                element(output, key, value)
            }
        }
        let _ = write!(output, "</{name}>");
    } else {
        let text = value
            .as_str()
            .map(escape)
            .unwrap_or_else(|| value.to_string());
        let _ = write!(output, ">{text}</{name}>");
    }
}

pub(super) fn reply(as_json: bool, result: Result<Value, Failure>) -> Response {
    let mut value = json!({"version":"1.16.1","type":"Palveni","serverVersion":env!("CARGO_PKG_VERSION"),"openSubsonic":true});
    match result {
        Ok(Value::Object(payload)) => {
            value["status"] = json!("ok");
            for (k, v) in payload {
                value[k] = v;
            }
        }
        Ok(_) => {
            value["status"] = json!("ok");
        }
        Err(Failure(code, message)) => {
            value["status"] = json!("failed");
            value["error"] = json!({"code":code,"message":message});
        }
    }
    let mut response = if as_json {
        Json(json!({"subsonic-response":value})).into_response()
    } else {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        element(&mut xml, "subsonic-response", &value);
        (
            [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            xml,
        )
            .into_response()
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}
