//! Media-related data models.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Media type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movies,
    TvShows,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaType::Movies => write!(f, "movies"),
            MediaType::TvShows => write!(f, "tvshows"),
        }
    }
}

/// Video file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    /// Full path to the file.
    pub path: PathBuf,
    /// File name without path.
    pub filename: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modified time.
    pub modified: chrono::DateTime<chrono::Utc>,
    /// Whether this is a sample file.
    pub is_sample: bool,
    /// Parent directory.
    pub parent_dir: PathBuf,
}

/// Video metadata extracted from ffprobe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// Resolution (e.g., "2160p", "1080p").
    pub resolution: String,
    /// Video format (e.g., "BluRay", "WEB-DL").
    pub format: String,
    /// Video codec (e.g., "hevc", "h264").
    pub video_codec: String,
    /// Bit depth (e.g., 8, 10).
    pub bit_depth: u8,
    /// Audio codec (e.g., "dts", "ac3", "aac").
    pub audio_codec: String,
    /// Audio channels (e.g., "5.1", "7.1").
    pub audio_channels: String,
}

/// TMDB metadata for a movie.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovieMetadata {
    /// TMDB ID.
    pub tmdb_id: u64,
    /// IMDB ID.
    pub imdb_id: Option<String>,
    /// Original title (usually English).
    pub original_title: String,
    /// Localized title.
    pub title: String,
    /// Original language.
    pub original_language: String,
    /// Release year.
    pub year: u16,
    /// Full release date (YYYY-MM-DD).
    pub release_date: Option<String>,
    /// Overview/synopsis.
    pub overview: Option<String>,
    /// Tagline.
    pub tagline: Option<String>,
    /// Runtime in minutes.
    pub runtime: Option<u32>,
    /// Genres.
    pub genres: Vec<String>,
    /// Production countries.
    pub countries: Vec<String>,
    /// Production companies/studios.
    pub studios: Vec<String>,
    /// User rating (0-10).
    pub rating: Option<f32>,
    /// Vote count.
    pub votes: Option<u32>,
    /// Poster URLs.
    pub poster_urls: Vec<String>,
    /// Backdrop URL.
    pub backdrop_url: Option<String>,
    /// Directors.
    pub directors: Vec<String>,
    /// Writers.
    pub writers: Vec<String>,
    /// Main actors with roles.
    pub actors: Vec<String>,
    /// Actor roles (parallel to actors).
    pub actor_roles: Vec<String>,
    /// Certification/rating (e.g., "PG-13").
    pub certification: Option<String>,
}

/// TMDB metadata for a TV show.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TvShowMetadata {
    /// TMDB ID.
    pub tmdb_id: u64,
    /// IMDB ID.
    pub imdb_id: Option<String>,
    /// Original name.
    pub original_name: String,
    /// Localized name.
    pub name: String,
    /// Original language.
    pub original_language: String,
    /// First air year.
    pub year: u16,
    /// Overview/synopsis.
    pub overview: Option<String>,
    /// Poster URLs.
    pub poster_urls: Vec<String>,
}

/// TMDB metadata for a TV episode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    /// Season number.
    pub season_number: u16,
    /// Episode number.
    pub episode_number: u16,
    /// Episode name.
    pub name: String,
    /// Original episode name.
    pub original_name: Option<String>,
    /// Air date.
    pub air_date: Option<String>,
    /// Overview.
    pub overview: Option<String>,
}



