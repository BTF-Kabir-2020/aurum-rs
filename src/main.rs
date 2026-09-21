// Modules live in src/lib.rs (used by binary, integration tests and docs examples).

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;

use aurum::cli::{backup as backup_cmd, config_cmd, doctor, market};
use aurum::config::Config;

const BANNER: &str = "Aurum — Financial Market Intelligence Engine";

#[derive(Parser)]
#[command(name = "aurum", version, about = BANNER)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the deterministic offline DEMO replay (fixture → rules, no credentials).
    Demo {
        /// Playback speed 1..10 (higher = faster frames, README §30).
        #[arg(long, short, default_value_t = 2)]
        speed: u8,
    },
    /// Diagnostics: config, database, migrations, provider, permissions.
    Doctor,
    /// Create a default config.toml scaffold (never overwrites).
    Init,
    /// Inspect / validate configuration.
    #[command(subcommand, name = "config")]
    Config(ConfigCommand),
    /// Current quote for a symbol (Massive; offline uses demo fixture).
    Quote {
        /// Symbol: XAUUSD, XAU/USD or C:XAUUSD
        symbol: String,
        /// Force offline DEMO provider.
        #[arg(long)]
        offline: bool,
    },
    /// Fetch and store candle history for a symbol.
    History {
        symbol: String,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        offline: bool,
    },
    /// Rule-based signal (RSI/EMA/momentum) for a symbol, as JSON.
    Signal {
        symbol: String,
        #[arg(long)]
        offline: bool,
    },
    /// Poll the signal in a plain-text loop until Ctrl+C (TUI lands in Phase 12).
    Watch {
        symbol: String,
        #[arg(long)]
        offline: bool,
    },
    /// Start the HTTP API server.
    Server {
        /// Bind address override; default comes from config/env
        /// [env: AURUM_SERVER_HOST + AURUM_SERVER_PORT]
        #[arg(long)]
        bind: Option<SocketAddr>,
    },
    /// Backup management.
    #[command(subcommand, name = "backup")]
    Backup(BackupCommand),
    /// Print version information.
    Version,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print effective config with secrets redacted.
    Show,
    /// Report exactly what is missing or invalid; never prints secrets.
    Validate,
    /// Create a default config.toml scaffold.
    Init {
        /// Where to write; defaults to ./config.toml
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BackupCommand {
    /// Create a consistent backup of the active database.
    Create {
        /// Destination file (never overwritten).
        #[arg(short, long, default_value = "backups/backup.db")]
        output: PathBuf,
    },
    /// Verify a backup file (readability, integrity, schema).
    Verify { file: PathBuf },
    /// Restore from a backup (keeps a safety copy of the live DB).
    Restore { file: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Command::Version) => println!("aurum {}", env!("CARGO_PKG_VERSION")),
        Some(Command::Demo { speed }) => market::run_demo(Config::load()?, speed).await?,
        Some(Command::Doctor) => doctor::run(Config::load()?).await?,
        Some(Command::Init) => config_cmd::run_init(Some(PathBuf::from("config.toml")))?,
        Some(Command::Config(any)) => ConfigAction { sub: any }.run()?,
        Some(Command::Quote { symbol, offline }) => {
            market::run_quote(Config::load()?, &symbol, offline).await?
        }
        Some(Command::History {
            symbol,
            limit,
            offline,
        }) => market::run_history(Config::load()?, &symbol, limit, offline).await?,
        Some(Command::Signal { symbol, offline }) => {
            market::run_signal(Config::load()?, &symbol, offline).await?
        }
        Some(Command::Watch { symbol, offline }) => {
            aurum::tui::app::run(Config::load()?, &symbol, offline).await?
        }
        Some(Command::Server { bind }) => {
            let config = Config::load()?;
            let bind = bind.unwrap_or_else(|| {
                SocketAddr::new(
                    config
                        .server_host
                        .parse()
                        .unwrap_or(std::net::IpAddr::from([127, 0, 0, 1])),
                    config.server_port,
                )
            });
            run_server(bind).await?
        }
        Some(Command::Backup(any)) => match any {
            BackupCommand::Create { output } => {
                backup_cmd::run_create(Config::load()?, output).await?
            }
            BackupCommand::Verify { file } => backup_cmd::run_verify(file).await?,
            BackupCommand::Restore { file } => backup_cmd::run_restore(file).await?,
        },
        None => println!("{BANNER}"),
    }
    Ok(())
}

impl ConfigAction {
    fn run(self) -> aurum::error::Result<()> {
        match self.sub {
            ConfigCommand::Show => config_cmd::run_show(),
            ConfigCommand::Validate => config_cmd::run_validate(),
            ConfigCommand::Init { path } => config_cmd::run_init(path),
        }
    }
}

struct ConfigAction {
    sub: ConfigCommand,
}

async fn run_server(bind: SocketAddr) -> anyhow::Result<()> {
    let config = Config::load()?;
    let app_state: std::sync::Arc<aurum::state::App> =
        std::sync::Arc::new(aurum::state::App::from_config(config.clone()).await?);
    let router = aurum::api::routes::router(app_state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(
        "listening on {bind} (mode={}) — /api/v1/* live",
        config.mode.as_str()
    );
    // Graceful shutdown (docs/RESILIENCE.md §4): SIGINT/SIGTERM → stop
    // accepting requests → DB pool closed → exit 0.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received — flushing and closing");
}
