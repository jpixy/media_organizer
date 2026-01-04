//! Integration tests for the metadata extraction module.
//!
//! Tests cover:
//! - Directory classification (season, quality, category, organized, title)
//! - Filename metadata extraction

use media_organizer::core::metadata::{
    classify_directory, extract_from_filename, CategoryType, DirectoryType,
};

// ========== DIRECTORY CLASSIFICATION TESTS ==========

#[test]
fn test_classify_season_directory() {
    // English patterns
    assert_eq!(
        classify_directory("Season 01"),
        DirectoryType::SeasonDirectory(1)
    );
    assert_eq!(
        classify_directory("Season 1"),
        DirectoryType::SeasonDirectory(1)
    );
    assert_eq!(
        classify_directory("S02"),
        DirectoryType::SeasonDirectory(2)
    );

    // Chinese patterns
    assert_eq!(
        classify_directory("第一季"),
        DirectoryType::SeasonDirectory(1)
    );
    assert_eq!(
        classify_directory("第2季"),
        DirectoryType::SeasonDirectory(2)
    );
}

#[test]
fn test_classify_quality_directory() {
    assert_eq!(classify_directory("4K"), DirectoryType::QualityDirectory);
    assert_eq!(classify_directory("1080p"), DirectoryType::QualityDirectory);
    assert_eq!(classify_directory("720p"), DirectoryType::QualityDirectory);
    assert_eq!(classify_directory("BluRay"), DirectoryType::QualityDirectory);
    assert_eq!(classify_directory("WEB-DL"), DirectoryType::QualityDirectory);
}

#[test]
fn test_classify_category_directory() {
    // Region categories
    match classify_directory("韩剧") {
        DirectoryType::CategoryDirectory(CategoryType::Region(code)) => {
            assert_eq!(code, "KR");
        }
        _ => panic!("Expected CategoryDirectory(Region)"),
    }

    match classify_directory("日剧") {
        DirectoryType::CategoryDirectory(CategoryType::Region(code)) => {
            assert_eq!(code, "JP");
        }
        _ => panic!("Expected CategoryDirectory(Region)"),
    }

    // Year categories
    match classify_directory("2024") {
        DirectoryType::CategoryDirectory(CategoryType::Year(year)) => {
            assert_eq!(year, 2024);
        }
        _ => panic!("Expected CategoryDirectory(Year)"),
    }
}

#[test]
fn test_classify_organized_directory() {
    // Full format with IMDB and TMDB ID
    let result = classify_directory("[罚罪2](2025)-tt36771056-tmdb296146");
    match result {
        DirectoryType::OrganizedDirectory(info) => {
            assert_eq!(info.title, "罚罪2");
            assert_eq!(info.year, Some(2025));
            assert_eq!(info.tmdb_id, 296146);
            assert_eq!(info.imdb_id, Some("tt36771056".to_string()));
        }
        _ => panic!("Expected OrganizedDirectory, got {:?}", result),
    }

    // Chinese title only format
    let result = classify_directory("[黑客帝国](1999)-tt0133093-tmdb603");
    match result {
        DirectoryType::OrganizedDirectory(info) => {
            assert_eq!(info.title, "黑客帝国");
            assert_eq!(info.year, Some(1999));
            assert_eq!(info.tmdb_id, 603);
        }
        _ => panic!("Expected OrganizedDirectory, got {:?}", result),
    }
}

#[test]
fn test_classify_title_directory() {
    // Chinese title with year
    let result = classify_directory("复仇者联盟 (2012)");
    match result {
        DirectoryType::TitleDirectory(info) => {
            assert!(info.chinese_title.is_some());
            assert_eq!(info.year, Some(2012));
        }
        _ => panic!("Expected TitleDirectory, got {:?}", result),
    }

    // Mixed Chinese/English title
    let result = classify_directory("复仇者联盟 The Avengers 2012");
    match result {
        DirectoryType::TitleDirectory(info) => {
            assert!(info.chinese_title.is_some() || info.english_title.is_some());
        }
        _ => panic!("Expected TitleDirectory, got {:?}", result),
    }
}

// ========== FILENAME EXTRACTION TESTS ==========

#[test]
fn test_extract_from_filename() {
    // Standard SxxExx format
    let result = extract_from_filename("Breaking.Bad.S01E01.1080p.mp4");
    assert_eq!(result.season, Some(1));
    assert_eq!(result.episode, Some(1));

    // Chinese episode format
    let result = extract_from_filename("第01集.mp4");
    assert_eq!(result.episode, Some(1));
}

#[test]
fn test_extract_season_episode_variations() {
    // s01e01 lowercase
    let result = extract_from_filename("show.s01e05.mkv");
    assert_eq!(result.season, Some(1));
    assert_eq!(result.episode, Some(5));

    // S01E01 uppercase
    let result = extract_from_filename("Show.S02E10.mkv");
    assert_eq!(result.season, Some(2));
    assert_eq!(result.episode, Some(10));

    // Episode only
    let result = extract_from_filename("E05.mkv");
    assert_eq!(result.episode, Some(5));
}

#[test]
fn test_extract_year_from_filename() {
    let result = extract_from_filename("Movie.2024.1080p.mkv");
    assert_eq!(result.year, Some(2024));

    let result = extract_from_filename("Movie (2023).mkv");
    assert_eq!(result.year, Some(2023));
}
