//! Plan executor module.
//!
//! Executes operations defined in a plan:
//! - mkdir: Create directories
//! - move: Move video files
//! - create: Generate NFO files
//! - download: Download posters

use crate::generators::nfo;
use crate::models::media::MediaType;
use crate::models::plan::{Operation, OperationType, Plan, PlanItem, PlanItemStatus};
use crate::models::rollback::{Rollback, RollbackAction, RollbackActionType, RollbackOpType, RollbackOperation};
use crate::utils::hash;
use crate::Result;
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

/// Executor configuration.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Whether to verify checksums after moving files.
    pub verify_checksum: bool,
    /// Whether to create backup before overwriting.
    pub backup_on_overwrite: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            verify_checksum: true,
            backup_on_overwrite: true,
        }
    }
}

/// Plan executor.
pub struct Executor {
    config: ExecutorConfig,
    http_client: reqwest::Client,
}

impl Executor {
    /// Create a new executor with default configuration.
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a new executor with custom configuration.
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Execute a plan.
    pub async fn execute(&self, plan: &Plan) -> Result<Rollback> {
        println!("{}", "🚀 Executing plan...".bold().cyan());
        println!();

        // Validate plan first
        self.validate(plan)?;

        // Initialize rollback structure
        let mut rollback = Rollback {
            version: "1.0".to_string(),
            plan_id: Uuid::new_v4().to_string(),
            executed_at: Utc::now().to_rfc3339(),
            operations: Vec::new(),
        };

        let mut seq: u32 = 0;
        let mut success_count = 0;
        let mut error_count = 0;

        // Calculate total operations
        let total_ops: usize = plan.items.iter()
            .filter(|i| i.status == PlanItemStatus::Pending)
            .map(|i| i.operations.len())
            .sum();

        // Create progress bar
        let pb = ProgressBar::new(total_ops as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        // Execute operations for each item
        let mut op_idx = 0;
        for item in &plan.items {
            if item.status != PlanItemStatus::Pending {
                tracing::debug!("Skipping item {} with status {:?}", item.id, item.status);
                continue;
            }

            for op in &item.operations {
                op_idx += 1;
                let progress_msg = format!(
                    "[{}/{}] {:?}: {}",
                    op_idx,
                    total_ops,
                    op.op,
                    op.to.file_name().unwrap_or_default().to_string_lossy()
                );
                pb.set_message(progress_msg);
                pb.inc(1);

                // Log detailed info for each operation
                tracing::info!(
                    "Execute [{}/{}]: {:?} - {}",
                    op_idx,
                    total_ops,
                    op.op,
                    op.to.display()
                );

                match self.execute_operation(op, item, plan).await {
                    Ok(rollback_op) => {
                        seq += 1;
                        if let Some(mut rb_op) = rollback_op {
                            rb_op.seq = seq;
                            rb_op.executed = true;
                            rollback.operations.push(rb_op);
                        }
                        success_count += 1;
                        tracing::debug!("  ✓ Success");
                    }
                    Err(e) => {
                        tracing::error!("Operation failed: {} - {}", op.to.display(), e);
                        println!("{} {} - {}", "ERROR".red().bold(), op.to.display(), e);
                        error_count += 1;
                        // Continue with remaining operations
                    }
                }
            }
        }

        pb.finish_with_message("Done!");
        println!();

        // Print summary
        println!("{}", "📊 Execution Summary".bold().green());
        println!("  {} {}", "Successful operations:".bold(), success_count);
        println!("  {} {}", "Failed operations:".bold(), error_count);
        println!();

        Ok(rollback)
    }

    /// Validate a plan before execution.
    pub fn validate(&self, plan: &Plan) -> Result<()> {
        println!("🔍 Validating plan...");

        let mut errors = Vec::new();

        for item in &plan.items {
            if item.status != PlanItemStatus::Pending {
                continue;
            }

            // Check if source file exists
            if !item.source.path.exists() {
                errors.push(format!(
                    "Source file not found: {}",
                    item.source.path.display()
                ));
            }

            // Check for target conflicts
            if item.target.full_path.exists() {
                if !self.config.backup_on_overwrite {
                    errors.push(format!(
                        "Target file already exists: {}",
                        item.target.full_path.display()
                    ));
                }
            }
        }

        if !errors.is_empty() {
            println!("{}", "❌ Validation failed:".bold().red());
            for error in &errors {
                println!("  - {}", error);
            }
            return Err(crate::Error::PlanValidationError(
                format!("{} errors found", errors.len())
            ));
        }

        println!("{}", "✅ Validation passed".green());
        Ok(())
    }

    /// Execute a single operation.
    async fn execute_operation(
        &self,
        op: &Operation,
        item: &PlanItem,
        plan: &Plan,
    ) -> Result<Option<RollbackOperation>> {
        match op.op {
            OperationType::Mkdir => self.execute_mkdir(op),
            OperationType::Move => self.execute_move(op),
            OperationType::Create => self.execute_create(op, item, plan),
            OperationType::Download => self.execute_download(op).await,
        }
    }

    /// Execute mkdir operation.
    fn execute_mkdir(&self, op: &Operation) -> Result<Option<RollbackOperation>> {
        let path = &op.to;

        if path.exists() {
            tracing::debug!("Directory already exists: {:?}", path);
            return Ok(None);
        }

        fs::create_dir_all(path)?;
        tracing::debug!("Created directory: {:?}", path);

        Ok(Some(RollbackOperation {
            seq: 0,
            op_type: RollbackOpType::Mkdir,
            from: path.clone(),
            to: path.clone(),
            checksum: None,
            rollback: RollbackAction {
                op: RollbackActionType::Rmdir,
                path: path.clone(),
                to: None,
            },
            executed: false,
        }))
    }

    /// Execute move operation.
    fn execute_move(&self, op: &Operation) -> Result<Option<RollbackOperation>> {
        let from = op.from.as_ref().ok_or_else(|| {
            crate::Error::ExecuteError("Move operation missing 'from' path".to_string())
        })?;
        let to = &op.to;

        // Calculate checksum before move
        let checksum = if self.config.verify_checksum {
            Some(hash::sha256_file(from)?)
        } else {
            None
        };

        // Create parent directory if needed
        if let Some(parent) = to.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Move file
        fs::rename(from, to)?;
        tracing::debug!("Moved: {:?} -> {:?}", from, to);

        // Verify checksum after move
        if self.config.verify_checksum {
            if let Some(ref original_checksum) = checksum {
                let new_checksum = hash::sha256_file(to)?;
                if original_checksum != &new_checksum {
                    return Err(crate::Error::ExecuteError(
                        format!("Checksum mismatch after moving: {:?}", to)
                    ));
                }
            }
        }

        Ok(Some(RollbackOperation {
            seq: 0,
            op_type: RollbackOpType::Move,
            from: from.clone(),
            to: to.clone(),
            checksum,
            rollback: RollbackAction {
                op: RollbackActionType::Move,
                path: to.clone(),
                to: Some(from.clone()),
            },
            executed: false,
        }))
    }

    /// Execute create operation (NFO file).
    fn execute_create(
        &self,
        op: &Operation,
        item: &PlanItem,
        plan: &Plan,
    ) -> Result<Option<RollbackOperation>> {
        let path = &op.to;
        
        // Generate content based on content_ref
        let content = match op.content_ref.as_deref() {
            Some("nfo") => {
                match plan.media_type {
                    Some(MediaType::Movies) => {
                        if let Some(ref metadata) = item.movie_metadata {
                            nfo::generate_movie_nfo(metadata)
                        } else {
                            return Err(crate::Error::ExecuteError(
                                "Missing movie metadata for NFO generation".to_string()
                            ));
                        }
                    }
                    Some(MediaType::TvShows) => {
                        // Check if this is tvshow.nfo (show-level) or episode.nfo
                        let is_tvshow_nfo = path.file_name()
                            .map(|n| n.to_string_lossy() == "tvshow.nfo")
                            .unwrap_or(false);
                        
                        if is_tvshow_nfo {
                            // Generate show-level NFO
                            if let Some(ref show) = item.tvshow_metadata {
                                nfo::generate_tvshow_nfo(show)
                            } else {
                                return Err(crate::Error::ExecuteError(
                                    "Missing TV show metadata for NFO generation".to_string()
                                ));
                            }
                        } else {
                            // Generate episode-level NFO
                            if let (Some(ref show), Some(ref episode)) = (&item.tvshow_metadata, &item.episode_metadata) {
                                nfo::generate_episode_nfo(show, episode)
                            } else if let Some(ref show) = item.tvshow_metadata {
                                nfo::generate_tvshow_nfo(show)
                            } else {
                                return Err(crate::Error::ExecuteError(
                                    "Missing TV show metadata for NFO generation".to_string()
                                ));
                            }
                        }
                    }
                    None => {
                        return Err(crate::Error::ExecuteError(
                            "Unknown media type for NFO generation".to_string()
                        ));
                    }
                }
            }
            _ => {
                return Err(crate::Error::ExecuteError(
                    format!("Unknown content_ref: {:?}", op.content_ref)
                ));
            }
        };

        // Skip if file already exists (for TV show NFO deduplication)
        if path.exists() {
            tracing::debug!("File already exists, skipping: {:?}", path);
            return Ok(None);
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write file
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        tracing::debug!("Created file: {:?}", path);

        Ok(Some(RollbackOperation {
            seq: 0,
            op_type: RollbackOpType::Create,
            from: path.clone(),
            to: path.clone(),
            checksum: None,
            rollback: RollbackAction {
                op: RollbackActionType::Delete,
                path: path.clone(),
                to: None,
            },
            executed: false,
        }))
    }

    /// Execute download operation (poster).
    async fn execute_download(&self, op: &Operation) -> Result<Option<RollbackOperation>> {
        let url = op.url.as_ref().ok_or_else(|| {
            crate::Error::ExecuteError("Download operation missing 'url'".to_string())
        })?;
        let path = &op.to;

        // Skip if file already exists (optimization)
        if path.exists() {
            tracing::debug!("Poster already exists, skipping: {:?}", path);
            return Ok(None);
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Download file
        let response = self.http_client.get(url).send().await?;
        
        if !response.status().is_success() {
            tracing::warn!("Failed to download poster: {} - {}", url, response.status());
            return Ok(None);
        }

        let bytes = response.bytes().await?;
        fs::write(path, &bytes)?;
        tracing::debug!("Downloaded: {} -> {:?}", url, path);

        Ok(Some(RollbackOperation {
            seq: 0,
            op_type: RollbackOpType::Download,
            from: path.clone(),
            to: path.clone(),
            checksum: None,
            rollback: RollbackAction {
                op: RollbackActionType::Delete,
                path: path.clone(),
                to: None,
            },
            executed: false,
        }))
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a plan (convenience function).
pub async fn execute_plan(plan: &Plan) -> Result<Rollback> {
    let executor = Executor::new();
    executor.execute(plan).await
}

/// Validate a plan before execution (convenience function).
pub fn validate_plan(plan: &Plan) -> Result<()> {
    let executor = Executor::new();
    executor.validate(plan)
}

/// Save rollback to a JSON file.
pub fn save_rollback(rollback: &Rollback, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(rollback)?;
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    file.write_all(json.as_bytes())?;

    tracing::info!("Rollback saved to {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert!(config.verify_checksum);
        assert!(config.backup_on_overwrite);
    }

    #[test]
    fn test_validate_empty_plan() {
        let plan = Plan::default();
        let executor = Executor::new();
        let result = executor.validate(&plan);
        assert!(result.is_ok());
    }
}


