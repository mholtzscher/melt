//! Text formatting helpers

/// Truncate text to at most `max_chars` Unicode scalar values, appending `...` when truncated.
///
/// UI strings can contain non-ASCII author names or messages, so truncating by byte index can
/// panic when the cut falls inside a multi-byte character.
pub fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let keep_chars = max_chars - 3;
    let mut truncated: String = text.chars().take(keep_chars).collect();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_non_ascii_text_on_char_boundary() {
        assert_eq!(
            truncate_with_ellipsis("Huỳnh Thiện Lộc", 14),
            "Huỳnh Thiện..."
        );
    }

    #[test]
    fn leaves_short_text_unchanged() {
        assert_eq!(truncate_with_ellipsis("short", 10), "short");
    }

    #[test]
    fn leaves_exact_boundary_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn truncates_ascii_text_to_max_chars() {
        assert_eq!(truncate_with_ellipsis("1234567890123456", 15), "123456789012...");
    }

    #[test]
    fn respects_small_max_chars() {
        assert_eq!(truncate_with_ellipsis("long", 0), "");
        assert_eq!(truncate_with_ellipsis("long", 1), ".");
        assert_eq!(truncate_with_ellipsis("long", 2), "..");
        assert_eq!(truncate_with_ellipsis("long", 3), "...");
    }
}
