//! Axum JSON blob store (AD-6).
//!
//! Exactly two routes: `POST /api/workspaces` (stores a JSON blob, returns
//! an 8-character URL-safe slug) and `GET /api/workspaces/:slug`.
//! Payloads are hard-limited to 64 KB. No math evaluation, no auth, no
//! mutation endpoints. Persistent backing (sqlx) lands with Epic 5; the
//! tracer keeps an in-memory store behind the same interface.

mod routes;
mod store;

use std::sync::Arc;

use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let state = Arc::new(store::Store::new());
    let app = Router::new()
        .route("/api/workspaces", axum::routing::post(routes::save))
        .route("/api/workspaces/:slug", get(routes::load))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
