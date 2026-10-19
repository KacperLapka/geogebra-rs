//! Route handlers: save/load workspace blobs.
//!
//! Tracer contract (5.1): `POST /api/workspaces` accepts the flat
//! `serialize_workspace()` document verbatim
//! (`{version:1, functions, params, viewport, style?}`), stores it whole,
//! and returns `{slug}`. `GET` returns the identical blob.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::Value;

use crate::store::Store;

/// Hard payload limit enforced before parsing (AD-6): 64 KB.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

fn is_slug(s: &str) -> bool {
    s.len() == 8
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

pub async fn save(State(store): State<Arc<Store>>, body: axum::body::Bytes) -> Response {
    let started = Instant::now();
    if body.len() > MAX_BODY_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "workspace exceeds 64 KB").into_response();
    }
    let parsed: Result<Value, _> = serde_json::from_slice(&body);
    let Ok(doc) = parsed else {
        return (StatusCode::BAD_REQUEST, "invalid JSON").into_response();
    };
    let Some(obj) = doc.as_object() else {
        return (StatusCode::BAD_REQUEST, "invalid workspace").into_response();
    };
    match obj.get("version").and_then(|v| v.as_u64()) {
        Some(1) => {}
        _ => return (StatusCode::BAD_REQUEST, "missing version").into_response(),
    }
    for key in ["functions", "params", "viewport"] {
        if obj.get(key).is_none() {
            return (StatusCode::BAD_REQUEST, "invalid workspace").into_response();
        }
    }
    if let Some(viewport) = obj.get("viewport").and_then(|v| v.as_object()) {
        for key in ["x_min", "x_max", "y_min", "y_max"] {
            if !viewport.get(key).is_some_and(|v| v.is_number()) {
                return (StatusCode::BAD_REQUEST, "invalid workspace").into_response();
            }
        }
    } else {
        return (StatusCode::BAD_REQUEST, "invalid workspace").into_response();
    }
    let slug = match store.put(doc).await {
        Ok(slug) => slug,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response(),
    };
    debug_assert!(is_slug(&slug));
    debug_assert!(started.elapsed().as_millis() < 50);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "slug": slug })),
    )
        .into_response()
}

pub async fn load(State(store): State<Arc<Store>>, Path(slug): Path<String>) -> Response {
    if !is_slug(&slug) {
        return (StatusCode::NOT_FOUND, "unknown workspace").into_response();
    }
    match store.get(&slug).await {
        Ok(Some(blob)) => (StatusCode::OK, Json(blob)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "unknown workspace").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    fn tracer_doc() -> Value {
        serde_json::json!({
            "version": 1,
            "functions": {"f": "a*x^2"},
            "params": {"a": 2.0},
            "viewport": {"x_min": -10.0, "x_max": 10.0, "y_min": -10.0, "y_max": 10.0}
        })
    }

    async fn test_store() -> Arc<Store> {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        Store::ensure_table(&pool).await.unwrap();
        Arc::new(Store::new(pool))
    }

    async fn body_json(resp: Response) -> (StatusCode, Value) {
        let (mut parts, body) = resp.into_parts();
        let status = parts.status;
        let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
        let value: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::String(
                String::from_utf8_lossy(&bytes).to_string(),
            ))
        };
        let _ = &mut parts;
        (status, value)
    }

    async fn status_text(resp: Response) -> (StatusCode, String) {
        let (parts, body) = resp.into_parts();
        let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
        (
            parts.status,
            String::from_utf8_lossy(&bytes).to_string(),
        )
    }

    #[tokio::test]
    async fn save_then_load_round_trips_identical_blob() {
        let store = test_store().await;
        let doc = tracer_doc();
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        let (status, saved) = body_json(save(State(store.clone()), bytes).await).await;
        assert_eq!(status, StatusCode::CREATED);
        let slug = saved["slug"].as_str().unwrap().to_string();
        assert!(is_slug(&slug));

        let (status, loaded) = body_json(load(State(store), Path(slug)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(loaded, doc);
    }

    #[tokio::test]
    async fn styled_blob_round_trips_opaque() {
        let store = test_store().await;
        let mut doc = tracer_doc();
        doc["functions"]["g"] = serde_json::json!("b*sin(x)");
        doc["style"] = serde_json::json!({"f": {"color": "#0F4C81"}});
        assert!(serde_json::to_string(&doc).unwrap().len() < 10 * 1024);
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        let (_, saved) = body_json(save(State(store.clone()), bytes).await).await;
        let slug = saved["slug"].as_str().unwrap().to_string();
        let (status, loaded) = body_json(load(State(store), Path(slug)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(loaded, doc);
    }

    #[tokio::test]
    async fn unknown_and_malformed_slugs_are_404() {
        let store = test_store().await;
        let (status, text) = status_text(load(State(store.clone()), Path("AAAAAAAA".to_string())).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(text, "unknown workspace");
        for bad in ["short", "way-too-long-slug", "bad slug!", ""] {
            let (status, text) =
                status_text(load(State(store.clone()), Path(bad.to_string())).await).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "slug={bad}");
            assert_eq!(text, "unknown workspace", "slug={bad}");
        }
    }

    #[tokio::test]
    async fn oversize_payload_is_413() {
        let store = test_store().await;
        let big = vec![b'x'; MAX_BODY_BYTES + 1];
        let (status, text) =
            status_text(save(State(store.clone()), axum::body::Bytes::from(big)).await).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(text, "workspace exceeds 64 KB");
        assert_eq!(store.count().await, 0);
    }

    #[tokio::test]
    async fn exactly_max_body_is_accepted() {
        let store = test_store().await;
        let mut doc = tracer_doc();
        let base_len = serde_json::to_vec(&doc).unwrap().len();
        let pad = MAX_BODY_BYTES - base_len;
        doc["functions"]["f"] = serde_json::json!(format!("a*x^2{}", "x".repeat(pad)));
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        assert_eq!(bytes.len(), MAX_BODY_BYTES);
        let (status, _) = body_json(save(State(store), bytes).await).await;
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn db_failure_is_500() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        Store::ensure_table(&pool).await.unwrap();
        pool.close().await;
        let store = Arc::new(Store::new(pool));
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&tracer_doc()).unwrap());
        let (status, text) = status_text(save(State(store.clone()), bytes).await).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(text, "db error");
        let (status, text) =
            status_text(load(State(store), Path("AAAAAAAA".to_string())).await).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(text, "db error");
    }

    #[tokio::test]
    async fn invalid_documents_are_400() {
        let store = test_store().await;
        // invalid JSON
        let (status, text) = status_text(
            save(State(store.clone()), axum::body::Bytes::from("{oops")).await,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(text, "invalid JSON");
        // missing version
        let mut doc = tracer_doc();
        doc.as_object_mut().unwrap().remove("version");
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        let (status, text) = status_text(save(State(store.clone()), bytes).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(text, "missing version");
        // version 0
        let mut doc = tracer_doc();
        doc["version"] = serde_json::json!(0);
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        let (status, _) = status_text(save(State(store.clone()), bytes).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // missing viewport key
        let mut doc = tracer_doc();
        doc.as_object_mut().unwrap().remove("viewport");
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&doc).unwrap());
        let (status, _) = status_text(save(State(store.clone()), bytes).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(store.count().await, 0);
    }
}
