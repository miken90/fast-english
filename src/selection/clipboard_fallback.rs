use crate::selection::{Rect, SelectionContext, Tier};

/// Tier 3: Get selection via clipboard intercept.
///
/// Strategy: WM_COPY on focused control first, then SendInput Ctrl+C fallback.
/// Waits for hotkey modifier keys to release before simulating copy.
///
/// **Clipboard safety note (RT-5):** `clip.clear()` destroys ALL clipboard formats
/// (images, HTML, rich text, etc.) but backup/restore only covers plain text.
/// Non-text clipboard content is lost on every invocation, even on capture failure.
#[cfg(target_os = "windows")]
pub fn get_selection_via_clipboard() -> Option<SelectionContext> {
    use arboard::Clipboard;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    struct ClipboardRestoreGuard<'a> {
        clip: &'a mut Clipboard,
        backup: Option<String>,
    }

    impl Drop for ClipboardRestoreGuard<'_> {
        fn drop(&mut self) {
            if let Some(original) = self.backup.take() {
                if let Err(e) = self.clip.set_text(&original) {
                    tracing::warn!("Failed to restore clipboard text: {}", e);
                }
            }
        }
    }

    // 1. Wait for hotkey modifiers (Ctrl, Alt, T) to release
    wait_for_modifiers_release();

    // 2. Backup current clipboard
    let mut clip = Clipboard::new().ok()?;
    let backup = clip.get_text().ok();
    let restore_guard = ClipboardRestoreGuard {
        clip: &mut clip,
        backup,
    };

    // 3. Clear clipboard
    restore_guard.clip.clear().ok();

    // 4. Try WM_COPY first, verify clipboard changed; if not, fall back to Ctrl+C
    try_wm_copy();
    let text = match poll_clipboard_text(restore_guard.clip, 200) {
        Some(t) => t,
        None => {
            // WM_COPY didn't produce text — fall back to SendInput Ctrl+C
            send_ctrl_c();
            std::thread::sleep(std::time::Duration::from_millis(80));
            poll_clipboard_text(restore_guard.clip, 500)?
        }
    };

    // 5. Get cursor position for popup placement
    let bounds = unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            Some(Rect::from_point(pt.x as f64, pt.y as f64, 200.0, 20.0))
        } else {
            None
        }
    };

    tracing::debug!(len = text.len(), "Clipboard capture succeeded");

    Some(SelectionContext {
        text,
        bounds,
        source_app: String::new(),
        detection_tier: Tier::ClipboardFallback,
        is_editable: false,
    })
}

#[cfg(not(target_os = "windows"))]
pub fn get_selection_via_clipboard() -> Option<SelectionContext> {
    None
}

/// Wait until Ctrl, Alt, and T keys are all released (max 1200ms).
/// Prevents SendInput Ctrl+C from being received as Ctrl+Alt+C.
#[cfg(target_os = "windows")]
fn wait_for_modifiers_release() {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let deadline = Instant::now() + Duration::from_millis(1200);
    loop {
        let ctrl_down = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } < 0;
        let alt_down = unsafe { GetAsyncKeyState(VK_MENU.0 as i32) } < 0;
        let t_down = unsafe { GetAsyncKeyState(0x54) } < 0; // 'T' key

        if !ctrl_down && !alt_down && !t_down {
            // Extra delay for key-up events to propagate to target app
            std::thread::sleep(Duration::from_millis(100));
            return;
        }
        if Instant::now() >= deadline {
            tracing::warn!("Modifier release timeout — proceeding anyway");
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Try sending WM_COPY to the focused control.
#[cfg(target_os = "windows")]
fn try_wm_copy() {
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(0, &mut gui_info).is_err() {
            return;
        }
        let focused = gui_info.hwndFocus;
        if focused.0.is_null() {
            return;
        }
        // WM_COPY = 0x0301
        SendMessageW(focused, 0x0301, None, None);
    }
}

/// Poll clipboard for text content, retrying up to `timeout_ms` milliseconds.
#[cfg(target_os = "windows")]
fn poll_clipboard_text(clip: &mut arboard::Clipboard, timeout_ms: u64) -> Option<String> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Ok(text) = clip.get_text() {
            if !text.is_empty() {
                return Some(text);
            }
        }
        if Instant::now() >= deadline {
            tracing::debug!("Clipboard poll timeout — no text captured");
            return None;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[cfg(target_os = "windows")]
fn send_ctrl_c() {
    use std::mem;
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_C,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_C,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];

    unsafe {
        SendInput(&inputs, mem::size_of::<INPUT>() as i32);
    }
}
