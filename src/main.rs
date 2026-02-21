mod api;
mod display;
mod quota;

use anyhow::Result;
use chrono::{Datelike, TimeZone, Utc};
use clap::Parser;
use display::DisplayStyle;
use quota::{DisplayData, QuotaInfo};

#[derive(Parser)]
#[command(name = "cpm")]
#[command(about = "Compare GitHub Copilot quota remaining and remaining days in the month")]
struct Cli {
    /// Display style
    #[arg(long, value_enum, default_value = "progress")]
    style: DisplayStyle,

    /// Manually specify the remaining percentage of Premium requests (0-100)
    /// If specified, displays using this value without using gh CLI
    #[arg(long, value_name = "PERCENT")]
    premium: Option<f64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = if let Some(pct) = cli.premium {
        // Directly specified with --premium
        if !(0.0..=100.0).contains(&pct) {
            anyhow::bail!("Please specify a value between 0-100 for --premium");
        }
        build_data_from_percent(pct, 300)
    } else {
        // Attempt to fetch via gh CLI
        match api::fetch() {
            Ok(response) => {
                let pi = &response.quota_snapshots.premium_interactions;
                let (days_remaining, days_total) = calc_days(&response.quota_reset_date_utc);
                DisplayData {
                    quotas: vec![
                        unlimited_quota("Chat"),
                        unlimited_quota("Code Completion"),
                        QuotaInfo {
                            label: "Premium".into(),
                            entitlement: pi.entitlement,
                            remaining: pi.remaining,
                            unlimited: false,
                            percent_remaining: pi.percent_remaining,
                        },
                    ],
                    days_remaining,
                    days_total,
                    reset_date: response.quota_reset_date_utc,
                }
            }
            Err(e) => {
                // If failed, prompt for percentage in TUI
                eprintln!("Failed to fetch via gh CLI: {}", e);
                let pct = display::prompt_premium_percent()?;
                build_data_from_percent(pct, 300)
            }
        }
    };

    display::render_and_wait(data, &cli.style)?;
    Ok(())
}

/// Build DisplayData from percentage and limit (assumes reset date is the 1st of next month)
fn build_data_from_percent(percent_remaining: f64, entitlement: u64) -> DisplayData {
    let now = Utc::now();
    let reset = next_month_start(now.year(), now.month());
    let (days_remaining, days_total) = calc_days(&reset);
    let remaining = (entitlement as f64 * percent_remaining / 100.0).round() as u64;

    DisplayData {
        quotas: vec![
            unlimited_quota("Chat"),
            unlimited_quota("Code Completion"),
            QuotaInfo {
                label: "Premium".into(),
                entitlement,
                remaining,
                unlimited: false,
                percent_remaining,
            },
        ],
        days_remaining,
        days_total,
        reset_date: reset,
    }
}

fn unlimited_quota(label: &str) -> QuotaInfo {
    QuotaInfo {
        label: label.into(),
        entitlement: 0,
        remaining: 0,
        unlimited: true,
        percent_remaining: 100.0,
    }
}

fn calc_days(reset_date: &chrono::DateTime<Utc>) -> (i64, i64) {
    let now = Utc::now();
    let days_remaining = (*reset_date - now).num_days().max(0);
    let days_total = days_in_month(now.year(), now.month()) as i64;
    (days_remaining, days_total)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = next_month_start(year, month);
    let first = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
    (next - first).num_days() as u32
}

fn next_month_start(year: i32, month: u32) -> chrono::DateTime<Utc> {
    if month == 12 {
        Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap()
    } else {
        Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0).unwrap()
    }
}
