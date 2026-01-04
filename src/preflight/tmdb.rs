//! TMDB API preflight check.

use super::CheckResult;
use crate::services::tmdb::TmdbClient;

/// Check if TMDB API is accessible.
pub async fn check() -> CheckResult {
    match TmdbClient::from_env() {
        Ok(client) => match client.verify_api_key().await {
            Ok(true) => CheckResult::ok("TMDB API", "connected"),
            Ok(false) => CheckResult::fail(
                "TMDB API",
                "invalid API key",
                "Check your TMDB_API_KEY environment variable",
            ),
            Err(_) => CheckResult::fail(
                "TMDB API",
                "connection failed",
                "Check your network connection",
            ),
        },
        Err(_) => CheckResult::fail(
            "TMDB API",
            "API key not configured",
            "Set TMDB_API_KEY environment variable",
        ),
    }
}
