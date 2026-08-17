use crate::selection::SelectionContext;
use crate::ui::webview_bridge::{self, BridgeContext};
use crate::ui::webview_popup::WebViewPopupCommand;

use super::popup_runtime::AppState;
use crate::app::PopupState;

pub(crate) fn show_popup(state: &mut AppState, ctx: &SelectionContext) {
    use crate::ui::popup_position::{
        pick_anchor, place_popup, popup_width, work_area_for_anchor, POPUP_INITIAL_HEIGHT,
    };

    // Without a selection the popup is centred. The caret cannot stand in for
    // one: it reports a position whenever any app holds keyboard focus.
    let has_selection = !ctx.text.is_empty() || ctx.bounds.is_some();
    let anchor = if has_selection {
        pick_anchor(ctx.bounds.as_ref())
    } else {
        None
    };

    let work = work_area_for_anchor(anchor);
    let size = (
        work.to_physical(popup_width(&work)),
        work.to_physical(POPUP_INITIAL_HEIGHT),
    );
    let (x, y) = place_popup(anchor, size, &work);

    state.popup_work = work;
    state.popup_x = x as f64;
    state.popup_y = y as f64;

    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Show {
        x: x as f64,
        y: y as f64,
        width: size.0 as f64,
        height: size.1 as f64,
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
        history: &state.history,
    };
    let json = webview_bridge::popup_state_to_json(&state.popup_state, &bridge_ctx);
    let _ = state
        .webview_popup_tx
        .send(WebViewPopupCommand::SetState { json });
}

pub(crate) fn dismiss_popup(state: &mut AppState) {
    state.popup_visible = false;
    state.manual_capture_in_flight = false;
    state.ignore_auto_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
    // Dismissing mid-stream stops the HTTP read too — a hidden popup must not
    // leave a task writing into it.
    state.abort_active_request();
    state.popup_state = PopupState::Hidden;
    let _ = state.webview_popup_tx.send(WebViewPopupCommand::Hide);
}
