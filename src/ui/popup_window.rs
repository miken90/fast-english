use crate::app::AppEvent;

/// Legacy iced popup-window module.
///
/// P5 cutover routes popup/settings rendering through WebView threads.
/// Keep this module only for event-shape compatibility helpers.
#[allow(dead_code)]
pub(crate) fn map_app_event(event: AppEvent) -> AppEvent {
    event
}
