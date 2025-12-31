//! Plan generation module.
//!
//! Coordinates the entire planning process:
//! 1. Scan directory for video files
//! 2. Parse filenames with AI
//! 3. Query TMDB for metadata
//! 4. Extract video metadata with ffprobe
//! 5. Generate target paths and operations
//! 6. Output plan.json

use crate::core::parser::{self, FilenameParser, ParsedFilename};
use crate::core::scanner::scan_directory;
use crate::generators::{filename as gen_filename, folder as gen_folder};
use crate::models::media::{MediaType, MovieMetadata, TvShowMetadata, EpisodeMetadata, VideoFile, VideoMetadata};
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

        // Step 2: Process videos (pass source for correct cache key calculation)
        let (items, unknown) = self
            .process_videos(&scan_result.videos, source, target, media_type)
            .await?;

        // Step 3: Process samples
        let samples = self.process_samples(&scan_result.samples, &items, target);

        // Step 3.5: SAFETY CHECK - Detect duplicate target paths
        // This prevents data loss from files overwriting each other
        self.validate_no_duplicate_targets(&items)?;

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
    /// 
    /// NEW DESIGN: Process by top-level directory for better caching:
    /// 1. Group videos by their top-level directory (relative to source)
    /// 2. For each group, call AI + TMDB only ONCE
    /// 3. Use regex to extract episode numbers for remaining files
    async fn process_videos(
        &self,
        videos: &[VideoFile],
        source: &Path,
        target: &Path,
        media_type: MediaType,
    ) -> Result<(Vec<PlanItem>, Vec<UnknownItem>)> {
        let mut items = Vec::new();
        let mut unknown = Vec::new();

        if videos.is_empty() {
            return Ok((items, unknown));
        }

        // Step 1: Group videos by top-level directory (relative to source)
        let groups = self.group_by_top_level_dir(videos, source);
        tracing::info!("Grouped {} videos into {} directories", videos.len(), groups.len());

        // Cache for TV show metadata by top-level directory
        let mut tvshow_cache: std::collections::HashMap<PathBuf, (TvShowMetadata, Option<EpisodeMetadata>)> = 
            std::collections::HashMap::new();

        // Create progress bar
        let pb = ProgressBar::new(videos.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        // Step 2: Process each group
        for (top_dir, group_videos) in &groups {
            // Check if we already have cached metadata for this directory
            let cached_show = if media_type == MediaType::TvShows {
                tvshow_cache.get(top_dir).cloned()
            } else {
                None
            };

            // Process the first video with AI to get metadata (if not cached)
            let first_video = &group_videos[0];
            pb.set_message(format!("Processing: {} ({} files)", 
                top_dir.file_name().unwrap_or_default().to_string_lossy(),
                group_videos.len()
            ));

            // For the first video (or if no cache), use AI parsing
            let first_result = if cached_show.is_none() {
                self.process_single_video_with_cache(first_video, target, media_type, None).await
            } else {
                self.process_single_video_with_cache(first_video, target, media_type, cached_show.as_ref()).await
            };

            match first_result {
                Ok(Some((item, show_meta))) => {
                    // Cache the TV show metadata for this top-level directory
                    if media_type == MediaType::TvShows {
                        if let Some(ref meta) = show_meta {
                            tvshow_cache.insert(top_dir.clone(), meta.clone());
                        }
                    }
                    items.push(item);
                    pb.inc(1);

                    // Now process remaining files in this group using cached metadata
                    let cached = tvshow_cache.get(top_dir).cloned();
                    for video in group_videos.iter().skip(1) {
                        pb.set_message(format!("Processing: {}", &video.filename));
                        
                        match self.process_single_video_with_cache(video, target, media_type, cached.as_ref()).await {
                            Ok(Some((item, _))) => {
                                items.push(item);
                            }
                            Ok(None) => {
                                unknown.push(UnknownItem {
                                    source: video.clone(),
                                    reason: "Failed to extract episode info".to_string(),
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
                        pb.inc(1);
                    }
                }
                Ok(None) => {
                    // First video failed, mark all videos in this group as unknown
                    for video in group_videos {
                        unknown.push(UnknownItem {
                            source: video.clone(),
                            reason: "Failed to parse or find metadata for directory".to_string(),
                        });
                        pb.inc(1);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to process directory {:?}: {}", top_dir, e);
                    for video in group_videos {
                        unknown.push(UnknownItem {
                            source: video.clone(),
                            reason: e.to_string(),
                        });
                        pb.inc(1);
                    }
                }
            }
        }

        pb.finish_with_message("Done!");
        Ok((items, unknown))
    }

    /// Group videos by their immediate parent directory.
    /// 
    /// This is the correct grouping for TV shows:
    /// - /Videos/TV_Shows/黑盒子/01.mp4 → parent_dir: /Videos/TV_Shows/黑盒子
    /// - /Videos/TV_Shows/赵露思合集/后浪/01.mp4 → parent_dir: /Videos/TV_Shows/赵露思合集/后浪
    /// - /Videos/TV_Shows/赵露思合集/陈芊芊/01.mp4 → parent_dir: /Videos/TV_Shows/赵露思合集/陈芊芊
    /// 
    /// Each parent directory represents a single TV show/season.
    fn group_by_top_level_dir(
        &self,
        videos: &[VideoFile],
        _source: &Path,
    ) -> std::collections::HashMap<PathBuf, Vec<VideoFile>> {
        let mut groups: std::collections::HashMap<PathBuf, Vec<VideoFile>> = std::collections::HashMap::new();

        for video in videos {
            // Use the immediate parent directory as the grouping key
            // This correctly handles both simple and nested directory structures
            groups.entry(video.parent_dir.clone()).or_default().push(video.clone());
        }

        groups
    }

    /// Process a single video file with optional cached TV show metadata.
    /// Returns the PlanItem and the TV show metadata (for caching).
    /// 
    /// OPTIMIZATION: For TV shows with cached metadata, we extract episode numbers
    /// from filename using regex instead of calling AI for each file.
    async fn process_single_video_with_cache(
        &self,
        video: &VideoFile,
        target: &Path,
        media_type: MediaType,
        cached_show: Option<&(TvShowMetadata, Option<EpisodeMetadata>)>,
    ) -> Result<Option<(PlanItem, Option<(TvShowMetadata, Option<EpisodeMetadata>)>)>> {
        // Step 1: Parse filename - use regex for TV shows with cache, AI otherwise
        let parsed = if media_type == MediaType::TvShows && cached_show.is_some() {
            // FAST PATH: Extract episode number from filename using regex (no AI call)
            let (season, episode) = parser::extract_episode_from_filename(&video.filename);
            tracing::debug!(
                "Regex extracted from {}: S{:?}E{:?}",
                video.filename,
                season,
                episode
            );
            
            if episode.is_none() {
                tracing::debug!("Could not extract episode number from: {}", video.filename);
                return Ok(None);
            }
            
            // Create a minimal parsed result with episode info
            ParsedFilename {
                title: cached_show.as_ref().map(|(s, _)| s.name.clone()),
                original_title: cached_show.as_ref().map(|(s, _)| s.original_name.clone()),
                year: cached_show.as_ref().map(|(s, _)| s.year),
                season,
                episode,
                confidence: 1.0, // High confidence for regex match
                raw_response: Some("regex_extracted".to_string()),
            }
        } else {
            // NORMAL PATH: Use AI to parse filename
            let parse_input = self.build_parse_input(video);
            let parsed = self.parser.parse(&parse_input, media_type).await?;

            if !self.parser.is_valid(&parsed) {
                tracing::debug!("Low confidence parsing for: {}", video.filename);
                return Ok(None);
            }
            parsed
        };

        // Step 2: Query TMDB based on media type
        let (movie_metadata, tvshow_metadata) = match media_type {
            MediaType::Movies => {
                let movie = self.query_tmdb_movie(&parsed).await?;
                if movie.is_none() {
                    tracing::debug!("No TMDB match for movie: {}", video.filename);
                    return Ok(None);
                }
                (movie, None)
            }
            MediaType::TvShows => {
                // Use cached show metadata if available (same directory = same show)
                if let Some((cached_show_meta, _)) = cached_show {
                    tracing::info!(
                        "Using cached TV show for {}: {} (S{:?}E{:?})",
                        video.filename,
                        cached_show_meta.name,
                        parsed.season,
                        parsed.episode
                    );
                    // Get episode info for this specific file using regex-extracted numbers
                    let episode = if let (Some(season), Some(ep)) = (parsed.season, parsed.episode) {
                        if let Some(client) = &self.tmdb_client {
                            match client.get_episode_details(cached_show_meta.tmdb_id, season, ep).await {
                                Ok(ep_details) => Some(EpisodeMetadata {
                                    season_number: season,
                                    episode_number: ep,
                                    name: ep_details.name,
                                    original_name: None,
                                    air_date: ep_details.air_date,
                                    overview: ep_details.overview,
                                }),
                                Err(_) => Some(EpisodeMetadata {
                                    season_number: season,
                                    episode_number: ep,
                                    name: format!("Episode {}", ep),
                                    original_name: None,
                                    air_date: None,
                                    overview: None,
                                }),
                            }
                        } else {
                            Some(EpisodeMetadata {
                                season_number: season,
                                episode_number: ep,
                                name: format!("Episode {}", ep),
                                original_name: None,
                                air_date: None,
                                overview: None,
                            })
                        }
                    } else {
                        None
                    };
                    (None, Some((cached_show_meta.clone(), episode)))
                } else {
                    // No cache, query TMDB with folder name as fallback
                    // Try to get meaningful folder name (skip quality descriptors)
                    let folder_name = self.get_meaningful_folder_name(&video.parent_dir);
                    let (show, mut episode) = self.query_tmdb_tvshow_with_folder(&parsed, folder_name.as_deref()).await?;
                    if show.is_none() {
                        tracing::debug!("No TMDB match for TV show: {}", video.filename);
                        return Ok(None);
                    }
                    let show_meta = show.unwrap();
                    
                    // If episode is None (AI didn't parse season/episode), try regex extraction
                    if episode.is_none() {
                        let (regex_season, regex_ep) = parser::extract_episode_from_filename(&video.filename);
                        tracing::debug!(
                            "Regex extraction for first file {}: S{:?}E{:?}",
                            video.filename,
                            regex_season,
                            regex_ep
                        );
                        
                        if let (Some(season), Some(ep)) = (regex_season, regex_ep) {
                            if let Some(client) = &self.tmdb_client {
                                match client.get_episode_details(show_meta.tmdb_id, season, ep).await {
                                    Ok(ep_details) => {
                                        episode = Some(EpisodeMetadata {
                                            season_number: season,
                                            episode_number: ep,
                                            name: ep_details.name,
                                            original_name: None,
                                            air_date: ep_details.air_date,
                                            overview: ep_details.overview,
                                        });
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to get episode details for S{}E{}: {}", season, ep, e);
                                        episode = Some(EpisodeMetadata {
                                            season_number: season,
                                            episode_number: ep,
                                            name: format!("Episode {}", ep),
                                            original_name: None,
                                            air_date: None,
                                            overview: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    
                    (None, Some((show_meta, episode)))
                }
            }
        };

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
            &tvshow_metadata,
            &parsed,
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
            tvshow_metadata: tvshow_metadata.as_ref().map(|(show, _)| show.clone()),
            episode_metadata: tvshow_metadata.as_ref().and_then(|(_, ep)| ep.clone()),
            video_metadata,
            target: target_info,
            operations,
        };

        // Return item and tvshow metadata for caching
        Ok(Some((item, tvshow_metadata)))
    }

    /// Get a meaningful folder name from the path, skipping quality descriptors.
    /// Returns the first ancestor directory that looks like a show name.
    fn get_meaningful_folder_name(&self, path: &Path) -> Option<String> {
        // Quality descriptor patterns to skip
        let is_quality_desc = |name: &str| -> bool {
            let lower = name.to_lowercase();
            lower.contains("1080") || lower.contains("720") 
                || lower.contains("2160") || lower.contains("4k")
                || lower.contains("内封") || lower.contains("外挂")
                || lower.contains("字幕") || lower.starts_with("season")
        };

        // Try immediate parent first
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !is_quality_desc(name) {
                return Some(name.to_string());
            }
        }

        // Try grandparent if immediate parent is a quality descriptor
        if let Some(parent) = path.parent() {
            if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
                if !is_quality_desc(name) {
                    return Some(name.to_string());
                }
            }
        }

        None
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
    /// Format country folder name from ISO code and country name.
    /// Returns format like "CN_China", "US_UnitedStates", "KR_SouthKorea".
    /// Format country folder name from ISO code and country name.
    /// Uses original_language to pick the best country for co-productions.
    fn format_country_folder(
        &self,
        codes: &[String],
        names: &[String],
        original_language: &str,
    ) -> Option<String> {
        if codes.is_empty() || names.is_empty() {
            return None;
        }

        // Map language code to likely country code
        let lang_to_country = |lang: &str| -> Option<&str> {
            match lang {
                "ko" => Some("KR"),
                "ja" => Some("JP"),
                "zh" => Some("CN"),
                "en" => Some("US"), // Default English to US
                "fr" => Some("FR"),
                "de" => Some("DE"),
                "es" => Some("ES"),
                "it" => Some("IT"),
                "ru" => Some("RU"),
                "pt" => Some("BR"),
                "hi" => Some("IN"),
                "th" => Some("TH"),
                _ => None,
            }
        };

        // Try to find a country matching the original language
        let preferred_country = lang_to_country(original_language);
        
        let (code, name) = if let Some(pref_code) = preferred_country {
            // Find the index of the preferred country
            if let Some(idx) = codes.iter().position(|c| c.eq_ignore_ascii_case(pref_code)) {
                (&codes[idx], &names[idx])
            } else {
                // Preferred country not in list, use first
                (&codes[0], &names[0])
            }
        } else {
            // No language preference, use first country
            (&codes[0], &names[0])
        };

        // Remove spaces from country name and capitalize each word
        let name_no_spaces: String = name
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect();

        Some(format!("{}_{}", code.to_uppercase(), name_no_spaces))
    }

    /// SAFETY CHECK: Validate that no two items have the same target path.
    /// This prevents data loss from files overwriting each other.
    fn validate_no_duplicate_targets(&self, items: &[PlanItem]) -> Result<()> {
        use std::collections::HashMap;
        
        let mut target_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        
        for item in items {
            for op in &item.operations {
                if matches!(op.op, OperationType::Move) {
                    target_to_sources
                        .entry(op.to.clone())
                        .or_default()
                        .push(item.source.path.clone());
                }
            }
        }
        
        let duplicates: Vec<_> = target_to_sources
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .collect();
        
        if !duplicates.is_empty() {
            let mut error_msg = String::from("CRITICAL: Duplicate target paths detected! This would cause data loss.\n\n");
            
            for (target, sources) in duplicates.iter() {
                error_msg.push_str(&format!("Target: {:?}\n", target));
                error_msg.push_str("  Would overwrite these source files:\n");
                for src in sources.iter() {
                    error_msg.push_str(&format!("    - {:?}\n", src));
                }
                error_msg.push('\n');
            }
            
            let total_affected: usize = duplicates.iter().map(|(_, s)| s.len()).sum();
            error_msg.push_str(&format!(
                "Total: {} duplicate targets affecting {} files.\n",
                duplicates.len(),
                total_affected
            ));
            error_msg.push_str("Plan generation aborted to prevent data loss.");
            
            tracing::error!("{}", error_msg);
            return Err(crate::Error::other(error_msg));
        }
        
        tracing::info!("Safety check passed: No duplicate target paths");
        Ok(())
    }

    /// Add shortened versions of a long title to the queries list.
    /// This helps match titles like "破坏不在场证明 特别篇 钟表店侦探与祖父的不在场证明"
    /// which should match "破坏不在场证明 特别篇" on TMDB.
    fn add_shortened_queries(&self, queries: &mut Vec<String>, title: &str) {
        // Split by common delimiters
        let delimiters = [" - ", " – ", "：", ":", " 钟", " 与", " 和"];
        
        for delim in delimiters {
            if let Some(pos) = title.find(delim) {
                let shortened = title[..pos].trim().to_string();
                if shortened.len() >= 4 && !queries.contains(&shortened) {
                    tracing::debug!("Adding shortened query: {}", shortened);
                    queries.push(shortened);
                }
            }
        }
        
        // For very long titles (>20 chars), try taking just the first part before space
        if title.chars().count() > 20 {
            // Split by space and take progressively longer parts
            let parts: Vec<&str> = title.split_whitespace().collect();
            if parts.len() >= 2 {
                // Try first two parts
                let shortened = parts[..2.min(parts.len())].join(" ");
                if shortened.len() >= 4 && !queries.contains(&shortened) {
                    queries.push(shortened);
                }
            }
        }
    }

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

        // Prepare search queries: prefer Chinese title first, then original title
        let mut queries: Vec<String> = Vec::new();
        if let Some(title) = &parsed.title {
            if !title.is_empty() {
                queries.push(title.clone());
                // Also try shorter versions for long titles
                self.add_shortened_queries(&mut queries, title);
            }
        }
        if let Some(orig) = &parsed.original_title {
            if !orig.is_empty() && !queries.contains(orig) {
                queries.push(orig.clone());
            }
        }

        if queries.is_empty() {
            return Ok(None);
        }

        // Try each query, with and without year
        for query in &queries {
            let query = query.as_str();
            tracing::debug!("TMDB search: query='{}', year={:?}", query, parsed.year);

            // First try with year (if available)
            if let Some(year) = parsed.year {
                let results = client.search_movie(query, Some(year)).await?;
                if !results.is_empty() {
                    let best = self.select_best_movie_match(&results, query);
                    // Verify the match is reasonable (not a completely different movie)
                    if self.is_reasonable_match(query, &best.title, &best.original_title) {
                        tracing::info!("TMDB found: {} (with year {})", best.title, year);
                        return self.get_movie_details(client, best.id).await;
                    }
                }
            }

            // Try without year
            let results = client.search_movie(query, None).await?;
            if !results.is_empty() {
                let best = self.select_best_movie_match(&results, query);
                if self.is_reasonable_match(query, &best.title, &best.original_title) {
                    tracing::info!("TMDB found: {} (without year)", best.title);
                    return self.get_movie_details(client, best.id).await;
                }
            }
        }

        tracing::warn!("TMDB: No reasonable match found for queries {:?}", queries);
        Ok(None)
    }

    /// Check if the TMDB match is reasonable (title similarity).
    fn is_reasonable_match(&self, query: &str, tmdb_title: &str, tmdb_orig: &str) -> bool {
        let query_lower = query.to_lowercase();
        let title_lower = tmdb_title.to_lowercase();
        let orig_lower = tmdb_orig.to_lowercase();

        // Check if query appears in either title
        if title_lower.contains(&query_lower) || query_lower.contains(&title_lower) {
            return true;
        }
        if orig_lower.contains(&query_lower) || query_lower.contains(&orig_lower) {
            return true;
        }

        // Check for significant word overlap (for CJK languages)
        let query_chars: std::collections::HashSet<char> = query.chars().collect();
        let title_chars: std::collections::HashSet<char> = tmdb_title.chars().collect();

        // At least 50% character overlap for CJK
        let common = query_chars.intersection(&title_chars).count();
        let min_len = query_chars.len().min(title_chars.len());
        if min_len > 0 && common * 2 >= min_len {
            return true;
        }

        false
    }

    /// Query TMDB for TV show metadata.
    async fn query_tmdb_tvshow(
        &self,
        parsed: &ParsedFilename,
    ) -> Result<(Option<TvShowMetadata>, Option<EpisodeMetadata>)> {
        self.query_tmdb_tvshow_with_folder(parsed, None).await
    }

    /// Query TMDB for TV show metadata with optional folder name as fallback.
    async fn query_tmdb_tvshow_with_folder(
        &self,
        parsed: &ParsedFilename,
        folder_name: Option<&str>,
    ) -> Result<(Option<TvShowMetadata>, Option<EpisodeMetadata>)> {
        let client = match &self.tmdb_client {
            Some(c) => c,
            None => return Ok((None, None)),
        };

        // Prepare search queries: prefer Chinese title first, then folder name
        let mut queries: Vec<String> = Vec::new();
        
        // Helper to clean up search query
        let clean_query = |s: &str| -> String {
            s.replace('.', " ")
                .replace('_', " ")
                .trim()
                .to_string()
        };
        
        if let Some(title) = &parsed.title {
            if !title.is_empty() {
                queries.push(clean_query(title));
            }
        }
        if let Some(orig) = &parsed.original_title {
            if !orig.is_empty() {
                let cleaned = clean_query(orig);
                if !queries.contains(&cleaned) {
                    queries.push(cleaned);
                }
            }
        }
        
        // Add folder name as additional query if available
        // Clean up folder name: remove prefixes like "Z_" and suffixes like ".4集"
        // Skip if it looks like a quality descriptor (e.g., "1080P 内封字幕")
        if let Some(folder) = folder_name {
            // Skip quality descriptors
            let is_quality_desc = folder.contains("1080") || folder.contains("720") 
                || folder.contains("2160") || folder.contains("4K")
                || folder.contains("内封") || folder.contains("外挂");
            
            if !is_quality_desc {
                let cleaned = folder
                    .trim_start_matches("Z_")
                    .trim_start_matches("z_")
                    .split('.').next().unwrap_or(folder)
                    .replace('.', " ")
                    .replace('_', " ");
                if !cleaned.is_empty() && !queries.iter().any(|q| q.contains(&cleaned) || cleaned.contains(q.as_str())) {
                    tracing::debug!("Adding folder name as query: {}", cleaned);
                    queries.push(cleaned);
                }
            }
        }

        if queries.is_empty() {
            return Ok((None, None));
        }
        
        let queries_ref: Vec<&str> = queries.iter().map(|s| s.as_str()).collect();

        // Try each query
        for query in &queries_ref {
            tracing::debug!("TMDB TV search: query='{}', year={:?}", query, parsed.year);

            // Search for TV show
            let results = client.search_tv(query, parsed.year).await?;
            
            if results.is_empty() {
                // Try without year
                let results = client.search_tv(query, None).await?;
                if results.is_empty() {
                    continue;
                }
                
                // Select best match from results
                if let Some(best) = self.select_best_tv_match(query, &results) {
                    return self.get_tvshow_details(client, best.id, parsed).await;
                }
                continue;
            }

            // Select best match from results
            if let Some(best) = self.select_best_tv_match(query, &results) {
                tracing::info!("TMDB TV found: {} ({})", best.name, best.first_air_date.as_deref().unwrap_or("?"));
                return self.get_tvshow_details(client, best.id, parsed).await;
            }

            // Try without year if no good match
            let results_no_year = client.search_tv(query, None).await?;
            if !results_no_year.is_empty() {
                if let Some(best) = self.select_best_tv_match(query, &results_no_year) {
                    return self.get_tvshow_details(client, best.id, parsed).await;
                }
            }
        }

        tracing::warn!("TMDB TV: No reasonable match found for queries {:?}", queries);
        Ok((None, None))
    }

    /// Select the best TV show match from search results.
    /// Prioritizes: exact match > shorter prefix match > contains match
    fn select_best_tv_match<'a>(
        &self,
        query: &str,
        results: &'a [crate::services::tmdb::TvSearchItem],
    ) -> Option<&'a crate::services::tmdb::TvSearchItem> {
        if results.is_empty() {
            return None;
        }

        let query_lower = query.to_lowercase();
        let mut best_idx = 0;
        let mut best_score: i32 = -1;

        for (i, show) in results.iter().enumerate() {
            let name_lower = show.name.to_lowercase();
            let orig_lower = show.original_name.to_lowercase();

            let mut score: i32 = 0;

            // Exact match gets highest score
            if name_lower == query_lower || orig_lower == query_lower {
                score += 1000;
            }
            // Query is a prefix of the result name - prefer SHORTEST match (most specific)
            else if name_lower.starts_with(&query_lower) || orig_lower.starts_with(&query_lower) {
                // Shorter result name = better match (e.g., "战地青春之歌" < "战地青春：直击...")
                score += 500;
                // BONUS for shorter names (closer to query length)
                let len_diff = show.name.chars().count() as i32 - query.chars().count() as i32;
                score -= len_diff * 10; // Penalize longer names heavily
            }
            // Result name is contained in query (query is more specific)
            else if query_lower.contains(&name_lower) || query_lower.contains(&orig_lower) {
                score += 400;
            }
            // Query is contained in result name
            else if name_lower.contains(&query_lower) || orig_lower.contains(&query_lower) {
                score += 100;
            }
            // Check character overlap
            else {
                let query_chars: std::collections::HashSet<char> = query.chars().collect();
                let name_chars: std::collections::HashSet<char> = show.name.chars().collect();
                let common = query_chars.intersection(&name_chars).count();
                let min_len = query_chars.len().min(name_chars.len());
                if min_len > 0 && common * 2 >= min_len {
                    score += 50;
                } else {
                    continue; // Not a good match
                }
            }

            tracing::debug!(
                "TV match candidate: {} (score: {})",
                show.name,
                score
            );

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        if best_score >= 0 {
            tracing::debug!(
                "Selected best TV match: {} (score: {})",
                results[best_idx].name,
                best_score
            );
            Some(&results[best_idx])
        } else {
            None
        }
    }

    /// Get TV show details from TMDB.
    async fn get_tvshow_details(
        &self,
        client: &TmdbClient,
        tv_id: u64,
        parsed: &ParsedFilename,
    ) -> Result<(Option<TvShowMetadata>, Option<EpisodeMetadata>)> {
        let details = client.get_tv_details(tv_id).await?;

        // Extract year from first_air_date
        let year = details
            .first_air_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok())
            .unwrap_or(0);

        // Get poster URL
        let poster_urls = details.poster_path
            .as_ref()
            .map(|p| vec![format!("https://image.tmdb.org/t/p/original{}", p)])
            .unwrap_or_default();

        // Get backdrop URL
        let backdrop_url = details.backdrop_path.as_ref().map(|p| {
            client.get_poster_url(p, "original")
        });

        // Extract genres
        let genres = details.genres
            .as_ref()
            .map(|g| g.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract countries
        let countries = details.production_countries
            .as_ref()
            .map(|c| c.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract country codes (ISO 3166-1)
        let country_codes = details.production_countries
            .as_ref()
            .map(|c| c.iter().map(|x| x.iso_3166_1.clone()).collect())
            .unwrap_or_default();

        // Extract networks
        let networks = details.networks
            .as_ref()
            .map(|n| n.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract creators
        let creators = details.created_by
            .as_ref()
            .map(|c| c.iter().map(|x| x.name.clone()).collect())
            .unwrap_or_default();

        // Extract actors (top 10)
        let actors = details.credits
            .as_ref()
            .and_then(|c| c.cast.as_ref())
            .map(|cast| {
                cast.iter()
                    .take(10)
                    .map(|c| crate::models::media::Actor {
                        name: c.name.clone(),
                        role: c.character.clone(),
                        order: c.order,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let show = TvShowMetadata {
            tmdb_id: details.id,
            imdb_id: details.external_ids.and_then(|e| e.imdb_id),
            original_name: details.original_name,
            name: details.name,
            original_language: details.original_language,
            year,
            first_air_date: details.first_air_date,
            overview: details.overview,
            tagline: details.tagline,
            genres,
            countries,
            country_codes,
            networks,
            rating: details.vote_average,
            votes: details.vote_count,
            number_of_seasons: details.number_of_seasons,
            number_of_episodes: details.number_of_episodes,
            status: details.status,
            creators,
            actors,
            poster_urls,
            backdrop_url,
        };

        // If we have season/episode info, get episode details
        // Note: parsed.season/episode may be None for the first file which uses AI parsing.
        // The actual episode info will be extracted by regex in process_single_video_with_cache.
        // So we return None here - the caller will handle getting episode details.
        let episode = if let (Some(season), Some(ep)) = (parsed.season, parsed.episode) {
            match client.get_episode_details(tv_id, season, ep).await {
                Ok(ep_details) => Some(EpisodeMetadata {
                    season_number: season,
                    episode_number: ep,
                    name: ep_details.name,
                    original_name: None,
                    air_date: ep_details.air_date,
                    overview: ep_details.overview,
                }),
                Err(_) => Some(EpisodeMetadata {
                    season_number: season,
                    episode_number: ep,
                    name: format!("Episode {}", ep),
                    original_name: None,
                    air_date: None,
                    overview: None,
                }),
            }
        } else {
            // Season/episode not parsed from input - return None
            // The caller should extract episode info from filename using regex
            None
        };

        Ok((Some(show), episode))
    }

    /// Select the best movie match from search results.
    /// Prioritizes: 1) exact title match, 2) already released movies with most votes.
    fn select_best_movie_match<'a>(
        &self,
        results: &'a [crate::services::tmdb::MovieSearchItem],
        query_title: &str,
    ) -> &'a crate::services::tmdb::MovieSearchItem {
        use chrono::Datelike;
        let current_year = chrono::Utc::now().year() as u16;
        
        // Normalize query title for comparison
        let query_normalized = self.normalize_title(query_title);

        // Filter out future movies and find the best match
        let mut best_idx = 0;
        let mut best_score: i64 = -1;

        for (i, movie) in results.iter().enumerate() {
            // Extract year from release_date
            let year: u16 = movie
                .release_date
                .as_ref()
                .and_then(|d| d.split('-').next())
                .and_then(|y| y.parse().ok())
                .unwrap_or(0);

            // Skip future movies (year > current year + 1)
            // Allow movies releasing next year (pre-release content is common)
            if year > current_year + 1 {
                tracing::debug!(
                    "Skipping far future movie: {} ({})",
                    movie.title,
                    year
                );
                continue;
            }

            // Check for exact title match (highest priority)
            let title_normalized = self.normalize_title(&movie.title);
            let orig_title_normalized = self.normalize_title(&movie.original_title);
            
            let exact_match = title_normalized == query_normalized 
                || orig_title_normalized == query_normalized;
            
            // Score calculation:
            // - Exact match: +100000 (highest priority)
            // - Vote count: up to ~10000 for popular movies
            // - Valid date: +100
            let exact_match_bonus: i64 = if exact_match { 100000 } else { 0 };
            let vote_count = movie.vote_count.unwrap_or(0) as i64;
            let date_bonus: i64 = if year > 0 { 100 } else { 0 };
            
            let score = exact_match_bonus + vote_count + date_bonus;

            tracing::debug!(
                "Movie candidate: {} (year={}, votes={}, exact={}, score={})",
                movie.title, year, vote_count, exact_match, score
            );

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        tracing::debug!(
            "Selected best match: {} (score: {})",
            results[best_idx].title,
            best_score
        );

        &results[best_idx]
    }
    
    /// Normalize title for comparison (lowercase, remove punctuation/spaces).
    fn normalize_title(&self, title: &str) -> String {
        title
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
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

        // Extract country codes (ISO 3166-1)
        let country_codes = details
            .production_countries
            .as_ref()
            .map(|c| c.iter().map(|x| x.iso_3166_1.clone()).collect())
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
            country_codes,
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
        tvshow_metadata: &Option<(TvShowMetadata, Option<EpisodeMetadata>)>,
        parsed: &ParsedFilename,
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

        let (folder_name, filename, nfo_name, season_folder) = match media_type {
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

                (folder, filename, nfo, None)
            }
            MediaType::TvShows => {
                let (show, episode) = tvshow_metadata.as_ref().ok_or_else(|| {
                    crate::Error::other("Missing TV show metadata")
                })?;

                // TV show folder: ShowName (Year)
                let folder = gen_folder::generate_tvshow_folder(show);
                
                // Season folder: Season XX
                let season_num = parsed.season.unwrap_or(1);
                let season_folder_name = format!("Season {:02}", season_num);
                
                // Episode filename
                let ep_num = parsed.episode.unwrap_or(1);
                let ep_meta = episode.clone().unwrap_or_else(|| EpisodeMetadata {
                    season_number: season_num,
                    episode_number: ep_num,
                    name: format!("Episode {}", ep_num),
                    original_name: None,
                    air_date: None,
                    overview: None,
                });
                let filename = gen_filename::generate_episode_filename(
                    show,
                    &ep_meta,
                    video_metadata,
                    extension,
                );
                
                // For TV shows, use tvshow.nfo in root folder (not per-episode)
                // Jellyfin/Kodi will fetch episode info automatically
                let nfo = "tvshow.nfo".to_string();

                (folder, filename, nfo, Some(season_folder_name))
            }
        };

        // Get country folder name (e.g., "CN_China", "US_UnitedStates")
        // Uses original_language to determine the best country for co-productions
        let country_folder = match media_type {
            MediaType::Movies => {
                movie_metadata.as_ref().and_then(|m| {
                    self.format_country_folder(
                        &m.country_codes,
                        &m.countries,
                        &m.original_language,
                    )
                })
            }
            MediaType::TvShows => {
                tvshow_metadata.as_ref().and_then(|(show, _)| {
                    self.format_country_folder(
                        &show.country_codes,
                        &show.countries,
                        &show.original_language,
                    )
                })
            }
        }.unwrap_or_else(|| "Unknown".to_string());

        // Build target paths with country folder layer
        let country_path = target.join(&country_folder);
        let show_folder = country_path.join(&folder_name);
        let target_folder = if let Some(ref season_dir) = season_folder {
            show_folder.join(season_dir)
        } else {
            show_folder.clone()
        };
        let target_file = target_folder.join(&filename);
        
        // For TV shows, NFO goes in show root folder; for movies, in movie folder
        let target_nfo = if season_folder.is_some() {
            show_folder.join(&nfo_name)
        } else {
            target_folder.join(&nfo_name)
        };

        // Operation 1: Create directory (including parent dirs)
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

        // Operation 3: Create NFO file (for TV shows, only if not already exists)
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
            // For movies: poster in movie folder (which is target_folder)
            // For TV shows: poster in show folder (which is show_folder, not season folder)
            // Both already include the country_folder layer
            let poster_folder = if season_folder.is_some() {
                show_folder.clone()  // TV shows: use show_folder (includes country_folder)
            } else {
                target_folder.clone()  // Movies: use target_folder (includes country_folder)
            };
            
            let poster_url = movie_metadata.as_ref()
                .and_then(|m| m.poster_urls.first().cloned())
                .or_else(|| tvshow_metadata.as_ref()
                    .and_then(|(s, _)| s.poster_urls.first().cloned()));
            
            if let Some(url) = poster_url {
                let poster_path = poster_folder.join("poster.jpg");
                operations.push(Operation {
                    op: OperationType::Download,
                    from: None,
                    to: poster_path,
                    url: Some(url),
                    content_ref: None,
                });
            }
        }

        let display_folder = if let Some(ref season_dir) = season_folder {
            format!("{}/{}", folder_name, season_dir)
        } else {
            folder_name.clone()
        };

        let target_info = TargetInfo {
            folder: display_folder,
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


