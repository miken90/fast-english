use crate::settings::AppSettings;

/// Editable settings form state (separate from persisted AppSettings).
#[derive(Debug, Clone)]
pub struct SettingsForm {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub auto_popup: bool,
    pub auto_start: bool,
    pub save_feedback: Option<String>,
    pub test_state: ConnectionTestState,
}

#[derive(Debug, Clone)]
pub enum ConnectionTestState {
    Idle,
    Testing,
    #[allow(dead_code)]
    Success(String),
    Failure(String),
}

impl SettingsForm {
    /// Initialize form from current settings + stored keys.
    pub fn from_settings(settings: &AppSettings) -> Self {
        let api_key = crate::persistence::KeyStore::load("api")
            .ok()
            .flatten()
            .unwrap_or_default();

        Self {
            base_url: settings.base_url.clone(),
            model: settings.model.clone(),
            api_key,
            auto_popup: settings.auto_popup_enabled,
            auto_start: settings.auto_start,
            save_feedback: None,
            test_state: ConnectionTestState::Idle,
        }
    }

    /// Apply form values back to AppSettings and persist keys.
    pub fn apply_to_settings(&self, settings: &mut AppSettings) -> Result<(), String> {
        if !self.base_url.trim_start().starts_with("https://") {
            return Err("Base URL must start with https://".to_string());
        }

        settings.base_url = self.base_url.clone();
        settings.model = self.model.clone();
        settings.auto_popup_enabled = self.auto_popup;
        settings.auto_start = self.auto_start;

        crate::persistence::KeyStore::save("api", &self.api_key)
            .map_err(|e| format!("Failed to save API key: {}", e))?;
        crate::persistence::set_auto_start(self.auto_start)
            .map_err(|e| format!("Failed to set auto-start: {}", e))?;
        settings
            .save()
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        Ok(())
    }
}
