use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Block, Borders, Gauge, Paragraph},
};

use crate::quota::{DisplayData, QuotaInfo};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum DisplayStyle {
    Progress,
    Text,
    Graph,
}

// ────────────────────────────────────────────
// TUI input form (when --premium is not specified)
// ────────────────────────────────────────────

/// Prompt for the remaining percentage of premium_interactions in the TUI.
/// Example input: "23.4" → returns 23.4f64
pub fn prompt_premium_percent() -> Result<f64> {
    color_eyre::install().ok();
    let mut terminal = ratatui::init();
    let result = run_prompt(&mut terminal);
    ratatui::restore();
    result.map_err(|e| anyhow::anyhow!(e))
}

fn run_prompt(terminal: &mut DefaultTerminal) -> std::io::Result<f64> {
    let mut input = String::new();
    let mut error: Option<String> = None;

    loop {
        let inp = input.clone();
        let err = error.clone();
        terminal.draw(move |frame| render_prompt(frame, &inp, err.as_deref()))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Enter => match input.trim().parse::<f64>() {
                    Ok(v) if (0.0..=100.0).contains(&v) => return Ok(v),
                    Ok(_) => error = Some("Please enter a value between 0 and 100".into()),
                    Err(_) => error = Some("Please enter a number (e.g. 23.4)".into()),
                },
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                    input.push(c);
                    error = None;
                }
                KeyCode::Backspace => {
                    input.pop();
                    error = None;
                }
                KeyCode::Esc => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "Cancelled",
                    ));
                }
                _ => {}
            }
        }
    }
}

fn render_prompt(frame: &mut Frame, input: &str, error: Option<&str>) {
    let area = frame.area();
    let popup = centered_rect(50, 40, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Description
            Constraint::Length(3), // Input field
            Constraint::Length(2), // Error or hint
        ])
        .split(popup);

    let block = Block::default()
        .title(" Enter Premium Request Remaining ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(block, popup);

    let desc = Paragraph::new(
        "Failed to fetch via gh CLI.\nPlease enter the remaining percentage from your GitHub billing settings.",
    )
    .style(Style::default().fg(Color::Gray));
    frame.render_widget(desc, chunks[0]);

    let input_widget = Paragraph::new(format!("> {}█", input))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Remaining % (0-100)"),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(input_widget, chunks[1]);

    let hint = if let Some(e) = error {
        Paragraph::new(e).style(Style::default().fg(Color::Red))
    } else {
        Paragraph::new("Enter to confirm  /  Esc to cancel")
            .style(Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(hint, chunks[2]);
}

/// Create a centered popup rect within the given area.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}

// ────────────────────────────────────────────
// Main display
// ────────────────────────────────────────────

pub fn render_and_wait(data: DisplayData, style: &DisplayStyle) -> Result<()> {
    color_eyre::install().ok();
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &data, style);
    ratatui::restore();
    result.map_err(|e| anyhow::anyhow!(e))
}

fn run_app(
    terminal: &mut DefaultTerminal,
    data: &DisplayData,
    style: &DisplayStyle,
) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, data, style))?;
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            if key.kind == crossterm::event::KeyEventKind::Press {
                break Ok(());
            }
        }
    }
}

fn render(frame: &mut Frame, data: &DisplayData, style: &DisplayStyle) {
    match style {
        DisplayStyle::Progress => render_progress(frame, frame.area(), data),
        DisplayStyle::Text => render_text(frame, frame.area(), data),
        DisplayStyle::Graph => render_graph(frame, frame.area(), data),
    }
}

// ────────────────────────────────────────────
// Progress bar display
// ────────────────────────────────────────────

fn render_progress(frame: &mut Frame, area: Rect, data: &DisplayData) {
    let tracked: Vec<&QuotaInfo> = data.quotas.iter().filter(|q| !q.unlimited).collect();

    // Build constraints dynamically to avoid index-out-of-bounds when
    // the number of tracked quotas changes (e.g. chat becomes limited).
    // Layout: title(1 row + 1 separator) + each quota(1 row each) + month gauge + footer
    let mut constraints = vec![
        Constraint::Length(3), // title
        Constraint::Length(1), // separator
    ];
    for _ in &tracked {
        constraints.push(Constraint::Length(3)); // gauge per quota
        constraints.push(Constraint::Length(1)); // gap
    }
    constraints.push(Constraint::Length(3)); // month progress gauge
    constraints.push(Constraint::Length(1)); // gap
    constraints.push(Constraint::Min(2)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " GitHub Copilot Quota ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "| Reset: {}  {} days remaining",
                data.reset_date.format("%Y-%m-%d"),
                data.days_remaining
            ),
            Style::default().fg(Color::White),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[idx]);
    idx += 2;

    // One gauge per tracked quota
    for quota in &tracked {
        let color = pace_color(quota.percent_used(), data.days_remaining, data.days_total);
        let label = Span::styled(
            format!(
                "{}/{} used ({:.1}% remaining)",
                quota.entitlement - quota.remaining,
                quota.entitlement,
                quota.percent_remaining
            ),
            Style::default().fg(Color::Black),
        );
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(quota.label.clone())
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(color))
            .percent(quota.percent_used() as u16)
            .label(label);
        frame.render_widget(gauge, chunks[idx]);
        idx += 2;
    }

    // Month progress gauge
    let month_used_pct = ((data.days_total - data.days_remaining) as f64
        / data.days_total.max(1) as f64
        * 100.0) as u16;
    let month_gauge = Gauge::default()
        .block(
            Block::default()
                .title("Month Progress")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Blue))
        .percent(month_used_pct)
        .label(Span::styled(
            format!(
                "{} / {} days elapsed",
                data.days_total - data.days_remaining,
                data.days_total
            ),
            Style::default().fg(Color::Black),
        ));
    frame.render_widget(month_gauge, chunks[idx]);
    idx += 2;

    // Footer with pace summary
    let mut footer_lines = vec![Line::from(pace_summary(data))];
    if let Some(budget_text) = daily_budget_text(data) {
        footer_lines.push(Line::from(""));
        footer_lines.push(Line::from(Span::styled(
            budget_text,
            Style::default().fg(Color::Cyan),
        )));
    }
    let footer = Paragraph::new(footer_lines).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, chunks[idx]);
}

// ────────────────────────────────────────────
// Text display
// ────────────────────────────────────────────

fn render_text(frame: &mut Frame, area: Rect, data: &DisplayData) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "GitHub Copilot Quota Status",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Reset date: {}  Remaining this month: {} / {} days",
            data.reset_date.format("%Y-%m-%d"),
            data.days_remaining,
            data.days_total
        )),
        Line::from(""),
    ];

    for quota in &data.quotas {
        if quota.unlimited {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:20}", quota.label),
                    Style::default().fg(Color::White),
                ),
                Span::styled(" Unlimited", Style::default().fg(Color::Green)),
            ]));
        } else {
            let color = pace_color(quota.percent_used(), data.days_remaining, data.days_total);
            let pace = pace_label(quota.percent_used(), data.days_remaining, data.days_total);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:20}", quota.label),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        "Remaining {:4} / {:4}  ({:5.1}% remaining)  {}",
                        quota.remaining, quota.entitlement, quota.percent_remaining, pace
                    ),
                    Style::default().fg(color),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(pace_summary(data)));

    if let Some(budget_text) = daily_budget_text(data) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            budget_text,
            Style::default().fg(Color::Cyan),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Press any key to exit]",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" cpm - Copilot Quota "),
    );
    frame.render_widget(para, area);
}

// ────────────────────────────────────────────
// Graph (bar chart) display
// ────────────────────────────────────────────

fn render_graph(frame: &mut Frame, area: Rect, data: &DisplayData) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(10), Constraint::Length(5)])
        .split(area);

    let month_used_pct =
        (data.days_total - data.days_remaining) as u64 * 100 / data.days_total.max(1) as u64;

    let mut bar_data: Vec<(String, u64)> = data
        .quotas
        .iter()
        .filter(|q| !q.unlimited)
        .map(|q| (q.label.clone(), q.percent_used() as u64))
        .collect();
    bar_data.push(("Month Elapsed".to_string(), month_used_pct));

    let bars: Vec<(&str, u64)> = bar_data.iter().map(|(k, v)| (k.as_str(), *v)).collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .title(" Usage Rate (%) — [Press any key to exit] ")
                .borders(Borders::ALL),
        )
        .data(&bars)
        .bar_width(14)
        .bar_gap(2)
        .max(100)
        .bar_style(Style::default().fg(Color::Yellow))
        .value_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(chart, chunks[0]);

    let mut footer_lines = vec![Line::from(pace_summary(data))];
    if let Some(budget_text) = daily_budget_text(data) {
        footer_lines.push(Line::from(""));
        footer_lines.push(Line::from(Span::styled(
            budget_text,
            Style::default().fg(Color::Cyan),
        )));
    }
    let footer = Paragraph::new(footer_lines).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, chunks[1]);
}

// ────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────

/// Returns a color based on how the quota usage pace compares to the month progress.
fn pace_color(percent_used: f64, days_remaining: i64, days_total: i64) -> Color {
    let month_used_pct = (days_total - days_remaining) as f64 / days_total.max(1) as f64 * 100.0;
    let diff = percent_used - month_used_pct;
    if diff > 15.0 {
        Color::Red
    } else if diff > 5.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Returns a short pace label based on how quota usage compares to month progress.
fn pace_label(percent_used: f64, days_remaining: i64, days_total: i64) -> &'static str {
    let month_used_pct = (days_total - days_remaining) as f64 / days_total.max(1) as f64 * 100.0;
    let diff = percent_used - month_used_pct;
    if diff > 15.0 {
        "⚠  Overusing"
    } else if diff > 5.0 {
        "△  Slightly fast"
    } else if diff < -10.0 {
        "◎  Plenty left"
    } else {
        "✓  Good pace"
    }
}

/// Summarizes pace for all tracked (non-unlimited) quotas.
fn pace_summary(data: &DisplayData) -> String {
    let messages: Vec<String> = data
        .quotas
        .iter()
        .filter(|q| !q.unlimited)
        .map(|q| {
            let label = pace_label(q.percent_used(), data.days_remaining, data.days_total);
            format!("{}: {}", q.label, label)
        })
        .collect();

    if messages.is_empty() {
        "No tracked quotas (all unlimited)".to_string()
    } else {
        messages.join("  |  ")
    }
}

/// Calculate daily budget for premium requests remaining
fn daily_budget_text(data: &DisplayData) -> Option<String> {
    // Find the Premium quota
    let premium = data.quotas.iter().find(|q| q.label == "Premium")?;

    if premium.unlimited || data.days_remaining <= 0 {
        return None;
    }

    let daily_budget = premium.remaining as f64 / data.days_remaining as f64;
    Some(format!(
        "You can use up to {:.1} premium requests per day until reset",
        daily_budget
    ))
}
