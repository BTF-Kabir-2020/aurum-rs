use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("invalid symbol: {0}")]
    Symbol(String),

    #[error("invalid timeframe: {0}")]
    Timeframe(String),

    #[error("invalid mode: {0}")]
    Mode(String),

    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },

    #[error("provider error: {0}")]
    Provider(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("missing credential: {0}")]
    MissingCredential(&'static str),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("toml parse error: {0}")]
    Toml(String),

    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Error {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn missing_credential(name: &'static str) -> Self {
        Self::MissingCredential(name)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_actionable() {
        let e = Error::config("unknown mode");
        assert_eq!(e.to_string(), "configuration error: unknown mode");
    }

    #[test]
    fn missing_credential_names_variable() {
        let e = Error::missing_credential("MASSIVE_API_KEY");
        assert!(e.to_string().contains("MASSIVE_API_KEY"));
    }
}
