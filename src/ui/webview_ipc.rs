use serde::{Deserialize, Serialize};

/// Messages sent from Rust to JavaScript via evaluate_script.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum RustToJs {
    SetState(serde_json::Value),
    ShowPopup,
    HidePopup,
    Resize { width: f64, height: f64 },
}

/// Form data payload sent by JS with TestConnection / SaveSettings.
#[derive(Debug, Clone, Deserialize)]
pub struct SettingsFormPayload {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub auto_popup: bool,
    pub auto_start: bool,
}

/// Payload for translate confirmation. `text` is optional for backward compatibility.
#[derive(Debug, Clone, Deserialize)]
pub struct TranslateConfirmPayload {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// Messages sent from JavaScript to Rust via window.ipc.postMessage.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum JsToRust {
    // Popup actions
    ToolSelected {
        kind: String,
    },
    Dismiss,
    BackToOptions,
    CopyResult,
    ReplaceResult,
    Retry,
    OpenSettings,
    // Translate actions
    TranslateFromChanged {
        lang_id: String,
    },
    TranslateToChanged {
        lang_id: String,
    },
    TranslateSwapLangs,
    TranslateConfirm(TranslateConfirmPayload),
    PromptEditorConfirm {
        text: String,
    },
    TranslateCopyResult,
    TranslateReplaceResult,
    // Settings actions
    SettingsChanged {
        field: String,
        value: serde_json::Value,
    },
    TestConnection(SettingsFormPayload),
    SaveSettings(SettingsFormPayload),
    CloseSettings,
    // Layout
    ResizePopup {
        height: f64,
    },
}

impl RustToJs {
    /// Serialize to JSON string for use in evaluate_script.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl JsToRust {
    /// Deserialize from JSON string received via IPC postMessage.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Check if WebView2 Runtime is installed on the system.
#[cfg(target_os = "windows")]
pub fn check_webview2_runtime() -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let paths = [
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
        (
            HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
    ];

    for (root, path) in paths {
        if let Ok(key) = RegKey::predef(root).open_subkey(path) {
            if let Ok(version) = key.get_value::<String, _>("pv") {
                if !version.is_empty() && version != "0.0.0.0" {
                    return Ok(version);
                }
            }
        }
    }

    Err("WebView2 Runtime not found. Please install it from \
         https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
        .to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn check_webview2_runtime() -> Result<String, String> {
    Err("WebView2 is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_to_js_show_popup_serializes() {
        let msg = RustToJs::ShowPopup;
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"ShowPopup\""));
    }

    #[test]
    fn rust_to_js_resize_serializes() {
        let msg = RustToJs::Resize {
            width: 400.0,
            height: 300.0,
        };
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"Resize\""));
        assert!(json.contains("400"));
    }

    #[test]
    fn js_to_rust_dismiss_deserializes() {
        let json = r#"{"type":"Dismiss"}"#;
        let msg = JsToRust::from_json(json).unwrap();
        assert!(matches!(msg, JsToRust::Dismiss));
    }

    #[test]
    fn js_to_rust_tool_selected_deserializes() {
        let json = r#"{"type":"ToolSelected","payload":{"kind":"rewrite"}}"#;
        let msg = JsToRust::from_json(json).unwrap();
        assert!(matches!(msg, JsToRust::ToolSelected { kind } if kind == "rewrite"));
    }

    #[test]
    fn js_to_rust_test_connection_deserializes() {
        let json = r#"{"type":"TestConnection","payload":{"base_url":"https://api.openai.com/v1","model":"gpt-4","api_key":"sk-test","auto_popup":true,"auto_start":false}}"#;
        let msg = JsToRust::from_json(json).unwrap();
        assert!(matches!(msg, JsToRust::TestConnection(_)));
    }
}
