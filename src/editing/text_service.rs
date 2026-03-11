use crate::editing::clipboard;
use anyhow::Result;
use std::time::Duration;

/// Replace the currently selected text in the focused application.
///
/// Strategy: clipboard + Ctrl+V simulation (most reliable across all apps).
pub async fn replace_selection(new_text: &str) -> Result<()> {
    let backup = clipboard::backup();

    clipboard::set_text(new_text)?;

    // Wait for clipboard to settle
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Simulate Ctrl+V
    #[cfg(target_os = "windows")]
    send_ctrl_v();

    // Wait for paste to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Restore original clipboard
    clipboard::restore(backup)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn send_ctrl_v() {
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
                    wVk: VK_V,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
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
