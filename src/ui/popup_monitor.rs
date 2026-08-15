//! Per-monitor desktop geometry queries.
//!
//! Every value returned here is in **physical pixels** and refers to the
//! monitor that actually contains the requested point — never the primary
//! monitor. Secondary monitors can sit at negative screen coordinates, so
//! primary-only queries silently pull the popup back onto the primary display.

use super::popup_position::WorkArea;

/// Work area (desktop minus taskbar/appbars) of the monitor nearest to a
/// physical screen point, together with that monitor's DPI scale.
#[cfg(target_os = "windows")]
pub fn work_area_for_point(x: f32, y: f32) -> WorkArea {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    unsafe {
        let point = POINT {
            x: x as i32,
            y: y as i32,
        };
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if monitor.is_invalid() {
            return WorkArea::fallback();
        }

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return WorkArea::fallback();
        }

        // GetDpiForMonitor reports the monitor's own scale under PerMonitorV2,
        // unlike GetDeviceCaps(LOGPIXELSX) which always reports the primary.
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let scale = if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok()
            && dpi_x > 0
        {
            dpi_x as f32 / 96.0
        } else {
            1.0
        };

        let rc = info.rcWork;
        let width = (rc.right - rc.left) as f32;
        let height = (rc.bottom - rc.top) as f32;
        if width <= 0.0 || height <= 0.0 {
            return WorkArea::fallback();
        }

        WorkArea {
            left: rc.left as f32,
            top: rc.top as f32,
            width,
            height,
            scale,
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn work_area_for_point(_x: f32, _y: f32) -> WorkArea {
    WorkArea::fallback()
}

/// Work area of the monitor the mouse is currently on.
pub fn work_area_for_cursor() -> WorkArea {
    match cursor_pos() {
        Some((x, y)) => work_area_for_point(x, y),
        None => WorkArea::fallback(),
    }
}

/// Mouse position in physical screen pixels.
#[cfg(target_os = "windows")]
pub fn cursor_pos() -> Option<(f32, f32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            Some((pt.x as f32, pt.y as f32))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_pos() -> Option<(f32, f32)> {
    None
}

/// Caret position of the focused control in physical screen pixels.
#[cfg(target_os = "windows")]
pub fn caret_pos() -> Option<(f32, f32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};
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
            Some((pt.x as f32, pt.y as f32))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn caret_pos() -> Option<(f32, f32)> {
    None
}
