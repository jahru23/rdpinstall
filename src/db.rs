use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

pub fn prepare(db_path: &str) -> Result<()> {
    if let Some(dir) = Path::new(db_path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("buat direktori {}", dir.display()))?;
        }
    }
    Ok(())
}

pub async fn connect(db_path: &str) -> Result<sqlx::SqlitePool> {
    let opts = SqliteConnectOptions::from_str(db_path)
        .with_context(|| format!("path database tidak valid: {db_path}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(10));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await?;
    Ok(pool)
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
