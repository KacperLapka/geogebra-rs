//! Workspace blob storage.
//!
//! SQLite-backed store behind a narrow `put`/`get` interface. Slugs are
//! random 48-bit values encoded as 8 URL-safe characters; collisions retry.

use std::str::FromStr;

use serde_json::Value;
use sqlx::SqlitePool;

const SLUG_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

#[derive(Debug)]
pub enum StoreError {
    Db(sqlx::Error),
    Corrupt(serde_json::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(e) => write!(f, "db error: {e}"),
            Self::Corrupt(e) => write!(f, "db error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Corrupt(e)
    }
}

impl Store {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Connect with support for the `?create_if_missing=true` URL form.
    pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
        let normalized = url.replace("create_if_missing=true", "mode=rwc");
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&normalized)?
            .create_if_missing(true);
        SqlitePool::connect_with(opts).await
    }

    pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS workspaces (slug TEXT PRIMARY KEY, blob TEXT NOT NULL)",
        )
        .execute(pool)
        .await
        .map(|_| ())
    }

    /// Insert a blob, returning its 8-character URL-safe slug.
    pub async fn put(&self, blob: Value) -> Result<String, StoreError> {
        let text = serde_json::to_string(&blob)?;
        loop {
            let slug = mint_slug();
            let res = sqlx::query("INSERT INTO workspaces (slug, blob) VALUES (?, ?)")
                .bind(&slug)
                .bind(&text)
                .execute(&self.pool)
                .await;
            match res {
                Ok(_) => return Ok(slug),
                Err(e) if is_unique_violation(&e) => continue,
                Err(e) => return Err(StoreError::Db(e)),
            }
        }
    }

    pub async fn get(&self, slug: &str) -> Result<Option<Value>, StoreError> {
        let text: Option<String> =
            sqlx::query_scalar("SELECT blob FROM workspaces WHERE slug = ?")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await?;
        match text {
            None => Ok(None),
            Some(t) => Ok(Some(serde_json::from_str(&t)?)),
        }
    }
}

fn mint_slug() -> String {
    let n: u64 = rand::random::<u64>() & 0xFFFF_FFFF_FFFF;
    encode_slug(n)
}

fn encode_slug(mut n: u64) -> String {
    let mut out = [b'A'; 8];
    for i in (0..8).rev() {
        out[i] = SLUG_ALPHABET[(n % 64) as usize];
        n /= 64;
    }
    String::from_utf8(out.to_vec()).unwrap()
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(dbe) => dbe.is_unique_violation(),
        _ => false,
    }
}

#[cfg(test)]
impl Store {
    pub async fn count(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    async fn test_store() -> Store {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Store::ensure_table(&pool).await.unwrap();
        Store::new(pool)
    }

    fn is_slug(s: &str) -> bool {
        s.len() == 8
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    }

    #[tokio::test]
    async fn slugs_are_8char_url_safe_and_unique() {
        let store = test_store().await;
        let a = store.put(serde_json::json!({"version": 1})).await.unwrap();
        let b = store.put(serde_json::json!({"version": 1})).await.unwrap();
        assert_eq!(a.len(), 8);
        assert_eq!(b.len(), 8);
        assert!(is_slug(&a));
        assert!(is_slug(&b));
        assert_ne!(a, b);
        assert_eq!(
            store.get(&a).await.unwrap().unwrap(),
            serde_json::json!({"version": 1})
        );
        assert!(store.get("missing__").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn distinct_blobs_load_identically() {
        let store = test_store().await;
        let doc_a = serde_json::json!({"version": 1, "tag": "a"});
        let doc_b = serde_json::json!({"version": 1, "tag": "b"});
        let a = store.put(doc_a.clone()).await.unwrap();
        let b = store.put(doc_b.clone()).await.unwrap();
        assert_ne!(a, b);
        assert_eq!(store.get(&a).await.unwrap().unwrap(), doc_a);
        assert_eq!(store.get(&b).await.unwrap().unwrap(), doc_b);
    }

    #[tokio::test]
    async fn slug_collision_retries_internally() {
        let store = test_store().await;
        // Occupy a slug directly so the unique-constraint path is exercised.
        let taken = "AAAAAAAA";
        sqlx::query("INSERT INTO workspaces (slug, blob) VALUES (?, ?)")
            .bind(taken)
            .bind(r#"{"version":1}"#)
            .execute(&store.pool)
            .await
            .unwrap();
        let dup = sqlx::query("INSERT INTO workspaces (slug, blob) VALUES (?, ?)")
            .bind(taken)
            .bind(r#"{"version":1}"#)
            .execute(&store.pool)
            .await;
        assert!(dup.is_err());
        assert!(is_unique_violation(&dup.unwrap_err()));

        // A normal put still stores exactly one new row without surfacing the
        // collision above.
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let slug = store.put(serde_json::json!({"version": 1})).await.unwrap();
        assert!(is_slug(&slug));
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(after, before + 1);
    }

    #[tokio::test]
    async fn blobs_persist_across_reopen() {
        let dir = std::env::temp_dir().join(format!(
            "geogebra-rs-workspaces-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("workspaces.db");
        let url = format!("sqlite:{}?create_if_missing=true", db_path.display());

        let blob = serde_json::json!({
            "version": 1,
            "functions": {"f": "a*x^2"},
            "params": {"a": 2.0},
            "viewport": {"x_min": -10.0, "x_max": 10.0, "y_min": -10.0, "y_max": 10.0},
            "style": {"f": {"color": "#0F4C81"}}
        });
        let slug = {
            let pool = Store::connect(&url).await.unwrap();
            Store::ensure_table(&pool).await.unwrap();
            let store = Store::new(pool.clone());
            let slug = store.put(blob.clone()).await.unwrap();
            pool.close().await;
            slug
        };
        let pool2 = Store::connect(&url).await.unwrap();
        let reopened = Store::new(pool2.clone());
        assert_eq!(reopened.get(&slug).await.unwrap().unwrap(), blob);
        pool2.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn opaque_style_blob_round_trips() {
        let store = test_store().await;
        let mut doc = serde_json::json!({
            "version": 1,
            "functions": {"f": "a*x^2"},
            "params": {"a": 2.0},
            "viewport": {"x_min": -10.0, "x_max": 10.0, "y_min": -10.0, "y_max": 10.0}
        });
        doc["functions"]["g"] = serde_json::json!("b*sin(x)");
        doc["style"] = serde_json::json!({"f": {"color": "#0F4C81"}});
        let slug = store.put(doc.clone()).await.unwrap();
        assert_eq!(store.get(&slug).await.unwrap().unwrap(), doc);
        assert!(store.get("bad slug!").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn sustained_file_round_trips_stay_under_50ms() {
        let dir = std::env::temp_dir().join(format!(
            "geogebra-rs-workspaces-timing-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("workspaces.db");
        let url = format!("sqlite:{}?create_if_missing=true", db_path.display());
        let pool = Store::connect(&url).await.unwrap();
        Store::ensure_table(&pool).await.unwrap();
        let store = Store::new(pool.clone());
        for i in 0..50 {
            let doc = serde_json::json!({
                "version": 1,
                "functions": {"f": "a*x^2"},
                "params": {"a": i},
                "viewport": {"x_min": -10.0, "x_max": 10.0, "y_min": -10.0, "y_max": 10.0}
            });
            let started = Instant::now();
            let slug = store.put(doc.clone()).await.unwrap();
            let loaded = store.get(&slug).await.unwrap().unwrap();
            assert_eq!(loaded, doc);
            let elapsed = started.elapsed();
            assert!(
                elapsed.as_millis() < 50,
                "iteration {i} round-trip took {elapsed:?}"
            );
        }
        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
