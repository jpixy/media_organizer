//! Plan command implementation.
//!
//! Implements the `plan movies` and `plan tvshows` subcommands.
//! Coordinates scanning, parsing, TMDB lookup, and plan generation.

use crate::core::planner::{self, Planner};
use crate::models::media::MediaType;
use crate::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Execute the plan command for movies.
pub async fn plan_movies(
    source: &Path,
    target: Option<&Path>,
    output: Option<&Path>,
) -> Result<()> {
    println!("{}", "🎬 Planning movies organization...".bold().cyan());
    println!();

    plan_media(source, target, output, MediaType::Movies).await
}

/// Execute the plan command for TV shows.
pub async fn plan_tvshows(
    source: &Path,
    target: Option<&Path>,
    output: Option<&Path>,
) -> Result<()> {
    println!("{}", "📺 Planning TV shows organization...".bold().cyan());
    println!();

    plan_media(source, target, output, MediaType::TvShows).await
}

/// Common planning logic for both movies and TV shows.
async fn plan_media(
    source: &Path,
    target: Option<&Path>,
    output: Option<&Path>,
    media_type: MediaType,
) -> Result<()> {
    // Validate source path
    if !source.exists() {
        return Err(crate::Error::PathNotFound(source.display().to_string()));
    }
    if !source.is_dir() {
        return Err(crate::Error::NotADirectory(source.display().to_string()));
    }

    // Determine target path
    let target_path = match target {
        Some(t) => t.to_path_buf(),
        None => {
            // Default: create _organized directory next to source
            let source_name = source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("videos");
            let organized_name = format!("{}_organized", source_name);
            source
                .parent()
                .map(|p| p.join(organized_name))
                .unwrap_or_else(|| PathBuf::from(format!("{}_organized", source_name)))
        }
    };

    // Print configuration
    println!("  {} {}", "Source:".bold(), source.display());
    println!("  {} {}", "Target:".bold(), target_path.display());
    println!("  {} {}", "Type:".bold(), media_type);
    println!();

    // Create planner and generate plan
    let planner = Planner::new()?;
    let plan = planner.generate(source, &target_path, media_type).await?;

    // Print summary
    println!();
    println!("{}", "📋 Plan Summary".bold().green());
    println!("  {} {}", "Videos to organize:".bold(), plan.items.len());
    println!("  {} {}", "Sample files:".bold(), plan.samples.len());
    println!("  {} {}", "Unknown/failed:".bold(), plan.unknown.len());

    // Calculate total operations
    let total_ops: usize = plan.items.iter().map(|i| i.operations.len()).sum();
    println!("  {} {}", "Total operations:".bold(), total_ops);
    println!();

    // Ensure target directory exists before saving plan
    if !target_path.exists() {
        std::fs::create_dir_all(&target_path)?;
    }

    // Determine output path (prefer target directory)
    let output_path = match output {
        Some(o) => o.to_path_buf(),
        None => planner::default_plan_path(source, Some(&target_path)),
    };

    // Save plan
    planner::save_plan(&plan, &output_path)?;
    println!(
        "{} {}",
        "✅ Plan saved to:".bold().green(),
        output_path.display()
    );

    // Save to sessions
    match planner::save_to_sessions(&plan) {
        Ok(session_dir) => {
            println!(
                "{} {}",
                "📁 Session saved to:".bold(),
                session_dir.display()
            );
        }
        Err(e) => {
            tracing::warn!("Failed to save session: {}", e);
        }
    }

    // Print next steps
    println!();
    println!("{}", "📝 Next Steps:".bold().yellow());
    println!(
        "  1. Review the plan: {}",
        format!("cat {}", output_path.display()).cyan()
    );
    println!(
        "  2. Execute the plan: {}",
        format!("media-organizer execute {}", output_path.display()).cyan()
    );

    // Warn about unknown files
    if !plan.unknown.is_empty() {
        println!();
        println!("{}", "⚠️  Unknown Files:".bold().yellow());
        for item in &plan.unknown {
            println!(
                "  {} - {}",
                item.source.filename.red(),
                item.reason
            );
        }
    }

    Ok(())
}


