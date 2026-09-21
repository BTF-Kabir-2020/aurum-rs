use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Titled border block, per README §29 layout.
pub fn frame_widget(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_alignment(Alignment::Center)
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::DarkGray))
}

/// Screen text from an application snapshot (pure, testable).
#[allow(clippy::too_many_arguments)]
pub fn screen_lines(
    mode: &str,
    source: &str,
    symbol_display: &str,
    price: f64,
    ema: f64,
    rsi: f64,
    momentum: f64,
    action: &str,
    strength: u8,
    max_strength: u8,
    updated_local: &str,
    data_age_secs: u64,
) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        blank_span_line(format!("Symbol       {symbol_display}")),
        blank_span_line(format!("Mode         {}", mode.to_uppercase())),
        blank_span_line(format!("Price        {price:.2}")),
        Line::from(""),
        blank_span_line(format!("EMA(20)      {ema:.2}")),
        blank_span_line(format!("RSI(14)      {rsi:.2}")),
        blank_span_line(format!("Momentum     {momentum:.2}%")),
        Line::from(""),
        Line::from(vec![
            ratatui::text::Span::styled("SIGNAL       ", Style::default().fg(Color::Gray)),
            ratatui::text::Span::styled(
                action.to_string(),
                Style::default()
                    .fg(action_color(action))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            ratatui::text::Span::styled("STRENGTH     ", Style::default().fg(Color::Gray)),
            ratatui::text::Span::styled(
                format!("{strength}/{max_strength} (rule score, not AI)"),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        blank_span_line(format!("Data source  {source}")),
        blank_span_line(format!("Updated      {updated_local}")),
        blank_span_line(format!("Data age     {data_age_secs:02}s")),
    ]
}

fn blank_span_line(text: String) -> Line<'static> {
    Line::from(ratatui::text::Span::styled(
        text,
        Style::default().fg(Color::White),
    ))
}

pub fn action_color(action: &str) -> Color {
    match action {
        "BUY" => Color::Green,
        "SELL" => Color::Red,
        _ => Color::Gray,
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub mode: String,
    pub source: String,
    pub symbol: String,
    pub price: f64,
    pub ema20: f64,
    pub rsi14: f64,
    pub momentum: f64,
    pub action: String,
    pub strength: u8,
    pub max_strength: u8,
    pub updated_local: String,
    pub data_age_secs: u64,
}

/// Full-screen render for one frame.
pub fn render_snapshot(frame: &mut Frame, snapshot: &Snapshot) {
    let block = frame_widget("AURUM — Financial Market Intelligence Engine");
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let lines = screen_lines(
        &snapshot.mode,
        &snapshot.source,
        &snapshot.symbol,
        snapshot.price,
        snapshot.ema20,
        snapshot.rsi14,
        snapshot.momentum,
        &snapshot.action,
        snapshot.strength,
        snapshot.max_strength,
        &snapshot.updated_local,
        snapshot.data_age_secs,
    );

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Plain-text preview of the same content (for tests and non-TTY runs).
#[allow(clippy::too_many_arguments)]
pub fn preview_text(
    mode: &str,
    source: &str,
    symbol_display: &str,
    price: f64,
    ema: f64,
    rsi: f64,
    momentum: f64,
    action: &str,
    strength: u8,
    max_strength: u8,
    updated_local: &str,
    data_age_secs: u64,
) -> String {
    let lines = screen_lines(
        mode,
        source,
        symbol_display,
        price,
        ema,
        rsi,
        momentum,
        action,
        strength,
        max_strength,
        updated_local,
        data_age_secs,
    );
    let mut out = String::from("AURUM — Financial Market Intelligence Engine\n");
    for l in lines {
        for s in &l.spans {
            out.push_str(&s.content);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            mode: "demo".into(),
            source: "fixture".into(),
            symbol: "XAU/USD".into(),
            price: 3692.39,
            ema20: 3689.04,
            rsi14: 28.31,
            momentum: 0.10,
            action: "HOLD".into(),
            strength: 0,
            max_strength: 3,
            updated_local: "22:41:08".into(),
            data_age_secs: 1,
        }
    }

    #[test]
    fn preview_contains_readme_fields() {
        let s = snapshot();
        let text = preview_text(
            &s.mode,
            &s.source,
            &s.symbol,
            s.price,
            s.ema20,
            s.rsi14,
            s.momentum,
            &s.action,
            s.strength,
            s.max_strength,
            &s.updated_local,
            s.data_age_secs,
        );
        for needle in [
            "AURUM",
            "Symbol       XAU/USD",
            "Mode         DEMO",
            "Price        3692.39",
            "EMA(20)      3689.04",
            "RSI(14)      28.31",
            "Momentum     0.10%",
            "SIGNAL       HOLD",
            "STRENGTH     0/3",
            "Data age     01s",
        ] {
            assert!(text.contains(needle), "missing '{needle}' in:\n{text}");
        }
        assert!(
            text.contains("rule score, not AI"),
            "strength must not say AI-confidence"
        );
    }

    #[test]
    fn action_colors_match_three_sides() {
        assert_eq!(action_color("BUY"), Color::Green);
        assert_eq!(action_color("SELL"), Color::Red);
        assert_eq!(action_color("HOLD"), Color::Gray);
    }
}
