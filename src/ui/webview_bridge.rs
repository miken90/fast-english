use crate::app::{PopupState, DEFAULT_TOOLS, TRANSLATE_LANGS};
use crate::ui::popup_position::{
    ERROR_HEIGHT, OPTIONS_HEIGHT, PROCESSING_HEIGHT, PROMPT_EDITOR_HEIGHT, RESULT_HEIGHT,
    TRANSLATE_HEIGHT, TRANSLATE_RESULT_HEIGHT,
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
}

/// Escape HTML special chars to prevent XSS when AI output gets injected into JS.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

/// Convert PopupState to JSON string for JS setState().
pub fn popup_state_to_json(state: &PopupState, ctx: &BridgeContext) -> String {
    match state {
        PopupState::Hidden => json!({"view": "hidden"}).to_string(),

        PopupState::Options(sel_ctx) => {
            let preview = sel_ctx.text.chars().take(70).collect::<String>();
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
            json!({
                "view": "options",
                "preview": escape_html(&preview),
                "tools": tools
            })
            .to_string()
        }

        PopupState::TranslateSelection(sel_ctx) => {
            let languages = lang_list();
            json!({
                "view": "translate-select",
                "from": lang_obj(&ctx.translate_from),
                "to": lang_obj(&ctx.translate_to),
                "languages": languages,
                "original": escape_html(&sel_ctx.text)
            })
            .to_string()
        }

        PopupState::PromptEditor(sel_ctx) => json!({
            "view": "prompt-editor",
            "original": escape_html(&sel_ctx.text)
        })
        .to_string(),

        PopupState::TranslateResult { .. } => {
            let languages = lang_list();
            json!({
                "view": "translate-result",
                "original": escape_html(&ctx.translate_original_content),
                "translated": escape_html(&ctx.translate_output_text),
                "from": lang_obj(&ctx.translate_from),
                "to": lang_obj(&ctx.translate_to),
                "languages": languages,
                "copyFeedback": ctx.copy_feedback
            })
            .to_string()
        }

        PopupState::Processing { tool, .. } => {
            json!({"view": "processing", "tool": escape_html(&tool.to_string())}).to_string()
        }

        PopupState::Result { output, tool, .. } => json!({
            "view": "result",
            "output": escape_html(output),
            "tool": escape_html(&tool.to_string()),
            "copyFeedback": ctx.copy_feedback
        })
        .to_string(),

        PopupState::Error { message, .. } => {
            json!({"view": "error", "message": escape_html(message)}).to_string()
        }
    }
}

/// Serialize SettingsForm to JSON for settings WebView.
pub fn settings_to_json(form: &SettingsForm) -> String {
    let masked_key = mask_api_key(&form.api_key);
    json!({
        "base_url": form.base_url,
        "model": form.model,
        "api_key": masked_key,
        "auto_popup": form.auto_popup,
        "auto_start": form.auto_start
    })
    .to_string()
}

/// Get popup height for a given state (same constants as popup_position.rs, as f64).
pub fn height_for_state(state: &PopupState) -> f64 {
    match state {
        PopupState::Hidden => 0.0,
        PopupState::Options(_) => OPTIONS_HEIGHT as f64,
        PopupState::TranslateSelection(_) => TRANSLATE_HEIGHT as f64,
        PopupState::PromptEditor(_) => PROMPT_EDITOR_HEIGHT as f64,
        PopupState::TranslateResult { .. } => TRANSLATE_RESULT_HEIGHT as f64,
        PopupState::Processing { .. } => PROCESSING_HEIGHT as f64,
        PopupState::Result { .. } => RESULT_HEIGHT as f64,
        PopupState::Error { .. } => ERROR_HEIGHT as f64,
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
