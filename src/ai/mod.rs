pub mod client;
pub mod openai;
pub mod anthropic;
pub mod prompt;

/// Truncate a string at a character boundary (safe for multi-byte UTF-8).
pub(crate) fn truncate_safe(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
