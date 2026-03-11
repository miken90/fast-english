use crate::app::{AppEvent, PopupState};
use crate::selection::SelectionContext;
use crate::settings::AppSettings;
use crate::ui::{popup_position, settings_views};
use iced::window;
use iced::{Point, Size, Task};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

use super::popup::Message;

/// Offscreen position used to hide the popup without closing the window.
const OFFSCREEN: Point = Point::new(-9999.0, -9999.0);

// -- Popup window management (single persistent window) --

/// Create the popup window at startup (offscreen). Called once from `run_with`.
pub(crate) fn create_popup() -> (window::Id, Task<Message>) {
    let (id, open_task) = window::open(window::Settings {
        size: Size::new(340.0, 280.0),
        position: window::Position::Specific(OFFSCREEN),
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        visible: true,
        resizable: false,
        exit_on_close_request: false,
        platform_specific: window::settings::PlatformSpecific {
            skip_taskbar: true,
            ..Default::default()
        },
        ..Default::default()
    });
    info!("Creating persistent popup window {:?}", id);
    (id, open_task.map(Message::PopupOpened))
}

/// Show popup at the correct position for the given selection.
/// Moves the persistent window onscreen and resizes it.
pub(crate) fn show_popup(popup_window_id: window::Id, ctx: &SelectionContext) -> Task<Message> {
    let (screen_w, screen_h) = popup_position::get_screen_size();
    let placement = popup_position::calculate_position(ctx.bounds.as_ref(), screen_w, screen_h);

    info!(
        x = placement.x,
        y = placement.y,
        "Showing popup {:?}",
        popup_window_id
    );
    Task::batch([
        window::move_to(popup_window_id, Point::new(placement.x, placement.y)),
        window::resize(
            popup_window_id,
            Size::new(placement.width, placement.height),
        ),
    ])
}

/// Hide the popup by moving it offscreen. Window stays alive for reuse.
pub(crate) fn hide_popup(
    popup_state: &mut PopupState,
    popup_window_id: Option<window::Id>,
) -> Task<Message> {
    *popup_state = PopupState::Hidden;
    if let Some(id) = popup_window_id {
        window::move_to(id, OFFSCREEN)
    } else {
        Task::none()
    }
}

/// Resize the popup window to fit the given state.
pub(crate) fn resize_for_state(
    popup_window_id: Option<window::Id>,
    state: &PopupState,
) -> Task<Message> {
    let height = match state {
        PopupState::Hidden => return Task::none(),
        PopupState::Options(_) => popup_position::OPTIONS_HEIGHT,
        PopupState::ToneSelection(_) => popup_position::TONE_HEIGHT,
        PopupState::TranslateSelection(_) => popup_position::TRANSLATE_HEIGHT,
        PopupState::TranslateResult { .. } => popup_position::TRANSLATE_RESULT_HEIGHT,
        PopupState::Processing { .. } => popup_position::PROCESSING_HEIGHT,
        PopupState::Result { .. } => popup_position::RESULT_HEIGHT,
        PopupState::Error { .. } => popup_position::ERROR_HEIGHT,
    };
    if let Some(id) = popup_window_id {
        window::resize(id, Size::new(popup_position::POPUP_WIDTH, height))
    } else {
        Task::none()
    }
}

// -- Settings window management --

pub(crate) fn open_settings(
    settings_window_id: &Option<window::Id>,
    settings_form: &mut Option<settings_views::SettingsForm>,
    settings: &AppSettings,
) -> Task<Message> {
    if settings_window_id.is_some() {
        return Task::none();
    }

    *settings_form = Some(settings_views::SettingsForm::from_settings(settings));

    let (id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 600.0),
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        visible: true,
        resizable: false,
        exit_on_close_request: false,
        platform_specific: window::settings::PlatformSpecific {
            skip_taskbar: true,
            ..Default::default()
        },
        ..Default::default()
    });

    info!("Opening settings window {:?}", id);

    open_task.map(Message::SettingsWindowOpened)
}

pub(crate) fn close_settings(
    settings_form: &mut Option<settings_views::SettingsForm>,
    settings_window_id: &mut Option<window::Id>,
) -> Task<Message> {
    *settings_form = None;
    if let Some(id) = settings_window_id.take() {
        window::close(id)
    } else {
        Task::none()
    }
}

// -- Event bridging --

/// Bridge tokio mpsc::Receiver<AppEvent> into an iced stream of Messages.
pub(crate) fn channel_stream(
    rx: Arc<Mutex<Option<mpsc::Receiver<AppEvent>>>>,
) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(50, move |mut sender| async move {
        let mut receiver = {
            let mut guard = rx.lock().unwrap_or_else(|e| e.into_inner());
            guard.take()
        };

        if let Some(ref mut rx) = receiver {
            loop {
                match rx.recv().await {
                    Some(event) => {
                        let msg = map_app_event(event);
                        let _ = sender.try_send(msg);
                    }
                    None => break,
                }
            }
        }

        std::future::pending::<()>().await;
    })
}

fn map_app_event(event: AppEvent) -> Message {
    match event {
        AppEvent::TextSelected(ctx) => Message::TextSelected(ctx),
        AppEvent::SelectionCleared => Message::SelectionCleared,
        AppEvent::HotkeyPressed => Message::HotkeyPressed,
        AppEvent::TrayClicked => Message::TrayClicked,
        AppEvent::SettingsRequested => Message::SettingsRequested,
        AppEvent::QuitRequested => Message::QuitRequested,
    }
}
