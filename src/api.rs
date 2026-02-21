use anyhow::{Context, Result};
use std::process::Command;

use crate::quota::CopilotUserResponse;

/// Get token with `gh auth token` and call REST API directly
pub fn fetch() -> Result<CopilotUserResponse> {
    let token = get_token()?;

    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/copilot_internal/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "cpm")
        .send()
        .context("API request failed")?;

    if !response.status().is_success() {
        anyhow::bail!("API returned error: {}", response.status());
    }

    response.json().context("Failed to parse JSON")
}

fn get_token() -> Result<String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("gh command not found. Please install GitHub CLI (gh).")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get gh auth token. Please login with `gh auth login`.");
    }

    Ok(String::from_utf8(output.stdout)
        .context("Failed to convert token to UTF-8")?
        .trim()
        .to_string())
}
