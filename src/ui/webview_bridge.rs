use crate::app::{PopupState, DEFAULT_TOOLS, TRANSLATE_LANGS};
use crate::ui::popup_position::{
    ERROR_MIN_HEIGHT, OPTIONS_MIN_HEIGHT, PROCESSING_MIN_HEIGHT, PROMPT_EDITOR_MIN_HEIGHT,
    RESULT_MIN_HEIGHT, TRANSLATE_MIN_HEIGHT, TRANSLATE_RESULT_MIN_HEIGHT,
};
use crate::ui::settings_views::SettingsForm;
use serde_json::json;

/// Extra context needed for rendering beyond what PopupState holds.
pub struct BridgeContext {
    pub translate_from: crate::app::TranslateLanguage,
    pub translate_to: crate::app::TranslateLanguage,
    /// Text from editor (possibly edited original).
    pub translate_original_content: String,
    pub translate_output_text: String,
    pub copy_feedback: bool,
    pub theme_preference: String,
}

/// Escape HTML special chars to prevent XSS when AI output gets injected into JS.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

fn with_theme(mut value: serde_json::Value, theme: &str) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("theme".to_string(), json!(theme));
    }
    value
}

/// Convert PopupState to JSON string for JS setState().
pub fn popup_state_to_json(state: &PopupState, ctx: &BridgeContext) -> String {
    match state {
        PopupState::Hidden => {
            with_theme(json!({"view": "hidden"}), &ctx.theme_preference).to_string()
        }

        PopupState::Options(sel_ctx) => {
            let preview = sel_ctx.text.chars().take(70).collect::<String>();
            let has_text = !sel_ctx.text.trim().is_empty();
            let tools: Vec<_> = DEFAULT_TOOLS
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "title": t.title,
                        "description": t.short_description
                    })
                })
                .collect();
            with_theme(
                json!({
                    "view": "options",
                    "preview": escape_html(&preview),
                    "hasText": has_text,
                    "tools": tools
                }),
                &ctx.theme_preference,
            )
            .to_string()
        }

        PopupState::TranslateSelection(sel_ctx) => {
            let languages = lang_list();
            with_theme(
                json!({
                    "view": "translate-select",
                    "from": lang_obj(&ctx.translate_from),
                    "to": lang_obj(&ctx.translate_to),
                    "languages": languages,
                    "original": escape_html(&sel_ctx.text)
                }),
                &ctx.theme_preference,
            )
            .to_string()
        }

        PopupState::PromptEditor(sel_ctx) => with_theme(
            json!({
                "view": "prompt-editor",
                "original": escape_html(&sel_ctx.text)
            }),
            &ctx.theme_preference,
        )
        .to_string(),

        PopupState::TranslateResult { .. } => {
            let languages = lang_list();
            with_theme(
                json!({
                    "view": "translate-result",
                    "original": escape_html(&ctx.translate_original_content),
                    "translated": escape_html(&ctx.translate_output_text),
                    "from": lang_obj(&ctx.translate_from),
                    "to": lang_obj(&ctx.translate_to),
                    "languages": languages,
                    "copyFeedback": ctx.copy_feedback
                }),
                &ctx.theme_preference,
            )
            .to_string()
        }

        PopupState::Processing { tool, .. } => with_theme(
            json!({"view": "processing", "tool": escape_html(&tool.to_string())}),
            &ctx.theme_preference,
        )
        .to_string(),

        PopupState::Result { output, tool, .. } => with_theme(
            json!({
                "view": "result",
                "output": escape_html(output),
                "tool": escape_html(&tool.to_string()),
                "copyFeedback": ctx.copy_feedback
            }),
            &ctx.theme_preference,
        )
        .to_string(),

        PopupState::Error { message, .. } => with_theme(
            json!({"view": "error", "message": escape_html(message)}),
            &ctx.theme_preference,
        )
        .to_string(),
    }
}

/// Serialize SettingsForm to JSON for settings WebView.
pub fn settings_to_json(form: &SettingsForm) -> String {
    let masked_key = mask_api_key(&form.api_key);
    let version = env!("CARGO_PKG_VERSION");
    json!({
        "base_url": form.base_url,
        "model": form.model,
        "api_key": masked_key,
        "auto_popup": form.auto_popup,
        "auto_start": form.auto_start,
        "theme": form.theme,
        "version": version
    })
    .to_string()
}

/// Get minimum popup height for a given state (initial hint, not authoritative).
/// JS `autoResizePopup()` is the sole resize authority after initial render.
pub fn min_height_for_state(state: &PopupState) -> f64 {
    match state {
        PopupState::Hidden => 0.0,
        PopupState::Options(_) => OPTIONS_MIN_HEIGHT as f64,
        PopupState::TranslateSelection(_) => TRANSLATE_MIN_HEIGHT as f64,
        PopupState::PromptEditor(_) => PROMPT_EDITOR_MIN_HEIGHT as f64,
        PopupState::TranslateResult { .. } => TRANSLATE_RESULT_MIN_HEIGHT as f64,
        PopupState::Processing { .. } => PROCESSING_MIN_HEIGHT as f64,
        PopupState::Result { .. } => RESULT_MIN_HEIGHT as f64,
        PopupState::Error { .. } => ERROR_MIN_HEIGHT as f64,
    }
}

// --- helpers ---

fn lang_obj(lang: &crate::app::TranslateLanguage) -> serde_json::Value {
    json!({"id": lang.id, "name": lang.name})
}

fn lang_list() -> Vec<serde_json::Value> {
    TRANSLATE_LANGS
        .iter()
        .map(|l| json!({"id": l.id, "name": l.name}))
        .collect()
}

fn mask_api_key(key: &str) -> String {
    if key.len() > 4 {
        format!("****{}", &key[key.len() - 4..])
    } else {
        "****".to_string()
    }
}
