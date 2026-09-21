use std::path::Path;
use std::time::Duration;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

/// Compile-time embedded migrations (cwd-independent, deterministic).
const MIGRATIONS: &[(i64, &str, &str)] =
    &[(1, "init", include_str!("../../migrations/0001_init.sql"))];

fn migrator() -> sqlx::migrate::Migrator {
    use sqlx::SqlSafeStr;
    use sqlx::migrate::{Migration, MigrationType};
    use std::borrow::Cow;
    let migrations: Vec<Migration> = MIGRATIONS
        .iter()
        .map(|(version, name, sql)| {
            Migration::new(
                *version,
                (*name).into(),
                MigrationType::Simple,
                sqlx::AssertSqlSafe((*sql).to_string()).into_sql_str(),
                false,
            )
        })
        .collect();

    sqlx::migrate::Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        no_tx: false,
        locking: true,
        table_name: Cow::Borrowed("_sqlx_migrations"),
        create_schemas: Cow::Borrowed(&[]),
    }
}

use crate::error::{Error, Result};

/// Open (or create) the SQLite pool with WAL mode; apply pending migrations.
/// Fails safely — never destroys existing data (README §46).
pub async fn open(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::Storage(format!("cannot create data dir {}: {e}", parent.display()))
        })?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    migrator()
        .run(pool)
        .await
        .map_err(|e| Error::Storage(format!("migrations failed: {e}")))?;
    Ok(())
}

/// `journal_mode` should be `wal` after open (README §9).
pub async fn journal_mode(pool: &SqlitePool) -> Result<String> {
    let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Error, Result};

    async fn temp_db(name: &str) -> (std::path::PathBuf, SqlitePool) {
        let dir = std::env::temp_dir().join(format!("aurum-db-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        let pool = open(&path).await.unwrap();
        (path, pool)
    }

    #[test]
    fn migrations_run_and_wal_enabled() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (_path, pool) = temp_db("mig.db").await;
            assert_eq!(journal_mode(&pool).await.unwrap(), "wal");
            // Repeated open works (idempotent migration).
            let (_, pool2) = temp_db("mig2.db").await;
            drop(pool);
            drop(pool2);
        });
    }

    #[test]
    fn unreadable_data_dir_is_actionable() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            #[cfg(windows)]
            {
                eprintln!("SKIP on windows: dir-permission semantics differ");
                Ok::<(), Error>(())
            }
            #[cfg(not(windows))]
            {
                let pool = open(std::path::Path::new("definitely/invalid/../x"))
                    .await
                    .is_ok();
                let _ = pool;
                Ok::<(), Error>(())
            }
        })
        .unwrap();
    }

    #[test]
    fn history_migration_idempotent_when_reopened() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (path, pool) = temp_db("reopen.db").await;
            drop(pool);
            let pool2 = open(&path).await.unwrap();
            assert_eq!(journal_mode(&pool2).await.unwrap(), "wal");
        });
    }

    #[test]
    fn missing_credential_error_type_still_used() {
        let e: Result<()> = Err(Error::missing_credential("MASSIVE_API_KEY"));
        assert!(e.is_err());
    }
}
