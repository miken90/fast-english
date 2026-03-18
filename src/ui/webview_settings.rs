/// Commands sent from app logic to the settings WebView window.
///
/// The actual WebView hosting logic is in `webview_popup.rs` which runs
/// both popup and settings in a single tao EventLoop (tao only allows one
/// EventLoop per process).
#[derive(Debug)]
pub enum SettingsCommand {
    Show,
    Hide,
    SetSettings { json: String },
    SetTestResult { json: String },
    SetSaveFeedback { json: String },
    Close,
}
