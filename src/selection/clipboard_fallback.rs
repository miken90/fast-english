use crate::selection::{Rect, SelectionContext, Tier};

/// Tier 3: Get selection via clipboard intercept.
///
/// Strategy: WM_COPY on focused control first, then SendInput Ctrl+C fallback.
/// Waits for hotkey modifier keys to release before simulating copy.
#[cfg(target_os = "windows")]
pub fn get_selection_via_clipboard() -> Option<SelectionContext> {
    use arboard::Clipboard;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    // 1. Wait for hotkey modifiers (Ctrl, Alt, T) to release
    wait_for_modifiers_release();

    // 2. Backup current clipboard
    let mut clip = Clipboard::new().ok()?;
    let backup = clip.get_text().ok();

    // 3. Clear clipboard
    clip.clear().ok();

    // 4. Try WM_COPY on focused control first, then SendInput Ctrl+C
    if !try_wm_copy() {
        send_ctrl_c();
    }

    // 5. Poll clipboard for up to 500ms
    let text = poll_clipboard_text(&mut clip)?;

    // 6. Restore original clipboard
    if let Some(original) = backup {
        let _ = clip.set_text(&original);
    }

    // 7. Get cursor position for popup placement
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

/// Wait until Ctrl, Alt, and T keys are all released (max 500ms).
/// Prevents SendInput Ctrl+C from being received as Ctrl+Alt+C.
#[cfg(target_os = "windows")]
fn wait_for_modifiers_release() {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        let ctrl_down = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } < 0;
        let alt_down = unsafe { GetAsyncKeyState(VK_MENU.0 as i32) } < 0;
        let t_down = unsafe { GetAsyncKeyState(0x54) } < 0; // 'T' key

        if !ctrl_down && !alt_down && !t_down {
            // Small extra delay for key-up events to propagate
            std::thread::sleep(Duration::from_millis(30));
            return;
        }
        if Instant::now() >= deadline {
            tracing::warn!("Modifier release timeout — proceeding anyway");
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Try sending WM_COPY to the focused control. Returns true if message was sent.
#[cfg(target_os = "windows")]
fn try_wm_copy() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(0, &mut gui_info).is_err() {
            return false;
        }
        let focused = gui_info.hwndFocus;
        if focused.0.is_null() {
            return false;
        }
        // WM_COPY = 0x0301
        SendMessageW(focused, 0x0301, None, None);
        true
    }
}

/// Poll clipboard for text content, retrying up to 500ms.
#[cfg(target_os = "windows")]
fn poll_clipboard_text(clip: &mut arboard::Clipboard) -> Option<String> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_millis(500);
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
