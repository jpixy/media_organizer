//! Integration tests for the scanner module.
//!
//! Tests cover:
//! - Directory scanning with video files
//! - Sample folder detection
//! - Error handling for non-existent paths

use media_organizer::core::scanner::scan_directory;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_scan_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let result = scan_directory(temp_dir.path()).unwrap();

    assert_eq!(result.videos.len(), 0);
    assert_eq!(result.samples.len(), 0);
}

#[test]
fn test_scan_with_video_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a mock video file (just an empty file with video extension)
    let video_path = temp_dir.path().join("movie.mkv");
    fs::write(&video_path, "fake video content").unwrap();

    let result = scan_directory(temp_dir.path()).unwrap();

    assert_eq!(result.videos.len(), 1);
    assert_eq!(result.videos[0].filename, "movie.mkv");
    assert!(!result.videos[0].is_sample);
}

#[test]
fn test_scan_with_sample_folder() {
    let temp_dir = TempDir::new().unwrap();

    // Create Sample folder with video
    // Note: Sample folders are now treated as extras and skipped during scanning
    // They will be moved as-is with the parent movie
    let sample_dir = temp_dir.path().join("Sample");
    fs::create_dir(&sample_dir).unwrap();
    fs::write(sample_dir.join("sample.mkv"), "fake sample").unwrap();

    // Create regular video
    fs::write(temp_dir.path().join("movie.mkv"), "fake video").unwrap();

    let result = scan_directory(temp_dir.path()).unwrap();

    // Only the regular movie should be scanned
    // Sample folder videos are skipped (treated as extras)
    assert_eq!(result.videos.len(), 1);
    assert_eq!(result.samples.len(), 0); // Sample folder files are skipped
}

#[test]
fn test_scan_nonexistent_path() {
    let result = scan_directory(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

#[test]
fn test_scan_with_multiple_video_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create videos with different extensions
    fs::write(temp_dir.path().join("movie1.mkv"), "fake").unwrap();
    fs::write(temp_dir.path().join("movie2.mp4"), "fake").unwrap();
    fs::write(temp_dir.path().join("movie3.avi"), "fake").unwrap();
    fs::write(temp_dir.path().join("document.txt"), "not video").unwrap();

    let result = scan_directory(temp_dir.path()).unwrap();

    assert_eq!(result.videos.len(), 3);
    assert!(result.videos.iter().all(|v| !v.is_sample));
}

#[test]
fn test_scan_nested_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested directory structure
    let nested = temp_dir.path().join("Season 01").join("Episode 01");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("video.mkv"), "fake").unwrap();

    let result = scan_directory(temp_dir.path()).unwrap();

    assert_eq!(result.videos.len(), 1);
}

