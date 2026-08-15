use crate::app::{AppEvent, PopupState, TranslateLanguage, WritingToolKind, TRANSLATE_LANGS};
use crate::selection::{SelectionContext, Tier};
use crate::ui::popup_monitor;
use crate::ui::popup_position::{clamp_popup_height, clamp_popup_y, popup_width};
use crate::ui::settings_views::{ConnectionTestState, SettingsForm};
use crate::ui::webview_bridge;
use crate::ui::webview_ipc::JsToRust;
use crate::ui::webview_popup::WebViewPopupCommand;
use crate::ui::webview_settings::SettingsCommand;
use serde_json::json;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error};

use super::popup_ai::start_ai_request;
use super::popup_runtime::{AppState, InternalEvent};
use super::popup_view::{dismiss_popup, push_popup_state, show_popup};

pub(crate) fn handle_app_event(
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

        AppEvent::HotkeyPressed => {
            if state.popup_visible {
                dismiss_popup(state);
                return true;
            }
            if state.manual_capture_in_flight {
                return true;
            }
            state.manual_capture_in_flight = true;
            let tx = internal_tx.clone();
            tokio::spawn(async move {
                // Always send ManualCaptureResult so the in-flight guard is cleared.
                // should_open_manual_capture_result rejects empty text for allow_empty=false.
                let ctx = crate::selection::monitor::try_manual_capture()
                    .await
                    .unwrap_or_else(|| SelectionContext {
                        text: String::new(),
                        bounds: None,
                        source_app: String::from("manual"),
                        detection_tier: Tier::ClipboardFallback,
                        is_editable: false,
                    });
                let _ = tx.send(InternalEvent::ManualCaptureResult {
                    ctx,
                    allow_empty: false,
                });
            });
        }

        AppEvent::OpenPopupRequested => {
            if state.popup_visible {
                dismiss_popup(state);
                return true;
            }
            if state.manual_capture_in_flight {
                return true;
            }
            state.manual_capture_in_flight = true;
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
                let _ = tx.send(InternalEvent::ManualCaptureResult {
                    ctx,
                    allow_empty: true,
                });
            });
        }

        AppEvent::SettingsRequested => {
            open_settings(state);
        }

        AppEvent::QuitRequested => {
            return false;
        }
    }

    true
}

pub(crate) fn handle_webview_ipc(
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
            open_settings(state);
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
            let work = state.popup_work;
            let height_css = clamp_popup_height(height as f32, &work);
            let height_physical = work.to_physical(height_css);

            // Growing downward can push the popup past the work area, so the
            // top edge follows the new height.
            let y = clamp_popup_y(state.popup_y as f32, height_physical, &work) as f64;
            if (y - state.popup_y).abs() > 0.5 {
                state.popup_y = y;
                let _ = state.webview_popup_tx.send(WebViewPopupCommand::Move {
                    x: state.popup_x,
                    y: state.popup_y,
                });
            }

            let _ = state.webview_popup_tx.send(WebViewPopupCommand::Resize {
                width: work.to_physical(popup_width(&work)) as f64,
                height: height_physical as f64,
            });
        }

        JsToRust::PopupDragStart => {
            // Mouse coordinates from the WebView are CSS pixels, which differ
            // per monitor scale. Tracking the OS cursor instead keeps the drag
            // 1:1 on any display without a unit conversion to get wrong.
            state.popup_drag_origin =
                popup_monitor::cursor_pos().map(|c| (c, (state.popup_x, state.popup_y)));
        }

        JsToRust::PopupDragMove => {
            let Some(((start_x, start_y), (origin_x, origin_y))) = state.popup_drag_origin else {
                return;
            };
            let Some((cursor_x, cursor_y)) = popup_monitor::cursor_pos() else {
                return;
            };

            state.popup_x = origin_x + (cursor_x - start_x) as f64;
            state.popup_y = origin_y + (cursor_y - start_y) as f64;
            // A drag can cross onto another display, and later resizes must
            // clamp against whichever monitor the popup ended up on.
            state.popup_work = popup_monitor::work_area_for_point(cursor_x, cursor_y);

            let _ = state.webview_popup_tx.send(WebViewPopupCommand::Move {
                x: state.popup_x,
                y: state.popup_y,
            });
        }
    }
}

pub(crate) fn handle_internal_event(state: &mut AppState, event: InternalEvent) {
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

        InternalEvent::ManualCaptureResult { ctx, allow_empty } => {
            state.manual_capture_in_flight = false;
            // RT-2: Prevent stale async result from reopening a dismissed popup
            if state.popup_visible {
                return;
            }
            if !should_open_manual_capture_result(&ctx, allow_empty) {
                return;
            }
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

// --- Helper functions ---

fn open_settings(state: &mut AppState) {
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

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

fn default_tool() -> WritingToolKind {
    WritingToolKind::EnhanceWriting
}

fn context_can_replace(context: &SelectionContext) -> bool {
    context.is_editable
}

pub(super) fn should_open_manual_capture_result(ctx: &SelectionContext, allow_empty: bool) -> bool {
    // RT-4: When allow_empty is false (hotkey path), require actual text content.
    // Bounds alone (e.g. UIA caret position with empty text) is not a meaningful capture.
    allow_empty || !ctx.text.is_empty()
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

#[cfg(test)]
mod tests {
    use super::should_open_manual_capture_result;
    use crate::selection::{Rect, SelectionContext, Tier};

    fn selection_context(text: &str, has_bounds: bool) -> SelectionContext {
        SelectionContext {
            text: text.to_string(),
            bounds: has_bounds.then(|| Rect::from_point(0.0, 0.0, 1.0, 1.0)),
            source_app: String::from("test"),
            detection_tier: Tier::ClipboardFallback,
            is_editable: false,
        }
    }

    #[test]
    fn manual_hotkey_result_requires_text_content() {
        // Empty text, no bounds → rejected
        assert!(!should_open_manual_capture_result(
            &selection_context("", false),
            false
        ));
        // Has text, no bounds → accepted
        assert!(should_open_manual_capture_result(
            &selection_context("text", false),
            false
        ));
        // RT-4: Bounds-only (UIA caret position with empty text) → rejected for hotkey path
        assert!(!should_open_manual_capture_result(
            &selection_context("", true),
            false
        ));
    }

    #[test]
    fn tray_open_allows_empty_manual_result() {
        assert!(should_open_manual_capture_result(
            &selection_context("", false),
            true
        ));
    }
}
