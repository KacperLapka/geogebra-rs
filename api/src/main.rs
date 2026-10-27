

mod routes;
mod store;

use std::sync::Arc;
use axum::{Router, routing::get};
use tokio::net::TcpListener;

//local fallback connection
const DEFAULT_DATABASE_URL: &str = "postgres://postgres:password@localhost:5432/my_blob_store";

#[tokio::main]
async fn main() {
	
	//read url from env or use the fallback
	let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
	
	println!("Connecting to database...");
	
	//connect to PostgreSQL
	let pool = store::Store::connect(&db_url).await.expect("Failed to connect to PostgreSQL");
	

	store::Store::ensure_table(&pool).await.expect("Failed to create workspace table");

	//wrapped in Arc (atomically reference counted pointer (thats a long name, truly))
	//so that it can be safely used across threads
	let state = Arc::new(store::Store::new(pool));

	//making the router and mapping http routes to handlers
	let app = Router::new()
		.route("/api/workspaces", axum::routing::post(routes::save))
		.route("/api/workspaces/{slug}", get(routes::load))
		.with_state(state);

	//binding the tcp listener to localhost port: for testing
	let listener = TcpListener::bind("127.0.0.1:3000")
		.await
		.expect("Failed to bind TCP listener:");
	
	println!("Server running at http://127.0.0.1:3000");

	axum::serve(listener, app)
		.await
		.expect("Server failed to run");
}








