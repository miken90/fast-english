use crate::app::AppEvent;
use tokio::sync::mpsc;

/// Menu item identifiers for tray context menu actions.
struct MenuIds {
    open_popup: tray_icon::menu::MenuId,
    settings: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

/// Create the system tray icon with context menu.
///
/// Spawns a background thread for menu item events.
/// Returns None if tray creation fails.
#[cfg(target_os = "windows")]
pub fn create_tray(event_tx: mpsc::Sender<AppEvent>) -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let open_popup_id = MenuId::new("open-popup");
    let settings_id = MenuId::new("settings");
    let quit_id = MenuId::new("quit");

    let open_popup_item = MenuItem::with_id(open_popup_id.clone(), "Open Popup", true, None);
    let settings_item = MenuItem::with_id(settings_id.clone(), "Settings...", true, None);
    let quit_item = MenuItem::with_id(quit_id.clone(), "Quit Fast English", true, None);

    let menu = Menu::new();
    let _ = menu.append(&open_popup_item);
    let _ = menu.append(&settings_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);

    let icon_rgba = build_tray_icon();
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
        open_popup: open_popup_id,
        settings: settings_id,
        quit: quit_id,
    };

    // Spawn menu event receiver thread
    let menu_tx = event_tx.clone();
    std::thread::spawn(move || {
        use tray_icon::menu::MenuEvent;
        loop {
            if let Ok(event) = MenuEvent::receiver().recv() {
                if event.id == ids.open_popup {
                    let _ = menu_tx.blocking_send(AppEvent::OpenPopupRequested);
                } else if event.id == ids.settings {
                    let _ = menu_tx.blocking_send(AppEvent::SettingsRequested);
                } else if event.id == ids.quit {
                    let _ = menu_tx.blocking_send(AppEvent::QuitRequested);
                }
            }
        }
    });

    Some(tray)
}

/// Detect whether Windows is currently in dark mode.
///
/// Reads `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`
/// -> `SystemUsesLightTheme`. Returns `true` if dark mode is active (value = 0),
/// `false` if light mode (value = 1). Defaults to dark on any read error.
#[cfg(target_os = "windows")]
fn detect_system_dark_mode() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    match hkcu.open_subkey(path) {
        Ok(key) => {
            let value: u32 = key.get_value("SystemUsesLightTheme").unwrap_or(0);
            // 0 = dark mode, 1 = light mode
            value == 0
        }
        Err(_) => {
            tracing::warn!("Could not read Windows theme registry key; defaulting to dark mode");
            true
        }
    }
}

/// Select and decode the appropriate tray icon based on system theme.
///
/// Embeds 32×32 PNGs at compile time and decodes + resizes to 16×16 RGBA at startup.
/// - Dark mode  → blue bg + white F⚡ mark
/// - Light mode → white bg + blue F⚡ mark
#[cfg(target_os = "windows")]
fn build_tray_icon() -> Vec<u8> {
    const TRAY_DARK: &[u8] = include_bytes!("../../resources/tray_dark.png");
    const TRAY_LIGHT: &[u8] = include_bytes!("../../resources/tray_light.png");

    let png_data = if detect_system_dark_mode() {
        TRAY_DARK
    } else {
        TRAY_LIGHT
    };

    let img = image::load_from_memory(png_data)
        .expect("Failed to decode tray icon PNG")
        .resize_exact(16, 16, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    img.into_raw()
}

#[cfg(not(target_os = "windows"))]
pub fn create_tray(_event_tx: mpsc::Sender<AppEvent>) -> Option<()> {
    tracing::warn!("System tray not available on this platform");
    None
}
