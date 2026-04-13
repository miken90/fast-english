use crate::selection::SelectionContext;
use crate::ui::popup_position::{
    EDITOR_MAX_RATIO, EDITOR_MIN_HEIGHT, ERROR_HEIGHT, PROCESSING_HEIGHT,
    RESULT_MAX_RATIO, RESULT_MIN_HEIGHT,
};
use crate::ui::webview_bridge::{self, BridgeContext};
use crate::ui::webview_popup::WebViewPopupCommand;

use super::popup_runtime::AppState;
use crate::app::PopupState;

pub(crate) fn show_popup(state: &mut AppState, ctx: &SelectionContext) {
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

pub(crate) fn push_popup_state(state: &AppState) {
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

pub(crate) fn dismiss_popup(state: &mut AppState) {
    state.popup_visible = false;
    state.manual_capture_in_flight = false;
    state.ignore_auto_until =
        Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
    state.active_request_id = None;
    state.popup_state = PopupState::Hidden;
    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Hide);
}

pub(crate) fn calculate_height_for_mode(
    mode: &str,
    content_height: Option<f64>,
    screen_height: f32,
) -> f64 {
    match mode {
        "editor" | "editor-translate" => {
            let min = EDITOR_MIN_HEIGHT as f64;
            let max_h = (screen_height as f64) * (EDITOR_MAX_RATIO as f64);
            content_height.unwrap_or(min).clamp(min, max_h)
        }
        "result" | "result-compact" | "result-expanded" => {
            let min = RESULT_MIN_HEIGHT as f64;
            let max_h = (screen_height as f64) * (RESULT_MAX_RATIO as f64);
            content_height.unwrap_or(min).clamp(min, max_h)
        }
        "processing" => PROCESSING_HEIGHT as f64,
        "error" => ERROR_HEIGHT as f64,
        _ => EDITOR_MIN_HEIGHT as f64,
    }
}
