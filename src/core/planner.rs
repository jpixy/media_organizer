//! Plan generation module.
//!
//! Coordinates the entire planning process:
//! 1. Scan directory for video files
//! 2. Parse filenames with AI
//! 3. Query TMDB for metadata
//! 4. Extract video metadata with ffprobe
//! 5. Generate target paths and operations
//! 6. Output plan.json

use crate::core::parser::{FilenameParser, ParsedFilename};
use crate::core::scanner::scan_directory;
use crate::generators::{filename as gen_filename, folder as gen_folder};
use crate::models::media::{MediaType, MovieMetadata, VideoFile, VideoMetadata};
use crate::models::plan::{
    Operation, OperationType, ParsedInfo, Plan, PlanItem, PlanItemStatus, SampleItem, TargetInfo,
    UnknownItem,
};
use crate::services::{ffprobe, tmdb::TmdbClient};
use crate::Result;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Planner configuration.
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Minimum confidence threshold for parsed filenames.
    pub min_confidence: f32,
    /// Whether to download posters.
    pub download_posters: bool,
    /// Poster size for TMDB.
    pub poster_size: String,
    /// Whether to generate NFO files.
    pub generate_nfo: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            download_posters: true,
            poster_size: "w500".to_string(),
            generate_nfo: true,
        }
    }
}

/// Plan generator.
pub struct Planner {
    config: PlannerConfig,
    parser: FilenameParser,
    tmdb_client: Option<TmdbClient>,
}

impl Planner {
    /// Create a new planner with default configuration.
    pub fn new() -> Result<Self> {
        let tmdb_client = TmdbClient::from_env().ok();
        Ok(Self {
            config: PlannerConfig::default(),
            parser: FilenameParser::new(),
            tmdb_client,
        })
    }

    /// Create a new planner with custom configuration.
    pub fn with_config(config: PlannerConfig) -> Result<Self> {
        let tmdb_client = TmdbClient::from_env().ok();
        Ok(Self {
            config,
            parser: FilenameParser::new(),
            tmdb_client,
        })
    }

    /// Generate a plan for organizing videos.
    pub async fn generate(
        &self,
        source: &Path,
        target: &Path,
        media_type: MediaType,
    ) -> Result<Plan> {
        tracing::info!("Generating plan for {:?}", source);
        tracing::info!("Target directory: {:?}", target);
        tracing::info!("Media type: {}", media_type);

        // Step 1: Scan directory
        println!("📁 Scanning directory...");
        let scan_result = scan_directory(source)?;
        println!(
            "   Found {} videos, {} samples",
            scan_result.videos.len(),
            scan_result.samples.len()
        );

        if scan_result.videos.is_empty() {
            tracing::warn!("No video files found in {:?}", source);
        }

        // Step 2: Process videos
        let (items, unknown) = self
            .process_videos(&scan_result.videos, target, media_type)
            .await?;

        // Step 3: Process samples
        let samples = self.process_samples(&scan_result.samples, &items, target);

        // Step 4: Create plan
        let plan = Plan {
            version: "1.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            media_type: Some(media_type),
            source_path: source.to_path_buf(),
            target_path: target.to_path_buf(),
            items,
            samples,
            unknown,
        };

        Ok(plan)
    }

    /// Process video files: parse, query TMDB, extract metadata.
    async fn process_videos(
        &self,
        videos: &[VideoFile],
        target: &Path,
        media_type: MediaType,
    ) -> Result<(Vec<PlanItem>, Vec<UnknownItem>)> {
        let mut items = Vec::new();
        let mut unknown = Vec::new();

        if videos.is_empty() {
            return Ok((items, unknown));
        }

        // Create progress bar
        let pb = ProgressBar::new(videos.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        for video in videos {
            pb.set_message(format!("Processing: {}", &video.filename));
            pb.inc(1);

            match self.process_single_video(video, target, media_type).await {
                Ok(Some(item)) => items.push(item),
                Ok(None) => {
                    unknown.push(UnknownItem {
                        source: video.clone(),
                        reason: "Failed to parse or find metadata".to_string(),
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to process {}: {}", video.filename, e);
                    unknown.push(UnknownItem {
                        source: video.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        pb.finish_with_message("Done!");
        Ok((items, unknown))
    }

    /// Process a single video file.
    async fn process_single_video(
        &self,
        video: &VideoFile,
        target: &Path,
        media_type: MediaType,
    ) -> Result<Option<PlanItem>> {
        // Step 1: Parse filename with AI
        // If file is in a subdirectory, include parent dir name for better context
        let parse_input = self.build_parse_input(video);
        let parsed = self.parser.parse(&parse_input, media_type).await?;

        if !self.parser.is_valid(&parsed) {
            tracing::debug!("Low confidence parsing for: {}", video.filename);
            return Ok(None);
        }

        // Step 2: Query TMDB
        let movie_metadata = if media_type == MediaType::Movies {
            self.query_tmdb_movie(&parsed).await?
        } else {
            None
        };

        if movie_metadata.is_none() && media_type == MediaType::Movies {
            tracing::debug!("No TMDB match for: {}", video.filename);
            return Ok(None);
        }

        // Step 3: Extract video metadata with ffprobe + filename parsing
        let ffprobe_metadata = ffprobe::extract_metadata(&video.path).unwrap_or_default();
        let filename_metadata = ffprobe::parse_metadata_from_filename(&video.filename);
        
        // Merge: prefer ffprobe data, but use filename data as fallback
        let video_metadata = ffprobe::merge_metadata(ffprobe_metadata, filename_metadata);
        
        tracing::debug!(
            "Video metadata for {}: resolution={}, format={}, codec={}",
            video.filename,
            video_metadata.resolution,
            video_metadata.format,
            video_metadata.video_codec
        );

        // Step 4: Generate target paths
        let (target_info, operations) = self.generate_target_info(
            video,
            &movie_metadata,
            &video_metadata,
            target,
            media_type,
        )?;

        // Step 5: Create plan item
        let item = PlanItem {
            id: Uuid::new_v4().to_string(),
            status: PlanItemStatus::Pending,
            source: video.clone(),
            parsed: ParsedInfo {
                title: parsed.title,
                original_title: parsed.original_title,
                year: parsed.year,
                confidence: parsed.confidence,
                raw_response: parsed.raw_response,
            },
            movie_metadata,
            tvshow_metadata: None,
            video_metadata,
            target: target_info,
            operations,
        };

        Ok(Some(item))
    }

    /// Build the input string for AI parsing.
    /// If the file is in a subdirectory with a meaningful name, include it for better context.
    fn build_parse_input(&self, video: &VideoFile) -> String {
        // Get parent directory name
        let parent_name = video.parent_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check if filename lacks meaningful title info
        // (e.g., just year + format like "2024 SP.mp4" or "E01.mkv")
        let filename_seems_minimal = self.is_minimal_filename(&video.filename);
        
        // Check if parent directory has meaningful name
        let parent_has_title = !parent_name.is_empty() 
            && parent_name != "Movies" 
            && parent_name != "movies"
            && parent_name != "TvShows"
            && parent_name != "tvshows"
            && !parent_name.starts_with(".")
            && parent_name.len() > 3;
        
        if filename_seems_minimal && parent_has_title {
            // Combine parent dir name and filename for better context
            tracing::info!(
                "Using parent dir for context: '{}' + '{}'",
                parent_name, video.filename
            );
            format!("{} - {}", parent_name, video.filename)
        } else {
            video.filename.clone()
        }
    }

    /// Check if a filename lacks meaningful title information.
    fn is_minimal_filename(&self, filename: &str) -> bool {
        let name = filename.to_lowercase();
        
        // Simple check: if filename is very short or mostly numbers/common identifiers
        let name_without_ext = name.rsplit('.').skip(1).collect::<Vec<_>>().join(".");
        let alphanumeric_count = name_without_ext.chars().filter(|c| c.is_alphanumeric()).count();
        
        // If the meaningful part is very short, consider it minimal
        if alphanumeric_count <= 8 {
            return true;
        }
        
        // Check for year-only pattern like "2024 SP"
        if name.contains("sp") || name.contains("ova") || name.contains("特别") {
            let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 4 && digits.parse::<u16>().map(|y| y >= 1990 && y <= 2030).unwrap_or(false) {
                return true;
            }
        }
        
        false
    }

    /// Query TMDB for movie metadata.
    async fn query_tmdb_movie(&self, parsed: &ParsedFilename) -> Result<Option<MovieMetadata>> {
        let client = match &self.tmdb_client {
            Some(c) => c,
            None => return Ok(None),
        };

        // Use original title or title for search
        let query = parsed
            .original_title
            .as_ref()
            .or(parsed.title.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return Ok(None);
        }

        // Search TMDB
        let results = client.search_movie(query, parsed.year).await?;

        if results.is_empty() {
            // Try without year
            let results = client.search_movie(query, None).await?;
            if results.is_empty() {
                return Ok(None);
            }
            return self.get_movie_details(client, results[0].id).await;
        }

        self.get_movie_details(client, results[0].id).await
    }

    /// Get movie details from TMDB.
    async fn get_movie_details(
        &self,
        client: &TmdbClient,
        movie_id: u64,
    ) -> Result<Option<MovieMetadata>> {
        let details = client.get_movie_details(movie_id).await?;

        // Extract year from release date
        let year = details
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok())
            .unwrap_or(0);

        // Extract credits from details (now included via append_to_response)
        let credits = details.credits.as_ref();

        // Extract directors
        let directors = credits
            .map(|c| {
                c.crew
                    .iter()
                    .filter(|m| m.job == "Director")
                    .map(|m| m.name.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Extract writers
        let writers = credits
            .map(|c| {
                c.crew
                    .iter()
                    .filter(|m| m.job == "Writer" || m.job == "Screenplay")
                    .take(5)
                    .map(|m| m.name.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Extract actors and their roles
        let (actors, actor_roles): (Vec<String>, Vec<String>) = credits
            .map(|c| {
                c.cast
                    .iter()
                    .take(15)
                    .map(|m| {
                        (
                            m.name.clone(),
                            m.character.clone().unwrap_or_default(),
                        )
                    })
                    .unzip()
            })
            .unwrap_or_default();

        // Extract genres
        let genres = details
            .genres
            .as_ref()
            .map(|g| g.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract production countries
        let countries = details
            .production_countries
            .as_ref()
            .map(|c| c.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract studios
        let studios = details
            .production_companies
            .as_ref()
            .map(|c| c.iter().take(3).map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract certification (from release_dates)
        let certification = details.release_dates.as_ref().and_then(|rd| {
            // Try to find US certification first, then CN
            for country in &["US", "CN"] {
                if let Some(c) = rd.results.iter().find(|r| r.iso_3166_1 == *country) {
                    if let Some(cert) = c.release_dates.iter()
                        .filter_map(|r| r.certification.as_ref())
                        .find(|c| !c.is_empty())
                    {
                        return Some(cert.clone());
                    }
                }
            }
            None
        });

        // Build poster URLs
        let mut poster_urls = Vec::new();
        if let Some(ref poster_path) = details.poster_path {
            poster_urls.push(client.get_poster_url(poster_path, &self.config.poster_size));
        }

        // Build backdrop URL
        let backdrop_url = details.backdrop_path.as_ref().map(|p| {
            client.get_poster_url(p, "original")
        });

        Ok(Some(MovieMetadata {
            tmdb_id: details.id,
            imdb_id: details.imdb_id,
            original_title: details.original_title,
            title: details.title,
            original_language: details.original_language,
            year,
            release_date: details.release_date,
            overview: details.overview,
            tagline: details.tagline,
            runtime: details.runtime,
            genres,
            countries,
            studios,
            rating: details.vote_average,
            votes: details.vote_count,
            poster_urls,
            backdrop_url,
            directors,
            writers,
            actors,
            actor_roles,
            certification,
        }))
    }

    /// Generate target path information and operations.
    fn generate_target_info(
        &self,
        video: &VideoFile,
        movie_metadata: &Option<MovieMetadata>,
        video_metadata: &VideoMetadata,
        target: &Path,
        media_type: MediaType,
    ) -> Result<(TargetInfo, Vec<Operation>)> {
        let mut operations = Vec::new();

        // Get file extension
        let extension = video
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        let (folder_name, filename, nfo_name) = match media_type {
            MediaType::Movies => {
                let metadata = movie_metadata.as_ref().ok_or_else(|| {
                    crate::Error::other("Missing movie metadata")
                })?;

                let folder = gen_folder::generate_movie_folder(metadata, None);
                let filename = gen_filename::generate_movie_filename(
                    metadata,
                    video_metadata,
                    None,
                    extension,
                );
                let nfo = "movie.nfo".to_string();

                (folder, filename, nfo)
            }
            MediaType::TvShows => {
                // For TV shows, we'd need TvShowMetadata - simplified for now
                return Err(crate::Error::other("TV show planning not yet implemented"));
            }
        };

        // Build target paths
        let target_folder = target.join(&folder_name);
        let target_file = target_folder.join(&filename);
        let target_nfo = target_folder.join(&nfo_name);

        // Operation 1: Create directory
        operations.push(Operation {
            op: OperationType::Mkdir,
            from: None,
            to: target_folder.clone(),
            url: None,
            content_ref: None,
        });

        // Operation 2: Move video file
        operations.push(Operation {
            op: OperationType::Move,
            from: Some(video.path.clone()),
            to: target_file.clone(),
            url: None,
            content_ref: None,
        });

        // Operation 3: Create NFO file
        if self.config.generate_nfo {
            operations.push(Operation {
                op: OperationType::Create,
                from: None,
                to: target_nfo.clone(),
                url: None,
                content_ref: Some("nfo".to_string()),
            });
        }

        // Operation 4: Download poster
        if self.config.download_posters {
            if let Some(ref metadata) = movie_metadata {
                if let Some(poster_url) = metadata.poster_urls.first() {
                    let poster_path = target_folder.join("poster.jpg");
                    operations.push(Operation {
                        op: OperationType::Download,
                        from: None,
                        to: poster_path,
                        url: Some(poster_url.clone()),
                        content_ref: None,
                    });
                }
            }
        }

        let target_info = TargetInfo {
            folder: folder_name,
            filename,
            full_path: target_file,
            nfo: nfo_name,
            poster: Some("poster.jpg".to_string()),
        };

        Ok((target_info, operations))
    }

    /// Process sample files.
    fn process_samples(
        &self,
        samples: &[VideoFile],
        items: &[PlanItem],
        target: &Path,
    ) -> Vec<SampleItem> {
        samples
            .iter()
            .filter_map(|sample| {
                // Try to find a matching item by parent directory
                let matching_item = items.iter().find(|item| {
                    sample.parent_dir == item.source.parent_dir
                        || sample.parent_dir.starts_with(&item.source.parent_dir)
                });

                matching_item.map(|item| {
                    let target_folder = target.join(&item.target.folder).join("Sample");
                    let target_file = target_folder.join(&sample.filename);

                    SampleItem {
                        source: sample.path.clone(),
                        target: target_file,
                    }
                })
            })
            .collect()
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new().expect("Failed to create default planner")
    }
}

/// Generate a plan for organizing videos (convenience function).
pub async fn generate_plan(source: &Path, target: &Path, media_type: MediaType) -> Result<Plan> {
    let planner = Planner::new()?;
    planner.generate(source, target, media_type).await
}

/// Save a plan to a JSON file.
pub fn save_plan(plan: &Plan, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(plan)?;
    
    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    file.write_all(json.as_bytes())?;

    tracing::info!("Plan saved to {:?}", path);
    Ok(())
}

/// Load a plan from a JSON file.
pub fn load_plan(path: &Path) -> Result<Plan> {
    let content = fs::read_to_string(path)?;
    let plan: Plan = serde_json::from_str(&content)?;
    Ok(plan)
}

/// Get the default plan output path.
/// Saves to target directory if provided, otherwise to source directory.
pub fn default_plan_path(source: &Path, target: Option<&Path>) -> PathBuf {
    let filename = format!(
        "plan_{}.json",
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    // Prefer target directory, fallback to source
    let base_dir = target.unwrap_or(source);
    base_dir.join(filename)
}

/// Get the sessions directory.
pub fn sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| crate::Error::other("Cannot find home directory"))?;
    let dir = home.join(".config").join("media_organizer").join("sessions");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Save plan to sessions directory.
pub fn save_to_sessions(plan: &Plan) -> Result<PathBuf> {
    let session_id = format!(
        "{}_{}", 
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );
    
    let sessions = sessions_dir()?;
    let session_dir = sessions.join(&session_id);
    fs::create_dir_all(&session_dir)?;

    let plan_path = session_dir.join("plan.json");
    save_plan(plan, &plan_path)?;

    tracing::info!("Session saved: {}", session_id);
    Ok(session_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_config_default() {
        let config = PlannerConfig::default();
        assert_eq!(config.min_confidence, 0.5);
        assert!(config.download_posters);
        assert!(config.generate_nfo);
        assert_eq!(config.poster_size, "w500");
    }

    #[test]
    fn test_default_plan_path() {
        let source = Path::new("/tmp/movies");
        let target = Path::new("/tmp/movies_organized");
        
        // Test with target
        let path = default_plan_path(source, Some(target));
        assert!(path.to_string_lossy().contains("plan_"));
        assert!(path.to_string_lossy().ends_with(".json"));
        assert!(path.starts_with(target));
        
        // Test without target (falls back to source)
        let path = default_plan_path(source, None);
        assert!(path.starts_with(source));
    }

    #[test]
    fn test_save_and_load_plan() {
        let plan = Plan {
            version: "1.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            media_type: Some(MediaType::Movies),
            source_path: PathBuf::from("/source"),
            target_path: PathBuf::from("/target"),
            items: vec![],
            samples: vec![],
            unknown: vec![],
        };

        let temp_dir = tempfile::TempDir::new().unwrap();
        let plan_path = temp_dir.path().join("test_plan.json");

        // Save
        save_plan(&plan, &plan_path).unwrap();
        assert!(plan_path.exists());

        // Load
        let loaded = load_plan(&plan_path).unwrap();
        assert_eq!(loaded.version, plan.version);
        assert_eq!(loaded.source_path, plan.source_path);
    }
}


