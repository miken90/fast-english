use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// AI provider enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
}

impl fmt::Display for AIProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAI => write!(f, "OpenAI"),
            Self::Anthropic => write!(f, "Anthropic"),
        }
    }
}

impl AIProvider {
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::OpenAI => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-4.1-mini",
            Self::Anthropic => "claude-3-5-sonnet-latest",
        }
    }
}

/// Per-provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: AIProvider,
    pub base_url: String,
    pub model: String,
}

impl ProviderConfig {
    pub fn openai_default() -> Self {
        Self {
            provider: AIProvider::OpenAI,
            base_url: AIProvider::OpenAI.default_base_url().to_string(),
            model: AIProvider::OpenAI.default_model().to_string(),
        }
    }

    pub fn anthropic_default() -> Self {
        Self {
            provider: AIProvider::Anthropic,
            base_url: AIProvider::Anthropic.default_base_url().to_string(),
            model: AIProvider::Anthropic.default_model().to_string(),
        }
    }
}

/// Application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub selected_provider: AIProvider,
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub auto_popup_enabled: bool,
    pub auto_start: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            selected_provider: AIProvider::OpenAI,
            openai: ProviderConfig::openai_default(),
            anthropic: ProviderConfig::anthropic_default(),
            auto_popup_enabled: true,
            auto_start: false,
        }
    }
}

impl AppSettings {
    /// Load settings from `%APPDATA%/ToneWriter/settings.json`.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Get the active provider's configuration.
    pub fn active_config(&self) -> &ProviderConfig {
        match self.selected_provider {
            AIProvider::OpenAI => &self.openai,
            AIProvider::Anthropic => &self.anthropic,
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ToneWriter")
            .join("settings.json")
    }
}
