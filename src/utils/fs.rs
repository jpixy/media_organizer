//! File system utilities.

use crate::Result;
use std::path::Path;

/// Check if a path exists and is a directory.
pub fn ensure_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(crate::Error::PathNotFound(path.display().to_string()));
    }
    if !path.is_dir() {
        return Err(crate::Error::NotADirectory(path.display().to_string()));
    }
    Ok(())
}

/// Create a directory and all parent directories.
pub fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Move a file from one location to another.
pub fn move_file(from: &Path, to: &Path) -> Result<()> {
    // Try rename first (fast, same filesystem)
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }

    // Fall back to copy + delete (cross filesystem)
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)?;
    Ok(())
}

/// Get file extension in lowercase.
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Check if a file is a video file based on extension.
pub fn is_video_file(path: &Path) -> bool {
    const VIDEO_EXTENSIONS: &[&str] = &[
        "mkv", "mp4", "avi", "mov", "wmv", "m4v", "ts", "m2ts", "flv", "webm", "mpg", "mpeg",
    ];

    get_extension(path)
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// Check if a path contains "sample" (case insensitive).
pub fn is_sample(path: &Path) -> bool {
    path.to_string_lossy().to_lowercase().contains("sample")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(&PathBuf::from("movie.mkv")));
        assert!(is_video_file(&PathBuf::from("movie.MP4")));
        assert!(!is_video_file(&PathBuf::from("movie.txt")));
        assert!(!is_video_file(&PathBuf::from("movie.nfo")));
    }

    #[test]
    fn test_is_sample() {
        assert!(is_sample(&PathBuf::from("/path/Sample/video.mkv")));
        assert!(is_sample(&PathBuf::from("/path/video.sample.mkv")));
        assert!(!is_sample(&PathBuf::from("/path/video.mkv")));
    }
}



