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
    #[arg(short = 's', long, value_enum, default_value = "progress")]
    style: DisplayStyle,

    /// Manually specify the remaining Premium request percentage (0-100).
    /// Skips the GitHub API call when provided.
    #[arg(short = 'p', long, value_name = "PERCENT")]
    percent: Option<f64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = if let Some(pct) = cli.percent {
        if !(0.0..=100.0).contains(&pct) {
            anyhow::bail!("--percent must be between 0 and 100");
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
pub(crate) fn build_data_from_percent(percent_remaining: f64, entitlement: u64) -> DisplayData {
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

pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    let next = next_month_start(year, month);
    let first = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
    (next - first).num_days() as u32
}

pub(crate) fn next_month_start(year: i32, month: u32) -> chrono::DateTime<Utc> {
    if month == 12 {
        Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap()
    } else {
        Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ── next_month_start ───────────────────────────────────────────────────

    #[test]
    fn next_month_start_normal_month() {
        let result = next_month_start(2024, 3);
        let expected = Utc.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn next_month_start_january() {
        let result = next_month_start(2024, 1);
        let expected = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn next_month_start_november() {
        // Month before the year-rollover boundary
        let result = next_month_start(2024, 11);
        let expected = Utc.with_ymd_and_hms(2024, 12, 1, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn next_month_start_december_rolls_year() {
        // December → January of next year
        let result = next_month_start(2024, 12);
        let expected = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn next_month_start_december_year_boundary_at_year_end() {
        let result = next_month_start(1999, 12);
        let expected = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    // ── days_in_month ──────────────────────────────────────────────────────

    #[test]
    fn days_in_month_january_31() {
        assert_eq!(days_in_month(2024, 1), 31);
    }

    #[test]
    fn days_in_month_april_30() {
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn days_in_month_june_30() {
        assert_eq!(days_in_month(2024, 6), 30);
    }

    #[test]
    fn days_in_month_september_30() {
        assert_eq!(days_in_month(2024, 9), 30);
    }

    #[test]
    fn days_in_month_november_30() {
        assert_eq!(days_in_month(2024, 11), 30);
    }

    #[test]
    fn days_in_month_december_31() {
        assert_eq!(days_in_month(2024, 12), 31);
    }

    #[test]
    fn days_in_month_february_leap_year() {
        // 2024 is a leap year
        assert_eq!(days_in_month(2024, 2), 29);
    }

    #[test]
    fn days_in_month_february_non_leap_year() {
        // 2023 is not a leap year
        assert_eq!(days_in_month(2023, 2), 28);
    }

    #[test]
    fn days_in_month_february_century_non_leap() {
        // 1900 is divisible by 100 but not 400 → not a leap year
        assert_eq!(days_in_month(1900, 2), 28);
    }

    #[test]
    fn days_in_month_february_400_year_leap() {
        // 2000 is divisible by 400 → leap year
        assert_eq!(days_in_month(2000, 2), 29);
    }

    // ── build_data_from_percent ────────────────────────────────────────────

    #[test]
    fn build_data_from_percent_quota_count() {
        let data = build_data_from_percent(50.0, 300);
        assert_eq!(data.quotas.len(), 3);
    }

    #[test]
    fn build_data_from_percent_chat_is_unlimited() {
        let data = build_data_from_percent(50.0, 300);
        let chat = data.quotas.iter().find(|q| q.label == "Chat").unwrap();
        assert!(chat.unlimited);
        assert_eq!(chat.entitlement, 0);
        assert_eq!(chat.remaining, 0);
    }

    #[test]
    fn build_data_from_percent_code_completions_is_unlimited() {
        let data = build_data_from_percent(50.0, 300);
        let cc = data
            .quotas
            .iter()
            .find(|q| q.label == "Code Completions")
            .unwrap();
        assert!(cc.unlimited);
        assert_eq!(cc.entitlement, 0);
        assert_eq!(cc.remaining, 0);
    }

    #[test]
    fn build_data_from_percent_premium_not_unlimited() {
        let data = build_data_from_percent(50.0, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert!(!p.unlimited);
    }

    #[test]
    fn build_data_from_percent_premium_entitlement_stored() {
        let data = build_data_from_percent(50.0, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.entitlement, 300);
    }

    #[test]
    fn build_data_from_percent_premium_remaining_50_pct() {
        // 50% of 300 = 150
        let data = build_data_from_percent(50.0, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.remaining, 150);
    }

    #[test]
    fn build_data_from_percent_premium_remaining_100_pct() {
        // 100% of 300 = 300
        let data = build_data_from_percent(100.0, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.remaining, 300);
    }

    #[test]
    fn build_data_from_percent_premium_remaining_0_pct() {
        // 0% of 300 = 0
        let data = build_data_from_percent(0.0, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.remaining, 0);
    }

    #[test]
    fn build_data_from_percent_premium_remaining_rounds() {
        // 33.3% of 300 = 99.9 → rounds to 100
        let data = build_data_from_percent(33.3, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.remaining, 100);
    }

    #[test]
    fn build_data_from_percent_premium_percent_remaining_stored() {
        let data = build_data_from_percent(42.5, 300);
        let p = data.quotas.iter().find(|q| q.label == "Premium").unwrap();
        assert_eq!(p.percent_remaining, 42.5);
    }

    #[test]
    fn build_data_from_percent_days_total_positive() {
        let data = build_data_from_percent(50.0, 300);
        assert!(data.days_total > 0, "days_total should be positive");
    }

    #[test]
    fn build_data_from_percent_days_remaining_non_negative() {
        let data = build_data_from_percent(50.0, 300);
        assert!(data.days_remaining >= 0, "days_remaining should be >= 0");
    }

    #[test]
    fn build_data_from_percent_reset_date_is_first_of_next_month() {
        let data = build_data_from_percent(50.0, 300);
        // reset_date must be the 1st of next month at 00:00:00 UTC
        use chrono::{Datelike, Timelike};
        assert_eq!(data.reset_date.day(), 1);
        assert_eq!(data.reset_date.hour(), 0);
        assert_eq!(data.reset_date.minute(), 0);
        assert_eq!(data.reset_date.second(), 0);
    }
}
