use crate::app::{AppEvent, PopupState, WritingToolKind};
use crate::selection::SelectionContext;
use crate::settings::{AIProvider, AppSettings};
use crate::ui::{popup_ai, popup_views, popup_window, settings_views};
use anyhow::Result;
use iced::window;
use iced::{Element, Subscription, Task, Theme};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Messages handled by the iced daemon.
#[derive(Debug, Clone)]
pub enum Message {
    // Window lifecycle
    PopupOpened(window::Id),
    PopupClosed(window::Id),
    SettingsWindowOpened(window::Id),

    // Forwarded from AppEvent channel
    TextSelected(SelectionContext),
    SelectionCleared,
    HotkeyPressed,
    TrayClicked,
    AiResponse(std::result::Result<String, String>),
    ManualCaptureResult(SelectionContext),

    // UI actions (from popup views)
    ToolSelected(WritingToolKind),
    ToneSelected(crate::app::ToneOption),
    BackToOptions,
    CopyResult,
    ReplaceResult,
    ReplaceDone,
    Retry,
    Cancel,
    Dismiss,

    // Settings actions
    SettingsRequested,
    SettingsClosed,
    SettingsProviderChanged(AIProvider),
    SettingsBaseUrlChanged(String),
    SettingsModelChanged(String),
    SettingsApiKeyChanged(String),
    SettingsAutoPopupToggled(bool),
    SettingsAutoStartToggled(bool),
    SettingsSave,
    TestConnection,
    TestConnectionResult(std::result::Result<String, String>),

    // System
    QuitRequested,
    Tick,
}

/// Daemon application state.
pub(crate) struct AppState {
    popup_state: PopupState,
    popup_window_id: Option<window::Id>,
    settings_window_id: Option<window::Id>,
    settings_form: Option<settings_views::SettingsForm>,
    current_selection: Option<SelectionContext>,
    copy_feedback: bool,
    settings: AppSettings,
    event_rx: Arc<Mutex<Option<mpsc::Receiver<AppEvent>>>>,
    last_tool: Option<WritingToolKind>,
}

/// Run the iced daemon. Blocks the calling thread.
pub fn run(
    event_rx: mpsc::Receiver<AppEvent>,
    _event_tx: mpsc::Sender<AppEvent>,
    settings: AppSettings,
    _runtime: tokio::runtime::Runtime,
) -> Result<()> {
    let rx = Arc::new(Mutex::new(Some(event_rx)));
    let rx_clone = Arc::clone(&rx);
    let settings_clone = settings.clone();

    iced::daemon("Fast English", update, view)
        .subscription(subscription)
        .theme(|_state, _window| Theme::Dark)
        .style(|_state, _theme| iced::daemon::Appearance {
            background_color: iced::Color::TRANSPARENT,
            text_color: iced::Color::WHITE,
        })
        .run_with(move || {
            let state = AppState {
                popup_state: PopupState::Hidden,
                popup_window_id: None,
                settings_window_id: None,
                settings_form: None,
                current_selection: None,
                copy_feedback: false,
                settings: settings_clone,
                event_rx: rx_clone,
                last_tool: None,
            };
            (state, Task::none())
        })
        .map_err(|e| anyhow::anyhow!("iced daemon error: {}", e))
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::TextSelected(ctx) => {
            match &state.popup_state {
                PopupState::Processing { .. }
                | PopupState::Result { .. }
                | PopupState::Error { .. }
                | PopupState::ToneSelection(_) => return Task::none(),
                _ => {}
            }
            if let PopupState::Options(current) = &state.popup_state {
                if current.text == ctx.text {
                    return Task::none();
                }
            }
            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Options(ctx.clone());
            if state.settings.auto_popup_enabled {
                return popup_window::show_popup(&mut state.popup_window_id, &ctx);
            }
        }

        Message::SelectionCleared => {
            if matches!(state.popup_state, PopupState::Options(_)) {
                return popup_window::hide_popup(
                    &mut state.popup_state,
                    &mut state.popup_window_id,
                );
            }
        }

        Message::HotkeyPressed | Message::TrayClicked => {
            // Always do a fresh capture — never rely on cached selection,
            // which may be stale data from a different app (e.g., browser).
            return Task::perform(
                async { crate::selection::monitor::try_manual_capture().await },
                |result| match result {
                    Some(ctx) => Message::ManualCaptureResult(ctx),
                    None => Message::Tick,
                },
            );
        }

        Message::ToolSelected(kind) => {
            if let PopupState::Options(ctx) = &state.popup_state {
                if matches!(kind, WritingToolKind::ChangeTone(_)) {
                    state.popup_state = PopupState::ToneSelection(ctx.clone());
                    return Task::none();
                }
                let text = ctx.text.clone();
                let ctx_clone = ctx.clone();
                state.last_tool = Some(kind.clone());
                state.popup_state = PopupState::Processing {
                    context: ctx_clone,
                    tool: kind.clone(),
                };
                return popup_ai::start_request(&state.settings, &text, &kind);
            }
        }

        Message::BackToOptions => {
            if let PopupState::ToneSelection(ctx) = &state.popup_state {
                state.popup_state = PopupState::Options(ctx.clone());
            }
        }

        Message::ToneSelected(tone) => {
            if let PopupState::ToneSelection(ctx) = &state.popup_state {
                let kind = WritingToolKind::ChangeTone(tone);
                let text = ctx.text.clone();
                let ctx_clone = ctx.clone();
                state.last_tool = Some(kind.clone());
                state.popup_state = PopupState::Processing {
                    context: ctx_clone,
                    tool: kind.clone(),
                };
                return popup_ai::start_request(&state.settings, &text, &kind);
            }
        }

        Message::ManualCaptureResult(ctx) => {
            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Options(ctx.clone());
            return popup_window::show_popup(&mut state.popup_window_id, &ctx);
        }

        Message::AiResponse(result) => {
            let ctx = popup_ai::extract_context(&state.popup_state);
            let tool = popup_ai::extract_tool(&state.popup_state);
            match result {
                Ok(output) => {
                    if let (Some(ctx), Some(tool)) = (ctx, tool) {
                        state.popup_state = PopupState::Result {
                            context: ctx,
                            tool,
                            output,
                        };
                    }
                }
                Err(msg) => {
                    if let Some(ctx) = ctx {
                        state.popup_state = PopupState::Error {
                            context: ctx,
                            message: msg,
                        };
                    }
                }
            }
        }

        Message::CopyResult => {
            if let PopupState::Result { output, .. } = &state.popup_state {
                let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(output.clone()));
                state.copy_feedback = true;
            }
        }

        Message::ReplaceResult => {
            if let PopupState::Result { output, .. } = &state.popup_state {
                let text = output.clone();
                let close_task = popup_window::hide_popup(
                    &mut state.popup_state,
                    &mut state.popup_window_id,
                );
                let replace_task = Task::perform(
                    async move {
                        if let Err(e) =
                            crate::editing::text_service::replace_selection(&text).await
                        {
                            tracing::error!("Replace failed: {}", e);
                        }
                    },
                    |_| Message::ReplaceDone,
                );
                return Task::batch([close_task, replace_task]);
            }
        }

        Message::ReplaceDone => {
            debug!("Text replacement completed");
        }

        Message::Retry => {
            if let PopupState::Error { context, .. } = &state.popup_state {
                if let Some(tool) = state.last_tool.clone() {
                    let ctx = context.clone();
                    state.popup_state = PopupState::Processing {
                        context: ctx.clone(),
                        tool: tool.clone(),
                    };
                    return popup_ai::start_request(&state.settings, &ctx.text, &tool);
                }
            }
        }

        Message::Cancel | Message::Dismiss => {
            return popup_window::hide_popup(
                &mut state.popup_state,
                &mut state.popup_window_id,
            );
        }

        // -- Settings --
        Message::SettingsRequested => {
            return popup_window::open_settings(
                &state.settings_window_id,
                &mut state.settings_form,
                &state.settings,
            );
        }
        Message::SettingsClosed => {
            return popup_window::close_settings(
                &mut state.settings_form,
                &mut state.settings_window_id,
            );
        }
        Message::SettingsProviderChanged(p) => {
            if let Some(form) = &mut state.settings_form {
                form.provider = p;
            }
        }
        Message::SettingsBaseUrlChanged(url) => {
            if let Some(form) = &mut state.settings_form {
                form.set_base_url(url);
            }
        }
        Message::SettingsModelChanged(model) => {
            if let Some(form) = &mut state.settings_form {
                form.set_model(model);
            }
        }
        Message::SettingsApiKeyChanged(key) => {
            if let Some(form) = &mut state.settings_form {
                form.set_api_key(key);
            }
        }
        Message::SettingsAutoPopupToggled(enabled) => {
            if let Some(form) = &mut state.settings_form {
                form.auto_popup = enabled;
            }
        }
        Message::SettingsAutoStartToggled(enabled) => {
            if let Some(form) = &mut state.settings_form {
                form.auto_start = enabled;
            }
        }
        Message::SettingsSave => {
            if let Some(form) = &mut state.settings_form {
                match form.apply_to_settings(&mut state.settings) {
                    Ok(()) => {
                        form.save_feedback = Some("Saved ✓".to_string());
                        info!("Settings saved");
                    }
                    Err(msg) => {
                        form.save_feedback = Some(format!("✗ {}", msg));
                        tracing::error!("Settings save failed: {}", msg);
                    }
                }
            }
        }
        Message::TestConnection => {
            if let Some(form) = &mut state.settings_form {
                form.test_state = settings_views::ConnectionTestState::Testing;
                let settings = state.settings.clone();
                return Task::perform(
                    async move {
                        let config = settings.active_config();
                        let api_key = crate::persistence::KeyStore::load(
                            &settings.selected_provider.to_string(),
                        )
                        .ok()
                        .flatten()
                        .unwrap_or_default();

                        if api_key.is_empty() {
                            return Err("No API key configured".to_string());
                        }

                        let client = popup_ai::create_ai_client(
                            &settings.selected_provider,
                            api_key,
                            config,
                        )?;

                        client.test_connection().await.map_err(|e| e.to_string())
                    },
                    |result| Message::TestConnectionResult(result),
                );
            }
        }
        Message::TestConnectionResult(result) => {
            if let Some(form) = &mut state.settings_form {
                match result {
                    Ok(msg) => {
                        form.test_state = settings_views::ConnectionTestState::Success(msg);
                    }
                    Err(msg) => {
                        form.test_state = settings_views::ConnectionTestState::Failure(msg);
                    }
                }
            }
        }

        // -- Window lifecycle --
        Message::PopupOpened(id) => {
            debug!("Popup window opened: {:?}", id);
            state.popup_window_id = Some(id);
        }
        Message::SettingsWindowOpened(id) => {
            debug!("Settings window opened: {:?}", id);
            state.settings_window_id = Some(id);
        }
        Message::PopupClosed(id) => {
            debug!("Window closed: {:?}", id);
            if state.popup_window_id == Some(id) {
                state.popup_window_id = None;
                state.popup_state = PopupState::Hidden;
            }
            if state.settings_window_id == Some(id) {
                state.settings_window_id = None;
                state.settings_form = None;
            }
        }

        Message::QuitRequested => return iced::exit(),
        Message::Tick => {
            if state.copy_feedback {
                state.copy_feedback = false;
            }
        }
    }

    Task::none()
}

fn view(state: &AppState, window_id: window::Id) -> Element<'_, Message> {
    if state.settings_window_id == Some(window_id) {
        if let Some(form) = &state.settings_form {
            return settings_views::settings_content(form);
        }
    }
    popup_views::popup_content(&state.popup_state, state.copy_feedback)
}

fn subscription(state: &AppState) -> Subscription<Message> {
    let mut subs = vec![];

    let rx = Arc::clone(&state.event_rx);
    subs.push(Subscription::run_with_id(
        "app-events",
        popup_window::channel_stream(rx),
    ));

    subs.push(window::close_events().map(Message::PopupClosed));

    if state.copy_feedback {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(1500)).map(|_| Message::Tick),
        );
    }

    Subscription::batch(subs)
}
