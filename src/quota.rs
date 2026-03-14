use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Response structure for GET /copilot_internal/user
#[derive(Debug, Deserialize)]
pub struct CopilotUserResponse {
    pub quota_snapshots: QuotaSnapshots,
    pub quota_reset_date_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct QuotaSnapshots {
    pub chat: QuotaEntry,
    pub completions: QuotaEntry,
    pub premium_interactions: QuotaEntry,
}

#[derive(Debug, Deserialize)]
pub struct QuotaEntry {
    pub entitlement: u64,
    pub remaining: u64,
    pub unlimited: bool,
    pub percent_remaining: f64,
}

/// Normalized quota info used for display and comparison
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

#[cfg(test)]
mod tests {
    use super::*;

    fn quota(entitlement: u64, percent_remaining: f64, unlimited: bool) -> QuotaInfo {
        QuotaInfo {
            label: "Test".into(),
            entitlement,
            remaining: 0,
            unlimited,
            percent_remaining,
        }
    }

    // percent_used ──────────────────────────────────────────────────────────

    #[test]
    fn percent_used_normal() {
        // 40% remaining → 60% used
        let q = quota(300, 40.0, false);
        assert_eq!(q.percent_used(), 60.0);
    }

    #[test]
    fn percent_used_fully_remaining() {
        // 100% remaining → 0% used
        let q = quota(300, 100.0, false);
        assert_eq!(q.percent_used(), 0.0);
    }

    #[test]
    fn percent_used_fully_exhausted() {
        // 0% remaining → 100% used
        let q = quota(300, 0.0, false);
        assert_eq!(q.percent_used(), 100.0);
    }

    #[test]
    fn percent_used_unlimited_returns_zero() {
        // unlimited flag short-circuits regardless of other fields
        let q = quota(300, 40.0, true);
        assert_eq!(q.percent_used(), 0.0);
    }

    #[test]
    fn percent_used_zero_entitlement_returns_zero() {
        // entitlement == 0 → guard returns 0.0
        let q = quota(0, 40.0, false);
        assert_eq!(q.percent_used(), 0.0);
    }

    #[test]
    fn percent_used_zero_entitlement_and_unlimited_returns_zero() {
        let q = quota(0, 0.0, true);
        assert_eq!(q.percent_used(), 0.0);
    }

    #[test]
    fn percent_used_fractional() {
        // 33.3% remaining → 66.7% used
        let q = quota(300, 33.3, false);
        let expected = 100.0 - 33.3f64;
        assert!((q.percent_used() - expected).abs() < 1e-10);
    }
}

/// All data needed for display
pub struct DisplayData {
    pub quotas: Vec<QuotaInfo>,
    pub days_remaining: i64,
    pub days_total: i64,
    pub reset_date: DateTime<Utc>,
}
