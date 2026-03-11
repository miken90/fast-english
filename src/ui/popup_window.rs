use crate::app::{AppEvent, PopupState};
use crate::selection::SelectionContext;
use crate::settings::AppSettings;
use crate::ui::{popup_position, settings_views};
use iced::window;
use iced::{Size, Task};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

use super::popup::Message;

// -- Popup window management --

pub(crate) fn show_popup(
    popup_window_id: &mut Option<window::Id>,
    ctx: &SelectionContext,
) -> Task<Message> {
    let close_task = if let Some(id) = popup_window_id.take() {
        window::close(id)
    } else {
        Task::none()
    };

    let (screen_w, screen_h) = popup_position::get_screen_size();
    let placement =
        popup_position::calculate_position(ctx.bounds.as_ref(), screen_w, screen_h);

    let (id, open_task) = window::open(window::Settings {
        size: Size::new(placement.width, placement.height),
        position: window::Position::Specific(iced::Point::new(placement.x, placement.y)),
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

    info!(x = placement.x, y = placement.y, "Opening popup {:?}", id);

    Task::batch([close_task, open_task.map(Message::PopupOpened)])
}

pub(crate) fn hide_popup(
    popup_state: &mut PopupState,
    popup_window_id: &mut Option<window::Id>,
) -> Task<Message> {
    *popup_state = PopupState::Hidden;
    if let Some(id) = popup_window_id.take() {
        window::close(id)
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
        size: Size::new(420.0, 480.0),
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
