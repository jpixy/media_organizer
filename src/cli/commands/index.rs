//! Index command implementation.

use crate::cli::args::IndexAction;
use crate::core::indexer;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Execute index subcommand.
pub async fn execute_index(action: IndexAction) -> Result<()> {
    match action {
        IndexAction::Scan {
            path,
            media_type,
            disk_label,
            force,
        } => scan_directory(&path, &media_type, disk_label, force).await,
        IndexAction::Stats => show_stats().await,
        IndexAction::List {
            disk_label,
            media_type,
        } => list_disk(&disk_label, &media_type).await,
        IndexAction::Verify { path } => verify_index(&path).await,
        IndexAction::Remove {
            disk_label,
            confirm,
        } => remove_disk(&disk_label, confirm).await,
    }
}

/// Scan and index a directory.
async fn scan_directory(
    path: &Path,
    media_type: &str,
    disk_label: Option<String>,
    force: bool,
) -> Result<()> {
    println!("{}", "[INDEX] Scanning directory...".bold().cyan());
    println!("  Path: {}", path.display());
    println!("  Media type: {}", media_type);

    // Detect or use provided disk label
    let label = disk_label.unwrap_or_else(|| {
        indexer::detect_disk_label(path).unwrap_or_else(|| "unknown".to_string())
    });
    println!("  Disk label: {}", label);

    // Get disk UUID
    let uuid = indexer::get_disk_uuid(path);
    if let Some(ref u) = uuid {
        println!("  Disk UUID: {}", u);
    }

    // Check if already indexed
    if !force {
        if let Ok(Some(existing)) = indexer::load_disk_index(&label) {
            println!(
                "{}",
                format!(
                    "[WARN] Disk '{}' already indexed ({} movies, {} TV shows)",
                    label, existing.disk.movie_count, existing.disk.tvshow_count
                )
                .yellow()
            );
            println!("  Use --force to re-index");
        }
    }

    println!();

    // Scan directory
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Scanning for NFO files...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let disk_index = indexer::scan_directory(path, &label, uuid, media_type)?;

    pb.finish_with_message("Scan complete");

    // Save disk index
    indexer::save_disk_index(&disk_index)?;

    // Update central index
    let mut central = indexer::load_central_index()?;
    indexer::merge_disk_into_central(&mut central, disk_index.clone());
    indexer::save_central_index(&central)?;

    // Print summary
    println!();
    println!("{}", "[INDEX] Complete!".bold().green());
    println!("  Movies indexed: {}", disk_index.disk.movie_count);
    println!("  TV shows indexed: {}", disk_index.disk.tvshow_count);
    println!(
        "  Total size: {:.2} GB",
        disk_index.disk.total_size_bytes as f64 / 1_073_741_824.0
    );
    println!();
    println!(
        "  Central index: {} movies, {} TV shows across {} disks",
        central.statistics.total_movies,
        central.statistics.total_tvshows,
        central.statistics.total_disks
    );

    Ok(())
}

/// Show collection statistics.
async fn show_stats() -> Result<()> {
    let index = indexer::load_central_index()?;

    println!("{}", "Media Collection Statistics".bold().cyan());
    println!("{}", "=".repeat(50));
    println!();

    // Disks
    println!("{}", "Disks:".bold());
    for (label, disk) in &index.disks {
        let status = if indexer::is_disk_online(label) {
            "Online".green()
        } else {
            "Offline".red()
        };
        println!(
            "  {} | {} movies | {} TV shows | {:.1} GB | {}",
            label.bold(),
            disk.movie_count,
            disk.tvshow_count,
            disk.total_size_bytes as f64 / 1_073_741_824.0,
            status
        );
    }
    println!("{}", "-".repeat(50));
    println!(
        "  {} | {} movies | {} TV shows | {:.1} GB",
        "Total".bold(),
        index.statistics.total_movies,
        index.statistics.total_tvshows,
        index.statistics.total_size_bytes as f64 / 1_073_741_824.0
    );
    println!();

    // By country
    if !index.statistics.by_country.is_empty() {
        println!("{}", "By Country:".bold());
        let mut countries: Vec<_> = index.statistics.by_country.iter().collect();
        countries.sort_by(|a, b| b.1.cmp(a.1));
        let total = index.statistics.total_movies + index.statistics.total_tvshows;
        for (country, count) in countries.iter().take(10) {
            let pct = **count as f64 / total as f64 * 100.0;
            let bar_len = (pct / 2.0) as usize;
            let bar = "█".repeat(bar_len);
            println!("  {} {:>15} {} ({:.0}%)", country, bar, count, pct);
        }
        println!();
    }

    // By decade
    if !index.statistics.by_decade.is_empty() {
        println!("{}", "By Decade:".bold());
        let mut decades: Vec<_> = index.statistics.by_decade.iter().collect();
        decades.sort_by(|a, b| b.0.cmp(a.0));
        let total = index.statistics.total_movies;
        for (decade, count) in decades.iter().take(5) {
            let pct = **count as f64 / total as f64 * 100.0;
            let bar_len = (pct / 2.0) as usize;
            let bar = "█".repeat(bar_len);
            println!("  {} {:>15} {} ({:.0}%)", decade, bar, count, pct);
        }
        println!();
    }

    // Collections
    println!("{}", "Collections:".bold());
    println!(
        "  Complete: {} collections",
        index.statistics.complete_collections
    );
    println!(
        "  Incomplete: {} collections",
        index.statistics.incomplete_collections
    );

    Ok(())
}

/// List contents of a specific disk.
async fn list_disk(disk_label: &str, media_type: &str) -> Result<()> {
    let index = indexer::load_central_index()?;

    let show_movies = media_type == "all" || media_type == "movies";
    let show_tvshows = media_type == "all" || media_type == "tvshows";

    if show_movies {
        let movies: Vec<_> = index
            .movies
            .iter()
            .filter(|m| m.disk == disk_label)
            .collect();

        if !movies.is_empty() {
            println!("{}", format!("Movies on {} ({}):", disk_label, movies.len()).bold());
            for movie in movies {
                println!(
                    "  [{}] {} ({})",
                    movie.year.map(|y| y.to_string()).unwrap_or_default(),
                    movie.title,
                    movie.country.as_deref().unwrap_or("??")
                );
            }
            println!();
        }
    }

    if show_tvshows {
        let tvshows: Vec<_> = index
            .tvshows
            .iter()
            .filter(|t| t.disk == disk_label)
            .collect();

        if !tvshows.is_empty() {
            println!(
                "{}",
                format!("TV Shows on {} ({}):", disk_label, tvshows.len()).bold()
            );
            for tvshow in tvshows {
                println!(
                    "  [{}] {} - {} episodes",
                    tvshow.year.map(|y| y.to_string()).unwrap_or_default(),
                    tvshow.title,
                    tvshow.episodes
                );
            }
        }
    }

    Ok(())
}

/// Verify index against actual files.
async fn verify_index(path: &Path) -> Result<()> {
    println!("{}", "[VERIFY] Verifying index...".bold().cyan());

    let label = indexer::detect_disk_label(path).unwrap_or_else(|| "unknown".to_string());
    println!("  Disk: {}", label);

    let index = indexer::load_central_index()?;
    let movies: Vec<_> = index.movies.iter().filter(|m| m.disk == label).collect();
    let tvshows: Vec<_> = index.tvshows.iter().filter(|t| t.disk == label).collect();

    let mut valid = 0;
    let mut missing = 0;

    for movie in &movies {
        let movie_path = path.join(&movie.relative_path);
        if movie_path.exists() {
            valid += 1;
        } else {
            missing += 1;
            println!("  [MISSING] {}", movie.title);
        }
    }

    for tvshow in &tvshows {
        let tvshow_path = path.join(&tvshow.relative_path);
        if tvshow_path.exists() {
            valid += 1;
        } else {
            missing += 1;
            println!("  [MISSING] {}", tvshow.title);
        }
    }

    println!();
    if missing == 0 {
        println!(
            "{}",
            format!("[OK] All {} entries valid", valid).bold().green()
        );
    } else {
        println!(
            "{}",
            format!("[WARN] {} valid, {} missing", valid, missing)
                .bold()
                .yellow()
        );
    }

    Ok(())
}

/// Remove a disk from the index.
async fn remove_disk(disk_label: &str, confirm: bool) -> Result<()> {
    if !confirm {
        println!(
            "{}",
            format!(
                "[WARN] This will remove all entries for disk '{}' from the index.",
                disk_label
            )
            .yellow()
        );
        println!("  Use --confirm to proceed");
        return Ok(());
    }

    let mut index = indexer::load_central_index()?;

    let movies_before = index.movies.len();
    let tvshows_before = index.tvshows.len();

    index.movies.retain(|m| m.disk != disk_label);
    index.tvshows.retain(|t| t.disk != disk_label);
    index.disks.remove(disk_label);

    let movies_removed = movies_before - index.movies.len();
    let tvshows_removed = tvshows_before - index.tvshows.len();

    index.rebuild_indexes();
    index.update_statistics();
    indexer::save_central_index(&index)?;

    // Remove disk index file
    let disk_index_path = indexer::disk_indexes_dir()?.join(format!("{}.json", disk_label));
    if disk_index_path.exists() {
        std::fs::remove_file(&disk_index_path)?;
    }

    println!(
        "{}",
        format!(
            "[OK] Removed disk '{}': {} movies, {} TV shows",
            disk_label, movies_removed, tvshows_removed
        )
        .bold()
        .green()
    );

    Ok(())
}

