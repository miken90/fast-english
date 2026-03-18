use anyhow::Result;
use async_trait::async_trait;

/// AI client trait defining the completion interface.
#[async_trait]
pub trait AIClient: Send + Sync {
    async fn process(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
    ) -> Result<String>;
    async fn test_connection(&self) -> Result<String>;
}
