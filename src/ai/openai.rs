use super::client::AIClient;
use super::prompt::PromptBuilder;
use super::sse::{SseDecoder, SseItem};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        http: reqwest::Client,
    ) -> Result<Self> {
        Ok(Self {
            api_key,
            base_url,
            model,
            http,
        })
    }

    /// The chat-completions body. Streaming and non-streaming differ only in
    /// the `stream` flag, so the request itself is built in one place.
    fn completion_body(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
        stream: bool,
    ) -> serde_json::Value {
        let prompt = PromptBuilder::new();
        let system_prompt = system_override.unwrap_or(prompt.system_prompt());
        let max_tokens = max_tokens_override.unwrap_or_else(|| prompt.max_tokens(text));

        serde_json::json!({
            "model": &self.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": prompt.user_prompt(text, instruction) }
            ],
            "temperature": 0.3,
            "max_tokens": max_tokens,
            "stream": stream,
        })
    }
}

#[async_trait]
impl AIClient for OpenAIClient {
    async fn process(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
    ) -> Result<String> {
        let body = self.completion_body(
            text,
            instruction,
            system_override,
            max_tokens_override,
            false,
        );

        let resp = self
            .http
            .post(format!("{}/chat/completions", &self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let preview = super::truncate_safe(&body_text, 200);
            anyhow::bail!("HTTP {}: {}", status.as_u16(), preview);
        }

        let data: OpenAIResponse = resp.json().await?;
        let content = data
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        Ok(content.to_string())
    }

    async fn process_streaming(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
        on_chunk: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> Result<String> {
        let body = self.completion_body(
            text,
            instruction,
            system_override,
            max_tokens_override,
            true,
        );

        let mut resp = self
            .http
            .post(format!("{}/chat/completions", &self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let preview = super::truncate_safe(&body_text, 200);
            anyhow::bail!("HTTP {}: {}", status.as_u16(), preview);
        }

        let mut decoder = SseDecoder::new();
        let mut full = String::new();

        // Reading chunk by chunk keeps this an await point per network read,
        // which is what lets a cancelled request abort the HTTP body promptly.
        while let Some(bytes) = resp.chunk().await? {
            for item in decoder.feed(&bytes) {
                match item {
                    SseItem::Delta(delta) => {
                        on_chunk(&delta);
                        full.push_str(&delta);
                    }
                    SseItem::Error(message) => anyhow::bail!("{}", message),
                    SseItem::Done => return Ok(full),
                }
            }
        }

        for item in decoder.finish() {
            match item {
                SseItem::Delta(delta) => {
                    on_chunk(&delta);
                    full.push_str(&delta);
                }
                SseItem::Error(message) => anyhow::bail!("{}", message),
                SseItem::Done => break,
            }
        }

        Ok(full)
    }

    async fn test_connection(&self) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "messages": [{ "role": "user", "content": "Say hello in 5 words." }],
            "max_tokens": 20,
        });

        let resp = self
            .http
            .post(format!("{}/chat/completions", &self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status.as_u16(), body_text);
        }

        let data: OpenAIResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "Connected".to_string()))
    }
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
}
