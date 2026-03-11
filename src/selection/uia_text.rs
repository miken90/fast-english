use crate::selection::{Rect, SelectionContext, Tier};

/// Tier 1: Detect text selection via UIA TextPattern.
///
/// Works with: Notepad, Word, VS Code, browsers, Windows Terminal.
///
/// Strategy:
/// 1. Get focused element via UIA
/// 2. Try to get TextPattern on the focused element
/// 3. If not found, walk up the tree to find a parent with TextPattern
/// 4. Call GetSelection() → iterate ranges → get text + bounds
#[cfg(target_os = "windows")]
pub fn try_get_selection() -> Option<SelectionContext> {
    use uiautomation::patterns::UITextPattern;
    use uiautomation::UIAutomation;

    let automation = UIAutomation::new().ok()?;
    let focused = automation.get_focused_element().ok()?;

    // Try TextPattern on focused element first, then walk up ancestors
    let text_pattern: UITextPattern = match focused.get_pattern() {
        Ok(p) => p,
        Err(_) => {
            // Walk up the UIA tree to find a TextPattern provider
            find_ancestor_text_pattern(&automation, &focused)?
        }
    };

    let ranges = text_pattern.get_selection().ok()?;

    for range in &ranges {
        let text = range.get_text(10_000).ok()?;

        // Skip empty or whitespace-only selections
        if text.trim().is_empty() {
            continue;
        }

        // Use focused element's bounding rect for popup positioning.
        // UIA TextRange doesn't expose bounding rects on all apps.
        let bounds = focused.get_bounding_rectangle().ok().map(|r| {
            Rect::from_point(
                r.get_left() as f64,
                r.get_top() as f64,
                r.get_width() as f64,
                r.get_height() as f64,
            )
        });

        let source_app = get_source_app_name(&focused);

        // Check if element is editable (TextPattern is often on read-only elements too)
        let is_editable = check_editable(&focused);

        return Some(SelectionContext {
            text,
            bounds,
            source_app,
            detection_tier: Tier::TextPattern,
            is_editable,
        });
    }

    None
}

#[cfg(not(target_os = "windows"))]
pub fn try_get_selection() -> Option<SelectionContext> {
    None
}

/// Walk up the UIA tree to find an ancestor with TextPattern.
///
/// Some apps (e.g., Chrome, VS Code) expose TextPattern on a parent element
/// rather than the focused leaf element.
#[cfg(target_os = "windows")]
fn find_ancestor_text_pattern(
    automation: &uiautomation::UIAutomation,
    element: &uiautomation::UIElement,
) -> Option<uiautomation::patterns::UITextPattern> {
    use uiautomation::patterns::UITextPattern;

    let walker = automation.create_tree_walker().ok()?;
    let mut current = walker.get_parent(element).ok()?;

    // Walk up max 5 levels to avoid infinite loops
    for _ in 0..5 {
        if let Ok(pattern) = current.get_pattern::<UITextPattern>() {
            return Some(pattern);
        }
        current = walker.get_parent(&current).ok()?;
    }

    None
}

/// Get the source application name from the focused element's process.
#[cfg(target_os = "windows")]
fn get_source_app_name(element: &uiautomation::UIElement) -> String {
    element
        .get_process_id()
        .ok()
        .and_then(|pid| process_name_from_pid(pid))
        .unwrap_or_default()
}

/// Check if the focused element is editable via ValuePattern read-only check.
#[cfg(target_os = "windows")]
fn check_editable(element: &uiautomation::UIElement) -> bool {
    use uiautomation::patterns::UIValuePattern;

    match element.get_pattern::<UIValuePattern>() {
        Ok(vp) => !vp.is_readonly().unwrap_or(true),
        Err(_) => {
            // No ValuePattern — might still be editable (e.g., rich text controls).
            // Default to true for TextPattern elements.
            true
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn process_name_from_pid_external(pid: u32) -> Option<String> {
    process_name_from_pid(pid)
}

#[cfg(target_os = "windows")]
fn process_name_from_pid(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let len = GetModuleBaseNameW(handle, None, &mut buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}
