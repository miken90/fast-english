use crate::settings::AppSettings;
use crate::ui::theme;
use iced::widget::{button, checkbox, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use super::popup::Message;

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
    #[allow(dead_code)] // inner value reserved for displaying model response
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
    /// Returns an error message if any save operation fails.
    pub fn apply_to_settings(&self, settings: &mut AppSettings) -> Result<(), String> {
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

/// Render the settings window content.
pub fn settings_content<'a>(form: &'a SettingsForm) -> Element<'a, Message> {
    let title = text("Settings").size(18);
    let close_btn = button(text("X").size(12))
        .on_press(Message::SettingsClosed)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: theme::TEXT_MUTED,
            ..Default::default()
        });

    let header = row![title, Space::with_width(Length::Fill), close_btn].align_y(Alignment::Center);

    // AI Connection section
    let base_url_row = labeled_input("Base URL", &form.base_url, Message::SettingsBaseUrlChanged);
    let model_row = labeled_input("Model", &form.model, Message::SettingsModelChanged);
    let api_key_row =
        labeled_secure_input("API Key", &form.api_key, Message::SettingsApiKeyChanged);

    // Connection test
    let is_testing = matches!(form.test_state, ConnectionTestState::Testing);
    let test_label = if is_testing {
        "Testing..."
    } else {
        "Test Connection"
    };
    let test_btn: Element<'_, Message> = if is_testing {
        button(text(test_label).size(13).center())
            .padding([6, 16])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.08))),
                text_color: theme::TEXT_MUTED,
                border: Border {
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
        ConnectionTestState::Success(_) => text("+ Connected")
            .size(12)
            .style(|_theme: &Theme| text::Style {
                color: Some(theme::SUCCESS),
            })
            .into(),
        ConnectionTestState::Failure(msg) => text(format!("x {}", msg))
            .size(12)
            .style(|_theme: &Theme| text::Style {
                color: Some(theme::ERROR),
            })
            .into(),
        _ => Space::new(0, 0).into(),
    };
    let test_row = row![test_btn, test_badge]
        .spacing(10)
        .align_y(Alignment::Center);

    let ai_section = section_card(
        "AI CONNECTION",
        column![base_url_row, model_row, api_key_row, test_row].spacing(10),
    );

    // Preferences section
    let auto_popup_toggle = checkbox("Auto-show popup on text selection", form.auto_popup)
        .on_toggle(Message::SettingsAutoPopupToggled)
        .text_size(13);
    let auto_start_toggle = checkbox("Start Fast English when Windows starts", form.auto_start)
        .on_toggle(Message::SettingsAutoStartToggled)
        .text_size(13);
    let hotkey_info = text("Global Hotkey: Ctrl + Alt + T")
        .size(12)
        .style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        });

    let prefs_section = section_card(
        "PREFERENCES",
        column![auto_popup_toggle, auto_start_toggle, hotkey_info].spacing(10),
    );

    // Save button + feedback
    let save_btn = action_button("Save Settings", Message::SettingsSave);
    let feedback: Element<'_, Message> = match &form.save_feedback {
        Some(msg) => {
            let is_ok = msg.contains("OK");
            text(msg.as_str())
                .size(12)
                .style(move |_theme: &Theme| text::Style {
                    color: Some(if is_ok { theme::SUCCESS } else { theme::ERROR }),
                })
                .into()
        }
        None => Space::new(0, 0).into(),
    };
    let actions = row![save_btn, feedback]
        .spacing(10)
        .align_y(Alignment::Center);

    // Version footer
    let version = text("Fast English v0.1.0")
        .size(11)
        .style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        });

    let content = column![
        header,
        separator(),
        ai_section,
        prefs_section,
        separator(),
        actions,
        version,
    ]
    .spacing(12)
    .width(Length::Fill);

    container(content).padding(16).width(Length::Fill).into()
}

// -- Helpers --

fn section_label(label: &'static str) -> Element<'static, Message> {
    text(label)
        .size(11)
        .style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        })
        .into()
}

fn section_card<'a>(
    label: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let inner = column![section_label(label), content.into()].spacing(8);
    container(inner)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_ELEVATED)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                color: theme::SURFACE_BORDER,
                width: 1.0,
            },
            ..Default::default()
        })
        .padding(14)
        .width(Length::Fill)
        .into()
}

fn labeled_input<'a>(
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(12).style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
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
            color: Some(theme::TEXT_MUTED),
        }),
        text_input("", value)
            .on_input(on_input)
            .size(13)
            .secure(true),
    ]
    .spacing(4)
    .into()
}

pub(crate) fn separator<'a>() -> Element<'a, Message> {
    container(Space::new(Length::Fill, 1))
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_BORDER)),
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
                background: Some(Background::Color(Color::from_rgba(
                    theme::PRIMARY.r,
                    theme::PRIMARY.g,
                    theme::PRIMARY.b,
                    alpha,
                ))),
                text_color: theme::PRIMARY_LIGHT,
                border: Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
