


use std::sync::Arc;
use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use serde_json::Value;
use crate::store::Store;

//checks if the given slug is correct
fn is_slug(s: &str) -> bool {
	s.len() == 8 && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c==b'_')
}

pub async fn save( State(store): State<Arc<Store>>, Json(doc): Json<Value> ) -> Result<(StatusCode, Json<Value>), StatusCode> {
	//just in case it isnt a json check if it is a json
	if !doc.is_object() {
		return Err(StatusCode::BAD_REQUEST);
	}

	let slug = store
		.put(&doc)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

	Ok(	
		( StatusCode::CREATED, Json(serde_json::json!( {"slug": slug} )) )	//so many brackets because its a tuple 
											//idk how to make this more readable tbh 
	)
}


pub async fn load(State(store): State<Arc<Store>>, Path(slug): Path<String>) -> Result<Json<Value>, StatusCode> {
	
	//throw an error if the slug is not valid
	if !is_slug(&slug) {
		return Err(StatusCode::NOT_FOUND);
	}

	//query for the blob
	let blob = store
		.get(&slug)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

	//check what the database returned
	match blob {
		Some(val) => Ok(Json(val)),
		None => Err(StatusCode::NOT_FOUND),
	}

}

















