use crate::app::{
    AppEvent, PopupState, TranslateLanguage, WritingToolKind, TONES, TRANSLATE_LANGS,
};
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
    last_tool: Option<WritingToolKind>,
    translate_from: TranslateLanguage,
    translate_to: TranslateLanguage,
    translate_original_content: String,
    translate_output_text: String,
    popup_visible: bool,
    copy_feedback: bool,
    last_auto_shown: Option<String>,
    ignore_auto_until: Option<Instant>,
    webview_popup_tx: std_mpsc::Sender<WebViewPopupCommand>,
    webview_settings_tx: std_mpsc::Sender<SettingsCommand>,
}

enum InternalEvent {
    AiResponse(std::result::Result<String, String>),
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

    let _webview_thread = spawn_webview_thread(webview_popup_rx, webview_settings_rx, webview_ipc_tx);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let state = AppState {
        popup_state: PopupState::Hidden,
        settings_form: None,
        current_selection: None,
        settings,
        http_client,
        last_tool: None,
        translate_from: TRANSLATE_LANGS[0].clone(),
        translate_to: TRANSLATE_LANGS[1].clone(),
        translate_original_content: String::new(),
        translate_output_text: String::new(),
        popup_visible: false,
        copy_feedback: false,
        last_auto_shown: None,
        ignore_auto_until: None,
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
                    | PopupState::ToneSelection(_)
                    | PopupState::TranslateSelection(_)
                    | PopupState::TranslateResult { .. }
            ) {
                return true;
            }

            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Options(ctx.clone());

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
            if matches!(state.popup_state, PopupState::Options(_)) {
                dismiss_popup(state);
            }
        }

        AppEvent::HotkeyPressed | AppEvent::TrayClicked => {
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

        JsToRust::BackToOptions => {
            if let PopupState::ToneSelection(ctx) | PopupState::TranslateSelection(ctx) =
                &state.popup_state
            {
                state.popup_state = PopupState::Options(ctx.clone());
                push_popup_state(state);
            } else if let PopupState::Result { context, .. }
            | PopupState::Error { context, .. }
            | PopupState::TranslateResult { context, .. } = &state.popup_state
            {
                state.popup_state = PopupState::Options(context.clone());
                push_popup_state(state);
            }
        }

        JsToRust::ToolSelected { kind } => {
            if let Some(tool) = map_tool_id_to_kind(&kind) {
                if let PopupState::Options(ctx) = &state.popup_state {
                    if matches!(tool, WritingToolKind::ChangeTone(_)) {
                        state.popup_state = PopupState::ToneSelection(ctx.clone());
                        push_popup_state(state);
                        return;
                    }

                    if matches!(tool, WritingToolKind::Translate { .. }) {
                        state.popup_state = PopupState::TranslateSelection(ctx.clone());
                        push_popup_state(state);
                        return;
                    }

                    start_ai_request(state, ctx.clone(), tool, internal_tx);
                }
            }
        }

        JsToRust::ToneSelected { tone_id } => {
            if let Some(tone) = TONES.iter().find(|t| t.id == tone_id).cloned() {
                if let PopupState::ToneSelection(ctx) = &state.popup_state {
                    start_ai_request(
                        state,
                        ctx.clone(),
                        WritingToolKind::ChangeTone(tone),
                        internal_tx,
                    );
                }
            }
        }

        JsToRust::TranslateFromChanged { lang_id } => {
            if let Some(lang) = find_translate_language(&lang_id) {
                state.translate_from = lang;
                if matches!(
                    state.popup_state,
                    PopupState::TranslateSelection(_) | PopupState::TranslateResult { .. }
                ) {
                    push_popup_state(state);
                }
            }
        }

        JsToRust::TranslateToChanged { lang_id } => {
            if let Some(lang) = find_translate_language(&lang_id) {
                state.translate_to = lang;
                if matches!(
                    state.popup_state,
                    PopupState::TranslateSelection(_) | PopupState::TranslateResult { .. }
                ) {
                    push_popup_state(state);
                }
            }
        }

        JsToRust::TranslateSwapLangs => {
            if state.translate_from.id != "auto" && state.translate_to.id != "auto" {
                std::mem::swap(&mut state.translate_from, &mut state.translate_to);
                push_popup_state(state);
            }
        }

        JsToRust::TranslateConfirm(payload) => {
            if let Some(lang) = find_translate_language(&payload.from) {
                state.translate_from = lang;
            }
            if let Some(lang) = find_translate_language(&payload.to) {
                state.translate_to = lang;
            }

            let maybe_ctx = match &state.popup_state {
                PopupState::TranslateSelection(ctx) => {
                    let mut ctx = ctx.clone();
                    if let Some(text) = payload.text.as_ref() {
                        ctx.text = text.clone();
                    }
                    Some(ctx)
                }
                PopupState::TranslateResult { context, .. } => {
                    let mut ctx = context.clone();
                    if let Some(text) = payload.text.as_ref() {
                        ctx.text = text.clone();
                    } else if !state.translate_original_content.is_empty() {
                        ctx.text = state.translate_original_content.clone();
                    }
                    Some(ctx)
                }
                _ => None,
            };

            if let Some(ctx) = maybe_ctx {
                if let Some(text) = payload.text {
                    state.translate_original_content = text;
                }

                let tool = WritingToolKind::Translate {
                    from: state.translate_from.clone(),
                    to: state.translate_to.clone(),
                };
                start_ai_request(state, ctx, tool, internal_tx);
            }
        }

        JsToRust::TranslateCopyResult => {
            let _ = arboard::Clipboard::new()
                .and_then(|mut cb| cb.set_text(state.translate_output_text.clone()));
            state.copy_feedback = true;
            push_popup_state(state);
        }

        JsToRust::TranslateReplaceResult => {
            let text = state.translate_output_text.clone();
            dismiss_popup(state);
            tokio::spawn(async move {
                if let Err(e) = crate::editing::text_service::replace_selection(&text).await {
                    error!("Replace failed: {}", e);
                }
            });
        }

        JsToRust::CopyResult => {
            if let PopupState::Result { output, .. } = &state.popup_state {
                let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(output.clone()));
                state.copy_feedback = true;
                push_popup_state(state);
            }
        }

        JsToRust::ReplaceResult => {
            if let PopupState::Result { output, .. } = &state.popup_state {
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
            if let PopupState::Error { context, .. } = &state.popup_state {
                if let Some(tool) = state.last_tool.clone() {
                    start_ai_request(state, context.clone(), tool, internal_tx);
                }
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
            let _ = state.webview_settings_tx.send(SettingsCommand::Hide);
        }

        JsToRust::ResizePopup { height } => {
            let _ = state.webview_popup_tx.send(WebViewPopupCommand::Resize {
                width: 340.0,
                height,
            });
        }
    }
}

fn handle_internal_event(state: &mut AppState, event: InternalEvent) {
    match event {
        InternalEvent::AiResponse(result) => match result {
            Ok(output) => {
                let ctx = popup_ai_context(&state.popup_state);
                let tool = popup_ai_tool(&state.popup_state);
                if let (Some(ctx), Some(tool)) = (ctx, tool) {
                    if matches!(tool, WritingToolKind::Translate { .. }) {
                        state.translate_original_content = ctx.text.clone();
                        state.translate_output_text = output;
                        state.popup_state = PopupState::TranslateResult { context: ctx };
                    } else {
                        state.popup_state = PopupState::Result {
                            context: ctx,
                            tool,
                            output,
                        };
                    }
                    push_popup_state(state);
                }
            }
            Err(msg) => {
                if let Some(ctx) = popup_ai_context(&state.popup_state) {
                    state.popup_state = PopupState::Error {
                        context: ctx,
                        message: msg,
                    };
                    push_popup_state(state);
                }
            }
        },

        InternalEvent::ManualCaptureResult(ctx) => {
            state.current_selection = Some(ctx.clone());
            state.popup_state = PopupState::Options(ctx.clone());
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
    ctx: SelectionContext,
    tool: WritingToolKind,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
) {
    let input = ctx.text.clone();
    state.last_tool = Some(tool.clone());
    state.popup_state = PopupState::Processing {
        context: ctx,
        tool: tool.clone(),
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
                    match client.process(&input, &instruction).await {
                        Ok(raw) => Ok(crate::ai::prompt::PromptBuilder::clean_output(&raw)),
                        Err(e) => Err(e.to_string()),
                    }
                }
                Err(e) => Err(e),
            }
        };

        let _ = internal_tx.send(InternalEvent::AiResponse(result));
    });
}

fn popup_ai_context(state: &PopupState) -> Option<SelectionContext> {
    match state {
        PopupState::Processing { context, .. } => Some(context.clone()),
        PopupState::Result { context, .. } => Some(context.clone()),
        PopupState::TranslateResult { context, .. } => Some(context.clone()),
        PopupState::Error { context, .. } => Some(context.clone()),
        PopupState::Options(ctx)
        | PopupState::ToneSelection(ctx)
        | PopupState::TranslateSelection(ctx) => Some(ctx.clone()),
        PopupState::Hidden => None,
    }
}

fn popup_ai_tool(state: &PopupState) -> Option<WritingToolKind> {
    match state {
        PopupState::Processing { tool, .. } => Some(tool.clone()),
        PopupState::Result { tool, .. } => Some(tool.clone()),
        _ => None,
    }
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
        WritingToolKind::ChangeTone(tone) => tone.prompt_instruction.to_string(),
        WritingToolKind::Translate { from, to } => {
            if from.id == "auto" {
                format!(
                    "Detect the language of the following text and translate it to {}. Return only the translated text.",
                    to.name
                )
            } else {
                format!(
                    "Translate the following text from {} to {}. Return only the translated text.",
                    from.name, to.name
                )
            }
        }
    }
}

fn dismiss_popup(state: &mut AppState) {
    state.popup_visible = false;
    state.ignore_auto_until = Some(Instant::now() + Duration::from_secs(1));
    state.popup_state = PopupState::Hidden;
    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Hide);
}

fn show_popup(state: &AppState, ctx: &SelectionContext) {
    let (sw, sh) = crate::ui::popup_position::get_screen_size();
    let (x, y, w, h) =
        crate::ui::popup_position::calculate_position_f64(ctx.bounds.as_ref(), sw, sh);
    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Show {
        x,
        y,
        width: w,
        height: h,
    });
}

fn push_popup_state(state: &AppState) {
    let bridge_ctx = BridgeContext {
        translate_from: state.translate_from.clone(),
        translate_to: state.translate_to.clone(),
        translate_original_content: state.translate_original_content.clone(),
        translate_output_text: state.translate_output_text.clone(),
        copy_feedback: state.copy_feedback,
    };
    let json = webview_bridge::popup_state_to_json(&state.popup_state, &bridge_ctx);
    let _ = state
        .webview_popup_tx
        .send(WebViewPopupCommand::SetState { json });

    if state.popup_visible {
        let h = webview_bridge::height_for_state(&state.popup_state);
        let _ = state.webview_popup_tx.send(WebViewPopupCommand::Resize {
            width: 380.0,
            height: h,
        });
    }
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

fn find_translate_language(lang_id: &str) -> Option<TranslateLanguage> {
    TRANSLATE_LANGS
        .iter()
        .find(|lang| lang.id == lang_id)
        .cloned()
}

fn map_tool_id_to_kind(kind: &str) -> Option<WritingToolKind> {
    match kind {
        "enhance-writing" | "enhance_writing" => Some(WritingToolKind::EnhanceWriting),
        "fix-grammar" | "fix_grammar" => Some(WritingToolKind::FixGrammar),
        "convert-prompt" | "convert_prompt" => Some(WritingToolKind::ConvertPrompt),
        "change-tone" | "change_tone" => Some(WritingToolKind::ChangeTone(TONES[0].clone())),
        "translate" => Some(WritingToolKind::Translate {
            from: TRANSLATE_LANGS[0].clone(),
            to: TRANSLATE_LANGS[1].clone(),
        }),
        _ => None,
    }
}

