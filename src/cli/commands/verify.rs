//! Verify command implementation.
//!
//! Uses ffprobe to verify video file integrity.

use crate::core::scanner;
use crate::services::ffprobe;
use crate::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Verification result for a single file.
#[derive(Debug)]
struct VerifyResult {
    path: std::path::PathBuf,
    success: bool,
    error: Option<String>,
}

/// Verify video file integrity.
pub async fn verify(path: &Path) -> Result<()> {
    println!("{}", "🔍 Verifying video files...".bold().cyan());
    println!();

    // Check if path exists
    if !path.exists() {
        return Err(crate::Error::PathNotFound(path.display().to_string()));
    }

    let files = if path.is_file() {
        // Single file
        vec![path.to_path_buf()]
    } else {
        // Directory - scan for videos
        println!("📁 Scanning directory: {}", path.display());
        let scan_result = scanner::scan_directory(path)?;
        let mut files: Vec<_> = scan_result.videos.iter().map(|v| v.path.clone()).collect();
        files.extend(scan_result.samples.iter().map(|v| v.path.clone()));
        files
    };

    if files.is_empty() {
        println!("No video files found.");
        return Ok(());
    }

    println!("Found {} video files to verify", files.len());
    println!();

    // Progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for file in &files {
        let filename = file.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        pb.set_message(format!("Verifying: {}", &filename));
        pb.inc(1);

        // Try to extract metadata with ffprobe
        match ffprobe::extract_metadata(file) {
            Ok(metadata) => {
                // Check if we got valid data
                if metadata.resolution != "unknown" && metadata.video_codec != "unknown" {
                    results.push(VerifyResult {
                        path: file.clone(),
                        success: true,
                        error: None,
                    });
                    success_count += 1;
                } else {
                    results.push(VerifyResult {
                        path: file.clone(),
                        success: false,
                        error: Some("Could not extract video/audio streams".to_string()),
                    });
                    error_count += 1;
                }
            }
            Err(e) => {
                results.push(VerifyResult {
                    path: file.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
                error_count += 1;
            }
        }
    }

    pb.finish_with_message("Done!");
    println!();

    // Print summary
    println!("{}", "📊 Verification Summary".bold().green());
    println!("  {} {}", "Valid files:".bold(), success_count);
    println!("  {} {}", "Invalid files:".bold(), error_count);
    println!();

    // Print failed files
    if error_count > 0 {
        println!("{}", "❌ Invalid Files:".bold().red());
        for result in &results {
            if !result.success {
                println!(
                    "  {} - {}",
                    result.path.display(),
                    result.error.as_deref().unwrap_or("Unknown error")
                );
            }
        }
        println!();
    }

    // Final status
    if error_count == 0 {
        println!("{}", "✅ All files verified successfully!".green());
    } else {
        println!(
            "{}",
            format!("⚠️  {} file(s) failed verification", error_count).yellow()
        );
    }

    Ok(())
}


