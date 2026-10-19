//! Axum JSON blob store (AD-6).
//!
//! Exactly two routes: `POST /api/workspaces` (stores a JSON blob, returns
//! an 8-character URL-safe slug) and `GET /api/workspaces/{slug}`.
//! Payloads are hard-limited to 64 KB. No math evaluation, no auth, no
//! mutation endpoints. Backed by a SQLite file via sqlx.

mod routes;
mod store;

use std::sync::Arc;

use axum::{Router, routing::get};

const DEFAULT_DATABASE_URL: &str = "sqlite:data/workspaces.db?create_if_missing=true";

#[tokio::main]
async fn main() {
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    ensure_parent_dir(&db_url);
    let pool = store::Store::connect(&db_url).await.unwrap();
    store::Store::ensure_table(&pool).await.unwrap();
    let state = Arc::new(store::Store::new(pool));
    let app = Router::new()
        .route("/api/workspaces", axum::routing::post(routes::save))
        .route("/api/workspaces/{slug}", get(routes::load))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn ensure_parent_dir(db_url: &str) {
    let Some(path) = db_url.strip_prefix("sqlite:") else {
        return;
    };
    let path = path.split('?').next().unwrap_or(path);
    if path == ":memory:" || path.is_empty() {
        return;
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create database directory");
        }
    }
}
