//! Filename generator.

use crate::models::media::{MovieMetadata, TvShowMetadata, VideoMetadata, EpisodeMetadata};

/// Generate movie filename.
///
/// Format: `[${originalTitle}]-[${title}](${edition})-${year}-${resolution}-${format}-${codec}-${bitDepth}bit-${audioCodec}-${audioChannels}`
pub fn generate_movie_filename(
    movie: &MovieMetadata,
    video: &VideoMetadata,
    edition: Option<&str>,
    extension: &str,
) -> String {
    let mut parts = Vec::new();

    // Handle title deduplication for Chinese movies
    let is_chinese = movie.original_language == "zh";
    let titles_same = normalize_title(&movie.original_title) == normalize_title(&movie.title);

    if is_chinese || titles_same {
        parts.push(format!("[{}]", sanitize_filename(&movie.title)));
    } else {
        parts.push(format!("[{}]", sanitize_filename(&movie.original_title)));
        parts.push(format!("[{}]", sanitize_filename(&movie.title)));
    }

    // Add edition if present
    if let Some(ed) = edition {
        parts.push(format!("({})", ed));
    }

    // Add year
    parts.push(format!("({})", movie.year));

    // Add video info
    parts.push(format!("-{}", video.resolution));
    parts.push(format!("-{}", video.format));
    parts.push(format!("-{}", video.video_codec));
    parts.push(format!("-{}bit", video.bit_depth));
    parts.push(format!("-{}", video.audio_codec));
    parts.push(format!("-{}", video.audio_channels));

    format!("{}.{}", parts.join(""), extension)
}

/// Generate TV episode filename.
///
/// Format: `[${showOriginalTitle}]-S${seasonNr2}E${episodeNr2}-[${originalTitle}]-[${title}]-${format}-${codec}-${bitDepth}bit-${audioCodec}-${audioChannels}`
pub fn generate_episode_filename(
    show: &TvShowMetadata,
    episode: &EpisodeMetadata,
    video: &VideoMetadata,
    extension: &str,
) -> String {
    let mut parts = Vec::new();

    // Show title
    let is_chinese = show.original_language == "zh";
    let titles_same = normalize_title(&show.original_name) == normalize_title(&show.name);

    if is_chinese || titles_same {
        parts.push(format!("[{}]", sanitize_filename(&show.name)));
    } else {
        parts.push(format!("[{}]", sanitize_filename(&show.original_name)));
    }

    // Season and episode number
    parts.push(format!(
        "-S{:02}E{:02}",
        episode.season_number, episode.episode_number
    ));

    // Episode title
    if let Some(ref orig_name) = episode.original_name {
        if orig_name != &episode.name {
            parts.push(format!("-[{}]", sanitize_filename(orig_name)));
        }
    }
    parts.push(format!("-[{}]", sanitize_filename(&episode.name)));

    // Video info (including resolution)
    parts.push(format!("-{}", video.resolution));
    parts.push(format!("-{}", video.format));
    parts.push(format!("-{}", video.video_codec));
    parts.push(format!("-{}bit", video.bit_depth));
    parts.push(format!("-{}", video.audio_codec));
    parts.push(format!("-{}", video.audio_channels));

    format!("{}.{}", parts.join(""), extension)
}

/// Sanitize a string for use in filenames.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Normalize title for comparison.
fn normalize_title(s: &str) -> String {
    s.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_movie_filename() {
        let movie = MovieMetadata {
            original_title: "Avatar".to_string(),
            title: "阿凡达".to_string(),
            original_language: "en".to_string(),
            year: 2009,
            ..Default::default()
        };

        let video = VideoMetadata {
            resolution: "2160p".to_string(),
            format: "BluRay".to_string(),
            video_codec: "x265".to_string(),
            bit_depth: 10,
            audio_codec: "TrueHD".to_string(),
            audio_channels: "7.1".to_string(),
        };

        let filename = generate_movie_filename(&movie, &video, None, "mkv");
        assert!(filename.contains("[Avatar]"));
        assert!(filename.contains("[阿凡达]"));
        assert!(filename.contains("2160p"));
        assert!(filename.contains("10bit"));
        assert!(filename.ends_with(".mkv"));
    }
}



