use crate::app::AppEvent;

/// Register the global hotkey (Ctrl+Alt+T) and return the manager handle.
///
/// Must be called from a thread with a Win32 message loop.
/// Returns None if registration fails (e.g., hotkey already taken).
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
    if let Err(e) = manager.register(hotkey) {
        tracing::error!("Failed to register Ctrl+Alt+T hotkey: {}", e);
        return None;
    }

    let tx = event_tx;
    std::thread::spawn(move || loop {
        if let Ok(_event) = GlobalHotKeyEvent::receiver().recv() {
            let _ = tx.blocking_send(AppEvent::HotkeyPressed);
        }
    });

    Some(manager)
}

#[cfg(not(target_os = "windows"))]
pub fn register_hotkey(_event_tx: tokio::sync::mpsc::Sender<AppEvent>) -> Option<()> {
    tracing::warn!("Global hotkey not available on this platform");
    None
}
