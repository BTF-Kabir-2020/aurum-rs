use std::path::Path;

use sqlx::SqlitePool;

use crate::error::{Error, Result};

/// Consistent online backup via SQLite `VACUUM INTO` (README §12):
/// atomic destination creation, no corruption, no silent overwrite.
pub async fn create(db: &SqlitePool, destination: &Path) -> Result<std::fs::Metadata> {
    if destination.exists() {
        return Err(Error::Storage(format!(
            "refusing to overwrite existing backup: {}",
            destination.display()
        )));
    }
    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Storage(format!("cannot create backup dir: {e}")))?;
    }

    let dest_str = destination.to_string_lossy().to_string();
    if dest_str.contains('\'') {
        return Err(Error::Storage("backup path must not contain quotes".into()));
    }

    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "VACUUM INTO '{}'",
        dest_str.replace('\\', "/")
    )))
    .execute(db)
    .await
    .map_err(|e| Error::Storage(format!("backup failed: {e}")))?;

    std::fs::metadata(destination).map_err(|e| Error::Storage(format!("backup missing: {e}")))
}

/// Verify a backup file: readable as SQLite, integrity ok, contains our schema
/// tables. Never executes unknown SQL from the file; opens read-only.
pub async fn verify(path: &Path) -> Result<()> {
    let uri = format!(
        "sqlite:{}?mode=ro",
        path.to_string_lossy().replace('\\', "/")
    );
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&uri)
        .await
        .map_err(|e| Error::Storage(format!("backup unreadable: {e}")))?;

    let result = async {
        let (mode,): (String,) = sqlx::query_as("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await?;
        if mode != "ok" {
            return Err(Error::Storage(format!("integrity_check failed: {mode}")));
        }
        for table in ["candles", "quotes", "signals", "app_metadata"] {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&pool)
            .await?;
            if n == 0 {
                return Err(Error::Storage(format!("backup missing table '{table}'")));
            }
        }
        Ok::<(), crate::error::Error>(())
    }
    .await;

    pool.close().await;
    result
}

/// Restore (README §12): validate backup → integrity → safety copy of the
/// current DB → replace → reopen + migrations validated by reopen.
pub async fn restore(pool: &SqlitePool, backup: &Path, live_db_path: &Path) -> Result<()> {
    verify(backup).await?;

    if live_db_path.exists() {
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let safety = live_db_path.with_extension(format!("safety-{stamp}.db"));
        std::fs::copy(live_db_path, &safety).map_err(|e| {
            Error::Storage(format!("cannot copy safety file {}: {e}", safety.display()))
        })?;
        tracing::info!("safety copy: {}", safety.display());
    }

    pool.close().await;
    std::fs::copy(backup, live_db_path)
        .map_err(|e| Error::Storage(format!("restore copy failed: {e}")))?;

    let reopened = crate::storage::db::open(live_db_path).await?;
    reopened.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::models::Quote;
    use crate::market::models::Symbol;
    use crate::market::{Candle, Timeframe};

    async fn setup(name: &str) -> (std::path::PathBuf, SqlitePool) {
        let dir = std::env::temp_dir().join(format!("aurum-bck-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(dir.join(format!("{name}.bk")));
        let pool = crate::storage::db::open(&path).await.unwrap();
        (path, pool)
    }

    fn dummy_quote() -> Quote {
        Quote::new(
            Symbol::normalize("XAUUSD").unwrap(),
            chrono::DateTime::from_timestamp_millis(42_000).unwrap(),
            10.0,
            10.3,
            "demo",
        )
    }

    #[tokio::test]
    async fn backup_create_verify_restore_roundtrip() {
        let (live_path, pool) = setup("live.db").await;
        crate::storage::save_quote(&pool, &dummy_quote())
            .await
            .unwrap();
        let before = crate::storage::count_quotes(&pool).await.unwrap();
        assert_eq!(before, 1);

        let backup = live_path.with_extension("bk");
        create(&pool, &backup).await.unwrap();
        assert!(backup.exists());
        verify(&backup).await.unwrap();

        // Diverge the live DB with a newer quote (different timestamp).
        let newer = Quote::new(
            Symbol::normalize("XAUUSD").unwrap(),
            chrono::DateTime::from_timestamp_millis(99_000).unwrap(),
            20.0,
            20.3,
            "demo",
        );
        crate::storage::save_quote(&pool, &newer).await.unwrap();
        assert_eq!(crate::storage::count_quotes(&pool).await.unwrap(), 2);

        // Restore brings back exactly the backed-up snapshot.
        restore(&pool, &backup, &live_path).await.unwrap();
        let reopened = crate::storage::db::open(&live_path).await.unwrap();
        assert_eq!(
            crate::storage::count_quotes(&reopened).await.unwrap(),
            before,
            "restore returns backup content, not the diverged live data"
        );
        reopened.close().await;
        pool.close().await;
    }

    #[tokio::test]
    async fn backup_refuses_overwrite() {
        let (live_path, pool) = setup("ovw.db").await;
        let backup = live_path.with_extension("bk");
        create(&pool, &backup).await.unwrap();
        let err = create(&pool, &backup).await.unwrap_err();
        assert!(err.to_string().contains("overwrite"));
    }

    #[tokio::test]
    async fn verify_rejects_garbage_file() {
        let dir = std::env::temp_dir().join(format!("aurum-bck-garbage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let garbage = dir.join(format!(
            "garbage-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&garbage, b"this is not a sqlite file at all").unwrap();
        assert!(
            verify(&garbage).await.is_err(),
            "garbage file must not verify"
        );
    }

    #[tokio::test]
    async fn candles_saved_and_counted() {
        let (_p, pool) = setup("cnd.db").await;
        let c = Candle {
            symbol: Symbol::normalize("XAUUSD").unwrap(),
            timestamp: chrono::DateTime::from_timestamp_millis(9_000).unwrap(),
            open: 3600.0,
            high: 3610.0,
            low: 3590.0,
            close: 3605.0,
            volume: 12.0,
            timeframe: Timeframe::M5,
            source: "replay".into(),
        };
        crate::storage::save_candles(&pool, &[c]).await.unwrap();
        assert_eq!(crate::storage::count_candles(&pool).await.unwrap(), 1);
    }
}
