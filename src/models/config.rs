//! Configuration model.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Ollama configuration.
    pub ollama: OllamaConfig,
    /// TMDB configuration.
    pub tmdb: TmdbConfig,
    /// Sessions directory.
    pub sessions_dir: PathBuf,
}

/// Ollama configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama host.
    pub host: String,
    /// Ollama port.
    pub port: u16,
    /// Model to use.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout: u64,
}

/// TMDB configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbConfig {
    /// API key.
    pub api_key: Option<String>,
    /// Language for responses.
    pub language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig::default(),
            tmdb: TmdbConfig::default(),
            sessions_dir: dirs_config_path().join("sessions"),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 11434,
            model: "qwen2.5:7b".to_string(),
            timeout: 60,
        }
    }
}

impl Default for TmdbConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("TMDB_API_KEY").ok(),
            language: "zh-CN".to_string(),
        }
    }
}

/// Get the configuration directory path.
fn dirs_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("media_organizer")
}

/// Load configuration from file.
pub fn load_config() -> Config {
    let config_path = dirs_config_path().join("config.toml");

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
    }

    Config::default()
}



