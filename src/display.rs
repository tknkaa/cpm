use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::quota::{DisplayData, QuotaInfo};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum DisplayStyle {
    Progress,
    Text,
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
        Paragraph::new("Enter to confirm  /  Esc to cancel").style(Style::default().fg(Color::Gray))
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
        let title = format!(
            "{}  {}/{} used  ({:.1}% remaining)",
            quota.label,
            quota.entitlement - quota.remaining,
            quota.entitlement,
            quota.percent_remaining,
        );
        let gauge = Gauge::default()
            .block(Block::default().title(title).borders(Borders::ALL))
            .gauge_style(Style::default().fg(color))
            .percent(quota.percent_used() as u16)
            .label("");
        frame.render_widget(gauge, chunks[idx]);
        idx += 2;
    }

    // Month progress gauge
    let month_used_pct = ((data.days_total - data.days_remaining) as f64
        / data.days_total.max(1) as f64
        * 100.0) as u16;
    let month_title = format!(
        "Month Progress  {} / {} days elapsed  ({}% elapsed)",
        data.days_total - data.days_remaining,
        data.days_total,
        month_used_pct,
    );
    let month_gauge = Gauge::default()
        .block(Block::default().title(month_title).borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue))
        .percent(month_used_pct)
        .label("");
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
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:20}", quota.label),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        "{:4} /{:4}  ({:5.1}% remaining)",
                        quota.remaining, quota.entitlement, quota.percent_remaining
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
        "[Press any key to exit]",
        Style::default().fg(Color::Gray),
    )));

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" cpm - Copilot Quota "),
    );
    frame.render_widget(para, area);
}

// ────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────

/// Returns a color based on how the quota usage pace compares to the month progress.
pub(crate) fn pace_color(percent_used: f64, days_remaining: i64, days_total: i64) -> Color {
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
pub(crate) fn pace_label(percent_used: f64, days_remaining: i64, days_total: i64) -> &'static str {
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
pub(crate) fn daily_budget_text(data: &DisplayData) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ── helpers ────────────────────────────────────────────────────────────

    fn metered(label: &str, remaining: u64, entitlement: u64, percent_remaining: f64) -> QuotaInfo {
        QuotaInfo {
            label: label.into(),
            entitlement,
            remaining,
            unlimited: false,
            percent_remaining,
        }
    }

    fn unlimited(label: &str) -> QuotaInfo {
        QuotaInfo {
            label: label.into(),
            entitlement: 0,
            remaining: 0,
            unlimited: true,
            percent_remaining: 100.0,
        }
    }

    fn display_data(quotas: Vec<QuotaInfo>, days_remaining: i64, days_total: i64) -> DisplayData {
        DisplayData {
            quotas,
            days_remaining,
            days_total,
            reset_date: Utc::now(),
        }
    }

    // ── pace_color ─────────────────────────────────────────────────────────

    // diff = percent_used - month_used_pct
    // month_used_pct = (days_total - days_remaining) / days_total * 100
    //
    // For days_total=30, days_remaining=15 → month_used_pct = 50%

    #[test]
    fn pace_color_red_when_diff_above_15() {
        // percent_used=70, month_used=50 → diff=20 → Red
        assert_eq!(pace_color(70.0, 15, 30), Color::Red);
    }

    #[test]
    fn pace_color_red_at_boundary_just_above_15() {
        // diff = 15.01 → Red
        assert_eq!(pace_color(65.01, 15, 30), Color::Red);
    }

    #[test]
    fn pace_color_yellow_when_diff_exactly_15() {
        // diff == 15.0 → NOT > 15 → falls to Yellow check: 15 > 5 → Yellow
        assert_eq!(pace_color(65.0, 15, 30), Color::Yellow);
    }

    #[test]
    fn pace_color_yellow_when_diff_between_5_and_15() {
        // percent_used=60, month_used=50 → diff=10 → Yellow
        assert_eq!(pace_color(60.0, 15, 30), Color::Yellow);
    }

    #[test]
    fn pace_color_yellow_at_boundary_just_above_5() {
        // diff = 5.01 → Yellow
        assert_eq!(pace_color(55.01, 15, 30), Color::Yellow);
    }

    #[test]
    fn pace_color_green_when_diff_exactly_5() {
        // diff == 5.0 → NOT > 5 → Green
        assert_eq!(pace_color(55.0, 15, 30), Color::Green);
    }

    #[test]
    fn pace_color_green_when_on_pace() {
        // percent_used==month_used → diff=0 → Green
        assert_eq!(pace_color(50.0, 15, 30), Color::Green);
    }

    #[test]
    fn pace_color_green_when_under_pace() {
        // percent_used=30, month_used=50 → diff=-20 → Green
        assert_eq!(pace_color(30.0, 15, 30), Color::Green);
    }

    #[test]
    fn pace_color_zero_days_total_uses_max1() {
        // days_total=0 → denominator clamped to max(1)=1
        // month_used_pct = (0 - 0) / 1 * 100 = 0%
        // percent_used=100, diff=100 → Red
        assert_eq!(pace_color(100.0, 0, 0), Color::Red);
    }

    // ── pace_label ─────────────────────────────────────────────────────────

    #[test]
    fn pace_label_overusing_when_diff_above_15() {
        assert_eq!(pace_label(70.0, 15, 30), "⚠  Overusing");
    }

    #[test]
    fn pace_label_slightly_fast_when_diff_between_5_and_15_inclusive() {
        // diff=10 → Slightly fast
        assert_eq!(pace_label(60.0, 15, 30), "△  Slightly fast");
        // boundary: diff=15 exactly → not >15, but >5 → Slightly fast
        assert_eq!(pace_label(65.0, 15, 30), "△  Slightly fast");
        // boundary: diff=5 exactly → not >5, not <-10 → Good pace
        assert_eq!(pace_label(55.0, 15, 30), "✓  Good pace");
    }

    #[test]
    fn pace_label_good_pace_when_on_pace() {
        assert_eq!(pace_label(50.0, 15, 30), "✓  Good pace");
    }

    #[test]
    fn pace_label_good_pace_in_dead_zone_minus10_to_0() {
        // diff=-5 → not <-10 → Good pace
        assert_eq!(pace_label(45.0, 15, 30), "✓  Good pace");
        // boundary: diff=-10 exactly → not <-10 → Good pace
        assert_eq!(pace_label(40.0, 15, 30), "✓  Good pace");
    }

    #[test]
    fn pace_label_plenty_left_when_diff_below_minus10() {
        // diff=-11 → Plenty left
        assert_eq!(pace_label(39.0, 15, 30), "◎  Plenty left");
    }

    #[test]
    fn pace_label_plenty_left_far_under_pace() {
        // percent_used=0, month_used=50 → diff=-50 → Plenty left
        assert_eq!(pace_label(0.0, 15, 30), "◎  Plenty left");
    }

    // ── daily_budget_text ─────────────────────────────────────────────────

    #[test]
    fn daily_budget_text_no_premium_quota_returns_none() {
        let data = display_data(vec![unlimited("Chat")], 10, 30);
        assert_eq!(daily_budget_text(&data), None);
    }

    #[test]
    fn daily_budget_text_unlimited_premium_returns_none() {
        let data = display_data(vec![unlimited("Premium")], 10, 30);
        assert_eq!(daily_budget_text(&data), None);
    }

    #[test]
    fn daily_budget_text_zero_days_remaining_returns_none() {
        let data = display_data(vec![metered("Premium", 100, 300, 33.3)], 0, 30);
        assert_eq!(daily_budget_text(&data), None);
    }

    #[test]
    fn daily_budget_text_negative_days_remaining_returns_none() {
        let data = display_data(vec![metered("Premium", 100, 300, 33.3)], -1, 30);
        assert_eq!(daily_budget_text(&data), None);
    }

    #[test]
    fn daily_budget_text_normal_case() {
        // 60 remaining, 10 days left → 6.0 per day
        let data = display_data(vec![metered("Premium", 60, 300, 20.0)], 10, 30);
        assert_eq!(
            daily_budget_text(&data),
            Some("You can use up to 6.0 premium requests per day until reset".to_string())
        );
    }

    #[test]
    fn daily_budget_text_fractional_budget_rounds_to_one_decimal() {
        // 100 remaining, 3 days → 33.333… → displayed as "33.3"
        let data = display_data(vec![metered("Premium", 100, 300, 33.3)], 3, 30);
        assert_eq!(
            daily_budget_text(&data),
            Some("You can use up to 33.3 premium requests per day until reset".to_string())
        );
    }

    #[test]
    fn daily_budget_text_one_day_remaining() {
        // 45 remaining, 1 day → 45.0 per day
        let data = display_data(vec![metered("Premium", 45, 300, 15.0)], 1, 30);
        assert_eq!(
            daily_budget_text(&data),
            Some("You can use up to 45.0 premium requests per day until reset".to_string())
        );
    }

    #[test]
    fn daily_budget_text_zero_remaining() {
        // 0 remaining, 10 days → 0.0 per day
        let data = display_data(vec![metered("Premium", 0, 300, 0.0)], 10, 30);
        assert_eq!(
            daily_budget_text(&data),
            Some("You can use up to 0.0 premium requests per day until reset".to_string())
        );
    }

    #[test]
    fn daily_budget_text_uses_premium_label_not_other_quotas() {
        // Even if other quotas are present, only "Premium" is used
        let data = display_data(
            vec![
                metered("Chat", 100, 200, 50.0),
                metered("Premium", 30, 300, 10.0),
            ],
            5,
            30,
        );
        assert_eq!(
            daily_budget_text(&data),
            Some("You can use up to 6.0 premium requests per day until reset".to_string())
        );
    }
}
