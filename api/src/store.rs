

use serde_json::Value;
use sqlx::PgPool;

//web compliant alphabet 
const SLUG_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Clone)]
pub struct Store {
	pool: PgPool,
}

//unifying errors into one so they can be handled easily
#[derive(Debug)]
pub enum StoreError { 
	Db(sqlx::Error),
	Corrupt(serde_json::Error),
	Migration(sqlx::migrate::MigrateError),
}

impl std::fmt::Display for StoreError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Db(e) => write!(f, "db error, {e}"),
			Self::Corrupt(e) => write!(f, "json error, {e}"),
			Self::Migration(e) => write!(f, "migration error, {e}"),
		}
	}
}

impl std::error::Error for StoreError {}


//automatically converts the error into the custom enum
impl From<sqlx::Error> for StoreError {
	fn from(e: sqlx::Error) -> Self {
		Self::Db(e)
	}
}


//automatically converts the error into the custom enum
impl From<serde_json::Error> for StoreError {
	fn from(e: serde_json::Error) -> Self {
		Self::Corrupt(e)
	}
}


//automatically converts the error into the custom enum
impl From<sqlx::migrate::MigrateError> for StoreError {
	fn from(e: sqlx::migrate::MigrateError) -> Self {
		Self::Migration(e)
	}
}


impl Store {
	

	pub fn new(pool:PgPool) -> Self {
		Self {pool}
	}
	
	//connects to PostreSQL using url
	pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
		PgPool::connect(url).await
	}

	//ensures that the table exists inside the db
	pub async fn ensure_table(pool: &PgPool) -> Result<(), StoreError> {
		sqlx::migrate!("./migrations")
			.run(pool)
			.await?;
		Ok(())
	}

	//turns json into text and inserts it with a random slug
	//if the slug exists retries with another one
	pub async fn put(&self, blob: &Value) -> Result<String, StoreError> {
		let text = serde_json::to_string(blob)?;
		loop {
			let slug = mint_slug();
			
			let res = sqlx::query("INSERT INTO workspaces (slug, blob) VALUES ($1, $2)")
				.bind(&slug)
				.bind(&text)
				.execute(&self.pool)
				.await;

			match res {
				Ok(_) => return Ok(slug),
				Err(e) if is_unique_violation(&e) => continue, 	// collision, retry making the slug
				Err(e) => return Err(StoreError::Db(e)),	// an actual error in the database
			}
		}
	}

	//sends the query matching the given slug
	pub async fn get(&self, slug: &str) -> Result<Option<Value>, StoreError> {
	
		let text: Option<String> = sqlx::query_scalar("SELECT blob FROM workspaces WHERE slug = $1")
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
	let n: u64 = rand::random::<u64>() & 0xFFFF_FFFF_FFFF;	//mask the lower 48 bits
	let mut out = [b'A'; 8];
	let mut val = n;
	for i in (0..8).rev() {
		out[i] = SLUG_ALPHABET[(val % 64) as usize];
		val /= 64;
	}
	String::from_utf8(out.to_vec()).unwrap()
}


fn is_unique_violation(e: &sqlx::Error) -> bool {
	match e {
		sqlx::Error::Database(dbe) => dbe.code().is_some_and(|code| code == "23505"),
		_ => false
	}
}

