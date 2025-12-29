//! Rollback execution module.
//!
//! Reverses operations performed by the executor:
//! - Move files back to original locations
//! - Delete created files (NFO, posters)
//! - Remove created directories

use crate::models::rollback::{Rollback, RollbackActionType, RollbackOperation};
use crate::utils::hash;
use crate::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Rollback executor.
pub struct RollbackExecutor {
    /// Whether to verify checksums before rollback.
    verify_checksum: bool,
}

impl RollbackExecutor {
    /// Create a new rollback executor.
    pub fn new() -> Self {
        Self {
            verify_checksum: true,
        }
    }

    /// Execute a rollback.
    pub async fn execute(&self, rollback: &Rollback, dry_run: bool) -> Result<RollbackResult> {
        if dry_run {
            println!("{}", "🔍 Dry run - no changes will be made".bold().yellow());
        } else {
            println!("{}", "⏪ Executing rollback...".bold().cyan());
        }
        println!();

        // Check for conflicts first
        let conflicts = self.check_conflicts(rollback)?;
        if !conflicts.is_empty() {
            println!("{}", "⚠️  Conflicts detected:".bold().yellow());
            for conflict in &conflicts {
                println!("  - {}", conflict);
            }
            if !dry_run {
                println!();
                println!("{}", "Proceeding with rollback anyway...".yellow());
            }
        }

        let mut result = RollbackResult {
            success_count: 0,
            skip_count: 0,
            error_count: 0,
            errors: Vec::new(),
        };

        // Execute operations in reverse order
        let operations: Vec<_> = rollback.operations.iter().rev().collect();

        let pb = ProgressBar::new(operations.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        for op in operations {
            pb.set_message(format!("{:?}: {}", op.rollback.op, op.rollback.path.display()));
            pb.inc(1);

            if dry_run {
                println!(
                    "  {} {:?} {}",
                    "[DRY RUN]".yellow(),
                    op.rollback.op,
                    op.rollback.path.display()
                );
                result.success_count += 1;
                continue;
            }

            match self.execute_rollback_op(op) {
                Ok(executed) => {
                    if executed {
                        result.success_count += 1;
                    } else {
                        result.skip_count += 1;
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}: {}", op.rollback.path.display(), e);
                    tracing::error!("Rollback operation failed: {}", error_msg);
                    result.errors.push(error_msg);
                    result.error_count += 1;
                }
            }
        }

        pb.finish_with_message("Done!");
        println!();

        Ok(result)
    }

    /// Check for conflicts before rollback.
    fn check_conflicts(&self, rollback: &Rollback) -> Result<Vec<String>> {
        let mut conflicts = Vec::new();

        for op in &rollback.operations {
            match op.rollback.op {
                RollbackActionType::Move => {
                    // Check if source file still exists at target location
                    if !op.rollback.path.exists() {
                        conflicts.push(format!(
                            "File not found at target: {}",
                            op.rollback.path.display()
                        ));
                    }

                    // Check if original location is occupied
                    if let Some(ref original_path) = op.rollback.to {
                        if original_path.exists() {
                            conflicts.push(format!(
                                "Original location occupied: {}",
                                original_path.display()
                            ));
                        }
                    }

                    // Check checksum if available
                    if self.verify_checksum {
                        if let Some(ref expected_checksum) = op.checksum {
                            if op.rollback.path.exists() {
                                if let Ok(current_checksum) = hash::sha256_file(&op.rollback.path) {
                                    if &current_checksum != expected_checksum {
                                        conflicts.push(format!(
                                            "File modified since execution: {}",
                                            op.rollback.path.display()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                RollbackActionType::Delete => {
                    // Check if file still exists
                    if !op.rollback.path.exists() {
                        // Not a conflict, just skip
                    }
                }
                RollbackActionType::Rmdir => {
                    // Check if directory is empty
                    if op.rollback.path.exists() {
                        if let Ok(mut entries) = fs::read_dir(&op.rollback.path) {
                            if entries.next().is_some() {
                                conflicts.push(format!(
                                    "Directory not empty: {}",
                                    op.rollback.path.display()
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Execute a single rollback operation.
    fn execute_rollback_op(&self, op: &RollbackOperation) -> Result<bool> {
        match op.rollback.op {
            RollbackActionType::Move => {
                let from = &op.rollback.path;
                let to = op.rollback.to.as_ref().ok_or_else(|| {
                    crate::Error::RollbackConflict("Move rollback missing 'to' path".to_string())
                })?;

                if !from.exists() {
                    tracing::warn!("Source file not found, skipping: {:?}", from);
                    return Ok(false);
                }

                // Create parent directory if needed
                if let Some(parent) = to.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }

                // Move file back
                fs::rename(from, to)?;
                tracing::debug!("Moved back: {:?} -> {:?}", from, to);
                Ok(true)
            }
            RollbackActionType::Delete => {
                let path = &op.rollback.path;

                if !path.exists() {
                    tracing::debug!("File already deleted, skipping: {:?}", path);
                    return Ok(false);
                }

                fs::remove_file(path)?;
                tracing::debug!("Deleted: {:?}", path);
                Ok(true)
            }
            RollbackActionType::Rmdir => {
                let path = &op.rollback.path;

                if !path.exists() {
                    tracing::debug!("Directory already removed, skipping: {:?}", path);
                    return Ok(false);
                }

                // Only remove if empty
                if let Ok(mut entries) = fs::read_dir(path) {
                    if entries.next().is_some() {
                        tracing::warn!("Directory not empty, skipping: {:?}", path);
                        return Ok(false);
                    }
                }

                fs::remove_dir(path)?;
                tracing::debug!("Removed directory: {:?}", path);
                Ok(true)
            }
        }
    }
}

impl Default for RollbackExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a rollback execution.
#[derive(Debug, Default)]
pub struct RollbackResult {
    /// Number of successful operations.
    pub success_count: usize,
    /// Number of skipped operations.
    pub skip_count: usize,
    /// Number of failed operations.
    pub error_count: usize,
    /// Error messages.
    pub errors: Vec<String>,
}

impl RollbackResult {
    /// Check if rollback was successful.
    pub fn is_success(&self) -> bool {
        self.error_count == 0
    }

    /// Print summary.
    pub fn print_summary(&self) {
        println!("{}", "📊 Rollback Summary".bold().green());
        println!("  {} {}", "Successful:".bold(), self.success_count);
        println!("  {} {}", "Skipped:".bold(), self.skip_count);
        println!("  {} {}", "Failed:".bold(), self.error_count);

        if !self.errors.is_empty() {
            println!();
            println!("{}", "❌ Errors:".bold().red());
            for error in &self.errors {
                println!("  - {}", error);
            }
        }
    }
}

/// Execute a rollback (convenience function).
pub async fn execute_rollback(rollback: &Rollback, dry_run: bool) -> Result<RollbackResult> {
    let executor = RollbackExecutor::new();
    executor.execute(rollback, dry_run).await
}

/// Load a rollback from a JSON file.
pub fn load_rollback(path: &Path) -> Result<Rollback> {
    let content = fs::read_to_string(path)?;
    let rollback: Rollback = serde_json::from_str(&content)?;
    Ok(rollback)
}

/// Save a rollback to a JSON file.
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
    fn test_rollback_result_default() {
        let result = RollbackResult::default();
        assert!(result.is_success());
        assert_eq!(result.success_count, 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_rollback_result_with_errors() {
        let result = RollbackResult {
            success_count: 5,
            skip_count: 1,
            error_count: 2,
            errors: vec!["error1".to_string(), "error2".to_string()],
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_load_save_rollback() {
        let rollback = Rollback::default();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("test_rollback.json");

        save_rollback(&rollback, &path).unwrap();
        assert!(path.exists());

        let loaded = load_rollback(&path).unwrap();
        assert_eq!(loaded.version, rollback.version);
    }
}


