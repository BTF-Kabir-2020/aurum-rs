use std::io::{Stdout, stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEventKind, poll};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::config::Config;
use crate::error::Result;
use crate::market::models::Symbol;
use crate::state::App;
use crate::tui::ui::Snapshot;

/// Run the live TUI for `aurum watch` until 'q' or Ctrl+C (README §29).
/// Low on CPU: blocking poll with the configured refresh interval,
/// no busy loops, terminal restored on all exit paths (README §49).
pub async fn run(config: Config, symbol_str: &str, offline: bool) -> Result<()> {
    let symbol = Symbol::normalize(symbol_str)?;
    let mut config = config;
    if offline {
        config.mode = crate::config::Mode::Demo;
    }
    let app: Arc<App> = Arc::new(App::from_config(config).await?);

    let refresh = Duration::from_millis(app.config.ui_refresh_ms.max(250));
    let mut terminal = setup_terminal()?;

    let result = event_loop(&app, &symbol, refresh, &mut terminal).await;

    teardown_terminal(&mut terminal)?;
    app.close_db().await;
    result
}

async fn event_loop(
    app: &App,
    symbol: &Symbol,
    refresh: Duration,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    let start = Instant::now();
    loop {
        // Compute once per tick (persistence included, like the text loop).
        let signal = app.compute_signal(symbol).await;
        let age_secs = start.elapsed().as_secs();
        let snapshot = match signal {
            Ok(sig) => Snapshot {
                mode: app.config.mode.as_str().to_string(),
                source: if app.is_demo().await {
                    "fixture"
                } else {
                    "massive"
                }
                .to_string(),
                symbol: symbol.display(),
                price: sig.price,
                ema20: sig.ema20,
                rsi14: sig.rsi14,
                momentum: sig.momentum,
                action: action_text(sig.action).to_string(),
                strength: sig.strength,
                max_strength: sig.max_strength,
                updated_local: chrono::Local::now().format("%H:%M:%S").to_string(),
                data_age_secs: age_secs,
            },
            Err(e) => Snapshot {
                mode: app.config.mode.as_str().to_string(),
                source: "error".to_string(),
                symbol: symbol.display(),
                price: 0.0,
                ema20: 0.0,
                rsi14: 0.0,
                momentum: 0.0,
                action: "HOLD".into(),
                strength: 0,
                max_strength: 0,
                updated_local: e.to_string(),
                data_age_secs: 0,
            },
        };

        let _ = terminal.draw(|frame| crate::tui::ui::render_snapshot(frame, &snapshot));

        // Event poll with a bound; Ctrl+C / 'q' exit cleanly.
        if poll(refresh)
            .map_err(|e| crate::error::Error::Io(std::io::Error::other(e.to_string())))?
            && let Ok(Event::Key(key)) = crossterm::event::read()
            && key.kind == KeyEventKind::Press
            && (matches!(key.code, KeyCode::Char('q'))
                || matches!(key.code, KeyCode::Char('c')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)))
        {
            return Ok(());
        }
    }
}

fn action_text(action: crate::signal::Action) -> &'static str {
    match action {
        crate::signal::Action::Buy => "BUY",
        crate::signal::Action::Sell => "SELL",
        crate::signal::Action::Hold => "HOLD",
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().map_err(tui_io)?;
    let mut out = stdout();
    crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen).map_err(tui_io)?;
    let backend = CrosstermBackend::new(out);
    Terminal::new(backend).map_err(tui_io)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )
    .map_err(tui_io)?;
    disable_raw_mode().map_err(tui_io)?;
    Ok(())
}

fn tui_io(e: std::io::Error) -> crate::error::Error {
    crate::error::Error::Io(e)
}
