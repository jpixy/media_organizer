//! Filename parser module using AI.
//!
//! Uses Ollama to parse video filenames and extract:
//! - Original title (usually English)
//! - Localized title (Chinese)
//! - Release year
//! - Media type hints (movie vs TV show)

use crate::models::media::MediaType;
use crate::services::ollama::OllamaClient;
use crate::Result;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// Parsed filename information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedFilename {
    /// Original title (usually English).
    pub original_title: Option<String>,
    /// Localized title (Chinese).
    pub title: Option<String>,
    /// Year of release.
    pub year: Option<u16>,
    /// Season number (for TV shows).
    pub season: Option<u16>,
    /// Episode number (for TV shows).
    pub episode: Option<u16>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Raw AI response for debugging.
    pub raw_response: Option<String>,
}

/// AI response structure for parsing.
/// Uses serde_json::Value for season/episode to handle both string ("S01") and number (1) formats.
#[derive(Debug, Deserialize)]
struct AiParseResponse {
    original_title: Option<String>,
    title: Option<String>,
    year: Option<u16>,
    season: Option<serde_json::Value>,
    episode: Option<serde_json::Value>,
    confidence: Option<f32>,
}

impl AiParseResponse {
    /// Parse season value from various formats: "S01", "1", 1, etc.
    fn parse_season(&self) -> Option<u16> {
        self.season.as_ref().and_then(|v| Self::parse_number(v))
    }

    /// Parse episode value from various formats: "E05", "5", 5, etc.
    fn parse_episode(&self) -> Option<u16> {
        self.episode.as_ref().and_then(|v| Self::parse_number(v))
    }

    /// Parse a number from various formats.
    fn parse_number(value: &serde_json::Value) -> Option<u16> {
        match value {
            serde_json::Value::Number(n) => n.as_u64().map(|n| n as u16),
            serde_json::Value::String(s) => {
                // Try direct parse first
                if let Ok(n) = s.parse::<u16>() {
                    return Some(n);
                }
                // Try extracting number from "S01", "E05", etc.
                let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                digits.parse::<u16>().ok()
            }
            _ => None,
        }
    }
}

/// Parser configuration.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum concurrent parsing requests.
    pub max_concurrent: usize,
    /// Minimum confidence threshold for valid results.
    pub min_confidence: f32,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            min_confidence: 0.5,
        }
    }
}

/// Filename parser using Ollama AI.
pub struct FilenameParser {
    client: OllamaClient,
    config: ParserConfig,
}

impl FilenameParser {
    /// Create a new parser with default configuration.
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new(),
            config: ParserConfig::default(),
        }
    }

    /// Create a new parser with custom configuration.
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            client: OllamaClient::new(),
            config,
        }
    }

    /// Create a new parser with custom Ollama client.
    pub fn with_client(client: OllamaClient) -> Self {
        Self {
            client,
            config: ParserConfig::default(),
        }
    }

    /// Generate the prompt for parsing a filename.
    /// 
    /// The prompt is in Chinese to better handle Chinese filenames and leverage
    /// the AI model's understanding of Chinese media naming conventions.
    fn generate_prompt(&self, filename: &str, media_type: MediaType) -> String {
        // Type hint: "This is a movie file" / "This is a TV show file"
        let type_hint = match media_type {
            MediaType::Movies => "这是一个电影文件",
            MediaType::TvShows => "这是一个电视剧/剧集文件",
        };

        format!(
            r#"你是一个视频文件名解析专家。请分析以下视频文件名，提取关键信息。

文件名: {filename}
提示: {type_hint}

请提取以下信息并以JSON格式返回：
1. original_title: 原始标题（通常是英文）
2. title: 中文标题（如果有的话）
3. year: 发行年份（4位数字）
4. season: 季数（仅电视剧，如S01表示第1季）
5. episode: 集数（仅电视剧，如E05表示第5集）
6. confidence: 你对解析结果的置信度（0.0到1.0之间的小数）

注意事项：
- 忽略分辨率（如1080p、4K、2160p）、编码格式（如x265、HEVC）、音频格式（如DTS、AAC）等技术信息
- 忽略发布组名称（通常在方括号或末尾）
- 如果文件名中包含中英文混合，请分别提取
- 如果无法确定某个字段，返回null
- **重要**: 年份必须是文件名中明确出现的4位数字(1900-2030)，不要猜测！如果文件名中没有年份，返回null
- 例如："动物农场.mp4"没有年份，应返回null；"雏菊 导演剪辑版 2006.mp4"年份是2006
- **重要**: 续集编号（如2、3、II、III）是标题的一部分！例如"刺杀小说家2"的标题是"刺杀小说家2"而不是"刺杀小说家"
- **重要**: 不要把紧跟在标题后面的数字当作分辨率。例如"刺杀小说家2.4k.mp4"中，"2"是续集编号，"4k"才是分辨率
- 常见续集模式：标题2、标题3、标题II、标题III、标题:副标题
- **重要**: 版本信息不是标题的一部分！如"导演剪辑版"、"加长版"、"未删减版"、"特效版"、"IMAX版"、"3D版"等都不应包含在标题中
- 例如："雏菊 导演剪辑版 2006.mp4" 的标题是"雏菊"，年份是2006

只返回JSON对象，不要包含其他文字：
{{"original_title": "...", "title": "...", "year": ..., "season": ..., "episode": ..., "confidence": ...}}"#
        )
    }

    /// Parse a single filename using AI.
    pub async fn parse(&self, filename: &str, media_type: MediaType) -> Result<ParsedFilename> {
        let prompt = self.generate_prompt(filename, media_type);

        tracing::debug!("Parsing filename: {}", filename);
        println!("    [AI] Parsing: {}...", filename);

        let start = std::time::Instant::now();
        
        // Call Ollama API with JSON format
        let response = self.client.generate_with_format(&prompt, Some("json")).await?;

        let elapsed = start.elapsed();
        println!("    [OK] Parsed in {:.1}s", elapsed.as_secs_f32());
        tracing::debug!("AI response: {}", response.response);

        // Parse the JSON response
        let parsed = self.parse_ai_response(&response.response, filename)?;

        // Validate the result
        let validated = self.validate_result(parsed)?;

        Ok(validated)
    }

    /// Parse AI response into ParsedFilename.
    fn parse_ai_response(&self, response: &str, filename: &str) -> Result<ParsedFilename> {
        // Try to parse as JSON
        match serde_json::from_str::<AiParseResponse>(response) {
            Ok(ai_response) => {
                // Normalize confidence to 0.0-1.0 range (AI sometimes returns 0-100)
                let raw_confidence = ai_response.confidence.unwrap_or(0.5);
                let confidence = if raw_confidence > 1.0 {
                    raw_confidence / 100.0
                } else {
                    raw_confidence
                };
                
                let season = ai_response.parse_season();
                let episode = ai_response.parse_episode();
                Ok(ParsedFilename {
                    original_title: ai_response.original_title,
                    title: ai_response.title,
                    year: ai_response.year,
                    season,
                    episode,
                    confidence,
                    raw_response: Some(response.to_string()),
                })
            }
            Err(e) => {
                tracing::warn!("Failed to parse AI response for '{}': {}", filename, e);
                // Return a low-confidence result with raw response
                Ok(ParsedFilename {
                    raw_response: Some(response.to_string()),
                    confidence: 0.0,
                    ..Default::default()
                })
            }
        }
    }

    /// Validate parsed result.
    fn validate_result(&self, mut parsed: ParsedFilename) -> Result<ParsedFilename> {
        // Validate year range (1900 - current year + 5)
        if let Some(year) = parsed.year {
            let current_year = chrono::Utc::now().year() as u16;
            if year < 1900 || year > current_year + 5 {
                tracing::warn!("Invalid year {}, ignoring", year);
                parsed.year = None;
                parsed.confidence *= 0.5;
            }
        }

        // Validate season/episode numbers
        if let Some(season) = parsed.season {
            if season == 0 || season > 100 {
                parsed.season = None;
            }
        }
        if let Some(episode) = parsed.episode {
            if episode == 0 || episode > 1000 {
                parsed.episode = None;
            }
        }

        // Validate titles are not empty
        if let Some(ref title) = parsed.original_title {
            if title.trim().is_empty() {
                parsed.original_title = None;
            }
        }
        if let Some(ref title) = parsed.title {
            if title.trim().is_empty() {
                parsed.title = None;
            }
        }

        // Adjust confidence if missing critical fields
        if parsed.original_title.is_none() && parsed.title.is_none() {
            parsed.confidence = 0.0;
        }

        Ok(parsed)
    }

    /// Parse multiple filenames in batch with concurrency control.
    pub async fn parse_batch(
        &self,
        filenames: &[String],
        media_type: MediaType,
    ) -> Vec<(String, Result<ParsedFilename>)> {
        use tokio::sync::Semaphore;
        use std::sync::Arc;

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent));
        let mut handles = Vec::new();

        for filename in filenames {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let filename = filename.clone();
            let prompt = self.generate_prompt(&filename, media_type);
            let client = self.client.clone();

            let handle = tokio::spawn(async move {
                let result = async {
                    let response = client.generate_with_format(&prompt, Some("json")).await?;
                    
                    // Parse response
                    let parsed: Result<ParsedFilename> = match serde_json::from_str::<AiParseResponse>(&response.response) {
                        Ok(ai_response) => {
                            // Normalize confidence to 0.0-1.0 range
                            let raw_confidence = ai_response.confidence.unwrap_or(0.5);
                            let confidence = if raw_confidence > 1.0 {
                                raw_confidence / 100.0
                            } else {
                                raw_confidence
                            };
                            
                            let season = ai_response.parse_season();
                            let episode = ai_response.parse_episode();
                            Ok(ParsedFilename {
                                original_title: ai_response.original_title,
                                title: ai_response.title,
                                year: ai_response.year,
                                season,
                                episode,
                                confidence,
                                raw_response: Some(response.response),
                            })
                        }
                        Err(_) => {
                            Ok(ParsedFilename {
                                raw_response: Some(response.response),
                                confidence: 0.0,
                                ..Default::default()
                            })
                        }
                    };
                    parsed
                }.await;

                drop(permit);
                (filename, result)
            });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!("Task failed: {}", e);
                }
            }
        }

        results
    }

    /// Check if a parsed result meets the minimum confidence threshold.
    pub fn is_valid(&self, parsed: &ParsedFilename) -> bool {
        parsed.confidence >= self.config.min_confidence
            && (parsed.original_title.is_some() || parsed.title.is_some())
    }
}

impl Default for FilenameParser {
    fn default() -> Self {
        Self::new()
    }
}

// Make OllamaClient cloneable for batch processing
impl Clone for OllamaClient {
    fn clone(&self) -> Self {
        OllamaClient::new()
    }
}

/// Parse a video filename using AI (convenience function).
pub async fn parse_filename(filename: &str, media_type: MediaType) -> Result<ParsedFilename> {
    let parser = FilenameParser::new();
    parser.parse(filename, media_type).await
}

/// Parse multiple filenames in batch (convenience function).
pub async fn parse_filenames(
    filenames: &[String],
    media_type: MediaType,
) -> Vec<(String, Result<ParsedFilename>)> {
    let parser = FilenameParser::new();
    parser.parse_batch(filenames, media_type).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_filename_default() {
        let parsed = ParsedFilename::default();
        assert!(parsed.original_title.is_none());
        assert!(parsed.title.is_none());
        assert!(parsed.year.is_none());
        assert_eq!(parsed.confidence, 0.0);
    }

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.min_confidence, 0.5);
    }

    #[test]
    fn test_generate_prompt_movie() {
        let parser = FilenameParser::new();
        let prompt = parser.generate_prompt("Avatar.2009.1080p.BluRay.mkv", MediaType::Movies);
        
        assert!(prompt.contains("Avatar.2009.1080p.BluRay.mkv"));
        assert!(prompt.contains("电影"));
    }

    #[test]
    fn test_generate_prompt_tvshow() {
        let parser = FilenameParser::new();
        let prompt = parser.generate_prompt("Breaking.Bad.S01E01.720p.mkv", MediaType::TvShows);
        
        assert!(prompt.contains("Breaking.Bad.S01E01.720p.mkv"));
        assert!(prompt.contains("电视剧"));
    }

    #[test]
    fn test_validate_year_range() {
        let parser = FilenameParser::new();
        
        // Valid year
        let parsed = ParsedFilename {
            year: Some(2020),
            confidence: 1.0,
            original_title: Some("Test".to_string()),
            ..Default::default()
        };
        let result = parser.validate_result(parsed).unwrap();
        assert_eq!(result.year, Some(2020));
        
        // Invalid year (too old)
        let parsed = ParsedFilename {
            year: Some(1800),
            confidence: 1.0,
            original_title: Some("Test".to_string()),
            ..Default::default()
        };
        let result = parser.validate_result(parsed).unwrap();
        assert!(result.year.is_none());
    }

    #[test]
    fn test_is_valid() {
        let parser = FilenameParser::new();
        
        // Valid result
        let parsed = ParsedFilename {
            original_title: Some("Avatar".to_string()),
            confidence: 0.8,
            ..Default::default()
        };
        assert!(parser.is_valid(&parsed));
        
        // Low confidence
        let parsed = ParsedFilename {
            original_title: Some("Avatar".to_string()),
            confidence: 0.3,
            ..Default::default()
        };
        assert!(!parser.is_valid(&parsed));
        
        // No title
        let parsed = ParsedFilename {
            confidence: 0.8,
            ..Default::default()
        };
        assert!(!parser.is_valid(&parsed));
    }
}

/// Extract season and episode numbers from filename using regex.
/// This avoids calling AI for each episode file.
/// 
/// Supports patterns like:
/// - "01.mp4", "02.mp4" (just episode number)
/// - "S01E01.mp4", "s01e05.mkv"
/// - "E01.mp4", "E05.mkv"
/// - "第01集.mp4", "第5集.mkv"
/// - "01 4K.mp4" (episode with quality suffix)
pub fn extract_episode_from_filename(filename: &str) -> (Option<u16>, Option<u16>) {
    // Remove extension
    let name = filename
        .rsplit('.')
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(".");
    
    let name_lower = name.to_lowercase();
    
    // Pattern 1: S01E01, s01e05
    if let Some(caps) = regex_match_sxxexx(&name_lower) {
        return caps;
    }
    
    // Pattern 2: Season 01 Episode 05
    if let Some(caps) = regex_match_season_episode(&name_lower) {
        return caps;
    }
    
    // Pattern 3: E01, E05 (episode only)
    if let Some(ep) = regex_match_exx(&name_lower) {
        return (Some(1), Some(ep)); // Default to season 1
    }
    
    // Pattern 4: 第01集, 第5集
    if let Some(ep) = regex_match_chinese_episode(&name) {
        return (Some(1), Some(ep)); // Default to season 1
    }
    
    // Pattern 5: Just a number at the start (01, 02, 1, 2)
    // Handle "01 4K.mp4", "02.mp4", etc.
    if let Some(ep) = regex_match_leading_number(&name) {
        return (Some(1), Some(ep)); // Default to season 1
    }
    
    (None, None)
}

fn regex_match_sxxexx(s: &str) -> Option<(Option<u16>, Option<u16>)> {
    // Match S01E01, s1e5, etc.
    let re = regex::Regex::new(r"s(\d{1,2})e(\d{1,3})").ok()?;
    let caps = re.captures(s)?;
    let season: u16 = caps.get(1)?.as_str().parse().ok()?;
    let episode: u16 = caps.get(2)?.as_str().parse().ok()?;
    Some((Some(season), Some(episode)))
}

fn regex_match_season_episode(s: &str) -> Option<(Option<u16>, Option<u16>)> {
    // Match "season 01 episode 05", "season1 episode5"
    let re = regex::Regex::new(r"season\s*(\d{1,2}).*episode\s*(\d{1,3})").ok()?;
    let caps = re.captures(s)?;
    let season: u16 = caps.get(1)?.as_str().parse().ok()?;
    let episode: u16 = caps.get(2)?.as_str().parse().ok()?;
    Some((Some(season), Some(episode)))
}

fn regex_match_exx(s: &str) -> Option<u16> {
    // Match E01, e5, EP01, ep05
    let re = regex::Regex::new(r"(?:^|[^a-z])e[p]?(\d{1,3})(?:[^0-9]|$)").ok()?;
    let caps = re.captures(s)?;
    caps.get(1)?.as_str().parse().ok()
}

fn regex_match_chinese_episode(s: &str) -> Option<u16> {
    // Match 第01集, 第5集
    let re = regex::Regex::new(r"第(\d{1,3})集").ok()?;
    let caps = re.captures(s)?;
    caps.get(1)?.as_str().parse().ok()
}

fn regex_match_leading_number(s: &str) -> Option<u16> {
    // Match numbers at the start: "01", "02", "1", "2"
    // Also handles "01 4K", "02 1080p"
    let trimmed = s.trim();
    let re = regex::Regex::new(r"^(\d{1,3})(?:\s|$|[^0-9])").ok()?;
    let caps = re.captures(trimmed)?;
    let num: u16 = caps.get(1)?.as_str().parse().ok()?;
    // Sanity check: episode numbers are usually 1-999
    if num >= 1 && num <= 999 {
        Some(num)
    } else {
        None
    }
}

/// Check if a filename matches the organized output format.
/// 
/// Organized formats:
/// - TV: `[Title]-S01E01-[Episode Name]-1080p-...`
/// - Movie: `[EnglishTitle][ChineseTitle](Year)-tt12345-tmdb67890-1080p-...`
pub fn is_organized_filename(filename: &str) -> bool {
    // TV show pattern: [Title]-S01E01-[...]
    let tv_pattern = regex::Regex::new(r"^\[.+\]-S\d{2}E\d{2,3}-\[.+\]-").ok();
    if let Some(re) = tv_pattern {
        if re.is_match(filename) {
            return true;
        }
    }
    
    // Movie pattern: [Title](Year)-tt...-tmdb...-
    // or [Title][Title](Year)-tt...-tmdb...-
    let movie_pattern = regex::Regex::new(r"^\[.+\](?:\[.+\])?\(\d{4}\)-(?:tt\d+)?-?tmdb\d+-").ok();
    if let Some(re) = movie_pattern {
        if re.is_match(filename) {
            return true;
        }
    }
    
    false
}

/// Parse an organized TV show filename to extract metadata.
/// 
/// Format: `[Title]-S01E01-[Episode Name]-1080p-WEB-DL-...`
/// Returns: (title, season, episode, episode_name)
pub fn parse_organized_tvshow_filename(filename: &str) -> Option<OrganizedTvShowInfo> {
    let re = regex::Regex::new(
        r"^\[([^\]]+)\]-S(\d{2})E(\d{2,3})-\[([^\]]+)\]-"
    ).ok()?;
    
    let caps = re.captures(filename)?;
    
    Some(OrganizedTvShowInfo {
        title: caps.get(1)?.as_str().to_string(),
        season: caps.get(2)?.as_str().parse().ok()?,
        episode: caps.get(3)?.as_str().parse().ok()?,
        episode_name: caps.get(4)?.as_str().to_string(),
    })
}

/// Parse an organized movie filename to extract metadata.
/// 
/// Format: `[EnglishTitle][ChineseTitle](Year)-tt12345-tmdb67890-1080p-...`
/// or: `[Title](Year)-tt12345-tmdb67890-1080p-...`
/// Returns: (original_title, title, year, imdb_id, tmdb_id)
pub fn parse_organized_movie_filename(filename: &str) -> Option<OrganizedMovieInfo> {
    // Try two-title format first: [English][Chinese](Year)
    let re_two = regex::Regex::new(
        r"^\[([^\]]+)\]\[([^\]]+)\]\((\d{4})\)-(?:tt(\d+))?-?tmdb(\d+)-"
    ).ok()?;
    
    if let Some(caps) = re_two.captures(filename) {
        return Some(OrganizedMovieInfo {
            original_title: Some(caps.get(1)?.as_str().to_string()),
            title: Some(caps.get(2)?.as_str().to_string()),
            year: caps.get(3)?.as_str().parse().ok()?,
            imdb_id: caps.get(4).map(|m| format!("tt{}", m.as_str())),
            tmdb_id: caps.get(5)?.as_str().parse().ok()?,
        });
    }
    
    // Single-title format: [Title](Year)
    let re_one = regex::Regex::new(
        r"^\[([^\]]+)\]\((\d{4})\)-(?:tt(\d+))?-?tmdb(\d+)-"
    ).ok()?;
    
    if let Some(caps) = re_one.captures(filename) {
        return Some(OrganizedMovieInfo {
            original_title: Some(caps.get(1)?.as_str().to_string()),
            title: None,
            year: caps.get(2)?.as_str().parse().ok()?,
            imdb_id: caps.get(3).map(|m| format!("tt{}", m.as_str())),
            tmdb_id: caps.get(4)?.as_str().parse().ok()?,
        });
    }
    
    None
}

/// Information extracted from an organized TV show filename.
#[derive(Debug, Clone)]
pub struct OrganizedTvShowInfo {
    pub title: String,
    pub season: u16,
    pub episode: u16,
    pub episode_name: String,
}

/// Information extracted from an organized movie filename.
#[derive(Debug, Clone)]
pub struct OrganizedMovieInfo {
    pub original_title: Option<String>,
    pub title: Option<String>,
    pub year: u16,
    pub imdb_id: Option<String>,
    pub tmdb_id: u64,
}

/// Information extracted from an organized TV show folder name.
#[derive(Debug, Clone)]
pub struct OrganizedTvShowFolderInfo {
    pub title: String,
    pub year: Option<u16>,
    pub imdb_id: Option<String>,
    pub tmdb_id: u64,
}

/// Check if a folder name matches the organized TV show folder format.
/// 
/// Format: `[Title](Year)-ttIMDB-tmdbID` or `[Title](Year)-tmdbID`
pub fn is_organized_tvshow_folder(dirname: &str) -> bool {
    // Pattern: [Title](Year)-tt...-tmdb... or [Title](Year)-tmdb...
    let re = regex::Regex::new(r"^\[.+\]\(\d{4}\)-(?:tt\d+)?-?tmdb\d+$").ok();
    if let Some(re) = re {
        if re.is_match(dirname) {
            return true;
        }
    }
    false
}

/// Parse an organized TV show folder name to extract metadata.
/// 
/// Format: `[Title](Year)-ttIMDB-tmdbID` or `[Title](Year)-tmdbID`
/// Example: `[罚罪2](2025)-tt36771056-tmdb296146`
pub fn parse_organized_tvshow_folder(dirname: &str) -> Option<OrganizedTvShowFolderInfo> {
    // Pattern with IMDB: [Title](Year)-ttIMDB-tmdbID
    let re_with_imdb = regex::Regex::new(
        r"^\[([^\]]+)\]\((\d{4})\)-tt(\d+)-tmdb(\d+)$"
    ).ok()?;
    
    if let Some(caps) = re_with_imdb.captures(dirname) {
        return Some(OrganizedTvShowFolderInfo {
            title: caps.get(1)?.as_str().to_string(),
            year: caps.get(2)?.as_str().parse().ok(),
            imdb_id: Some(format!("tt{}", caps.get(3)?.as_str())),
            tmdb_id: caps.get(4)?.as_str().parse().ok()?,
        });
    }
    
    // Pattern without IMDB: [Title](Year)-tmdbID
    let re_no_imdb = regex::Regex::new(
        r"^\[([^\]]+)\]\((\d{4})\)-tmdb(\d+)$"
    ).ok()?;
    
    if let Some(caps) = re_no_imdb.captures(dirname) {
        return Some(OrganizedTvShowFolderInfo {
            title: caps.get(1)?.as_str().to_string(),
            year: caps.get(2)?.as_str().parse().ok(),
            imdb_id: None,
            tmdb_id: caps.get(3)?.as_str().parse().ok()?,
        });
    }
    
    None
}

/// Convert OrganizedTvShowInfo to ParsedFilename for consistent processing.
impl From<OrganizedTvShowInfo> for ParsedFilename {
    fn from(info: OrganizedTvShowInfo) -> Self {
        ParsedFilename {
            original_title: None,
            title: Some(info.title),
            year: None,
            season: Some(info.season),
            episode: Some(info.episode),
            confidence: 1.0, // High confidence since we parsed our own format
            raw_response: None,
        }
    }
}

/// Convert OrganizedMovieInfo to ParsedFilename for consistent processing.
impl From<OrganizedMovieInfo> for ParsedFilename {
    fn from(info: OrganizedMovieInfo) -> Self {
        ParsedFilename {
            original_title: info.original_title,
            title: info.title,
            year: Some(info.year),
            season: None,
            episode: None,
            confidence: 1.0, // High confidence since we parsed our own format
            raw_response: None,
        }
    }
}

/// Extract season number from directory name.
/// Supports Chinese season patterns like:
/// - "第一季", "第二季", "第1季", "第2季"
/// - "Season 01", "Season 1", "S01", "S1"
/// - "第一部", "第二部" (treated as seasons)
pub fn extract_season_from_dirname(dirname: &str) -> Option<u16> {
    let name = dirname.trim();
    
    // Chinese numeral to number mapping
    let chinese_nums = [
        ("一", 1), ("二", 2), ("三", 3), ("四", 4), ("五", 5),
        ("六", 6), ("七", 7), ("八", 8), ("九", 9), ("十", 10),
        ("十一", 11), ("十二", 12), ("十三", 13), ("十四", 14), ("十五", 15),
    ];
    
    // Pattern 1: "第X季" or "第X部" with Chinese numerals
    for (cn, num) in &chinese_nums {
        if name.contains(&format!("第{}季", cn)) || name.contains(&format!("第{}部", cn)) {
            return Some(*num);
        }
    }
    
    // Pattern 2: "第N季" with Arabic numerals
    if let Some(re) = regex::Regex::new(r"第(\d{1,2})季").ok() {
        if let Some(caps) = re.captures(name) {
            if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                return Some(num);
            }
        }
    }
    
    // Pattern 3: "Season N", "Season 0N"
    if let Some(re) = regex::Regex::new(r"(?i)season\s*(\d{1,2})").ok() {
        if let Some(caps) = re.captures(name) {
            if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                return Some(num);
            }
        }
    }
    
    // Pattern 4: "S01", "S1" at end or with space
    if let Some(re) = regex::Regex::new(r"(?i)(?:^|[\s\-_])s(\d{1,2})(?:$|[\s\-_])").ok() {
        if let Some(caps) = re.captures(name) {
            if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                return Some(num);
            }
        }
    }
    
    None
}
