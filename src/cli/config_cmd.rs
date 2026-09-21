use std::path::PathBuf;

use crate::config::Config;
use crate::error::{Error, Result};

const SCAFFOLD: &str = r#"# Aurum config — created by `aurum init` / `aurum config init`.
# Never put secrets here: inject MASSIVE_API_KEY via environment or .env (gitignored).

[app]
mode = "demo"        # demo | historical | live
symbol = "XAUUSD"
timeframe = "5m"

[server]
host = "127.0.0.1"
port = 8080

[database]
path = "./data/aurum.db"
retention_days = 90

[provider]
name = "massive"

[ui]
refresh_ms = 1000
"#;

pub fn run_show() -> Result<()> {
    let cfg = Config::load()?;
    println!("{}", cfg.redacted_summary());
    Ok(())
}

pub fn run_validate() -> Result<()> {
    let cfg = Config::load()?;
    if let Err(e) = cfg.validate() {
        println!("[INVALID] config: {e}");
        std::process::exit(1);
    }
    if let Err(e) = cfg.validate_credentials(cfg.massive_api_key().as_deref()) {
        println!("[INVALID] credentials: {e}");
        std::process::exit(1);
    }
    println!("[OK] configuration valid (mode={})", cfg.mode.as_str());
    Ok(())
}

pub fn run_init(path: Option<PathBuf>) -> Result<()> {
    let path = path.unwrap_or_else(|| PathBuf::from("config.toml"));
    if path.exists() {
        return Err(Error::config(format!(
            "{} already exists (never silently overwrite)",
            path.display()
        )));
    }
    std::fs::write(&path, SCAFFOLD)
        .map_err(|e| Error::config(format!("cannot write {}: {e}", path.display())))?;
    println!("created {}", path.display());
    Ok(())
}
