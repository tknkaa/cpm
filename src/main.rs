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
#[command(about = "Compare your GitHub Copilot quota against the days left in the billing cycle")]
struct Cli {
    /// Display style
    #[arg(long, value_enum, default_value = "progress")]
    style: DisplayStyle,

    /// Manually specify the remaining Premium request percentage (0-100).
    /// Skips the GitHub API call when provided.
    #[arg(long, value_name = "PERCENT")]
    premium: Option<f64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = if let Some(pct) = cli.premium {
        if !(0.0..=100.0).contains(&pct) {
            anyhow::bail!("--premium must be between 0 and 100");
        }
        build_data_from_percent(pct, 300)
    } else {
        match api::fetch() {
            Ok(response) => {
                let s = &response.quota_snapshots;
                let (days_remaining, days_total) = calc_days(&response.quota_reset_date_utc);
                DisplayData {
                    quotas: vec![
                        quota_from_entry("Chat", &s.chat),
                        quota_from_entry("Code Completions", &s.completions),
                        quota_from_entry("Premium", &s.premium_interactions),
                    ],
                    days_remaining,
                    days_total,
                    reset_date: response.quota_reset_date_utc,
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch via GitHub API: {}", e);
                let pct = display::prompt_premium_percent()?;
                build_data_from_percent(pct, 300)
            }
        }
    };

    display::render_and_wait(data, &cli.style)?;
    Ok(())
}

/// Build DisplayData from a manually provided percentage.
/// Reset date is assumed to be the first day of next month.
fn build_data_from_percent(percent_remaining: f64, entitlement: u64) -> DisplayData {
    let now = Utc::now();
    let reset = next_month_start(now.year(), now.month());
    let (days_remaining, days_total) = calc_days(&reset);
    let remaining = (entitlement as f64 * percent_remaining / 100.0).round() as u64;

    DisplayData {
        quotas: vec![
            QuotaInfo {
                label: "Chat".into(),
                entitlement: 0,
                remaining: 0,
                unlimited: true,
                percent_remaining: 100.0,
            },
            QuotaInfo {
                label: "Code Completions".into(),
                entitlement: 0,
                remaining: 0,
                unlimited: true,
                percent_remaining: 100.0,
            },
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

fn quota_from_entry(label: &str, entry: &quota::QuotaEntry) -> QuotaInfo {
    QuotaInfo {
        label: label.into(),
        entitlement: entry.entitlement,
        remaining: entry.remaining,
        unlimited: entry.unlimited,
        percent_remaining: entry.percent_remaining,
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
