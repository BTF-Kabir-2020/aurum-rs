use std::path::PathBuf;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::state::App;

/// `aurum backup create` — consistent VACUUM INTO snapshot (README §12).
pub async fn run_create(_config: Config, output: PathBuf) -> Result<()> {
    let config = Config::load()?;
    let app = App::from_config(config).await?;
    let meta = crate::storage::backup::create(app.db().await, &output).await?;
    println!("backup created: {} size={}B", output.display(), meta.len());
    app.close_db().await;
    Ok(())
}

/// `aurum backup verify <file>` — readability + integrity + schema keys.
pub async fn run_verify(path: PathBuf) -> Result<()> {
    if !path.exists() {
        return Err(Error::Storage(format!(
            "backup not found: {}",
            path.display()
        )));
    }
    crate::storage::backup::verify(&path).await?;
    println!("[OK] backup valid: {}", path.display());
    Ok(())
}

/// `aurum backup restore <file>` — verify → safety copy → replace → reopen.
pub async fn run_restore(path: PathBuf) -> Result<()> {
    if !path.exists() {
        return Err(Error::Storage(format!(
            "backup not found: {}",
            path.display()
        )));
    }
    let config = Config::load()?;
    let app = App::from_config(config).await?;
    let db_path = app.config.db_path.clone();
    crate::storage::backup::restore(app.db().await, &path, &db_path).await?;
    println!("restored {} → {}", path.display(), db_path.display());
    Ok(())
}
