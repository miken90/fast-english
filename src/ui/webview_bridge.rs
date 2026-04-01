use crate::app::{PopupState, WritingToolKind, DEFAULT_TOOLS, TRANSLATE_LANGS};
use crate::ui::settings_views::SettingsForm;
use serde_json::json;

/// Extra context needed for rendering beyond what PopupState holds.
pub struct BridgeContext {
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

        PopupState::Editor {
            context,
            tool,
            source_text,
        } => {
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

            let mut payload = json!({
                "view": "editor",
                "tools": tools,
                "tool": {
                    "id": tool_id(tool),
                    "title": tool.to_string()
                },
                "sourceText": escape_html(source_text),
                "canReplace": context.is_editable
            });

            if let WritingToolKind::Translate { from, to } = tool {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("translateFrom".to_string(), lang_obj(from));
                    obj.insert("translateTo".to_string(), lang_obj(to));
                    obj.insert("languages".to_string(), json!(lang_list()));
                }
            }

            with_theme(payload, &ctx.theme_preference).to_string()
        }

        PopupState::Processing { tool, .. } => with_theme(
            json!({
                "view": "processing",
                "tool": {
                    "id": tool_id(tool),
                    "title": escape_html(&tool.to_string())
                }
            }),
            &ctx.theme_preference,
        )
        .to_string(),

        PopupState::Result {
            tool,
            source_text,
            output,
            can_replace,
            ..
        } => with_theme(
            json!({
                "view": "result",
                "tool": {
                    "id": tool_id(tool),
                    "title": escape_html(&tool.to_string())
                },
                "sourceText": escape_html(source_text),
                "output": escape_html(output),
                "canReplace": *can_replace,
                "copyFeedback": ctx.copy_feedback
            }),
            &ctx.theme_preference,
        )
        .to_string(),

        PopupState::Error {
            tool,
            source_text,
            message,
            ..
        } => with_theme(
            json!({
                "view": "error",
                "tool": {
                    "id": tool_id(tool),
                    "title": escape_html(&tool.to_string())
                },
                "sourceText": escape_html(source_text),
                "message": escape_html(message)
            }),
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

// --- helpers ---

fn tool_id(tool: &WritingToolKind) -> &'static str {
    match tool {
        WritingToolKind::EnhanceWriting => "enhance-writing",
        WritingToolKind::FixGrammar => "fix-grammar",
        WritingToolKind::ConvertPrompt => "convert-prompt",
        WritingToolKind::Translate { .. } => "translate",
    }
}

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
