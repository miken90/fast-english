use crate::app::AppEvent;

/// Register the global hotkey (Ctrl+Alt+T) and return the manager handle.
///
/// Must be called from a thread with a Win32 message loop.
/// Returns None if registration fails (e.g., hotkey already taken).
#[cfg(target_os = "windows")]
fn should_forward_hotkey_event(event: global_hotkey::GlobalHotKeyEvent, hotkey_id: u32) -> bool {
    event.id == hotkey_id && event.state == global_hotkey::HotKeyState::Pressed
}

#[cfg(target_os = "windows")]
pub fn register_hotkey(
    event_tx: tokio::sync::mpsc::Sender<AppEvent>,
) -> Option<global_hotkey::GlobalHotKeyManager> {
    use global_hotkey::{
        hotkey::{Code, HotKey, Modifiers},
        GlobalHotKeyEvent, GlobalHotKeyManager,
    };

    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create hotkey manager: {}", e);
            return None;
        }
    };

    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT);
    let hotkey_id = hotkey.id;
    if let Err(e) = manager.register(hotkey) {
        tracing::error!("Failed to register Ctrl+Alt+T hotkey: {}", e);
        return None;
    }

    let tx = event_tx;
    std::thread::spawn(move || loop {
        if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
            if should_forward_hotkey_event(event, hotkey_id) {
                let _ = tx.blocking_send(AppEvent::HotkeyPressed);
            }
        }
    });

    Some(manager)
}

#[cfg(not(target_os = "windows"))]
pub fn register_hotkey(_event_tx: tokio::sync::mpsc::Sender<AppEvent>) -> Option<()> {
    tracing::warn!("Global hotkey not available on this platform");
    None
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::should_forward_hotkey_event;
    use global_hotkey::{GlobalHotKeyEvent, HotKeyState};

    #[test]
    fn forwards_only_matching_pressed_event() {
        assert!(should_forward_hotkey_event(
            GlobalHotKeyEvent {
                id: 42,
                state: HotKeyState::Pressed,
            },
            42,
        ));
        assert!(!should_forward_hotkey_event(
            GlobalHotKeyEvent {
                id: 42,
                state: HotKeyState::Released,
            },
            42,
        ));
        assert!(!should_forward_hotkey_event(
            GlobalHotKeyEvent {
                id: 7,
                state: HotKeyState::Pressed,
            },
            42,
        ));
    }
}
