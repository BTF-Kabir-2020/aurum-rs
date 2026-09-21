use crate::config::Config;
use crate::error::Result;
use crate::state::App;

struct Check {
    name: String,
    ok: bool,
    detail: String,
}

impl Check {
    fn new(name: &str, ok: bool, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            ok,
            detail: detail.to_string(),
        }
    }
}

fn print(check: &Check) {
    if check.ok {
        println!("[OK]   {}", check.name);
    } else {
        println!("[FAIL] {} — {}", check.name, check.detail);
    }
}

/// `aurum doctor` — non-fatal diagnostics, exit code reflects failures.
pub async fn run(config: Config) -> Result<()> {
    println!("AURUM DOCTOR");

    let mut failures = 0usize;

    // Configuration (incl. mode-specific credentials).
    let cfg_ok = config
        .validate_credentials(config.massive_api_key().as_deref())
        .is_ok();
    print(&Check::new(
        "Configuration",
        cfg_ok,
        if cfg_ok {
            ""
        } else {
            "invalid config or missing MASSIVE_API_KEY for historical/live"
        },
    ));
    if !cfg_ok {
        failures += 1;
    }

    let app = match App::from_config(config).await {
        Ok(app) => app,
        Err(e) => {
            println!("[FAIL] Database + migrations — {e}");
            println!("1 check(s) failed");
            std::process::exit(1);
        }
    };

    let counts = (
        crate::storage::count_candles(app.db().await)
            .await
            .unwrap_or(-1),
        crate::storage::count_quotes(app.db().await)
            .await
            .unwrap_or(-1),
        crate::storage::count_signals(app.db().await)
            .await
            .unwrap_or(-1),
    );
    let db_ok = counts.0 >= 0 && counts.1 >= 0 && counts.2 >= 0;
    print(&Check::new("Database + migrations", db_ok, ""));
    if !db_ok {
        failures += 1;
    }

    let db_dir = app
        .config
        .db_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let probe = db_dir.join(".doctor-write-probe");
    let write_ok = std::fs::write(&probe, b"ok").is_ok() && std::fs::remove_file(&probe).is_ok();
    if !write_ok && db_ok {
        // docs/RESILIENCE.md §3: degrade, don't fail. Existing-DB flows keep
        // working; only NEW-file creation is blocked (Windows Docker Desktop
        // bind-mount quirk) — say so and keep doctor exit code OK.
        println!(
            "[WARN] Data directory create-new-file denied — existing DB flows unaffected; run backups from the host (`aurum backup create`) on such setups"
        );
    } else if !write_ok {
        println!("[FAIL] Data directory write permission — the database directory is not usable");
        failures += 1;
    } else {
        print(&Check::new("Data directory write permission", true, ""));
    }

    let b_dir_ok = std::fs::create_dir_all("./backups").is_ok();
    print(&Check::new("Backup directory", b_dir_ok, ""));
    if !b_dir_ok {
        failures += 1;
    }

    let demo = app.is_demo().await;
    let provider_ok = if demo {
        crate::market::ReplayProvider::load_default().is_ok()
    } else {
        app.config.massive_api_key().is_some()
    };
    print(&Check::new(
        "Provider configuration",
        provider_ok,
        if demo {
            ""
        } else {
            "MASSIVE_API_KEY required for historical/live"
        },
    ));
    println!(
        "counts: candles={} quotes={} signals={}",
        counts.0, counts.1, counts.2
    );

    // docs/RESILIENCE.md §3: optional capabilities degrade as [WARN], not failure.
    if app.config.mode == crate::config::Mode::Demo && app.config.massive_api_key().is_none() {
        println!("[WARN] MASSIVE_API_KEY not set — DEMO is unaffected; historical/live disabled");
    }
    app.close_db().await;

    if failures == 0 {
        println!("all checks passed");
    } else {
        println!("{failures} check(s) failed");
        std::process::exit(1);
    }
    Ok(())
}
