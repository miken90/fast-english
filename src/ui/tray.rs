use crate::app::AppEvent;
use tokio::sync::mpsc;

/// Menu item identifiers for tray context menu actions.
struct MenuIds {
    settings: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

/// Create the system tray icon with context menu.
///
/// Spawns two background threads: one for tray click events, one for menu item events.
/// Returns None if tray creation fails.
#[cfg(target_os = "windows")]
pub fn create_tray(event_tx: mpsc::Sender<AppEvent>) -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let settings_id = MenuId::new("settings");
    let quit_id = MenuId::new("quit");

    let settings_item = MenuItem::with_id(settings_id.clone(), "Settings...", true, None);
    let quit_item = MenuItem::with_id(quit_id.clone(), "Quit Fast English", true, None);

    let menu = Menu::new();
    let _ = menu.append(&settings_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);

    let icon_rgba = vec![0x33, 0x99, 0xFF, 0xFF].repeat(16 * 16);
    let icon = match Icon::from_rgba(icon_rgba, 16, 16) {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to create tray icon: {}", e);
            return None;
        }
    };

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Fast English — AI Writing Assistant")
        .with_icon(icon)
        .build()
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to build tray icon: {}", e);
            return None;
        }
    };

    let ids = MenuIds {
        settings: settings_id,
        quit: quit_id,
    };

    // Spawn menu event receiver thread
    let menu_tx = event_tx.clone();
    std::thread::spawn(move || {
        use tray_icon::menu::MenuEvent;
        loop {
            if let Ok(event) = MenuEvent::receiver().recv() {
                if event.id == ids.settings {
                    let _ = menu_tx.blocking_send(AppEvent::SettingsRequested);
                } else if event.id == ids.quit {
                    let _ = menu_tx.blocking_send(AppEvent::QuitRequested);
                }
            }
        }
    });

    // Spawn tray click event receiver thread
    let click_tx = event_tx;
    std::thread::spawn(move || {
        use tray_icon::TrayIconEvent;
        loop {
            if let Ok(event) = TrayIconEvent::receiver().recv() {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        ..
                    }
                ) {
                    let _ = click_tx.blocking_send(AppEvent::TrayClicked);
                }
            }
        }
    });

    Some(tray)
}

#[cfg(not(target_os = "windows"))]
pub fn create_tray(_event_tx: mpsc::Sender<AppEvent>) -> Option<()> {
    tracing::warn!("System tray not available on this platform");
    None
}
