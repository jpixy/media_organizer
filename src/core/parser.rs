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
#[derive(Debug, Deserialize)]
struct AiParseResponse {
    original_title: Option<String>,
    title: Option<String>,
    year: Option<u16>,
    season: Option<u16>,
    episode: Option<u16>,
    confidence: Option<f32>,
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
    fn generate_prompt(&self, filename: &str, media_type: MediaType) -> String {
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
- 年份通常是4位数字，范围在1900-2030之间
- **重要**: 续集编号（如2、3、II、III）是标题的一部分！例如"刺杀小说家2"的标题是"刺杀小说家2"而不是"刺杀小说家"
- **重要**: 不要把紧跟在标题后面的数字当作分辨率。例如"刺杀小说家2.4k.mp4"中，"2"是续集编号，"4k"才是分辨率
- 常见续集模式：标题2、标题3、标题II、标题III、标题:副标题
- **重要**: 版本信息不是标题的一部分！如"导演剪辑版"、"加长版"、"未删减版"、"特效版"、"IMAX版"、"3D版"等都不应包含在标题中
- 例如："雏菊 导演剪辑版 2006.mp4" 的标题是"雏菊"，不是"雏菊 导演剪辑版"

只返回JSON对象，不要包含其他文字：
{{"original_title": "...", "title": "...", "year": ..., "season": ..., "episode": ..., "confidence": ...}}"#
        )
    }

    /// Parse a single filename using AI.
    pub async fn parse(&self, filename: &str, media_type: MediaType) -> Result<ParsedFilename> {
        let prompt = self.generate_prompt(filename, media_type);

        tracing::debug!("Parsing filename: {}", filename);
        println!("    🤖 AI parsing: {} (CPU inference may take 1-3 min)...", filename);

        let start = std::time::Instant::now();
        
        // Call Ollama API with JSON format
        let response = self.client.generate_with_format(&prompt, Some("json")).await?;

        let elapsed = start.elapsed();
        println!("    ✓ Parsed in {:.1}s", elapsed.as_secs_f32());
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
                
                Ok(ParsedFilename {
                    original_title: ai_response.original_title,
                    title: ai_response.title,
                    year: ai_response.year,
                    season: ai_response.season,
                    episode: ai_response.episode,
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
                            
                            Ok(ParsedFilename {
                                original_title: ai_response.original_title,
                                title: ai_response.title,
                                year: ai_response.year,
                                season: ai_response.season,
                                episode: ai_response.episode,
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


