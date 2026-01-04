//! Central index management - scanning, building, and searching.

use crate::models::index::{
    CentralIndex, CollectionInfo, DiskIndex, MovieEntry, TvShowEntry,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration directory path.
fn config_dir() -> Result<PathBuf> {
    let config = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("media_organizer");
    Ok(config)
}

/// Path to central index file.
pub fn central_index_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("central_index.json"))
}

/// Path to disk indexes directory.
pub fn disk_indexes_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("disk_indexes"))
}

/// Load central index from disk.
pub fn load_central_index() -> Result<CentralIndex> {
    let path = central_index_path()?;
    if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read central index: {}", path.display()))?;
        let index: CentralIndex = serde_json::from_str(&content)
            .with_context(|| "Failed to parse central index")?;
        Ok(index)
    } else {
        Ok(CentralIndex::default())
    }
}

/// Save central index to disk.
pub fn save_central_index(index: &CentralIndex) -> Result<()> {
    let path = central_index_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Backup existing file
    if path.exists() {
        let backup_path = path.with_extension("json.backup");
        fs::copy(&path, &backup_path)?;
    }
    
    let content = serde_json::to_string_pretty(index)?;
    fs::write(&path, content)?;
    
    tracing::info!("Central index saved to: {}", path.display());
    Ok(())
}

/// Load disk index for a specific disk.
pub fn load_disk_index(disk_label: &str) -> Result<Option<DiskIndex>> {
    let path = disk_indexes_dir()?.join(format!("{}.json", disk_label));
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let index: DiskIndex = serde_json::from_str(&content)?;
        Ok(Some(index))
    } else {
        Ok(None)
    }
}

/// Save disk index to disk.
pub fn save_disk_index(index: &DiskIndex) -> Result<()> {
    let dir = disk_indexes_dir()?;
    fs::create_dir_all(&dir)?;
    
    let path = dir.join(format!("{}.json", index.disk.label));
    let content = serde_json::to_string_pretty(index)?;
    fs::write(&path, content)?;
    
    tracing::info!("Disk index saved to: {}", path.display());
    Ok(())
}

/// Detect disk label from mount path.
/// 
/// For paths like `/run/media/johnny/JMedia_M05/Movies`, returns "JMedia_M05".
pub fn detect_disk_label(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    
    // Pattern: /run/media/<user>/<label>/...
    if path_str.starts_with("/run/media/") {
        let parts: Vec<&str> = path_str.split('/').collect();
        if parts.len() >= 5 {
            return Some(parts[4].to_string());
        }
    }
    
    // Pattern: /media/<user>/<label>/...
    if path_str.starts_with("/media/") {
        let parts: Vec<&str> = path_str.split('/').collect();
        if parts.len() >= 4 {
            return Some(parts[3].to_string());
        }
    }
    
    // Pattern: /mnt/<label>/...
    if path_str.starts_with("/mnt/") {
        let parts: Vec<&str> = path_str.split('/').collect();
        if parts.len() >= 3 {
            return Some(parts[2].to_string());
        }
    }
    
    // Fallback: use directory name
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

/// Get disk UUID using lsblk command.
pub fn get_disk_uuid(path: &Path) -> Option<String> {
    // Try to get UUID using df and blkid
    let output = std::process::Command::new("df")
        .arg(path)
        .output()
        .ok()?;
    
    let df_output = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = df_output.lines().collect();
    if lines.len() < 2 {
        return None;
    }
    
    let device = lines[1].split_whitespace().next()?;
    
    let blkid_output = std::process::Command::new("blkid")
        .arg("-s")
        .arg("UUID")
        .arg("-o")
        .arg("value")
        .arg(device)
        .output()
        .ok()?;
    
    let uuid = String::from_utf8_lossy(&blkid_output.stdout)
        .trim()
        .to_string();
    
    if uuid.is_empty() {
        None
    } else {
        Some(uuid)
    }
}

/// Check if a disk is currently mounted/online.
pub fn is_disk_online(disk_label: &str) -> bool {
    // Check common mount points
    let paths = [
        format!("/run/media/{}/{}", whoami::username(), disk_label),
        format!("/media/{}/{}", whoami::username(), disk_label),
        format!("/mnt/{}", disk_label),
    ];
    
    for path in &paths {
        if Path::new(path).exists() {
            return true;
        }
    }
    
    false
}

/// Scan a directory for NFO files and build index entries.
pub fn scan_directory(
    path: &Path,
    disk_label: &str,
    disk_uuid: Option<String>,
    media_type: &str,
) -> Result<DiskIndex> {
    tracing::info!("Scanning directory: {}", path.display());
    
    let mut index = DiskIndex::default();
    index.disk.label = disk_label.to_string();
    index.disk.uuid = disk_uuid.clone();
    index.disk.base_path = path.to_string_lossy().to_string();
    index.disk.last_indexed = chrono::Utc::now().to_rfc3339();
    
    // Store path by media type for composite storage support
    index.disk.paths.insert(
        media_type.to_string(),
        path.to_string_lossy().to_string()
    );
    
    let nfo_pattern = if media_type == "movies" {
        "movie.nfo"
    } else {
        "tvshow.nfo"
    };
    
    let mut total_size: u64 = 0;
    
    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        
        if entry_path.is_file() {
            if let Some(filename) = entry_path.file_name() {
                if filename == nfo_pattern {
                    match parse_nfo_file(entry_path, disk_label, &index.disk.uuid, path) {
                        Ok(ParsedNfo::Movie(movie)) => {
                            total_size += movie.size_bytes;
                            index.movies.push(movie);
                        }
                        Ok(ParsedNfo::TvShow(tvshow)) => {
                            total_size += tvshow.size_bytes;
                            index.tvshows.push(tvshow);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse NFO {}: {}", entry_path.display(), e);
                        }
                    }
                }
            }
        }
    }
    
    index.disk.movie_count = index.movies.len();
    index.disk.tvshow_count = index.tvshows.len();
    index.disk.total_size_bytes = total_size;
    
    tracing::info!(
        "Scan complete: {} movies, {} TV shows",
        index.movies.len(),
        index.tvshows.len()
    );
    
    Ok(index)
}

/// Parsed NFO result.
enum ParsedNfo {
    Movie(MovieEntry),
    TvShow(TvShowEntry),
}

/// Parse a movie.nfo or tvshow.nfo file.
fn parse_nfo_file(
    nfo_path: &Path,
    disk_label: &str,
    disk_uuid: &Option<String>,
    base_path: &Path,
) -> Result<ParsedNfo> {
    let content = fs::read_to_string(nfo_path)?;
    let nfo_dir = nfo_path.parent().context("NFO has no parent directory")?;
    
    // Calculate relative path
    let relative_path = nfo_dir
        .strip_prefix(base_path)
        .unwrap_or(nfo_dir)
        .to_string_lossy()
        .to_string();
    
    // Calculate total size of video files in directory
    let size_bytes = calculate_directory_video_size(nfo_dir);
    
    // Determine if movie or tvshow based on root element
    if content.contains("<movie>") {
        let movie = parse_movie_nfo(&content, disk_label, disk_uuid, &relative_path, size_bytes)?;
        Ok(ParsedNfo::Movie(movie))
    } else if content.contains("<tvshow>") {
        let tvshow = parse_tvshow_nfo(&content, disk_label, disk_uuid, &relative_path, size_bytes)?;
        Ok(ParsedNfo::TvShow(tvshow))
    } else {
        anyhow::bail!("Unknown NFO format");
    }
}

/// Parse movie NFO content.
fn parse_movie_nfo(
    content: &str,
    disk_label: &str,
    disk_uuid: &Option<String>,
    relative_path: &str,
    size_bytes: u64,
) -> Result<MovieEntry> {
    // Simple XML parsing using regex (for robustness with malformed XML)
    let get_tag = |tag: &str| -> Option<String> {
        let pattern = format!(r"<{}>(.*?)</{}>", tag, tag);
        regex::Regex::new(&pattern)
            .ok()?
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };
    
    let get_all_tags = |tag: &str| -> Vec<String> {
        let pattern = format!(r"<{}>(.*?)</{}>", tag, tag);
        regex::Regex::new(&pattern)
            .map(|re| {
                re.captures_iter(content)
                    .filter_map(|c| c.get(1))
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    
    let title = get_tag("title").unwrap_or_else(|| "Unknown".to_string());
    let original_title = get_tag("originaltitle");
    let year = get_tag("year").and_then(|y| y.parse().ok());
    
    // TMDB ID from uniqueid or tmdbid tag
    let tmdb_id = get_tag("tmdbid")
        .or_else(|| {
            // Try to find <uniqueid type="tmdb">
            let pattern = r#"<uniqueid[^>]*type="tmdb"[^>]*>(\d+)</uniqueid>"#;
            regex::Regex::new(pattern)
                .ok()?
                .captures(content)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        })
        .and_then(|id| id.parse().ok());
    
    // IMDB ID
    let imdb_id = get_tag("imdbid").or_else(|| {
        let pattern = r#"<uniqueid[^>]*type="imdb"[^>]*>(tt\d+)</uniqueid>"#;
        regex::Regex::new(pattern)
            .ok()?
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    });
    
    // Collection info
    let collection_id = get_tag("tmdbcollectionid").and_then(|id| id.parse().ok());
    // First try <set><name>...</name></set> format (nested structure)
    let collection_name = {
        let pattern = r"(?s)<set>\s*<name>(.*?)</name>";
        regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(content))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty())
    }.or_else(|| {
        // Fallback: simple <set>name</set> format (flat structure)
        get_tag("set").filter(|s| !s.contains('<') && !s.is_empty())
    });
    
    // Collection total movies (from <set><totalmovies>N</totalmovies></set>)
    let collection_total_movies = {
        let pattern = r"(?s)<set>.*?<totalmovies>(\d+)</totalmovies>";
        regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(content))
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse().ok())
    };
    
    // Country
    let country = get_tag("country").map(|c| {
        // Convert full country name to code if needed
        country_name_to_code(&c)
    });
    
    let genres = get_all_tags("genre");
    let actors = get_all_tags("actor")
        .into_iter()
        .flat_map(|a| {
            // Try to extract name from <actor><name>...</name></actor>
            let pattern = r"<name>(.*?)</name>";
            regex::Regex::new(pattern)
                .ok()
                .and_then(|re| re.captures(&a))
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .or(Some(a))
        })
        .collect();
    
    let directors = get_all_tags("director");
    let runtime = get_tag("runtime").and_then(|r| r.parse().ok());
    let rating = get_tag("rating").and_then(|r| r.parse().ok());
    
    // Resolution from video info or filename
    let resolution = get_tag("resolution");
    
    Ok(MovieEntry {
        id: uuid::Uuid::new_v4().to_string(),
        disk: disk_label.to_string(),
        disk_uuid: disk_uuid.clone(),
        relative_path: relative_path.to_string(),
        title,
        original_title,
        year,
        tmdb_id,
        imdb_id,
        collection_id,
        collection_name,
        collection_total_movies,
        country,
        genres,
        actors,
        directors,
        runtime,
        rating,
        size_bytes,
        resolution,
        indexed_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Parse tvshow NFO content.
fn parse_tvshow_nfo(
    content: &str,
    disk_label: &str,
    disk_uuid: &Option<String>,
    relative_path: &str,
    size_bytes: u64,
) -> Result<TvShowEntry> {
    let get_tag = |tag: &str| -> Option<String> {
        let pattern = format!(r"<{}>(.*?)</{}>", tag, tag);
        regex::Regex::new(&pattern)
            .ok()?
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };
    
    let get_all_tags = |tag: &str| -> Vec<String> {
        let pattern = format!(r"<{}>(.*?)</{}>", tag, tag);
        regex::Regex::new(&pattern)
            .map(|re| {
                re.captures_iter(content)
                    .filter_map(|c| c.get(1))
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    
    let title = get_tag("title").unwrap_or_else(|| "Unknown".to_string());
    let original_title = get_tag("originaltitle");
    let year = get_tag("year")
        .or_else(|| get_tag("premiered").map(|p| p[..4].to_string()))
        .and_then(|y| y.parse().ok());
    
    let tmdb_id = get_tag("tmdbid")
        .or_else(|| {
            let pattern = r#"<uniqueid[^>]*type="tmdb"[^>]*>(\d+)</uniqueid>"#;
            regex::Regex::new(pattern)
                .ok()?
                .captures(content)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        })
        .and_then(|id| id.parse().ok());
    
    let imdb_id = get_tag("imdbid").or_else(|| {
        let pattern = r#"<uniqueid[^>]*type="imdb"[^>]*>(tt\d+)</uniqueid>"#;
        regex::Regex::new(pattern)
            .ok()?
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    });
    
    let country = get_tag("country").map(|c| country_name_to_code(&c));
    let genres = get_all_tags("genre");
    
    let actors: Vec<String> = get_all_tags("actor")
        .into_iter()
        .flat_map(|a| {
            let pattern = r"<name>(.*?)</name>";
            regex::Regex::new(pattern)
                .ok()
                .and_then(|re| re.captures(&a))
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .or(Some(a))
        })
        .collect();
    
    let seasons = get_tag("season").and_then(|s| s.parse().ok()).unwrap_or(1);
    let episodes = get_tag("episode").and_then(|e| e.parse().ok()).unwrap_or(0);
    
    Ok(TvShowEntry {
        id: uuid::Uuid::new_v4().to_string(),
        disk: disk_label.to_string(),
        disk_uuid: disk_uuid.clone(),
        relative_path: relative_path.to_string(),
        title,
        original_title,
        year,
        tmdb_id,
        imdb_id,
        country,
        genres,
        actors,
        seasons,
        episodes,
        size_bytes,
        indexed_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Calculate total size of video files in a directory (recursive).
fn calculate_directory_video_size(dir: &Path) -> u64 {
    let video_extensions = ["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts"];
    
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| video_extensions.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Convert country name to ISO 3166-1 alpha-2 code.
fn country_name_to_code(name: &str) -> String {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        "united states" | "usa" | "united states of america" => "US".to_string(),
        "china" | "中国" => "CN".to_string(),
        "united kingdom" | "uk" | "great britain" => "GB".to_string(),
        "japan" | "日本" => "JP".to_string(),
        "korea" | "south korea" | "韩国" => "KR".to_string(),
        "france" | "法国" => "FR".to_string(),
        "germany" | "德国" => "DE".to_string(),
        "india" | "印度" => "IN".to_string(),
        "italy" | "意大利" => "IT".to_string(),
        "spain" | "西班牙" => "ES".to_string(),
        "canada" | "加拿大" => "CA".to_string(),
        "australia" | "澳大利亚" => "AU".to_string(),
        "russia" | "俄罗斯" => "RU".to_string(),
        "hong kong" | "香港" => "HK".to_string(),
        "taiwan" | "台湾" => "TW".to_string(),
        // Unknown country name or already a 2-letter code - return as-is
        _ => name.to_string(),
    }
}

/// Merge disk index into central index.
/// 
/// Supports composite storage: if a disk already exists in the central index,
/// the new scan is merged by media type instead of completely replacing it.
/// This allows one disk label to have both movies and tvshows with different paths.
pub fn merge_disk_into_central(central: &mut CentralIndex, disk: DiskIndex) {
    let label = disk.disk.label.clone();
    
    // Determine what media types are being added in this scan
    let has_movies = !disk.movies.is_empty();
    let has_tvshows = !disk.tvshows.is_empty();
    
    // Update or merge disk info
    if let Some(existing_disk) = central.disks.get_mut(&label) {
        // Merge: keep existing paths, add new ones
        for (media_type, path) in &disk.disk.paths {
            existing_disk.paths.insert(media_type.clone(), path.clone());
        }
        // Update timestamp
        existing_disk.last_indexed = disk.disk.last_indexed.clone();
        // Update UUID if provided
        if disk.disk.uuid.is_some() {
            existing_disk.uuid = disk.disk.uuid.clone();
        }
        
        tracing::info!(
            "Merging into existing disk '{}': movies={}, tvshows={}",
            label, has_movies, has_tvshows
        );
    } else {
        // New disk: insert directly
        central.disks.insert(label.clone(), disk.disk.clone());
        tracing::info!(
            "Adding new disk '{}': movies={}, tvshows={}",
            label, has_movies, has_tvshows
        );
    }
    
    // Remove old entries ONLY for the media types being updated
    // This is the key change: we don't remove all entries, just the ones being replaced
    if has_movies {
        central.movies.retain(|m| m.disk != label);
        central.movies.extend(disk.movies);
    }
    
    if has_tvshows {
        central.tvshows.retain(|t| t.disk != label);
        central.tvshows.extend(disk.tvshows);
    }
    
    // Update disk counts in the disk info
    if let Some(disk_info) = central.disks.get_mut(&label) {
        disk_info.movie_count = central.movies.iter().filter(|m| m.disk == label).count();
        disk_info.tvshow_count = central.tvshows.iter().filter(|t| t.disk == label).count();
        disk_info.total_size_bytes = 
            central.movies.iter().filter(|m| m.disk == label).map(|m| m.size_bytes).sum::<u64>() +
            central.tvshows.iter().filter(|t| t.disk == label).map(|t| t.size_bytes).sum::<u64>();
    }
    
    // Rebuild indexes and update statistics
    central.rebuild_indexes();
    central.update_statistics();
    central.updated_at = chrono::Utc::now().to_rfc3339();
}

/// Search results container.
#[derive(Debug)]
pub struct SearchResults {
    pub movies: Vec<MovieEntry>,
    pub tvshows: Vec<TvShowEntry>,
    pub collections: Vec<CollectionInfo>,
}

/// Search the central index.
#[allow(clippy::too_many_arguments)]
pub fn search(
    index: &CentralIndex,
    title: Option<&str>,
    actor: Option<&str>,
    director: Option<&str>,
    collection: Option<&str>,
    year: Option<u16>,
    year_range: Option<(u16, u16)>,
    genre: Option<&str>,
    country: Option<&str>,
) -> SearchResults {
    let mut movie_ids: Option<std::collections::HashSet<String>> = None;
    let mut tvshow_ids: Option<std::collections::HashSet<String>> = None;
    
    // Helper to intersect sets
    fn intersect(
        existing: &mut Option<std::collections::HashSet<String>>,
        new: std::collections::HashSet<String>,
    ) {
        match existing {
            Some(set) => {
                set.retain(|id| new.contains(id));
            }
            None => {
                *existing = Some(new);
            }
        }
    }
    
    // Search by actor
    if let Some(actor_name) = actor {
        let actor_lower = actor_name.to_lowercase();
        let ids: std::collections::HashSet<String> = index
            .indexes
            .by_actor
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&actor_lower))
            .flat_map(|(_, ids)| ids.clone())
            .collect();
        intersect(&mut movie_ids, ids.clone());
        intersect(&mut tvshow_ids, ids);
    }
    
    // Search by director
    if let Some(director_name) = director {
        let director_lower = director_name.to_lowercase();
        let ids: std::collections::HashSet<String> = index
            .indexes
            .by_director
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&director_lower))
            .flat_map(|(_, ids)| ids.clone())
            .collect();
        intersect(&mut movie_ids, ids);
    }
    
    // Search by genre
    if let Some(genre_name) = genre {
        let genre_lower = genre_name.to_lowercase();
        let ids: std::collections::HashSet<String> = index
            .indexes
            .by_genre
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&genre_lower))
            .flat_map(|(_, ids)| ids.clone())
            .collect();
        intersect(&mut movie_ids, ids.clone());
        intersect(&mut tvshow_ids, ids);
    }
    
    // Search by country
    if let Some(country_code) = country {
        let country_upper = country_code.to_uppercase();
        if let Some(ids) = index.indexes.by_country.get(&country_upper) {
            let id_set: std::collections::HashSet<String> = ids.iter().cloned().collect();
            intersect(&mut movie_ids, id_set.clone());
            intersect(&mut tvshow_ids, id_set);
        } else {
            movie_ids = Some(std::collections::HashSet::new());
            tvshow_ids = Some(std::collections::HashSet::new());
        }
    }
    
    // Search by year or year range
    if let Some(y) = year {
        if let Some(ids) = index.indexes.by_year.get(&y) {
            let id_set: std::collections::HashSet<String> = ids.iter().cloned().collect();
            intersect(&mut movie_ids, id_set.clone());
            intersect(&mut tvshow_ids, id_set);
        } else {
            movie_ids = Some(std::collections::HashSet::new());
            tvshow_ids = Some(std::collections::HashSet::new());
        }
    } else if let Some((start, end)) = year_range {
        let ids: std::collections::HashSet<String> = (start..=end)
            .flat_map(|y| index.indexes.by_year.get(&y).cloned().unwrap_or_default())
            .collect();
        intersect(&mut movie_ids, ids.clone());
        intersect(&mut tvshow_ids, ids);
    }
    
    // Get movies
    let mut movies: Vec<MovieEntry> = if let Some(ref ids) = movie_ids {
        index
            .movies
            .iter()
            .filter(|m| ids.contains(&m.id))
            .cloned()
            .collect()
    } else {
        index.movies.clone()
    };
    
    // Get TV shows
    let mut tvshows: Vec<TvShowEntry> = if let Some(ref ids) = tvshow_ids {
        index
            .tvshows
            .iter()
            .filter(|t| ids.contains(&t.id))
            .cloned()
            .collect()
    } else {
        index.tvshows.clone()
    };
    
    // Filter by title
    if let Some(title_query) = title {
        let query_lower = title_query.to_lowercase();
        movies.retain(|m| {
            m.title.to_lowercase().contains(&query_lower)
                || m.original_title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
        });
        tvshows.retain(|t| {
            t.title.to_lowercase().contains(&query_lower)
                || t.original_title
                    .as_ref()
                    .map(|title| title.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
        });
    }
    
    // Sort by year descending
    movies.sort_by(|a, b| b.year.cmp(&a.year));
    tvshows.sort_by(|a, b| b.year.cmp(&a.year));
    
    // Search collections
    let collections: Vec<CollectionInfo> = if let Some(collection_query) = collection {
        let query_lower = collection_query.to_lowercase();
        index
            .collections
            .values()
            .filter(|c| c.name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    
    SearchResults {
        movies,
        tvshows,
        collections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::index::{CentralIndex, DiskIndex, DiskInfo, MovieEntry, TvShowEntry};
    use std::collections::HashMap;
    
    /// Create a test DiskIndex with movies
    fn create_movie_disk_index(label: &str, path: &str, movies: Vec<MovieEntry>) -> DiskIndex {
        let mut paths = HashMap::new();
        paths.insert("movies".to_string(), path.to_string());
        
        DiskIndex {
            version: "1.0".to_string(),
            disk: DiskInfo {
                label: label.to_string(),
                uuid: Some("test-uuid".to_string()),
                last_indexed: chrono::Utc::now().to_rfc3339(),
                movie_count: movies.len(),
                tvshow_count: 0,
                total_size_bytes: movies.iter().map(|m| m.size_bytes).sum(),
                base_path: path.to_string(),
                paths,
            },
            movies,
            tvshows: Vec::new(),
        }
    }
    
    /// Create a test DiskIndex with TV shows
    fn create_tvshow_disk_index(label: &str, path: &str, tvshows: Vec<TvShowEntry>) -> DiskIndex {
        let mut paths = HashMap::new();
        paths.insert("tvshows".to_string(), path.to_string());
        
        DiskIndex {
            version: "1.0".to_string(),
            disk: DiskInfo {
                label: label.to_string(),
                uuid: Some("test-uuid".to_string()),
                last_indexed: chrono::Utc::now().to_rfc3339(),
                movie_count: 0,
                tvshow_count: tvshows.len(),
                total_size_bytes: tvshows.iter().map(|t| t.size_bytes).sum(),
                base_path: path.to_string(),
                paths,
            },
            movies: Vec::new(),
            tvshows,
        }
    }
    
    /// Create a test movie entry
    fn create_test_movie(id: &str, title: &str, disk: &str, tmdb_id: u64) -> MovieEntry {
        MovieEntry {
            id: id.to_string(),
            disk: disk.to_string(),
            disk_uuid: Some("test-uuid".to_string()),
            relative_path: format!("{}/movie.nfo", title),
            title: title.to_string(),
            original_title: None,
            year: Some(2024),
            tmdb_id: Some(tmdb_id),
            imdb_id: None,
            collection_id: None,
            collection_name: None,
            collection_total_movies: None,
            country: Some("US".to_string()),
            genres: vec!["Action".to_string()],
            actors: vec!["Actor1".to_string()],
            directors: vec!["Director1".to_string()],
            runtime: Some(120),
            rating: Some(7.5),
            size_bytes: 1_000_000_000,
            resolution: Some("1080p".to_string()),
            indexed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    /// Create a test TV show entry
    fn create_test_tvshow(id: &str, title: &str, disk: &str, tmdb_id: u64) -> TvShowEntry {
        TvShowEntry {
            id: id.to_string(),
            disk: disk.to_string(),
            disk_uuid: Some("test-uuid".to_string()),
            relative_path: format!("{}/tvshow.nfo", title),
            title: title.to_string(),
            original_title: None,
            year: Some(2024),
            tmdb_id: Some(tmdb_id),
            imdb_id: None,
            country: Some("US".to_string()),
            genres: vec!["Drama".to_string()],
            actors: vec!["Actor1".to_string()],
            seasons: 3,
            episodes: 24,
            size_bytes: 5_000_000_000,
            indexed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    #[test]
    fn test_merge_disk_into_central_new_disk() {
        let mut central = CentralIndex::default();
        
        let movies = vec![
            create_test_movie("m1", "Movie 1", "TestDisk", 1001),
            create_test_movie("m2", "Movie 2", "TestDisk", 1002),
        ];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        
        merge_disk_into_central(&mut central, disk);
        
        assert_eq!(central.disks.len(), 1);
        assert_eq!(central.movies.len(), 2);
        assert_eq!(central.tvshows.len(), 0);
        
        let disk_info = central.disks.get("TestDisk").unwrap();
        assert_eq!(disk_info.movie_count, 2);
        assert_eq!(disk_info.tvshow_count, 0);
        assert!(disk_info.paths.contains_key("movies"));
    }
    
    #[test]
    fn test_merge_disk_into_central_composite_storage() {
        let mut central = CentralIndex::default();
        
        // First: add movies
        let movies = vec![
            create_test_movie("m1", "Movie 1", "TestDisk", 1001),
        ];
        let movie_disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, movie_disk);
        
        assert_eq!(central.movies.len(), 1);
        assert_eq!(central.tvshows.len(), 0);
        
        // Second: add tvshows (same disk label, different path)
        let tvshows = vec![
            create_test_tvshow("t1", "TV Show 1", "TestDisk", 2001),
        ];
        let tvshow_disk = create_tvshow_disk_index("TestDisk", "/mnt/TestDisk/TVShows", tvshows);
        merge_disk_into_central(&mut central, tvshow_disk);
        
        // Verify composite storage works
        assert_eq!(central.disks.len(), 1, "Should still be one disk");
        assert_eq!(central.movies.len(), 1, "Movies should be preserved");
        assert_eq!(central.tvshows.len(), 1, "TV shows should be added");
        
        let disk_info = central.disks.get("TestDisk").unwrap();
        assert_eq!(disk_info.movie_count, 1);
        assert_eq!(disk_info.tvshow_count, 1);
        assert!(disk_info.paths.contains_key("movies"), "Movies path should exist");
        assert!(disk_info.paths.contains_key("tvshows"), "TVShows path should exist");
        assert_eq!(disk_info.paths.get("movies").unwrap(), "/mnt/TestDisk/Movies");
        assert_eq!(disk_info.paths.get("tvshows").unwrap(), "/mnt/TestDisk/TVShows");
    }
    
    #[test]
    fn test_merge_disk_into_central_update_movies_only() {
        let mut central = CentralIndex::default();
        
        // Initial: add movies and tvshows
        let movies = vec![
            create_test_movie("m1", "Movie 1", "TestDisk", 1001),
        ];
        let movie_disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, movie_disk);
        
        let tvshows = vec![
            create_test_tvshow("t1", "TV Show 1", "TestDisk", 2001),
        ];
        let tvshow_disk = create_tvshow_disk_index("TestDisk", "/mnt/TestDisk/TVShows", tvshows);
        merge_disk_into_central(&mut central, tvshow_disk);
        
        assert_eq!(central.movies.len(), 1);
        assert_eq!(central.tvshows.len(), 1);
        
        // Update: re-scan movies with new movie
        let new_movies = vec![
            create_test_movie("m1", "Movie 1", "TestDisk", 1001),
            create_test_movie("m2", "Movie 2", "TestDisk", 1002),
        ];
        let updated_movie_disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", new_movies);
        merge_disk_into_central(&mut central, updated_movie_disk);
        
        // Verify: movies updated, tvshows preserved
        assert_eq!(central.movies.len(), 2, "Movies should be updated to 2");
        assert_eq!(central.tvshows.len(), 1, "TV shows should be preserved");
        
        let disk_info = central.disks.get("TestDisk").unwrap();
        assert_eq!(disk_info.movie_count, 2);
        assert_eq!(disk_info.tvshow_count, 1);
    }
    
    #[test]
    fn test_merge_disk_into_central_separate_disks() {
        let mut central = CentralIndex::default();
        
        // Disk 1: movies
        let movies = vec![
            create_test_movie("m1", "Movie 1", "Disk1", 1001),
        ];
        let disk1 = create_movie_disk_index("Disk1", "/mnt/Disk1/Movies", movies);
        merge_disk_into_central(&mut central, disk1);
        
        // Disk 2: different disk, movies
        let movies2 = vec![
            create_test_movie("m2", "Movie 2", "Disk2", 1002),
        ];
        let disk2 = create_movie_disk_index("Disk2", "/mnt/Disk2/Movies", movies2);
        merge_disk_into_central(&mut central, disk2);
        
        assert_eq!(central.disks.len(), 2);
        assert_eq!(central.movies.len(), 2);
        
        // Verify each disk has its own entry
        assert!(central.disks.contains_key("Disk1"));
        assert!(central.disks.contains_key("Disk2"));
    }
    
    #[test]
    fn test_search_by_title() {
        let mut central = CentralIndex::default();
        
        let movies = vec![
            create_test_movie("m1", "The Matrix", "TestDisk", 1001),
            create_test_movie("m2", "Inception", "TestDisk", 1002),
        ];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            Some("matrix"),
            None, None, None, None, None, None, None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "The Matrix");
    }
    
    #[test]
    fn test_search_by_year() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Movie 2020", "TestDisk", 1001);
        movie1.year = Some(2020);
        let mut movie2 = create_test_movie("m2", "Movie 2024", "TestDisk", 1002);
        movie2.year = Some(2024);
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None, None, None, None,
            Some(2024),
            None, None, None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "Movie 2024");
    }
    
    #[test]
    fn test_disk_info_paths_extensible() {
        let mut disk_info = DiskInfo {
            label: "TestDisk".to_string(),
            uuid: None,
            last_indexed: chrono::Utc::now().to_rfc3339(),
            movie_count: 0,
            tvshow_count: 0,
            total_size_bytes: 0,
            base_path: String::new(),
            paths: HashMap::new(),
        };
        
        // Add movies path
        disk_info.paths.insert("movies".to_string(), "/path/to/movies".to_string());
        
        // Add tvshows path
        disk_info.paths.insert("tvshows".to_string(), "/path/to/tvshows".to_string());
        
        // Future extensibility: add music path
        disk_info.paths.insert("music".to_string(), "/path/to/music".to_string());
        
        assert_eq!(disk_info.paths.len(), 3);
        assert!(disk_info.paths.contains_key("movies"));
        assert!(disk_info.paths.contains_key("tvshows"));
        assert!(disk_info.paths.contains_key("music"));
    }
    
    #[test]
    fn test_search_by_actor() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Movie A", "TestDisk", 1001);
        movie1.actors = vec!["Tom Hanks".to_string(), "Meg Ryan".to_string()];
        let mut movie2 = create_test_movie("m2", "Movie B", "TestDisk", 1002);
        movie2.actors = vec!["Brad Pitt".to_string()];
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None,
            Some("Tom Hanks"),
            None, None, None, None, None, None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "Movie A");
    }
    
    #[test]
    fn test_search_by_director() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Movie A", "TestDisk", 1001);
        movie1.directors = vec!["Steven Spielberg".to_string()];
        let mut movie2 = create_test_movie("m2", "Movie B", "TestDisk", 1002);
        movie2.directors = vec!["Christopher Nolan".to_string()];
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None, None,
            Some("Nolan"),
            None, None, None, None, None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "Movie B");
    }
    
    #[test]
    fn test_search_by_genre() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Action Movie", "TestDisk", 1001);
        movie1.genres = vec!["Action".to_string(), "Thriller".to_string()];
        let mut movie2 = create_test_movie("m2", "Comedy Movie", "TestDisk", 1002);
        movie2.genres = vec!["Comedy".to_string()];
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None, None, None, None, None, None,
            Some("Comedy"),
            None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "Comedy Movie");
    }
    
    #[test]
    fn test_search_by_country() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "US Movie", "TestDisk", 1001);
        movie1.country = Some("US".to_string());
        let mut movie2 = create_test_movie("m2", "CN Movie", "TestDisk", 1002);
        movie2.country = Some("CN".to_string());
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None, None, None, None, None, None, None,
            Some("CN"),
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "CN Movie");
    }
    
    #[test]
    fn test_search_tvshows() {
        let mut central = CentralIndex::default();
        
        let tvshows = vec![
            create_test_tvshow("t1", "Breaking Bad", "TestDisk", 2001),
            create_test_tvshow("t2", "Game of Thrones", "TestDisk", 2002),
        ];
        let disk = create_tvshow_disk_index("TestDisk", "/mnt/TestDisk/TVShows", tvshows);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            Some("breaking"),
            None, None, None, None, None, None, None,
        );
        
        assert_eq!(results.tvshows.len(), 1);
        assert_eq!(results.tvshows[0].title, "Breaking Bad");
    }
    
    #[test]
    fn test_collection_indexing() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Pirates 1", "TestDisk", 1001);
        movie1.collection_id = Some(100);
        movie1.collection_name = Some("Pirates of the Caribbean Collection".to_string());
        movie1.collection_total_movies = Some(5);
        
        let mut movie2 = create_test_movie("m2", "Pirates 2", "TestDisk", 1002);
        movie2.collection_id = Some(100);
        movie2.collection_name = Some("Pirates of the Caribbean Collection".to_string());
        movie2.collection_total_movies = Some(5);
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        // Verify collection was created
        assert_eq!(central.collections.len(), 1);
        let collection = central.collections.get(&100).unwrap();
        assert_eq!(collection.name, "Pirates of the Caribbean Collection");
        assert_eq!(collection.owned_count, 2);
        assert_eq!(collection.total_in_collection, 5);
        assert_eq!(collection.movies.len(), 2);
    }
    
    #[test]
    fn test_statistics_update() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "US Movie 2020", "TestDisk", 1001);
        movie1.country = Some("US".to_string());
        movie1.year = Some(2020);
        movie1.size_bytes = 1_000_000_000;
        
        let mut movie2 = create_test_movie("m2", "CN Movie 2024", "TestDisk", 1002);
        movie2.country = Some("CN".to_string());
        movie2.year = Some(2024);
        movie2.size_bytes = 2_000_000_000;
        
        let movies = vec![movie1, movie2];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        // Verify statistics
        assert_eq!(central.statistics.total_movies, 2);
        assert_eq!(central.statistics.total_disks, 1);
        assert_eq!(central.statistics.total_size_bytes, 3_000_000_000);
        assert_eq!(central.statistics.by_country.get("US"), Some(&1));
        assert_eq!(central.statistics.by_country.get("CN"), Some(&1));
        assert_eq!(central.statistics.by_decade.get("2020s"), Some(&2));
    }
    
    #[test]
    fn test_find_duplicates_same_tmdb_id() {
        let mut central = CentralIndex::default();
        
        // Same movie on two different disks (same TMDB ID)
        let movie1 = create_test_movie("m1", "The Matrix", "Disk1", 1001);
        let disk1 = create_movie_disk_index("Disk1", "/mnt/Disk1/Movies", vec![movie1]);
        merge_disk_into_central(&mut central, disk1);
        
        let movie2 = create_test_movie("m2", "The Matrix HD", "Disk2", 1001); // Same TMDB ID!
        let disk2 = create_movie_disk_index("Disk2", "/mnt/Disk2/Movies", vec![movie2]);
        merge_disk_into_central(&mut central, disk2);
        
        // Find duplicates by TMDB ID
        let mut tmdb_count: HashMap<u64, Vec<&MovieEntry>> = HashMap::new();
        for movie in &central.movies {
            if let Some(tmdb_id) = movie.tmdb_id {
                tmdb_count.entry(tmdb_id).or_default().push(movie);
            }
        }
        
        let duplicates: Vec<_> = tmdb_count.iter()
            .filter(|(_, movies)| movies.len() > 1)
            .collect();
        
        assert_eq!(duplicates.len(), 1);
        let (tmdb_id, movies) = duplicates[0];
        assert_eq!(*tmdb_id, 1001);
        assert_eq!(movies.len(), 2);
        
        // Verify they are on different disks
        let disks: std::collections::HashSet<_> = movies.iter().map(|m| &m.disk).collect();
        assert_eq!(disks.len(), 2);
    }
    
    #[test]
    fn test_remove_disk_from_central() {
        let mut central = CentralIndex::default();
        
        // Add movies to disk
        let movies = vec![
            create_test_movie("m1", "Movie 1", "TestDisk", 1001),
        ];
        let movie_disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, movie_disk);
        
        // Add tvshows to same disk
        let tvshows = vec![
            create_test_tvshow("t1", "TV Show 1", "TestDisk", 2001),
        ];
        let tvshow_disk = create_tvshow_disk_index("TestDisk", "/mnt/TestDisk/TVShows", tvshows);
        merge_disk_into_central(&mut central, tvshow_disk);
        
        assert_eq!(central.disks.len(), 1);
        assert_eq!(central.movies.len(), 1);
        assert_eq!(central.tvshows.len(), 1);
        
        // Simulate remove disk
        let disk_label = "TestDisk";
        central.movies.retain(|m| m.disk != disk_label);
        central.tvshows.retain(|t| t.disk != disk_label);
        central.disks.remove(disk_label);
        central.rebuild_indexes();
        central.update_statistics();
        
        // Verify disk is removed
        assert_eq!(central.disks.len(), 0);
        assert_eq!(central.movies.len(), 0);
        assert_eq!(central.tvshows.len(), 0);
        assert_eq!(central.statistics.total_movies, 0);
        assert_eq!(central.statistics.total_tvshows, 0);
    }
    
    #[test]
    fn test_backward_compatibility_base_path() {
        // Simulate loading old index with only base_path (no paths HashMap)
        let disk_info_json = r#"{
            "label": "OldDisk",
            "uuid": "old-uuid",
            "last_indexed": "2024-01-01T00:00:00Z",
            "movie_count": 10,
            "tvshow_count": 0,
            "total_size_bytes": 10000000,
            "base_path": "/mnt/OldDisk/Movies"
        }"#;
        
        let disk_info: DiskInfo = serde_json::from_str(disk_info_json).unwrap();
        
        // Verify backward compatibility - paths should be empty (default)
        assert_eq!(disk_info.label, "OldDisk");
        assert_eq!(disk_info.base_path, "/mnt/OldDisk/Movies");
        assert!(disk_info.paths.is_empty()); // Default empty HashMap
    }
    
    #[test]
    fn test_search_year_range() {
        let mut central = CentralIndex::default();
        
        let mut movie1 = create_test_movie("m1", "Movie 2018", "TestDisk", 1001);
        movie1.year = Some(2018);
        let mut movie2 = create_test_movie("m2", "Movie 2020", "TestDisk", 1002);
        movie2.year = Some(2020);
        let mut movie3 = create_test_movie("m3", "Movie 2024", "TestDisk", 1003);
        movie3.year = Some(2024);
        
        let movies = vec![movie1, movie2, movie3];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        let results = search(
            &central,
            None, None, None, None, None,
            Some((2019, 2022)), // Year range
            None, None,
        );
        
        assert_eq!(results.movies.len(), 1);
        assert_eq!(results.movies[0].title, "Movie 2020");
    }
    
    #[test]
    fn test_collection_complete_incomplete() {
        let mut central = CentralIndex::default();
        
        // Collection A: 2 of 2 movies owned (complete)
        let mut movie1 = create_test_movie("m1", "Trilogy A-1", "TestDisk", 1001);
        movie1.collection_id = Some(100);
        movie1.collection_name = Some("Complete Trilogy".to_string());
        movie1.collection_total_movies = Some(2);
        
        let mut movie2 = create_test_movie("m2", "Trilogy A-2", "TestDisk", 1002);
        movie2.collection_id = Some(100);
        movie2.collection_name = Some("Complete Trilogy".to_string());
        movie2.collection_total_movies = Some(2);
        
        // Collection B: 1 of 3 movies owned (incomplete)
        let mut movie3 = create_test_movie("m3", "Series B-1", "TestDisk", 1003);
        movie3.collection_id = Some(200);
        movie3.collection_name = Some("Incomplete Series".to_string());
        movie3.collection_total_movies = Some(3);
        
        let movies = vec![movie1, movie2, movie3];
        let disk = create_movie_disk_index("TestDisk", "/mnt/TestDisk/Movies", movies);
        merge_disk_into_central(&mut central, disk);
        
        // Verify collection statistics
        assert_eq!(central.statistics.complete_collections, 1);
        assert_eq!(central.statistics.incomplete_collections, 1);
        
        // Verify collection details
        let collection_a = central.collections.get(&100).unwrap();
        assert_eq!(collection_a.owned_count, 2);
        assert_eq!(collection_a.total_in_collection, 2);
        
        let collection_b = central.collections.get(&200).unwrap();
        assert_eq!(collection_b.owned_count, 1);
        assert_eq!(collection_b.total_in_collection, 3);
    }
}

