pub mod client;
pub mod openai;
pub mod prompt;
pub mod sse;

/// Create an AI client from settings.
pub fn create_ai_client(
    api_key: String,
    base_url: &str,
    model: &str,
    http_client: reqwest::Client,
) -> Result<Box<dyn client::AIClient>, String> {
    let client = openai::OpenAIClient::new(
        api_key,
        base_url.to_string(),
        model.to_string(),
        http_client,
    )
    .map_err(|e| format!("Failed to create AI client: {}", e))?;
    Ok(Box::new(client))
}

/// Truncate a string at a character boundary (safe for multi-byte UTF-8).
pub(crate) fn truncate_safe(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
