use crate::app::{AppEvent, PopupState, TranslateLanguage, WritingToolKind, TRANSLATE_LANGS};
use crate::selection::{SelectionContext, Tier};
use crate::settings::AppSettings;
use crate::ui::settings_views::{ConnectionTestState, SettingsForm};
use crate::ui::webview_bridge::{self, BridgeContext};
use crate::ui::webview_ipc::JsToRust;
use crate::ui::webview_popup::{spawn_webview_thread, WebViewPopupCommand};
use crate::ui::webview_settings::SettingsCommand;
use anyhow::Result;
use serde_json::json;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error};

/// Internal async message type kept for compatibility with popup_ai helpers.
#[derive(Debug, Clone)]
pub enum Message {
    AiResponse(std::result::Result<String, String>),
    TestConnectionResult(std::result::Result<String, String>),
    ReplaceDone,
}

struct AppState {
    popup_state: PopupState,
    settings_form: Option<SettingsForm>,
    current_selection: Option<SelectionContext>,
    settings: AppSettings,
    http_client: reqwest::Client,
    popup_visible: bool,
    popup_x: f64,
    popup_y: f64,
    copy_feedback: bool,
    last_auto_shown: Option<String>,
    ignore_auto_until: Option<Instant>,
    next_request_id: u64,
    active_request_id: Option<u64>,
    webview_popup_tx: std_mpsc::Sender<WebViewPopupCommand>,
    webview_settings_tx: std_mpsc::Sender<SettingsCommand>,
}

enum InternalEvent {
    AiResponse {
        request_id: u64,
        result: std::result::Result<String, String>,
    },
    ManualCaptureResult(SelectionContext),
    TestConnectionResult(std::result::Result<String, String>),
    ReplaceDone,
}

/// Run WebView runtime loop. Blocks until quit.
///
/// The main thread must pump Win32 messages for tray-icon and global-hotkey to
/// deliver events. The async event loop runs in a background thread instead.
pub fn run(
    event_rx: mpsc::Receiver<AppEvent>,
    _event_tx: mpsc::Sender<AppEvent>,
    settings: AppSettings,
    runtime: tokio::runtime::Runtime,
) -> Result<()> {
    let (webview_popup_tx, webview_popup_rx) = std_mpsc::channel::<WebViewPopupCommand>();
    let (webview_settings_tx, webview_settings_rx) = std_mpsc::channel::<SettingsCommand>();
    let (webview_ipc_tx, webview_ipc_rx) = mpsc::unbounded_channel::<JsToRust>();
    let (internal_tx, internal_rx) = mpsc::unbounded_channel::<InternalEvent>();

    let _webview_thread =
        spawn_webview_thread(webview_popup_rx, webview_settings_rx, webview_ipc_tx);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let state = AppState {
        popup_state: PopupState::Hidden,
        settings_form: None,
        current_selection: None,
        settings,
        http_client,
        popup_visible: false,
        popup_x: 0.0,
        popup_y: 0.0,
        copy_feedback: false,
        last_auto_shown: None,
        ignore_auto_until: None,
        next_request_id: 1,
        active_request_id: None,
        webview_popup_tx,
        webview_settings_tx,
    };

    push_popup_state(&state);

    // Spawn the async event loop in a background thread.
    // The main thread must remain free to pump Win32 messages (required by
    // tray-icon and global-hotkey crates on Windows).
    let main_thread_id = get_current_thread_id();
    std::thread::Builder::new()
        .name("event-loop".into())
        .spawn(move || {
            run_async_event_loop(
                runtime,
                event_rx,
                webview_ipc_rx,
                internal_tx,
                internal_rx,
                state,
                main_thread_id,
            );
        })
        .expect("Failed to spawn event loop thread");

    // Pump Win32 messages on the main thread so tray and hotkey events fire.
    run_win32_message_pump();

    Ok(())
}

/// Async event loop — runs on a background thread via `runtime.block_on()`.
fn run_async_event_loop(
    runtime: tokio::runtime::Runtime,
    mut event_rx: mpsc::Receiver<AppEvent>,
    mut webview_ipc_rx: mpsc::UnboundedReceiver<JsToRust>,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
    mut internal_rx: mpsc::UnboundedReceiver<InternalEvent>,
    mut state: AppState,
    main_thread_id: u32,
) {
    runtime.block_on(async move {
        let mut tick_1500ms = tokio::time::interval(Duration::from_millis(1500));
        tick_1500ms.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                maybe_event = event_rx.recv() => {
                    let Some(event) = maybe_event else {
                        break;
                    };
                    if !handle_app_event(&mut state, event, internal_tx.clone()) {
                        break;
                    }
                }

                maybe_ipc = webview_ipc_rx.recv() => {
                    let Some(ipc) = maybe_ipc else {
                        break;
                    };
                    handle_webview_ipc(&mut state, ipc, internal_tx.clone());
                }

                maybe_internal = internal_rx.recv() => {
                    let Some(internal) = maybe_internal else {
                        break;
                    };
                    handle_internal_event(&mut state, internal);
                }

                _ = tick_1500ms.tick() => {
                    if state.copy_feedback {
                        state.copy_feedback = false;
                        push_popup_state(&state);
                    }
                }
            }
        }

        let _ = state.webview_popup_tx.send(WebViewPopupCommand::Close);
        let _ = state.webview_settings_tx.send(SettingsCommand::Close);

        // Signal the main thread's Win32 message pump to exit.
        post_quit_to_thread(main_thread_id);
    });
}

/// Get current thread ID (Win32).
#[cfg(target_os = "windows")]
fn get_current_thread_id() -> u32 {
    unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
}

#[cfg(not(target_os = "windows"))]
fn get_current_thread_id() -> u32 {
    0
}

/// Post WM_QUIT to a specific thread to break its GetMessage loop.
#[cfg(target_os = "windows")]
fn post_quit_to_thread(thread_id: u32) {
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
            thread_id,
            windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn post_quit_to_thread(_thread_id: u32) {}

/// Pump Win32 messages on the current thread.
///
/// Keeps tray-icon and global-hotkey alive by dispatching their window messages.
/// Returns when WM_QUIT is received.
#[cfg(target_os = "windows")]
fn run_win32_message_pump() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };
    let mut msg = MSG::default();
    unsafe {
        // GetMessageW returns 0 on WM_QUIT, -1 on error.
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_win32_message_pump() {
    // On non-Windows, just park the thread until it's unparked.
    std::thread::park();
}

fn handle_app_event(
    state: &mut AppState,
    event: AppEvent,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
) -> bool {
    match event {
        AppEvent::TextSelected(ctx) => {
            if state.popup_visible {
                return true;
            }
            if state.ignore_auto_until.is_some_and(|t| Instant::now() < t) {
                return true;
            }

            let norm = normalize_text(&ctx.text);
            if state.last_auto_shown.as_ref() == Some(&norm) {
                return true;
            }

            if matches!(
                state.popup_state,
                PopupState::Processing { .. }
                    | PopupState::Result { .. }
                    | PopupState::Error { .. }
            ) {
                return true;
            }

            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Editor {
                context: ctx.clone(),
                tool: default_tool(),
                source_text: ctx.text.clone(),
            };

            if state.settings.auto_popup_enabled {
                state.last_auto_shown = Some(norm);
                state.popup_visible = true;
                show_popup(state, &ctx);
                push_popup_state(state);
            }
        }

        AppEvent::SelectionCleared => {
            if state.popup_visible {
                return true;
            }
            state.last_auto_shown = None;
            if matches!(state.popup_state, PopupState::Editor { .. }) {
                dismiss_popup(state);
            }
        }

        AppEvent::HotkeyPressed | AppEvent::OpenPopupRequested => {
            if state.popup_visible {
                dismiss_popup(state);
                return true;
            }
            let tx = internal_tx.clone();
            tokio::spawn(async move {
                let ctx = crate::selection::monitor::try_manual_capture()
                    .await
                    .unwrap_or_else(|| SelectionContext {
                        text: String::new(),
                        bounds: None,
                        source_app: String::from("manual"),
                        detection_tier: Tier::ClipboardFallback,
                        is_editable: false,
                    });
                let _ = tx.send(InternalEvent::ManualCaptureResult(ctx));
            });
        }

        AppEvent::SettingsRequested => {
            if state.settings_form.is_none() {
                state.settings_form = Some(SettingsForm::from_settings(&state.settings));
            }
            if let Some(form) = &state.settings_form {
                let _ = state.webview_settings_tx.send(SettingsCommand::Show);
                let _ = state
                    .webview_settings_tx
                    .send(SettingsCommand::SetSettings {
                        json: webview_bridge::settings_to_json(form),
                    });
                let _ = state
                    .webview_settings_tx
                    .send(SettingsCommand::SetTestResult {
                        json: json!({"status": "idle", "message": ""}).to_string(),
                    });
            }
        }

        AppEvent::QuitRequested => {
            return false;
        }
    }

    true
}

fn handle_webview_ipc(
    state: &mut AppState,
    ipc: JsToRust,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
) {
    match ipc {
        JsToRust::Dismiss => dismiss_popup(state),

        JsToRust::BackToEditor => {
            if let Some((context, tool, source_text)) = popup_state_for_editor(&state.popup_state) {
                state.popup_state = PopupState::Editor {
                    context,
                    tool,
                    source_text,
                };
                push_popup_state(state);
            }
        }

        JsToRust::ToolSelected { kind } => {
            if let Some(tool) = map_tool_id_to_kind(&kind, popup_tool(&state.popup_state)) {
                if let Some((context, source_text)) = popup_context_and_source(&state.popup_state) {
                    state.popup_state = PopupState::Editor {
                        context,
                        tool,
                        source_text,
                    };
                    push_popup_state(state);
                }
            }
        }

        JsToRust::UpdateSourceText { text } => {
            if let PopupState::Editor { source_text, .. } = &mut state.popup_state {
                *source_text = text;
            }
        }

        JsToRust::RunTool { text } => {
            if let PopupState::Editor { context, tool, .. }
            | PopupState::Result { context, tool, .. }
            | PopupState::Error { context, tool, .. } = &state.popup_state
            {
                start_ai_request(state, context.clone(), tool.clone(), text, internal_tx);
            }
        }

        JsToRust::TranslateFromChanged { lang_id } => {
            if let Some(new_from) = find_translate_language(&lang_id) {
                update_translate_tool(state, |_from, to| (new_from, to));
            }
        }

        JsToRust::TranslateToChanged { lang_id } => {
            if let Some(new_to) = find_translate_language(&lang_id) {
                update_translate_tool(state, |from, _to| (from, new_to));
            }
        }

        JsToRust::TranslateSwapLangs => {
            update_translate_tool(state, |from, to| {
                if from.id == "auto" || to.id == "auto" {
                    (from, to)
                } else {
                    (to, from)
                }
            });
        }

        JsToRust::UseOutputAsInput => {
            if let PopupState::Result {
                context,
                tool,
                output,
                ..
            } = &state.popup_state
            {
                state.popup_state = PopupState::Editor {
                    context: context.clone(),
                    tool: tool.clone(),
                    source_text: output.clone(),
                };
                push_popup_state(state);
            }
        }

        JsToRust::CopyResult => {
            if let PopupState::Result { output, .. } = &state.popup_state {
                let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(output.clone()));
                state.copy_feedback = true;
                push_popup_state(state);
            }
        }

        JsToRust::ReplaceResult => {
            if let PopupState::Result {
                output,
                can_replace: true,
                ..
            } = &state.popup_state
            {
                let text = output.clone();
                dismiss_popup(state);
                let tx = internal_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::editing::text_service::replace_selection(&text).await {
                        error!("Replace failed: {}", e);
                    }
                    let _ = tx.send(InternalEvent::ReplaceDone);
                });
            }
        }

        JsToRust::Retry => {
            if let PopupState::Error {
                context,
                tool,
                source_text,
                ..
            } = &state.popup_state
            {
                start_ai_request(
                    state,
                    context.clone(),
                    tool.clone(),
                    source_text.clone(),
                    internal_tx,
                );
            }
        }

        JsToRust::CancelProcessing => {
            // Soft-cancel by clearing active request id and returning to editor.
            // In-flight task may still complete but stale response is ignored.
            if let PopupState::Processing {
                context,
                tool,
                source_text,
            } = &state.popup_state
            {
                state.active_request_id = None;
                state.popup_state = PopupState::Editor {
                    context: context.clone(),
                    tool: tool.clone(),
                    source_text: source_text.clone(),
                };
                push_popup_state(state);
            }
        }

        JsToRust::OpenSettings => {
            if state.settings_form.is_none() {
                state.settings_form = Some(SettingsForm::from_settings(&state.settings));
            }
            if let Some(form) = &state.settings_form {
                let _ = state.webview_settings_tx.send(SettingsCommand::Show);
                let _ = state
                    .webview_settings_tx
                    .send(SettingsCommand::SetSettings {
                        json: webview_bridge::settings_to_json(form),
                    });
                let _ = state
                    .webview_settings_tx
                    .send(SettingsCommand::SetTestResult {
                        json: json!({"status": "idle", "message": ""}).to_string(),
                    });
            }
        }

        JsToRust::SettingsChanged { field, value } => {
            let form = state
                .settings_form
                .get_or_insert_with(|| SettingsForm::from_settings(&state.settings));
            let val_str = match &value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                other => other.to_string(),
            };
            match field.as_str() {
                "base_url" => form.base_url = val_str,
                "model" => form.model = val_str,
                "api_key" => form.api_key = val_str,
                "auto_popup" => {
                    form.auto_popup = value.as_bool().unwrap_or(false);
                }
                "auto_start" => {
                    form.auto_start = value.as_bool().unwrap_or(false);
                }
                "theme" => {
                    let normalized = match val_str.as_str() {
                        "light" | "dark" | "auto" => val_str,
                        _ => "auto".to_string(),
                    };
                    form.theme = normalized;
                    push_popup_state(state);
                }
                _ => {}
            }
        }

        JsToRust::TestConnection(payload) => {
            let form = state
                .settings_form
                .get_or_insert_with(|| SettingsForm::from_settings(&state.settings));
            // Sync form from payload (authoritative snapshot from JS)
            form.base_url = payload.base_url;
            form.model = payload.model;
            if !payload.api_key.starts_with("****") {
                form.api_key = payload.api_key;
            }
            form.auto_popup = payload.auto_popup;
            form.auto_start = payload.auto_start;
            if !payload.theme.is_empty() {
                form.theme = match payload.theme.as_str() {
                    "light" | "dark" | "auto" => payload.theme,
                    _ => "auto".to_string(),
                };
            }
            form.test_state = ConnectionTestState::Testing;

            let _ = state
                .webview_settings_tx
                .send(SettingsCommand::SetTestResult {
                    json: json!({"status": "testing", "message": ""}).to_string(),
                });

            let base_url = form.base_url.clone();
            let model = form.model.clone();
            let api_key = form.api_key.clone();
            let http_client = state.http_client.clone();
            let tx = internal_tx.clone();
            tokio::spawn(async move {
                let result = if api_key.is_empty() {
                    Err("No API key configured".to_string())
                } else {
                    match crate::ai::create_ai_client(api_key, &base_url, &model, http_client) {
                        Ok(client) => client.test_connection().await.map_err(|e| e.to_string()),
                        Err(e) => Err(e),
                    }
                };
                let _ = tx.send(InternalEvent::TestConnectionResult(result));
            });
        }

        JsToRust::SaveSettings(payload) => {
            let form = state
                .settings_form
                .get_or_insert_with(|| SettingsForm::from_settings(&state.settings));
            // Sync form from payload
            form.base_url = payload.base_url;
            form.model = payload.model;
            if !payload.api_key.starts_with("****") {
                form.api_key = payload.api_key;
            }
            form.auto_popup = payload.auto_popup;
            form.auto_start = payload.auto_start;
            if !payload.theme.is_empty() {
                form.theme = match payload.theme.as_str() {
                    "light" | "dark" | "auto" => payload.theme,
                    _ => "auto".to_string(),
                };
            }

            match form.apply_to_settings(&mut state.settings) {
                Ok(()) => {
                    form.save_feedback = Some("Saved OK".to_string());
                    let _ = state
                        .webview_settings_tx
                        .send(SettingsCommand::SetSaveFeedback {
                            json: json!({"message": "Saved OK", "isOk": true}).to_string(),
                        });
                }
                Err(msg) => {
                    form.save_feedback = Some(format!("Error: {}", msg));
                    let _ = state
                        .webview_settings_tx
                        .send(SettingsCommand::SetSaveFeedback {
                            json: json!({"message": format!("Error: {}", msg), "isOk": false})
                                .to_string(),
                        });
                }
            }
        }

        JsToRust::CloseSettings => {
            // Discard unsaved draft changes on close/cancel.
            state.settings_form = None;
            push_popup_state(state);
            let _ = state.webview_settings_tx.send(SettingsCommand::Hide);
        }

        JsToRust::ResizePopup { height } => {
            // JS autoResizePopup is sole resize authority. Accept any height ≥ min_height.
            let min_h = webview_bridge::min_height_for_state(&state.popup_state);
            let final_height = if height >= min_h { height } else { min_h };
            let _ = state.webview_popup_tx.send(WebViewPopupCommand::Resize {
                width: 380.0,
                height: final_height,
            });
        }

        JsToRust::MovePopup { dx, dy } => {
            state.popup_x += dx;
            state.popup_y += dy;
            let _ = state.webview_popup_tx.send(WebViewPopupCommand::Move {
                x: state.popup_x,
                y: state.popup_y,
            });
        }
    }
}

fn handle_internal_event(state: &mut AppState, event: InternalEvent) {
    match event {
        InternalEvent::AiResponse { request_id, result } => {
            if state.active_request_id != Some(request_id) {
                return;
            }

            match result {
                Ok(output) => {
                    if let PopupState::Processing {
                        context,
                        tool,
                        source_text,
                    } = &state.popup_state
                    {
                        state.popup_state = PopupState::Result {
                            context: context.clone(),
                            tool: tool.clone(),
                            source_text: source_text.clone(),
                            output,
                            can_replace: context_can_replace(context),
                        };
                        state.active_request_id = None;
                        push_popup_state(state);
                    }
                }
                Err(msg) => {
                    if let PopupState::Processing {
                        context,
                        tool,
                        source_text,
                    } = &state.popup_state
                    {
                        state.popup_state = PopupState::Error {
                            context: context.clone(),
                            tool: tool.clone(),
                            source_text: source_text.clone(),
                            message: msg,
                        };
                        state.active_request_id = None;
                        push_popup_state(state);
                    }
                }
            }
        }

        InternalEvent::ManualCaptureResult(ctx) => {
            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Editor {
                context: ctx.clone(),
                tool: default_tool(),
                source_text: ctx.text.clone(),
            };
            state.popup_visible = true;
            state.ignore_auto_until = None;
            show_popup(state, &ctx);
            push_popup_state(state);
        }

        InternalEvent::TestConnectionResult(result) => {
            if let Some(form) = &mut state.settings_form {
                match result {
                    Ok(msg) => {
                        form.test_state = ConnectionTestState::Success(msg.clone());
                        let _ = state
                            .webview_settings_tx
                            .send(SettingsCommand::SetTestResult {
                                json: json!({"status": "success", "message": msg}).to_string(),
                            });
                    }
                    Err(msg) => {
                        form.test_state = ConnectionTestState::Failure(msg.clone());
                        let _ = state
                            .webview_settings_tx
                            .send(SettingsCommand::SetTestResult {
                                json: json!({"status": "failure", "message": msg}).to_string(),
                            });
                    }
                }
            }
        }

        InternalEvent::ReplaceDone => {
            debug!("Text replacement completed");
        }
    }
}

fn start_ai_request(
    state: &mut AppState,
    context: SelectionContext,
    tool: WritingToolKind,
    source_text: String,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
) {
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.active_request_id = Some(request_id);

    let input = source_text.clone();
    state.popup_state = PopupState::Processing {
        context,
        tool: tool.clone(),
        source_text,
    };
    push_popup_state(state);

    let settings = state.settings.clone();
    let http_client = state.http_client.clone();
    tokio::spawn(async move {
        let api_key = crate::persistence::KeyStore::load("api")
            .ok()
            .flatten()
            .unwrap_or_default();

        let result = if api_key.is_empty() {
            Err("No API key configured. Open Settings to add one.".to_string())
        } else {
            match crate::ai::create_ai_client(
                api_key,
                &settings.base_url,
                &settings.model,
                http_client,
            ) {
                Ok(client) => {
                    let instruction = tool_instruction(&tool);
                    let system_override = system_override_for_tool(&tool);
                    let max_tokens = max_tokens_for_tool(&input, &tool);
                    match client
                        .process(&input, &instruction, system_override, max_tokens)
                        .await
                    {
                        Ok(raw) => Ok(clean_output_for_tool(&raw, &tool)),
                        Err(e) => Err(e.to_string()),
                    }
                }
                Err(e) => Err(e),
            }
        };

        let _ = internal_tx.send(InternalEvent::AiResponse { request_id, result });
    });
}

fn tool_instruction(tool: &WritingToolKind) -> String {
    match tool {
        WritingToolKind::EnhanceWriting => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "enhance-writing")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Improve the writing.")
            .to_string(),
        WritingToolKind::FixGrammar => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "fix-grammar")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Fix grammar.")
            .to_string(),
        WritingToolKind::ConvertPrompt => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "convert-prompt")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Turn this text into a clear prompt.")
            .to_string(),
        WritingToolKind::Translate { from, to } => {
            if from.id == "auto" {
                format!(
                    "Detect the language and translate to {}. Return only the translation.",
                    to.name
                )
            } else {
                format!(
                    "Translate from {} to {}. Return only the translation.",
                    from.name, to.name
                )
            }
        }
    }
}

fn system_override_for_tool(tool: &WritingToolKind) -> Option<&'static str> {
    match tool {
        WritingToolKind::ConvertPrompt => {
            Some(crate::ai::prompt::PromptBuilder::new().convert_prompt_system_prompt())
        }
        _ => None,
    }
}

fn clean_output_for_tool(raw: &str, tool: &WritingToolKind) -> String {
    match tool {
        WritingToolKind::ConvertPrompt => {
            crate::ai::prompt::PromptBuilder::clean_convert_prompt_output(raw)
        }
        _ => crate::ai::prompt::PromptBuilder::clean_output(raw),
    }
}

fn max_tokens_for_tool(input: &str, tool: &WritingToolKind) -> Option<usize> {
    match tool {
        WritingToolKind::ConvertPrompt => {
            Some(crate::ai::prompt::PromptBuilder::new().max_tokens_for_convert(input))
        }
        _ => None,
    }
}

fn dismiss_popup(state: &mut AppState) {
    state.popup_visible = false;
    state.ignore_auto_until = Some(Instant::now() + Duration::from_secs(1));
    state.active_request_id = None;
    state.popup_state = PopupState::Hidden;
    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Hide);
}

fn show_popup(state: &mut AppState, ctx: &SelectionContext) {
    let (sw, sh) = crate::ui::popup_position::get_screen_size();

    // No text selected → always center on screen.
    // Cannot rely on calculate_position because get_caret_pos() returns
    // a valid value whenever any app has keyboard focus (VS Code, browser, etc.).
    let has_selection = !ctx.text.is_empty() || ctx.bounds.is_some();
    let (x, y, w, h) = if has_selection {
        crate::ui::popup_position::calculate_position_f64(ctx.bounds.as_ref(), sw, sh)
    } else {
        crate::ui::popup_position::center_on_screen_f64(sw, sh)
    };

    state.popup_x = x;
    state.popup_y = y;

    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Show {
        x,
        y,
        width: w,
        height: h,
    });
}

fn push_popup_state(state: &AppState) {
    let theme_preference = state
        .settings_form
        .as_ref()
        .map(|f| f.theme.clone())
        .unwrap_or_else(|| state.settings.theme.clone());

    let bridge_ctx = BridgeContext {
        copy_feedback: state.copy_feedback,
        theme_preference,
    };
    let json = webview_bridge::popup_state_to_json(&state.popup_state, &bridge_ctx);
    let _ = state
        .webview_popup_tx
        .send(WebViewPopupCommand::SetState { json });
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

fn default_tool() -> WritingToolKind {
    WritingToolKind::EnhanceWriting
}

fn context_can_replace(context: &SelectionContext) -> bool {
    context.is_editable
}

fn popup_tool(state: &PopupState) -> Option<&WritingToolKind> {
    match state {
        PopupState::Editor { tool, .. }
        | PopupState::Processing { tool, .. }
        | PopupState::Result { tool, .. }
        | PopupState::Error { tool, .. } => Some(tool),
        PopupState::Hidden => None,
    }
}

fn popup_context_and_source(state: &PopupState) -> Option<(SelectionContext, String)> {
    match state {
        PopupState::Editor {
            context,
            source_text,
            ..
        }
        | PopupState::Processing {
            context,
            source_text,
            ..
        }
        | PopupState::Result {
            context,
            source_text,
            ..
        }
        | PopupState::Error {
            context,
            source_text,
            ..
        } => Some((context.clone(), source_text.clone())),
        PopupState::Hidden => None,
    }
}

fn popup_state_for_editor(
    state: &PopupState,
) -> Option<(SelectionContext, WritingToolKind, String)> {
    match state {
        PopupState::Editor {
            context,
            tool,
            source_text,
        }
        | PopupState::Processing {
            context,
            tool,
            source_text,
        }
        | PopupState::Result {
            context,
            tool,
            source_text,
            ..
        }
        | PopupState::Error {
            context,
            tool,
            source_text,
            ..
        } => Some((context.clone(), tool.clone(), source_text.clone())),
        PopupState::Hidden => None,
    }
}

fn update_translate_tool<F>(state: &mut AppState, apply: F)
where
    F: FnOnce(TranslateLanguage, TranslateLanguage) -> (TranslateLanguage, TranslateLanguage),
{
    let Some((context, tool, source_text)) = popup_state_for_editor(&state.popup_state) else {
        return;
    };

    let WritingToolKind::Translate { from, to } = tool else {
        return;
    };

    let (next_from, next_to) = apply(from, to);
    state.popup_state = PopupState::Editor {
        context,
        tool: WritingToolKind::Translate {
            from: next_from,
            to: next_to,
        },
        source_text,
    };
    push_popup_state(state);
}

fn find_translate_language(lang_id: &str) -> Option<TranslateLanguage> {
    TRANSLATE_LANGS
        .iter()
        .find(|lang| lang.id == lang_id)
        .cloned()
}

fn map_tool_id_to_kind(
    kind: &str,
    current_tool: Option<&WritingToolKind>,
) -> Option<WritingToolKind> {
    match kind {
        "enhance-writing" | "enhance_writing" => Some(WritingToolKind::EnhanceWriting),
        "fix-grammar" | "fix_grammar" => Some(WritingToolKind::FixGrammar),
        "convert-prompt" | "convert_prompt" => Some(WritingToolKind::ConvertPrompt),
        "translate" => {
            if let Some(WritingToolKind::Translate { from, to }) = current_tool {
                Some(WritingToolKind::Translate {
                    from: from.clone(),
                    to: to.clone(),
                })
            } else {
                Some(WritingToolKind::Translate {
                    from: TRANSLATE_LANGS[0].clone(),
                    to: TRANSLATE_LANGS[1].clone(),
                })
            }
        }
        _ => None,
    }
}
