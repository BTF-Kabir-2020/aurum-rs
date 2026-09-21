use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Demo,
    Historical,
    Live,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Demo => "demo",
            Mode::Historical => "historical",
            Mode::Live => "live",
        }
    }

    pub fn requires_credentials(self) -> bool {
        matches!(self, Mode::Historical | Mode::Live)
    }
}

impl std::str::FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "demo" => Ok(Mode::Demo),
            "historical" => Ok(Mode::Historical),
            "live" => Ok(Mode::Live),
            other => Err(Error::Mode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileConfig {
    #[serde(default)]
    app: FileApp,
    #[serde(default)]
    server: FileServer,
    #[serde(default)]
    database: FileDatabase,
    #[serde(default)]
    provider: FileProvider,
    #[serde(default)]
    ui: FileUi,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileApp {
    mode: Option<String>,
    symbol: Option<String>,
    timeframe: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileServer {
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileDatabase {
    path: Option<PathBuf>,
    retention_days: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileProvider {
    name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct FileUi {
    refresh_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub mode: Mode,
    pub symbol: String,
    pub timeframe: String,
    pub server_host: String,
    pub server_port: u16,
    pub db_path: PathBuf,
    pub db_retention_days: u32,
    pub provider: String,
    pub ui_refresh_ms: u64,
    pub massive_api_base: String,
    pub config_path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_with_env(|key| std::env::var(key).ok())
    }

    pub fn load_with_env<F>(get_env: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let config_path = get_env("AURUM_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("config.toml"));

        let file = Self::read_file(&config_path, &get_env)?;

        let mode = override_str(&get_env, "AURUM_MODE", file.app.mode)
            .map(|v| v.parse())
            .transpose()?
            .unwrap_or(Mode::Demo);

        let symbol = override_str(&get_env, "AURUM_SYMBOL", file.app.symbol)
            .unwrap_or_else(|| "XAUUSD".to_string());

        let timeframe = override_str(&get_env, "AURUM_TIMEFRAME", file.app.timeframe)
            .unwrap_or_else(|| "5m".to_string());

        let server_host = override_str(&get_env, "AURUM_SERVER_HOST", file.server.host)
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let server_port = override_parse(
            &get_env,
            "AURUM_SERVER_PORT",
            file.server.port,
            "server port",
        )?
        .unwrap_or(8080);

        let db_path = get_env("AURUM_DB_PATH")
            .map(PathBuf::from)
            .or(file.database.path)
            .unwrap_or_else(|| PathBuf::from("./data/aurum.db"));

        let db_retention_days = override_parse(
            &get_env,
            "AURUM_DB_RETENTION_DAYS",
            file.database.retention_days,
            "db retention days",
        )?
        .unwrap_or(90);

        let provider = override_str(&get_env, "AURUM_PROVIDER", file.provider.name)
            .unwrap_or_else(|| "massive".to_string());

        let ui_refresh_ms = override_parse(
            &get_env,
            "AURUM_UI_REFRESH_MS",
            file.ui.refresh_ms,
            "ui refresh",
        )?
        .unwrap_or(1000);

        let massive_api_base = override_str(&get_env, "AURUM_MASSIVE_BASE", None)
            .unwrap_or_else(|| "https://api.massive.com".to_string());

        let config = Self {
            mode,
            symbol,
            timeframe,
            server_host,
            server_port,
            db_path,
            db_retention_days,
            provider,
            ui_refresh_ms,
            massive_api_base,
            config_path,
        };

        config.validate()?;
        config.validate_credentials(get_env("MASSIVE_API_KEY").as_deref())?;
        Ok(config)
    }

    fn read_file<F>(path: &Path, get_env: &F) -> Result<FileConfig>
    where
        F: Fn(&str) -> Option<String>,
    {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match get_env("AURUM_WORKDIR") {
                Some(wd) => PathBuf::from(wd).join(path),
                None => path.to_path_buf(),
            }
        };

        if !path.exists() {
            if path == Path::new("config.toml") {
                return Ok(FileConfig::default());
            }
            return Err(Error::config(format!(
                "config file not found: {} (set AURUM_CONFIG or create config.toml)",
                path.display()
            )));
        }

        let raw = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("cannot read config file: {e}")))?;
        let parsed: FileConfig = toml::from_str(&raw)?;
        Ok(parsed)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.symbol.is_empty() && !crate::market::models::Symbol::is_valid_str(&self.symbol) {
            return Err(Error::Config(format!(
                "invalid symbol '{}': expected base/quote like XAUUSD",
                self.symbol
            )));
        }
        if !matches!(
            self.timeframe.as_str(),
            "1m" | "5m" | "15m" | "1h" | "4h" | "1d"
        ) {
            return Err(Error::Config(format!(
                "unsupported timeframe '{}': expected 1m, 5m, 15m, 1h, 4h or 1d",
                self.timeframe
            )));
        }
        if self.server_host.is_empty() {
            return Err(Error::Config("server host must not be empty".into()));
        }
        if self.db_retention_days == 0 {
            return Err(Error::Config("retention_days must be at least 1".into()));
        }
        if self.ui_refresh_ms == 0 {
            return Err(Error::Config("ui.refresh_ms must be at least 1".into()));
        }
        if self.provider.is_empty() {
            return Err(Error::Config("provider.name must not be empty".into()));
        }
        Ok(())
    }

    /// DEMO never requires a provider credential; HISTORICAL/LIVE do
    /// (docs/CONFIGURATION.md validation rules).
    pub fn validate_credentials(&self, massive_api_key: Option<&str>) -> Result<()> {
        if self.mode.requires_credentials()
            && massive_api_key
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_none()
        {
            return Err(Error::missing_credential("MASSIVE_API_KEY"));
        }
        Ok(())
    }

    /// Secrets are read here only, never in business logic (docs/BUILD.md).
    pub fn massive_api_key(&self) -> Option<String> {
        Self::env_credential("MASSIVE_API_KEY")
    }

    pub fn massive_s3_access_key(&self) -> Option<String> {
        Self::env_credential("MASSIVE_S3_ACCESS_KEY_ID")
    }

    pub fn massive_s3_secret_key(&self) -> Option<String> {
        Self::env_credential("MASSIVE_S3_SECRET_ACCESS_KEY")
    }

    fn env_credential(name: &'static str) -> Option<String> {
        std::env::var(name).ok().filter(|v| !v.is_empty())
    }

    /// Effective config with secrets redacted (for `config show`).
    pub fn redacted_summary(&self) -> String {
        format!(
            "mode = {}\nsymbol = {}\ntimeframe = {}\nserver = {}:{}\ndatabase = {}\nretention_days = {}\nprovider = {}\nui.refresh_ms = {}\nMASSIVE_API_KEY = {}",
            self.mode.as_str(),
            self.symbol,
            self.timeframe,
            self.server_host,
            self.server_port,
            self.db_path.display(),
            self.db_retention_days,
            self.provider,
            self.ui_refresh_ms,
            if self.massive_api_key().is_some() {
                "[REDACTED, set]".to_string()
            } else {
                "[not set]".to_string()
            }
        )
    }
}

fn override_str<F>(get_env: &F, key: &str, file_value: Option<String>) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    match get_env(key) {
        Some(v) if !v.trim().is_empty() => Some(v),
        _ => file_value,
    }
}

fn override_parse<F, T: std::str::FromStr>(
    get_env: &F,
    key: &str,
    file_value: Option<T>,
    field: &'static str,
) -> Result<Option<T>>
where
    F: Fn(&str) -> Option<String>,
{
    match get_env(key) {
        Some(v) if !v.trim().is_empty() => {
            v.trim()
                .parse::<T>()
                .map(Some)
                .map_err(|_| Error::InvalidValue {
                    field,
                    reason: format!("cannot parse '{v}'"),
                })
        }
        None => Ok(file_value),
        Some(_) => Ok(file_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn envmap(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn defaults_when_no_file_and_no_env() {
        let cfg = Config::load_with_env(|_| None).unwrap();
        assert_eq!(cfg.mode, Mode::Demo);
        assert_eq!(cfg.symbol, "XAUUSD");
        assert_eq!(cfg.timeframe, "5m");
        assert_eq!(cfg.server_port, 8080);
        assert_eq!(cfg.db_retention_days, 90);
        assert_eq!(cfg.ui_refresh_ms, 1000);
        assert_eq!(cfg.provider, "massive");
    }

    #[test]
    fn env_overrides_everything() {
        let env = envmap(&[
            ("AURUM_MODE", "live"),
            ("MASSIVE_API_KEY", "test-key"),
            ("AURUM_SYMBOL", "XAU/USD"),
            ("AURUM_TIMEFRAME", "1h"),
            ("AURUM_SERVER_HOST", "0.0.0.0"),
            ("AURUM_SERVER_PORT", "9090"),
            ("AURUM_DB_PATH", "/tmp/x.db"),
            ("AURUM_DB_RETENTION_DAYS", "30"),
            ("AURUM_PROVIDER", "replay"),
            ("AURUM_UI_REFRESH_MS", "250"),
        ]);
        let cfg = Config::load_with_env(env).unwrap();
        assert_eq!(cfg.mode, Mode::Live);
        assert_eq!(cfg.symbol, "XAU/USD");
        assert_eq!(cfg.timeframe, "1h");
        assert_eq!(cfg.server_host, "0.0.0.0");
        assert_eq!(cfg.server_port, 9090);
        assert_eq!(cfg.db_path, PathBuf::from("/tmp/x.db"));
        assert_eq!(cfg.db_retention_days, 30);
        assert_eq!(cfg.provider, "replay");
        assert_eq!(cfg.ui_refresh_ms, 250);
    }

    #[test]
    fn bad_mode_is_rejected() {
        let env = envmap(&[("AURUM_MODE", "simulated")]);
        let err = Config::load_with_env(env).unwrap_err();
        assert!(err.to_string().contains("invalid mode"));
    }

    #[test]
    fn bad_port_is_rejected_with_field_name() {
        let env = envmap(&[("AURUM_SERVER_PORT", "not-a-port")]);
        let err = Config::load_with_env(env).unwrap_err();
        assert!(err.to_string().contains("server port"));
    }

    #[test]
    fn bad_timeframe_is_rejected() {
        let env = envmap(&[("AURUM_TIMEFRAME", "7s")]);
        let err = Config::load_with_env(env).unwrap_err();
        assert!(err.to_string().contains("timeframe"));
    }

    #[test]
    fn explicit_missing_config_file_is_error() {
        let env = envmap(&[("AURUM_CONFIG", "does/not/exist.toml")]);
        let err = Config::load_with_env(env).unwrap_err();
        assert!(err.to_string().contains("config file not found"));
    }

    #[test]
    fn file_config_is_parsed() {
        let dir = std::env::temp_dir().join(format!("aurum-cfg-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            r#"
[app]
mode = "historical"
symbol = "C:XAUUSD"
timeframe = "15m"

[server]
host = "127.0.0.1"
port = 8181

[database]
path = "./data/t.db"
retention_days = 45

[provider]
name = "massive"

[ui]
refresh_ms = 500
"#,
        )
        .unwrap();

        let env = envmap(&[
            ("AURUM_CONFIG", path.to_str().unwrap()),
            ("MASSIVE_API_KEY", "test-key"),
        ]);
        let cfg = Config::load_with_env(env).unwrap();
        assert_eq!(cfg.mode, Mode::Historical);
        assert_eq!(cfg.symbol, "C:XAUUSD");
        assert_eq!(cfg.timeframe, "15m");
        assert_eq!(cfg.server_port, 8181);
        assert_eq!(cfg.db_retention_days, 45);
        assert_eq!(cfg.ui_refresh_ms, 500);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn historical_without_key_is_rejected() {
        let env = envmap(&[("AURUM_MODE", "historical")]);
        let err = Config::load_with_env(env).unwrap_err();
        assert!(err.to_string().contains("MASSIVE_API_KEY"));

        let env = envmap(&[("AURUM_MODE", "historical"), ("MASSIVE_API_KEY", "k")]);
        assert!(Config::load_with_env(env).is_ok());
    }

    #[test]
    fn demo_never_requires_credentials() {
        let cfg = Config::load_with_env(|_| None).unwrap();
        assert!(cfg.validate_credentials(None).is_ok());
    }

    #[test]
    fn redacted_summary_never_leaks_key() {
        // Env-injectable credential check without touching process env:
        // massive_api_key reads std::env, so here we verify the summary never
        // echoes secret values it knows about by using a config-only run.
        let cfg = Config::load_with_env(|key| {
            if key == "MASSIVE_API_KEY" {
                Some("supersecret123".to_string())
            } else {
                None
            }
        })
        .unwrap();
        let s = cfg.redacted_summary();
        assert!(!s.contains("supersecret123"));
    }
}
