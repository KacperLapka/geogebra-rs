//! Route handlers: save/load workspace blobs.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value;

use crate::store::Store;

/// Hard payload limit enforced before parsing (AD-6): 64 KB.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
pub struct SaveBody {
    pub version: u32,
    pub workspace: Value,
}

pub async fn save(
    State(store): State<Arc<Store>>,
    body: axum::body::Bytes,
) -> Response {
    if body.len() > MAX_BODY_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "workspace exceeds 64 KB").into_response();
    }
    let parsed: Result<SaveBody, _> = serde_json::from_slice(&body);
    let Ok(parsed) = parsed else {
        return (StatusCode::BAD_REQUEST, "invalid JSON").into_response();
    };
    if parsed.version == 0 {
        return (StatusCode::BAD_REQUEST, "missing version").into_response();
    }
    let slug = store.put(parsed.workspace);
    (StatusCode::CREATED, Json(serde_json::json!({ "slug": slug }))).into_response()
}

pub async fn load(State(store): State<Arc<Store>>, Path(slug): Path<String>) -> Response {
    match store.get(&slug) {
        Some(blob) => (StatusCode::OK, Json(blob)).into_response(),
        None => (StatusCode::NOT_FOUND, "unknown workspace").into_response(),
    }
}
