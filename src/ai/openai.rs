use super::client::AIClient;
use super::prompt::PromptBuilder;
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
    pub fn new(api_key: String, base_url: String, model: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { api_key, base_url, model, http })
    }
}

#[async_trait]
impl AIClient for OpenAIClient {
    async fn process(&self, text: &str, instruction: &str) -> Result<String> {
        let prompt = PromptBuilder::new();
        let body = serde_json::json!({
            "model": &self.model,
            "messages": [
                { "role": "system", "content": prompt.system_prompt() },
                { "role": "user", "content": prompt.user_prompt(text, instruction) }
            ],
            "temperature": 0.3,
            "max_tokens": prompt.max_tokens(text),
        });

        let resp = self.http
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
        let content = data.choices.first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        Ok(PromptBuilder::clean_output(content))
    }

    async fn test_connection(&self) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "messages": [{ "role": "user", "content": "Say hello in 5 words." }],
            "max_tokens": 20,
        });

        let resp = self.http
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
        Ok(data.choices.first()
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
