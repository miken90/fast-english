use crate::selection::{Rect, SelectionContext, Tier};

/// Tier 2: Detect text selection via UIA ValuePattern.
///
/// Works with: simple Edit controls, dialogs, legacy Win32 text fields.
///
/// ValuePattern gives us the full value of the control but NOT the selected
/// portion. This tier is mainly useful as:
/// - A signal that the element is editable
/// - Combined with EM_GETSEL (Edit Message) to get selection range
///
/// For most cases, this returns None and falls through to clipboard fallback.
/// Only produces a result for simple Edit controls where we can use Win32
/// EM_GETSEL to determine what's selected.
#[cfg(target_os = "windows")]
pub fn try_get_selection() -> Option<SelectionContext> {
    use uiautomation::patterns::UIValuePattern;
    use uiautomation::UIAutomation;

    let automation = UIAutomation::new().ok()?;
    let focused = automation.get_focused_element().ok()?;

    let value_pattern: UIValuePattern = focused.get_pattern().ok()?;

    // Check if the control is read-only — skip if so
    if value_pattern.is_readonly().unwrap_or(true) {
        return None;
    }

    // For Win32 Edit controls, try EM_GETSEL to get selection range
    #[cfg(target_os = "windows")]
    {
        let class = focused.get_classname().unwrap_or_default();
        if class == "Edit" || class == "RichEdit20W" || class == "RICHEDIT50W" {
            return try_edit_control_selection(&focused, &value_pattern);
        }
    }

    // For other ValuePattern controls, we can't determine the selection
    None
}

#[cfg(not(target_os = "windows"))]
pub fn try_get_selection() -> Option<SelectionContext> {
    None
}

/// Try to get selection from Win32 Edit controls using EM_GETSEL message.
#[cfg(target_os = "windows")]
fn try_edit_control_selection(
    element: &uiautomation::UIElement,
    value_pattern: &uiautomation::patterns::UIValuePattern,
) -> Option<SelectionContext> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Controls::EM_GETSEL;
    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

    let handle = element.get_native_window_handle().ok()?;
    let hwnd_val: isize = handle.into();
    let native_handle = HWND(hwnd_val as *mut std::ffi::c_void);

    // EM_GETSEL returns start in LOWORD, end in HIWORD
    let result = unsafe { SendMessageW(native_handle, EM_GETSEL, None, None) };
    let start = (result.0 & 0xFFFF) as usize;
    let end = ((result.0 >> 16) & 0xFFFF) as usize;

    if start == end {
        return None; // No selection (just a caret)
    }

    let full_value = value_pattern.get_value().ok()?;
    let chars: Vec<char> = full_value.chars().collect();
    if full_value.is_empty() || start >= end || end > chars.len() {
        return None;
    }

    let selected: String = chars[start..end].iter().collect();
    if selected.trim().is_empty() {
        return None;
    }

    let bounds = element.get_bounding_rectangle().ok().map(|r| {
        Rect::from_point(
            r.get_left() as f64,
            r.get_top() as f64,
            r.get_width() as f64,
            r.get_height() as f64,
        )
    });

    let source_app = element
        .get_process_id()
        .ok()
        .and_then(|pid| super::uia_text::process_name_from_pid_external(pid))
        .unwrap_or_default();

    Some(SelectionContext {
        text: selected,
        bounds,
        source_app,
        detection_tier: Tier::ValuePattern,
        is_editable: true,
    })
}
