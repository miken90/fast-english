use super::client::AIClient;
use super::prompt::PromptBuilder;
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: String, base_url: String, model: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { api_key, base_url, model, http })
    }
}

#[async_trait]
impl AIClient for AnthropicClient {
    async fn process(&self, text: &str, instruction: &str) -> Result<String> {
        let prompt = PromptBuilder::new();
        let body = serde_json::json!({
            "model": &self.model,
            "max_tokens": prompt.max_tokens(text),
            "system": prompt.system_prompt(),
            "messages": [
                { "role": "user", "content": prompt.user_prompt(text, instruction) }
            ],
        });

        let resp = self.http
            .post(format!("{}/messages", &self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let preview = super::truncate_safe(&body_text, 200);
            anyhow::bail!("HTTP {}: {}", status.as_u16(), preview);
        }

        let data: AnthropicResponse = resp.json().await?;
        let content = data.content.first()
            .map(|b| b.text.as_str())
            .unwrap_or("");

        Ok(PromptBuilder::clean_output(content))
    }

    async fn test_connection(&self) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "max_tokens": 20,
            "messages": [{ "role": "user", "content": "Say hello in 5 words." }],
        });

        let resp = self.http
            .post(format!("{}/messages", &self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status.as_u16(), body_text);
        }

        let data: AnthropicResponse = resp.json().await?;
        Ok(data.content.first()
            .map(|b| b.text.clone())
            .unwrap_or_else(|| "Connected".to_string()))
    }
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}
