use crate::selection::{Rect, SelectionContext, Tier};

/// Tier 3: Get selection via clipboard intercept (Ctrl+C simulation).
///
/// Only used on manual trigger. Works with: legacy apps, games, mintty/Git Bash.
#[cfg(target_os = "windows")]
pub fn get_selection_via_clipboard() -> Option<SelectionContext> {
    use arboard::Clipboard;
    use std::time::Duration;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    // 1. Backup current clipboard
    let mut clip = Clipboard::new().ok()?;
    let backup = clip.get_text().ok();

    // 2. Clear clipboard
    clip.clear().ok();

    // 3. Simulate Ctrl+C
    send_ctrl_c();

    // 4. Wait for clipboard update
    std::thread::sleep(Duration::from_millis(80));

    // 5. Read clipboard
    let text = clip.get_text().ok().filter(|t| !t.is_empty())?;

    // 6. Restore original clipboard
    if let Some(original) = backup {
        // Delay restoration to avoid race
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

    Some(SelectionContext {
        text,
        bounds,
        source_app: String::new(),
        detection_tier: Tier::ClipboardFallback,
        is_editable: false, // unknown — can't verify via clipboard fallback
    })
}

#[cfg(not(target_os = "windows"))]
pub fn get_selection_via_clipboard() -> Option<SelectionContext> {
    None
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
