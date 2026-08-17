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

    /// Stream the completion, handing each piece of text to `on_chunk` as it
    /// arrives, and return the full raw text.
    ///
    /// The default implementation runs the non-streaming path and emits the
    /// answer in one piece, so a provider that cannot stream still works
    /// unchanged through the streaming caller.
    async fn process_streaming(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
        on_chunk: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> Result<String> {
        let output = self
            .process(text, instruction, system_override, max_tokens_override)
            .await?;
        if !output.is_empty() {
            on_chunk(&output);
        }
        Ok(output)
    }

    async fn test_connection(&self) -> Result<String>;
}
