use crate::selection::Rect;

/// Popup dimensions.
pub const POPUP_WIDTH: f32 = 440.0;
pub const POPUP_INITIAL_HEIGHT: f32 = EDITOR_MIN_HEIGHT;

/// Height bounds (px) for popup states.
/// Dynamic sizing: JS measures content, Rust clamps to [min, max].
/// Max heights are ratios of screen height (0.7 = 70%).
pub const EDITOR_MIN_HEIGHT: f32 = 340.0;
pub const EDITOR_MAX_RATIO: f32 = 0.7;
pub const RESULT_MIN_HEIGHT: f32 = 360.0;
pub const RESULT_MAX_RATIO: f32 = 0.7;
pub const PROCESSING_HEIGHT: f32 = 155.0;
pub const ERROR_HEIGHT: f32 = 180.0;
const MARGIN: f32 = 8.0;
const ANCHOR_SPACING: f32 = 8.0;

/// Calculated popup position.
pub struct PopupPlacement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Get the primary screen dimensions (DPI-aware logical pixels).
#[cfg(target_os = "windows")]
pub fn get_screen_size() -> (f32, f32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as f32;
        let h = GetSystemMetrics(SM_CYSCREEN) as f32;
        if w <= 0.0 || h <= 0.0 {
            return (1920.0, 1080.0);
        }
        let scale = get_dpi_scale();
        (w / scale, h / scale)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_screen_size() -> (f32, f32) {
    (1920.0, 1080.0)
}

/// Calculate popup position using best available anchor.
///
/// Priority:
/// 1. Cursor position — if cursor is near/inside selection bounds (mouse select)
/// 2. Selection bounds bottom-left — when bounds available (keyboard select)
/// 3. Caret position from UIA — for focused text controls
/// 4. Center-screen popup — final fallback
pub fn calculate_position(
    selection_bounds: Option<&Rect>,
    screen_width: f32,
    screen_height: f32,
) -> PopupPlacement {
    let height = POPUP_INITIAL_HEIGHT;
    let cursor = get_cursor_pos();
    let caret = get_caret_pos();

    // Center-screen when no selection context exists.
    // Cursor alone is not an anchor (it's just where the mouse is).
    if selection_bounds.is_none() && caret.is_none() {
        let x = ((screen_width - POPUP_WIDTH) / 2.0)
            .clamp(MARGIN, (screen_width - POPUP_WIDTH - MARGIN).max(MARGIN));
        let y = ((screen_height - height) / 2.0)
            .clamp(MARGIN, (screen_height - height - MARGIN).max(MARGIN));
        return PopupPlacement {
            x,
            y,
            width: POPUP_WIDTH,
            height,
        };
    }

    let (anchor_x, anchor_y) = pick_anchor(cursor, selection_bounds, caret);

    // Clamp to screen edges so popup never overflows
    let x = anchor_x.clamp(MARGIN, (screen_width - POPUP_WIDTH - MARGIN).max(MARGIN));
    let below_y = anchor_y + ANCHOR_SPACING;
    let above_y = anchor_y - height - ANCHOR_SPACING;

    let y = if below_y + height <= screen_height - MARGIN {
        below_y
    } else if above_y >= MARGIN {
        above_y
    } else {
        (screen_height - height - MARGIN).max(MARGIN)
    };

    PopupPlacement {
        x,
        y: y.clamp(MARGIN, (screen_height - height - MARGIN).max(MARGIN)),
        width: POPUP_WIDTH,
        height,
    }
}

/// Calculate popup placement as f64 values for WebView (tao uses f64 for LogicalPosition/LogicalSize).
///
/// Returns `(x, y, width, height)` — all logical pixels as f64.
pub fn calculate_position_f64(
    selection_bounds: Option<&Rect>,
    screen_width: f32,
    screen_height: f32,
) -> (f64, f64, f64, f64) {
    let p = calculate_position(selection_bounds, screen_width, screen_height);
    (p.x as f64, p.y as f64, p.width as f64, p.height as f64)
}

/// Center the popup on screen. Used when no text is selected.
pub fn center_on_screen_f64(screen_width: f32, screen_height: f32) -> (f64, f64, f64, f64) {
    let w = POPUP_WIDTH;
    let h = POPUP_INITIAL_HEIGHT;
    let x = ((screen_width - w) / 2.0).clamp(MARGIN, (screen_width - w - MARGIN).max(MARGIN));
    let y = ((screen_height - h) / 2.0).clamp(MARGIN, (screen_height - h - MARGIN).max(MARGIN));
    (x as f64, y as f64, w as f64, h as f64)
}

/// Pick the best anchor point (all values in logical pixels).
fn pick_anchor(
    cursor: Option<(f32, f32)>,
    bounds: Option<&Rect>,
    caret: Option<(f32, f32)>,
) -> (f32, f32) {
    #[cfg(target_os = "windows")]
    let scale = get_dpi_scale();
    #[cfg(not(target_os = "windows"))]
    let scale = 1.0_f32;

    // Cursor is only meaningful as an anchor when selection bounds exist
    // (indicates mouse-driven text selection). Without bounds, cursor
    // position is just wherever the mouse happens to be — not useful.
    if let (Some((cx, cy)), Some(b)) = (cursor, bounds) {
        let bx = b.x as f32 / scale;
        let by = b.y as f32 / scale;
        let bw = b.width as f32 / scale;
        let bh = b.height as f32 / scale;

        // Cursor near selection → mouse-driven select → use cursor
        let m = 100.0;
        let near = cx >= bx - m && cx <= bx + bw + m && cy >= by - m && cy <= by + bh + m;
        if near {
            return (cx, cy);
        }

        // Cursor far → keyboard select → use bounds bottom-left
        return (bx, by + bh);
    }

    if let Some(b) = bounds {
        let bx = b.x as f32 / scale;
        let by = b.y as f32 / scale;
        let bh = b.height as f32 / scale;
        return (bx, by + bh);
    }

    if let Some((cx, cy)) = caret {
        return (cx, cy);
    }

    (POPUP_WIDTH / 2.0, POPUP_INITIAL_HEIGHT / 2.0)
}

// -- Platform helpers (DPI, cursor, caret) --

#[cfg(target_os = "windows")]
fn get_dpi_scale() -> f32 {
    use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSX};
    unsafe {
        let hdc = GetDC(None);
        let dpi = GetDeviceCaps(hdc, LOGPIXELSX) as f32;
        let _ = ReleaseDC(None, hdc);
        if dpi > 0.0 {
            dpi / 96.0
        } else {
            1.0
        }
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_pos() -> Option<(f32, f32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            let s = get_dpi_scale();
            Some((pt.x as f32 / s, pt.y as f32 / s))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_pos() -> Option<(f32, f32)> {
    None
}

#[cfg(target_os = "windows")]
fn get_caret_pos() -> Option<(f32, f32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(0, &mut gui_info).is_err() {
            return None;
        }
        let hwnd = gui_info.hwndFocus;
        if hwnd.0.is_null() {
            return None;
        }
        let mut pt = POINT {
            x: gui_info.rcCaret.left,
            y: gui_info.rcCaret.bottom,
        };
        if ClientToScreen(hwnd, &mut pt).as_bool() && (pt.x != 0 || pt.y != 0) {
            let s = get_dpi_scale();
            Some((pt.x as f32 / s, pt.y as f32 / s))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_caret_pos() -> Option<(f32, f32)> {
    None
}
