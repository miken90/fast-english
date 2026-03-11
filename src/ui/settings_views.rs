use crate::settings::{AIProvider, AppSettings};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Space};
use iced::{Alignment, Color, Element, Length, Theme};

use super::popup::Message;

/// Editable settings form state (separate from persisted AppSettings).
#[derive(Debug, Clone)]
pub struct SettingsForm {
    pub provider: AIProvider,
    pub openai_base_url: String,
    pub openai_model: String,
    pub openai_api_key: String,
    pub anthropic_base_url: String,
    pub anthropic_model: String,
    pub anthropic_api_key: String,
    pub auto_popup: bool,
    pub auto_start: bool,
    pub save_feedback: Option<String>,
    pub test_state: ConnectionTestState,
}

#[derive(Debug, Clone)]
pub enum ConnectionTestState {
    Idle,
    Testing,
    #[allow(dead_code)] // inner value reserved for displaying model response
    Success(String),
    Failure(String),
}

impl SettingsForm {
    /// Initialize form from current settings + stored keys.
    pub fn from_settings(settings: &AppSettings) -> Self {
        let openai_key = crate::persistence::KeyStore::load("OpenAI")
            .ok()
            .flatten()
            .unwrap_or_default();
        let anthropic_key = crate::persistence::KeyStore::load("Anthropic")
            .ok()
            .flatten()
            .unwrap_or_default();

        Self {
            provider: settings.selected_provider,
            openai_base_url: settings.openai.base_url.clone(),
            openai_model: settings.openai.model.clone(),
            openai_api_key: openai_key,
            anthropic_base_url: settings.anthropic.base_url.clone(),
            anthropic_model: settings.anthropic.model.clone(),
            anthropic_api_key: anthropic_key,
            auto_popup: settings.auto_popup_enabled,
            auto_start: settings.auto_start,
            save_feedback: None,
            test_state: ConnectionTestState::Idle,
        }
    }

    /// Apply form values back to AppSettings and persist keys.
    /// Returns an error message if any save operation fails.
    pub fn apply_to_settings(&self, settings: &mut AppSettings) -> Result<(), String> {
        settings.selected_provider = self.provider;
        settings.openai.base_url = self.openai_base_url.clone();
        settings.openai.model = self.openai_model.clone();
        settings.anthropic.base_url = self.anthropic_base_url.clone();
        settings.anthropic.model = self.anthropic_model.clone();
        settings.auto_popup_enabled = self.auto_popup;
        settings.auto_start = self.auto_start;

        crate::persistence::KeyStore::save("OpenAI", &self.openai_api_key)
            .map_err(|e| format!("Failed to save OpenAI key: {}", e))?;
        crate::persistence::KeyStore::save("Anthropic", &self.anthropic_api_key)
            .map_err(|e| format!("Failed to save Anthropic key: {}", e))?;
        crate::persistence::set_auto_start(self.auto_start)
            .map_err(|e| format!("Failed to set auto-start: {}", e))?;
        settings
            .save()
            .map_err(|e| format!("Failed to save settings: {}", e))?;
        Ok(())
    }

    /// Update the provider-specific base URL field.
    pub fn set_base_url(&mut self, url: String) {
        match self.provider {
            AIProvider::OpenAI => self.openai_base_url = url,
            AIProvider::Anthropic => self.anthropic_base_url = url,
        }
    }

    /// Update the provider-specific model field.
    pub fn set_model(&mut self, model: String) {
        match self.provider {
            AIProvider::OpenAI => self.openai_model = model,
            AIProvider::Anthropic => self.anthropic_model = model,
        }
    }

    /// Update the provider-specific API key field.
    pub fn set_api_key(&mut self, key: String) {
        match self.provider {
            AIProvider::OpenAI => self.openai_api_key = key,
            AIProvider::Anthropic => self.anthropic_api_key = key,
        }
    }
}

const PROVIDERS: &[AIProvider] = &[AIProvider::OpenAI, AIProvider::Anthropic];

/// Render the settings window content.
pub fn settings_content<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let title = text("Settings").size(18);
    let close_btn = button(text("✕").size(12))
        .on_press(Message::SettingsClosed)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..Default::default()
        });

    let header = row![title, Space::with_width(Length::Fill), close_btn].align_y(Alignment::Center);

    // Provider picker
    let provider_row = row![
        text("AI Provider").size(13),
        Space::with_width(Length::Fill),
        pick_list(
            PROVIDERS,
            Some(form.provider),
            Message::SettingsProviderChanged
        )
        .text_size(13),
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    // Active provider fields
    let (base_url, model, api_key, label) = match form.provider {
        AIProvider::OpenAI => (
            &form.openai_base_url,
            &form.openai_model,
            &form.openai_api_key,
            "OpenAI",
        ),
        AIProvider::Anthropic => (
            &form.anthropic_base_url,
            &form.anthropic_model,
            &form.anthropic_api_key,
            "Anthropic",
        ),
    };

    let section_header =
        text(format!("{} Configuration", label))
            .size(14)
            .style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
            });

    let base_url_row = labeled_input("Base URL", base_url, Message::SettingsBaseUrlChanged);
    let model_row = labeled_input("Model", model, Message::SettingsModelChanged);
    let api_key_row = labeled_secure_input("API Key", api_key, Message::SettingsApiKeyChanged);

    // Connection test
    let is_testing = matches!(form.test_state, ConnectionTestState::Testing);
    let test_label = if is_testing {
        "Testing…"
    } else {
        "Test Connection"
    };
    let test_btn: Element<'_, Message> = if is_testing {
        button(text(test_label).size(13).center())
            .padding([6, 16])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.5, 0.5, 0.5, 0.08,
                ))),
                text_color: Color::from_rgb(0.5, 0.5, 0.5),
                border: iced::Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        action_button(test_label, Message::TestConnection)
    };
    let test_badge: Element<'_, Message> = match &form.test_state {
        ConnectionTestState::Success(_) => text("✓ Connected")
            .size(12)
            .style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb(0.2, 0.8, 0.2)),
            })
            .into(),
        ConnectionTestState::Failure(msg) => text(format!("✗ {}", msg))
            .size(12)
            .style(|_theme: &Theme| text::Style {
                color: Some(Color::from_rgb(0.8, 0.3, 0.3)),
            })
            .into(),
        _ => Space::new(0, 0).into(),
    };
    let test_row = row![test_btn, test_badge]
        .spacing(10)
        .align_y(Alignment::Center);

    // Auto-popup toggle
    let auto_popup_toggle = checkbox("Auto-show popup on text selection", form.auto_popup)
        .on_toggle(Message::SettingsAutoPopupToggled)
        .text_size(13);

    // Auto-start toggle
    let auto_start_toggle = checkbox("Start Fast English when Windows starts", form.auto_start)
        .on_toggle(Message::SettingsAutoStartToggled)
        .text_size(13);

    // Hotkey info
    let hotkey_info = text("Global Hotkey: Ctrl + Alt + T")
        .size(12)
        .style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
        });

    // Save button + feedback
    let save_btn = action_button("Save Settings", Message::SettingsSave);
    let feedback: Element<'_, Message> = match &form.save_feedback {
        Some(msg) => {
            let is_ok = msg.contains('✓');
            text(msg.as_str())
                .size(12)
                .style(move |_theme: &Theme| text::Style {
                    color: Some(if is_ok {
                        Color::from_rgb(0.2, 0.8, 0.2)
                    } else {
                        Color::from_rgb(0.8, 0.3, 0.3)
                    }),
                })
                .into()
        }
        None => Space::new(0, 0).into(),
    };

    let actions = row![save_btn, feedback]
        .spacing(10)
        .align_y(Alignment::Center);

    let content = column![
        header,
        separator(),
        provider_row,
        section_header,
        base_url_row,
        model_row,
        api_key_row,
        test_row,
        separator(),
        auto_popup_toggle,
        auto_start_toggle,
        hotkey_info,
        separator(),
        actions,
    ]
    .spacing(10)
    .width(Length::Fill);

    container(content).padding(16).width(Length::Fill).into()
}

// -- Helpers --

fn labeled_input<'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(12).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
        }),
        text_input("", value).on_input(on_input).size(13),
    ]
    .spacing(4)
    .into()
}

fn labeled_secure_input<'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(12).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
        }),
        text_input("", value)
            .on_input(on_input)
            .size(13)
            .secure(true),
    ]
    .spacing(4)
    .into()
}

fn separator<'a>() -> Element<'a, Message> {
    container(Space::new(Length::Fill, 1))
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.5, 0.5, 0.5, 0.2,
            ))),
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
}

fn action_button(label: &str, msg: Message) -> Element<'_, Message> {
    button(text(label).size(13).center())
        .on_press(msg)
        .padding([6, 16])
        .style(|_theme: &Theme, status| {
            let alpha = match status {
                button::Status::Hovered => 0.2,
                _ => 0.12,
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.3, 0.5, 0.9, alpha,
                ))),
                text_color: Color::from_rgb(0.4, 0.6, 1.0),
                border: iced::Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
