use crate::app::{AppEvent, PopupState, TranslateLanguage, WritingToolKind};
use crate::selection::SelectionContext;
use crate::settings::AppSettings;
use crate::ui::{popup_ai, popup_views, popup_window, settings_views};
use anyhow::Result;
use iced::widget::text_editor;
use iced::window;
use iced::{Element, Subscription, Task, Theme};
use std::sync::{Arc, Mutex};
use std::time::Instant;
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
    TranslateFromChanged(TranslateLanguage),
    TranslateToChanged(TranslateLanguage),
    TranslateSwapLangs,
    TranslateConfirm,
    TranslateOriginalAction(text_editor::Action),
    TranslateCopyResult,
    TranslateReplaceResult,
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
    SpinnerTick,
    PollEscKey,
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
    /// Selected "from" language for translate feature.
    pub(crate) translate_from: TranslateLanguage,
    /// Selected "to" language for translate feature.
    pub(crate) translate_to: TranslateLanguage,
    /// Editable original text content for translate result.
    pub(crate) translate_original_content: text_editor::Content,
    /// Read-only translated output text.
    pub(crate) translate_output_text: String,
    /// True while popup is shown onscreen — suppresses auto-monitor events to prevent loop.
    popup_visible: bool,
    /// Normalized text of the last auto-shown selection — prevents re-show for same text.
    last_auto_shown: Option<String>,
    /// Cooldown: ignore auto TextSelected events until this instant passes.
    ignore_auto_until: Option<Instant>,
    /// Spinner animation frame index for processing state.
    spinner_tick: usize,
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
            let (popup_id, popup_task) = popup_window::create_popup();
            let state = AppState {
                popup_state: PopupState::Hidden,
                popup_window_id: Some(popup_id),
                settings_window_id: None,
                settings_form: None,
                current_selection: None,
                copy_feedback: false,
                settings: settings_clone,
                event_rx: rx_clone,
                last_tool: None,
                translate_from: crate::app::TRANSLATE_LANGS[0].clone(),
                translate_to: crate::app::TRANSLATE_LANGS[1].clone(),
                translate_original_content: text_editor::Content::new(),
                translate_output_text: String::new(),
                popup_visible: false,
                last_auto_shown: None,
                ignore_auto_until: None,
                spinner_tick: 0,
            };
            (state, popup_task)
        })
        .map_err(|e| anyhow::anyhow!("iced daemon error: {}", e))
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::TextSelected(ctx) => {
            // Suppress auto-monitor events while popup is visible
            if state.popup_visible {
                return Task::none();
            }
            // Suppress during cooldown after popup close
            if state
                .ignore_auto_until
                .map_or(false, |t| Instant::now() < t)
            {
                return Task::none();
            }
            // Suppress if same text already shown (prevents re-show loop)
            let norm = normalize_text(&ctx.text);
            if state.last_auto_shown.as_ref() == Some(&norm) {
                return Task::none();
            }
            match &state.popup_state {
                PopupState::Processing { .. }
                | PopupState::Result { .. }
                | PopupState::Error { .. }
                | PopupState::ToneSelection(_)
                | PopupState::TranslateSelection(_)
                | PopupState::TranslateResult { .. } => return Task::none(),
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
                state.last_auto_shown = Some(norm);
                state.popup_visible = true;
                if let Some(id) = state.popup_window_id {
                    return popup_window::show_popup(id, &ctx);
                }
            }
        }

        Message::SelectionCleared => {
            // Only trust clear events when popup is NOT visible (popup steals focus → fake clear)
            if state.popup_visible {
                return Task::none();
            }
            // Real clear — reset auto-shown tracker so same text can re-trigger later
            state.last_auto_shown = None;
            if matches!(state.popup_state, PopupState::Options(_)) {
                return popup_window::hide_popup(&mut state.popup_state, state.popup_window_id);
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
                    return popup_window::resize_for_state(
                        state.popup_window_id,
                        &state.popup_state,
                    );
                }
                if matches!(kind, WritingToolKind::Translate { .. }) {
                    state.popup_state = PopupState::TranslateSelection(ctx.clone());
                    return popup_window::resize_for_state(
                        state.popup_window_id,
                        &state.popup_state,
                    );
                }
                let text = ctx.text.clone();
                let ctx_clone = ctx.clone();
                state.last_tool = Some(kind.clone());
                state.popup_state = PopupState::Processing {
                    context: ctx_clone,
                    tool: kind.clone(),
                };
                let resize =
                    popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
                let ai_req = popup_ai::start_request(&state.settings, &text, &kind);
                return Task::batch([resize, ai_req]);
            }
        }

        Message::BackToOptions => {
            if let PopupState::ToneSelection(ctx) | PopupState::TranslateSelection(ctx) =
                &state.popup_state
            {
                state.popup_state = PopupState::Options(ctx.clone());
                return popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
            }
            if let PopupState::Result { context, .. }
            | PopupState::Error { context, .. }
            | PopupState::TranslateResult { context, .. } = &state.popup_state
            {
                state.popup_state = PopupState::Options(context.clone());
                return popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
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
                let resize =
                    popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
                let ai_req = popup_ai::start_request(&state.settings, &text, &kind);
                return Task::batch([resize, ai_req]);
            }
        }

        Message::TranslateFromChanged(lang) => {
            state.translate_from = lang;
        }

        Message::TranslateToChanged(lang) => {
            state.translate_to = lang;
        }

        Message::TranslateSwapLangs => {
            if state.translate_from.id != "auto" && state.translate_to.id != "auto" {
                std::mem::swap(&mut state.translate_from, &mut state.translate_to);
            }
        }

        Message::TranslateConfirm => {
            // Works from both TranslateSelection (initial) and TranslateResult (re-translate)
            let ctx = match &state.popup_state {
                PopupState::TranslateSelection(ctx) => Some(ctx.clone()),
                PopupState::TranslateResult { context, .. } => {
                    // Re-translate uses the (possibly edited) original text from editor
                    let mut ctx = context.clone();
                    ctx.text = state.translate_original_content.text();
                    Some(ctx)
                }
                _ => None,
            };
            if let Some(ctx) = ctx {
                let kind = WritingToolKind::Translate {
                    from: state.translate_from.clone(),
                    to: state.translate_to.clone(),
                };
                let text = ctx.text.clone();
                state.last_tool = Some(kind.clone());
                state.popup_state = PopupState::Processing {
                    context: ctx,
                    tool: kind.clone(),
                };
                let resize =
                    popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
                let ai_req = popup_ai::start_request(&state.settings, &text, &kind);
                return Task::batch([resize, ai_req]);
            }
        }

        Message::TranslateOriginalAction(action) => {
            state.translate_original_content.perform(action);
        }

        Message::TranslateCopyResult => {
            let _ = arboard::Clipboard::new()
                .and_then(|mut cb| cb.set_text(state.translate_output_text.clone()));
            state.copy_feedback = true;
        }

        Message::TranslateReplaceResult => {
            let text = state.translate_output_text.clone();
            state.popup_visible = false;
            state.ignore_auto_until = Some(Instant::now() + std::time::Duration::from_secs(2));
            let hide_task = popup_window::hide_popup(&mut state.popup_state, state.popup_window_id);
            let replace_task = Task::perform(
                async move {
                    if let Err(e) = crate::editing::text_service::replace_selection(&text).await {
                        tracing::error!("Replace failed: {}", e);
                    }
                },
                |_| Message::ReplaceDone,
            );
            return Task::batch([hide_task, replace_task]);
        }

        Message::ManualCaptureResult(ctx) => {
            // Manual hotkey always shows popup — bypasses auto guards
            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Options(ctx.clone());
            state.popup_visible = true;
            state.ignore_auto_until = None;
            if let Some(id) = state.popup_window_id {
                return popup_window::show_popup(id, &ctx);
            }
        }

        Message::AiResponse(result) => {
            let ctx = popup_ai::extract_context(&state.popup_state);
            let tool = popup_ai::extract_tool(&state.popup_state);
            match result {
                Ok(output) => {
                    if let (Some(ctx), Some(tool)) = (ctx, tool) {
                        if matches!(tool, WritingToolKind::Translate { .. }) {
                            let original = ctx.text.clone();
                            state.translate_original_content =
                                text_editor::Content::with_text(&original);
                            state.translate_output_text = output.clone();
                            state.popup_state = PopupState::TranslateResult {
                                context: ctx,
                            };
                        } else {
                            state.popup_state = PopupState::Result {
                                context: ctx,
                                tool,
                                output,
                            };
                        }
                        return popup_window::resize_for_state(
                            state.popup_window_id,
                            &state.popup_state,
                        );
                    }
                }
                Err(msg) => {
                    if let Some(ctx) = ctx {
                        state.popup_state = PopupState::Error {
                            context: ctx,
                            message: msg,
                        };
                        return popup_window::resize_for_state(
                            state.popup_window_id,
                            &state.popup_state,
                        );
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
                state.popup_visible = false;
                state.ignore_auto_until = Some(Instant::now() + std::time::Duration::from_secs(2));
                let hide_task =
                    popup_window::hide_popup(&mut state.popup_state, state.popup_window_id);
                let replace_task = Task::perform(
                    async move {
                        if let Err(e) = crate::editing::text_service::replace_selection(&text).await
                        {
                            tracing::error!("Replace failed: {}", e);
                        }
                    },
                    |_| Message::ReplaceDone,
                );
                return Task::batch([hide_task, replace_task]);
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
                    let resize =
                        popup_window::resize_for_state(state.popup_window_id, &state.popup_state);
                    let ai_req = popup_ai::start_request(&state.settings, &ctx.text, &tool);
                    return Task::batch([resize, ai_req]);
                }
            }
        }

        Message::Cancel | Message::Dismiss => {
            state.popup_visible = false;
            state.ignore_auto_until = Some(Instant::now() + std::time::Duration::from_secs(1));
            return popup_window::hide_popup(&mut state.popup_state, state.popup_window_id);
        }

        Message::PollEscKey => {
            if state.popup_visible && is_esc_pressed() {
                state.popup_visible = false;
                state.ignore_auto_until = Some(Instant::now() + std::time::Duration::from_secs(1));
                return popup_window::hide_popup(&mut state.popup_state, state.popup_window_id);
            }
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
        Message::SettingsBaseUrlChanged(url) => {
            if let Some(form) = &mut state.settings_form {
                form.base_url = url;
            }
        }
        Message::SettingsModelChanged(model) => {
            if let Some(form) = &mut state.settings_form {
                form.model = model;
            }
        }
        Message::SettingsApiKeyChanged(key) => {
            if let Some(form) = &mut state.settings_form {
                form.api_key = key;
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
                        form.save_feedback = Some("Saved OK".to_string());
                        info!("Settings saved");
                    }
                    Err(msg) => {
                        form.save_feedback = Some(format!("Error: {}", msg));
                        tracing::error!("Settings save failed: {}", msg);
                    }
                }
            }
        }
        Message::TestConnection => {
            if let Some(form) = &mut state.settings_form {
                form.test_state = settings_views::ConnectionTestState::Testing;
                let base_url = form.base_url.clone();
                let model = form.model.clone();
                let api_key = form.api_key.clone();
                return Task::perform(
                    async move {
                        if api_key.is_empty() {
                            return Err("No API key configured".to_string());
                        }

                        let client = popup_ai::create_ai_client(api_key, &base_url, &model)?;

                        client.test_connection().await.map_err(|e| e.to_string())
                    },
                    Message::TestConnectionResult,
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
            debug!("Popup window ready: {:?}", id);
            state.popup_window_id = Some(id);
        }
        Message::SettingsWindowOpened(id) => {
            debug!("Settings window opened: {:?}", id);
            state.settings_window_id = Some(id);
        }
        Message::PopupClosed(id) => {
            debug!("Window closed externally: {:?}", id);
            if state.popup_window_id == Some(id) {
                // Window destroyed by OS — need to recreate on next show
                state.popup_window_id = None;
                state.popup_state = PopupState::Hidden;
                state.popup_visible = false;
                state.ignore_auto_until = Some(Instant::now() + std::time::Duration::from_secs(1));
            }
            if state.settings_window_id == Some(id) {
                state.settings_window_id = None;
                state.settings_form = None;
            }
        }

        Message::QuitRequested => return iced::exit(),
        Message::SpinnerTick => {
            state.spinner_tick = state.spinner_tick.wrapping_add(1);
        }
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
    popup_views::popup_content(
        &state.popup_state,
        state.copy_feedback,
        state.spinner_tick,
        &state.translate_from,
        &state.translate_to,
        &state.translate_original_content,
        &state.translate_output_text,
    )
}

fn subscription(state: &AppState) -> Subscription<Message> {
    let mut subs = vec![];

    let rx = Arc::clone(&state.event_rx);
    subs.push(Subscription::run_with_id(
        "app-events",
        popup_window::channel_stream(rx),
    ));

    subs.push(window::close_events().map(Message::PopupClosed));

    // Poll ESC key globally when popup is visible (works even without window focus)
    if state.popup_visible {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::PollEscKey),
        );
    }

    if state.copy_feedback {
        subs.push(iced::time::every(std::time::Duration::from_millis(1500)).map(|_| Message::Tick));
    }

    if matches!(state.popup_state, PopupState::Processing { .. }) {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(120)).map(|_| Message::SpinnerTick),
        );
    }

    Subscription::batch(subs)
}

/// Normalize text for dedup comparison: trim whitespace and normalize line endings.
fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

/// Check if ESC key is currently pressed (global, works without focus).
#[cfg(target_os = "windows")]
fn is_esc_pressed() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
    unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 }
}

#[cfg(not(target_os = "windows"))]
fn is_esc_pressed() -> bool {
    false
}
