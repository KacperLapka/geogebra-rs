//! Workspace blob storage.
//!
//! In-memory map behind a narrow interface; Epic 5 swaps the body for a
//! sqlx-backed store without touching routes.

use std::collections::HashMap;
use std::sync::{Mutex, atomic::{AtomicU64, Ordering}};

const SLUG_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub struct Store {
    blobs: Mutex<HashMap<String, serde_json::Value>>,
    counter: AtomicU64,
}

impl Store {
    pub fn new() -> Self {
        Self {
            blobs: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(1),
        }
    }

    /// Insert a blob, returning its 8-character URL-safe slug.
    pub fn put(&self, blob: serde_json::Value) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let slug = encode_slug(n);
        self.blobs.lock().unwrap().insert(slug.clone(), blob);
        slug
    }

    pub fn get(&self, slug: &str) -> Option<serde_json::Value> {
        self.blobs.lock().unwrap().get(slug).cloned()
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_slug(mut n: u64) -> String {
    let mut out = [b'A'; 8];
    for i in (0..8).rev() {
        out[i] = SLUG_ALPHABET[(n % 64) as usize];
        n /= 64;
    }
    String::from_utf8(out.to_vec()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_8char_url_safe_and_unique() {
        let store = Store::new();
        let a = store.put(serde_json::json!({"version": 1}));
        let b = store.put(serde_json::json!({"version": 1}));
        assert_eq!(a.len(), 8);
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_eq!(store.get(&a).unwrap(), serde_json::json!({"version": 1}));
        assert!(store.get("missing__").is_none());
    }
}
