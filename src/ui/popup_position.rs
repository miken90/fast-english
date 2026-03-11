use crate::selection::Rect;

/// Popup dimensions.
pub const POPUP_WIDTH: f32 = 340.0;
pub const POPUP_INITIAL_HEIGHT: f32 = 220.0;
const MARGIN: f32 = 8.0;
const ANCHOR_SPACING: f32 = 8.0;

/// Calculated popup position.
pub struct PopupPlacement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Get the primary screen dimensions.
#[cfg(target_os = "windows")]
pub fn get_screen_size() -> (f32, f32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as f32;
        let h = GetSystemMetrics(SM_CYSCREEN) as f32;
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            (1920.0, 1080.0)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_screen_size() -> (f32, f32) {
    (1920.0, 1080.0)
}

/// Calculate popup position near the selection bounds.
///
/// Windows coordinates: y increases downward.
pub fn calculate_position(
    selection_bounds: Option<&Rect>,
    screen_width: f32,
    screen_height: f32,
) -> PopupPlacement {
    let height = POPUP_INITIAL_HEIGHT;

    let (anchor_x, anchor_y, anchor_w, anchor_h) = match selection_bounds {
        Some(b) => (b.x as f32, b.y as f32, b.width as f32, b.height as f32),
        None => {
            // No bounds: center on screen
            return PopupPlacement {
                x: (screen_width - POPUP_WIDTH) / 2.0,
                y: (screen_height - height) / 2.0,
                width: POPUP_WIDTH,
                height,
            };
        }
    };

    let anchor_center_x = anchor_x + anchor_w / 2.0;
    let anchor_bottom = anchor_y + anchor_h;

    // Horizontal: center on anchor, clamp to screen
    let mut x = anchor_center_x - POPUP_WIDTH / 2.0;
    x = x.clamp(MARGIN, screen_width - POPUP_WIDTH - MARGIN);

    // Vertical: prefer below selection
    let space_below = screen_height - anchor_bottom - MARGIN - ANCHOR_SPACING;
    let space_above = anchor_y - MARGIN - ANCHOR_SPACING;

    let y = if height <= space_below {
        anchor_bottom + ANCHOR_SPACING
    } else if height <= space_above {
        anchor_y - height - ANCHOR_SPACING
    } else if space_below >= space_above {
        anchor_bottom + ANCHOR_SPACING
    } else {
        anchor_y - height.min(space_above) - ANCHOR_SPACING
    };

    PopupPlacement {
        x,
        y,
        width: POPUP_WIDTH,
        height,
    }
}
