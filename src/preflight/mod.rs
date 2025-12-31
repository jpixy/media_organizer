//! Preflight checks module.

mod ffprobe;
mod ollama;
mod tmdb;

use crate::Result;
use colored::Colorize;

/// Result of a preflight check.
#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub success: bool,
    pub message: String,
    pub hint: Option<String>,
}

impl CheckResult {
    pub fn ok(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            message: message.to_string(),
            hint: None,
        }
    }

    pub fn fail(name: &str, message: &str, hint: &str) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            message: message.to_string(),
            hint: Some(hint.to_string()),
        }
    }
}

/// Run all preflight checks.
pub async fn run_preflight_checks() -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();

    // Check ffprobe
    results.push(ffprobe::check());

    // Check Ollama
    results.push(ollama::check().await);

    // Check TMDB
    results.push(tmdb::check().await);

    Ok(results)
}

/// Print preflight check results.
pub fn print_results(results: &[CheckResult]) {
    for result in results {
        if result.success {
            println!(
                "{} {}: {}",
                "[OK]".green(),
                result.name.bold(),
                result.message
            );
        } else {
            println!(
                "{} {}: {}",
                "[FAIL]".red(),
                result.name.bold(),
                result.message
            );
            if let Some(ref hint) = result.hint {
                println!("  {} {}", "->".yellow(), hint);
            }
        }
    }
}

/// Check if all preflight checks passed.
pub fn all_passed(results: &[CheckResult]) -> bool {
    results.iter().all(|r| r.success)
}



