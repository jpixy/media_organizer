//! Rollback command implementation.
//!
//! Reads a rollback.json file and reverses all operations
//! to restore the original state.

use crate::core::rollback::{self, RollbackExecutor};
use crate::Result;
use colored::Colorize;
use std::path::Path;

/// Execute a rollback.
pub async fn rollback(rollback_file: &Path, dry_run: bool) -> Result<()> {
    println!("{}", "⏪ Rollback command".bold().cyan());
    println!();

    // Validate rollback file exists
    if !rollback_file.exists() {
        return Err(crate::Error::PathNotFound(rollback_file.display().to_string()));
    }

    // Load rollback
    println!("📖 Loading rollback: {}", rollback_file.display());
    let rb = rollback::load_rollback(rollback_file)?;

    // Print rollback info
    println!("  {} {}", "Plan ID:".bold(), rb.plan_id);
    println!("  {} {}", "Executed at:".bold(), rb.executed_at);
    println!("  {} {}", "Operations:".bold(), rb.operations.len());
    println!();

    if dry_run {
        println!("{}", "🔍 Dry run mode - showing what would be done:".bold().yellow());
        println!();
    } else {
        println!("{}", "⚠️  This will reverse all previous operations!".bold().yellow());
        println!();
    }

    // Execute rollback
    let executor = RollbackExecutor::new();
    let result = executor.execute(&rb, dry_run).await?;

    // Print summary
    result.print_summary();
    println!();

    if result.is_success() {
        if dry_run {
            println!("{}", "✅ Dry run complete - no changes were made".green());
        } else {
            println!("{}", "✅ Rollback completed successfully!".green());
        }
    } else {
        println!("{}", "⚠️  Rollback completed with errors".yellow());
    }

    Ok(())
}


