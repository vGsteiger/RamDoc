/// UTF-8 boundary-safe truncation utilities.
///
/// This module provides functions to safely truncate UTF-8 strings at byte boundaries
/// without breaking multi-byte character sequences.

/// Truncates a string to at most `max_bytes` bytes, ensuring the result ends on a valid
/// UTF-8 character boundary.
///
/// If the input string is already within the limit, it is returned as-is. Otherwise,
/// the function finds the largest valid character boundary at or before `max_bytes`.
///
/// # Arguments
///
/// * `s` - The input string slice to truncate
/// * `max_bytes` - The maximum number of bytes for the result
///
/// # Returns
///
/// A string slice that is at most `max_bytes` bytes long and ends on a valid UTF-8
/// character boundary.
///
/// # Examples
///
/// ```
/// # use dokassist::llm::utf8::truncate_to_boundary;
/// let s = "Hello, 世界!";
/// let truncated = truncate_to_boundary(s, 10);
/// assert_eq!(truncated, "Hello, 世");
/// ```
pub fn truncate_to_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut boundary = max_bytes;
    while !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

/// Finds the first valid UTF-8 character boundary at or after `start_pos` within the string.
///
/// This is useful when you want to extract a tail portion of a string starting from
/// approximately a given position, but need to ensure the slice begins on a valid boundary.
///
/// # Arguments
///
/// * `s` - The input string slice
/// * `start_pos` - The desired starting position (may not be a valid boundary)
///
/// # Returns
///
/// The index of the first valid character boundary at or after `start_pos`. If `start_pos`
/// is beyond the string length, returns the string length.
///
/// # Examples
///
/// ```
/// # use dokassist::llm::utf8::find_boundary_forward;
/// let s = "Hello, 世界!";
/// let pos = find_boundary_forward(s, 8);
/// assert!(s.is_char_boundary(pos));
/// ```
pub fn find_boundary_forward(s: &str, start_pos: usize) -> usize {
    (start_pos..=s.len())
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(s.len())
}

/// Finds the last valid UTF-8 character boundary at or before `end_pos` within the string.
///
/// This is useful when you want to truncate a string to approximately a given length
/// using a reverse search within a limited range.
///
/// # Arguments
///
/// * `s` - The input string slice
/// * `end_pos` - The desired end position (may not be a valid boundary)
///
/// # Returns
///
/// The index of the last valid character boundary at or before `end_pos`. Returns 0 if
/// no valid boundary is found in the range.
///
/// # Examples
///
/// ```
/// # use dokassist::llm::utf8::find_boundary_backward;
/// let s = "Hello, 世界!";
/// let pos = find_boundary_backward(s, 10);
/// assert!(s.is_char_boundary(pos));
/// ```
pub fn find_boundary_backward(s: &str, end_pos: usize) -> usize {
    let end_pos = end_pos.min(s.len());
    (0..=end_pos)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_boundary_ascii() {
        let s = "Hello, World!";
        assert_eq!(truncate_to_boundary(s, 5), "Hello");
        assert_eq!(truncate_to_boundary(s, 13), "Hello, World!");
        assert_eq!(truncate_to_boundary(s, 100), "Hello, World!");
    }

    #[test]
    fn test_truncate_to_boundary_multibyte() {
        // "世" is 3 bytes (E4 B8 96), "界" is 3 bytes (E7 95 8C)
        let s = "Hello, 世界!";

        // Truncate to 10 bytes: "Hello, 世" (10 bytes exactly)
        assert_eq!(truncate_to_boundary(s, 10), "Hello, 世");

        // Truncate to 9 bytes: should stop at "Hello, " (7 bytes, before the multi-byte char)
        assert_eq!(truncate_to_boundary(s, 9), "Hello, ");

        // Truncate to 8 bytes: should stop at "Hello, " (7 bytes)
        assert_eq!(truncate_to_boundary(s, 8), "Hello, ");

        // Truncate to 13 bytes: "Hello, 世界" (13 bytes exactly)
        assert_eq!(truncate_to_boundary(s, 13), "Hello, 世界");

        // Truncate to 12 bytes: should stop at "Hello, 世" (10 bytes)
        assert_eq!(truncate_to_boundary(s, 12), "Hello, 世");
    }

    #[test]
    fn test_truncate_to_boundary_emoji() {
        // "😀" is 4 bytes (F0 9F 98 80)
        let s = "Hi 😀!";

        // Truncate to 6 bytes: "Hi 😀" (6 bytes exactly)
        assert_eq!(truncate_to_boundary(s, 6), "Hi 😀");

        // Truncate to 5 bytes: should stop at "Hi " (3 bytes, before the emoji)
        assert_eq!(truncate_to_boundary(s, 5), "Hi ");

        // Truncate to 3 bytes: "Hi " (3 bytes exactly)
        assert_eq!(truncate_to_boundary(s, 3), "Hi ");
    }

    #[test]
    fn test_truncate_to_boundary_zero() {
        let s = "Hello";
        assert_eq!(truncate_to_boundary(s, 0), "");
    }

    #[test]
    fn test_find_boundary_forward_ascii() {
        let s = "Hello, World!";
        assert_eq!(find_boundary_forward(s, 0), 0);
        assert_eq!(find_boundary_forward(s, 5), 5);
        assert_eq!(find_boundary_forward(s, 13), 13);
        assert_eq!(find_boundary_forward(s, 100), 13);
    }

    #[test]
    fn test_find_boundary_forward_multibyte() {
        // "世" is 3 bytes starting at index 7
        let s = "Hello, 世界!";

        // Start at 7 (beginning of "世"): should return 7
        assert_eq!(find_boundary_forward(s, 7), 7);

        // Start at 8 (middle of "世"): should find next boundary at 10
        assert_eq!(find_boundary_forward(s, 8), 10);

        // Start at 9 (middle of "世"): should find next boundary at 10
        assert_eq!(find_boundary_forward(s, 9), 10);

        // Start at 10 (beginning of "界"): should return 10
        assert_eq!(find_boundary_forward(s, 10), 10);
    }

    #[test]
    fn test_find_boundary_backward_ascii() {
        let s = "Hello, World!";
        assert_eq!(find_boundary_backward(s, 0), 0);
        assert_eq!(find_boundary_backward(s, 5), 5);
        assert_eq!(find_boundary_backward(s, 13), 13);
        assert_eq!(find_boundary_backward(s, 100), 13);
    }

    #[test]
    fn test_find_boundary_backward_multibyte() {
        // "世" is 3 bytes starting at index 7, "界" is 3 bytes starting at index 10
        let s = "Hello, 世界!";

        // End at 10 (beginning of "界"): should return 10
        assert_eq!(find_boundary_backward(s, 10), 10);

        // End at 9 (middle of "世"): should find previous boundary at 7
        assert_eq!(find_boundary_backward(s, 9), 7);

        // End at 8 (middle of "世"): should find previous boundary at 7
        assert_eq!(find_boundary_backward(s, 8), 7);

        // End at 7 (beginning of "世"): should return 7
        assert_eq!(find_boundary_backward(s, 7), 7);
    }

    #[test]
    fn test_find_boundary_backward_emoji() {
        // "😀" is 4 bytes (F0 9F 98 80) starting at index 3
        let s = "Hi 😀!";

        // End at 7 (after emoji, at "!"): should return 7
        assert_eq!(find_boundary_backward(s, 7), 7);

        // End at 5 (middle of emoji): should find previous boundary at 3
        assert_eq!(find_boundary_backward(s, 5), 3);

        // End at 4 (middle of emoji): should find previous boundary at 3
        assert_eq!(find_boundary_backward(s, 4), 3);

        // End at 3 (beginning of emoji): should return 3
        assert_eq!(find_boundary_backward(s, 3), 3);
    }

    #[test]
    fn test_boundary_helpers_empty_string() {
        let s = "";
        assert_eq!(truncate_to_boundary(s, 0), "");
        assert_eq!(truncate_to_boundary(s, 10), "");
        assert_eq!(find_boundary_forward(s, 0), 0);
        assert_eq!(find_boundary_backward(s, 0), 0);
    }

    #[test]
    fn test_truncate_preserves_valid_utf8() {
        let test_cases = vec![
            "Hello, World!",
            "こんにちは",
            "Привет мир",
            "مرحبا بالعالم",
            "🎉🎊🎈",
            "Mixed: Hello 世界 😀!",
        ];

        for s in test_cases {
            for max_bytes in 0..s.len() + 5 {
                let truncated = truncate_to_boundary(s, max_bytes);
                // The truncated result should always be valid UTF-8
                assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
                // And should not exceed max_bytes
                assert!(truncated.len() <= max_bytes);
            }
        }
    }
}
