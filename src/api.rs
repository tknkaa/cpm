use crate::quota::CopilotUserResponse;
use anyhow::{Context, Result};

/// Call REST API via `gh api` (uses gh's built-in auth and User-Agent)
pub fn fetch() -> Result<CopilotUserResponse> {
    let output = std::process::Command::new("gh")
        .args(["api", "/copilot_internal/user"])
        .output()
        .context("gh command failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh api returned error: {}", stderr);
    }

    serde_json::from_slice(&output.stdout).context("Failed to parse JSON")
}
