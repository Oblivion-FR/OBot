use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

/// Opens the database, creating it if needed, and applies the migrations built into the binary
pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// A fresh database for tests. One connection that never closes: each connection to
/// `:memory:` gets its own empty database.
#[cfg(test)]
pub async fn connect_in_memory() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory database opens");
    sqlx::migrate!().run(&pool).await.expect("migrations apply");
    pool
}
