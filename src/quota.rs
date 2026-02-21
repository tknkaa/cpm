use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Response structure of gh api /copilot_internal/user
#[derive(Debug, Deserialize)]
pub struct CopilotUserResponse {
    pub quota_snapshots: QuotaSnapshots,
    pub quota_reset_date_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct QuotaSnapshots {
    pub premium_interactions: QuotaEntry,
}

#[derive(Debug, Deserialize)]
pub struct QuotaEntry {
    pub entitlement: u64,
    pub remaining: u64,
    pub percent_remaining: f64,
}

/// Normalized quota information for display and comparison
#[derive(Debug, Clone)]
pub struct QuotaInfo {
    pub label: String,
    pub entitlement: u64,
    pub remaining: u64,
    pub unlimited: bool,
    pub percent_remaining: f64,
}

impl QuotaInfo {
    pub fn percent_used(&self) -> f64 {
        if self.unlimited || self.entitlement == 0 {
            return 0.0;
        }
        100.0 - self.percent_remaining
    }
}

/// All data required for display
pub struct DisplayData {
    pub quotas: Vec<QuotaInfo>,
    pub days_remaining: i64,
    pub days_total: i64,
    pub reset_date: DateTime<Utc>,
}
