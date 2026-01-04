//! Chinese text utilities.

/// Check if two strings are the same when normalized (handles Traditional/Simplified).
pub fn titles_equivalent(a: &str, b: &str) -> bool {
    // Basic normalization for now
    // TODO: Implement proper Traditional/Simplified Chinese conversion
    normalize(a) == normalize(b)
}

/// Normalize a string for comparison.
pub fn normalize(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Check if a string contains Chinese characters.
pub fn contains_chinese(s: &str) -> bool {
    s.chars().any(is_chinese_char)
}

/// Check if a character is a Chinese character.
fn is_chinese_char(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |  // CJK Unified Ideographs Extension A
        '\u{F900}'..='\u{FAFF}' |  // CJK Compatibility Ideographs
        '\u{20000}'..='\u{2A6DF}'  // CJK Unified Ideographs Extension B
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_chinese() {
        assert!(contains_chinese("阿凡达"));
        assert!(contains_chinese("Avatar 阿凡达"));
        assert!(!contains_chinese("Avatar"));
        assert!(!contains_chinese("The Matrix"));
    }

    #[test]
    fn test_titles_equivalent() {
        assert!(titles_equivalent("Avatar", "avatar"));
        assert!(titles_equivalent("The Matrix", "the matrix"));
        assert!(!titles_equivalent("Avatar", "Titanic"));
    }
}
