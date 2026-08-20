//! Popup geometry — pure math over physical pixels.
//!
//! Placement and height clamping are pure functions of an anchor point, a
//! window size, and the target monitor's work area, so they are unit-testable
//! without a desktop. All coordinates are physical pixels; CSS-pixel values
//! coming from the WebView are converted with the target monitor's scale.

use crate::selection::Rect;

use super::popup_monitor;

/// Popup width in CSS pixels.
///
/// The floor is the tool-tab row, measured from the popup document at its own
/// tokens (Segoe UI, 11.5px/600): four pills of 88.59 + 93.92 + 84.02 + 91.55
/// = 358.08, three 6px gaps (18), the switcher's 14px side padding (28) and the
/// container's 1px borders (2) — 406.08, below which the row scrolls sideways
/// and a tool falls out of view. That is a floor, not a target: at 406 the last
/// pill ends flush against the edge. 520 leaves 113.92px clear after it, so the
/// row reads as inset instead of edge to edge.
pub const POPUP_WIDTH: f32 = 520.0;
/// Height used for first placement, before the document reports its own.
///
/// The editor view holds one card now that the output card is gone, so its
/// minimum drops from 378.75 to 266.25 CSS px: 135 of chrome (a 1px border, 50
/// header and 37 tab row above, a 47 action row and 1px border below) over a
/// body that cannot go under 131.25 — a 22.5 label, an 80 text box, a 10px gap
/// and an 18.5 char count. That is 112.5 less than before: the removed card's
/// own 22.5 label and 80 box, plus the 10px gap that separated it. Shorter
/// than the minimum the body is squeezed while the card keeps its own height,
/// and the run button ends up drawn over the text.
///
/// The first frame is painted before any measurement comes back, so it also
/// has to cover ordinary content, not just the floor: the source box autosizes
/// to four lines (19.375 x 4 plus 26 of padding and border = 103.5), which
/// puts a typical editor view at 289.5. 296 clears both — 29.75 over the
/// minimum, 6.5 over the common case.
pub const POPUP_INITIAL_HEIGHT: f32 = 296.0;
/// Floor for a content-driven height, so a broken measurement cannot collapse
/// the window to nothing.
pub const POPUP_MIN_HEIGHT: f32 = 120.0;
/// Share of the monitor work area a popup may occupy at most.
const MAX_HEIGHT_RATIO: f32 = 0.9;
/// Gap kept between the popup and the work-area edges, in CSS pixels.
const MARGIN: f32 = 8.0;
/// Gap between the anchor point and the popup edge, in CSS pixels.
const ANCHOR_SPACING: f32 = 8.0;

/// Usable desktop area of one monitor, in physical pixels, plus its DPI scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

impl WorkArea {
    /// Used when the desktop cannot be queried (non-Windows, or a failed call).
    pub fn fallback() -> Self {
        WorkArea {
            left: 0.0,
            top: 0.0,
            width: 1920.0,
            height: 1080.0,
            scale: 1.0,
        }
    }

    pub fn right(&self) -> f32 {
        self.left + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.top + self.height
    }

    /// CSS pixels (what the WebView measures) → physical pixels.
    pub fn to_physical(self, css: f32) -> f32 {
        css * self.scale
    }

    /// Work-area height expressed in CSS pixels.
    pub fn height_css(&self) -> f32 {
        self.height / self.scale
    }

    /// Work-area width expressed in CSS pixels.
    pub fn width_css(&self) -> f32 {
        self.width / self.scale
    }
}

impl Default for WorkArea {
    fn default() -> Self {
        Self::fallback()
    }
}

/// Position the popup inside a monitor's work area.
///
/// `anchor` is the point the popup should attach below (physical screen
/// coordinates); `None` centres it. `size` is the popup's physical size.
/// The result is the popup's top-left corner, always inside the work area.
pub fn place_popup(anchor: Option<(f32, f32)>, size: (f32, f32), work: &WorkArea) -> (f32, f32) {
    let (w, h) = size;
    let margin = work.to_physical(MARGIN);
    let spacing = work.to_physical(ANCHOR_SPACING);

    let min_x = work.left + margin;
    let min_y = work.top + margin;
    // A popup wider or taller than the work area still starts at the top-left
    // corner rather than being pushed off-screen by a negative upper bound.
    let max_x = (work.right() - w - margin).max(min_x);
    let max_y = (work.bottom() - h - margin).max(min_y);

    let Some((ax, ay)) = anchor else {
        let x = (work.left + (work.width - w) / 2.0).clamp(min_x, max_x);
        let y = (work.top + (work.height - h) / 2.0).clamp(min_y, max_y);
        return (x, y);
    };

    let x = ax.clamp(min_x, max_x);

    let below = ay + spacing;
    let above = ay - h - spacing;
    let y = if below + h <= work.bottom() - margin {
        below
    } else if above >= min_y {
        above
    } else {
        max_y
    };

    (x, y.clamp(min_y, max_y))
}

/// Popup width in CSS pixels, narrowed on displays too small for the default.
pub fn popup_width(work: &WorkArea) -> f32 {
    POPUP_WIDTH.min(work.width_css() - 2.0 * MARGIN).max(240.0)
}

/// Clamp a content-measured height (CSS pixels) to something the monitor can
/// actually show. Returns CSS pixels.
pub fn clamp_popup_height(requested_css: f32, work: &WorkArea) -> f32 {
    let max_css = (work.height_css() * MAX_HEIGHT_RATIO).max(POPUP_MIN_HEIGHT);
    requested_css.clamp(POPUP_MIN_HEIGHT, max_css)
}

/// Keep a popup of `height` fully inside the work area by moving it up when its
/// bottom edge would fall outside. Returns the adjusted top edge (physical).
pub fn clamp_popup_y(y: f32, height: f32, work: &WorkArea) -> f32 {
    let margin = work.to_physical(MARGIN);
    let min_y = work.top + margin;
    let max_y = (work.bottom() - height - margin).max(min_y);
    y.clamp(min_y, max_y)
}

/// Pick the best anchor point for a new popup, in physical screen pixels.
///
/// Priority: cursor when it sits near the selection (mouse-driven select),
/// otherwise the selection's bottom-left, otherwise the caret. `None` means
/// there is no meaningful anchor and the popup should be centred.
pub fn pick_anchor(selection_bounds: Option<&Rect>) -> Option<(f32, f32)> {
    let cursor = popup_monitor::cursor_pos();
    let bounds =
        selection_bounds.map(|b| (b.x as f32, b.y as f32, b.width as f32, b.height as f32));

    // The cursor alone is not an anchor — without selection bounds it is just
    // wherever the mouse happens to rest.
    if let (Some((cx, cy)), Some((bx, by, bw, bh))) = (cursor, bounds) {
        let m = 100.0;
        let near = cx >= bx - m && cx <= bx + bw + m && cy >= by - m && cy <= by + bh + m;
        if near {
            return Some((cx, cy));
        }
        return Some((bx, by + bh));
    }

    if let Some((bx, by, _, bh)) = bounds {
        return Some((bx, by + bh));
    }

    popup_monitor::caret_pos()
}

/// Resolve the work area the popup should live on for a given anchor.
pub fn work_area_for_anchor(anchor: Option<(f32, f32)>) -> WorkArea {
    match anchor {
        Some((x, y)) => popup_monitor::work_area_for_point(x, y),
        None => popup_monitor::work_area_for_cursor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Narrowest window the whole tool-tab row still fits in, in CSS pixels —
    /// measured from the popup document; see `POPUP_WIDTH` for the breakdown.
    const TABS_MIN_WIDTH: f32 = 406.08;
    /// Shortest window the editor view lays out in without one section
    /// covering another; see `POPUP_INITIAL_HEIGHT` for the breakdown.
    const LAYOUT_MIN_HEIGHT: f32 = 266.25;
    /// The four-line source box a freshly opened editor actually lays out at,
    /// on top of the same 135 of chrome. The first frame has to cover this or
    /// the window visibly grows the moment the measurement lands.
    const LAYOUT_TYPICAL_HEIGHT: f32 = 289.5;

    /// 1920x1080 at 100%, positioned to the LEFT of the primary monitor.
    fn secondary_left() -> WorkArea {
        WorkArea {
            left: -1920.0,
            top: 0.0,
            width: 1920.0,
            height: 1032.0,
            scale: 1.0,
        }
    }

    /// 2560x1600 at 150%, primary, taskbar reserved at the bottom.
    fn primary_hidpi() -> WorkArea {
        WorkArea {
            left: 0.0,
            top: 0.0,
            width: 2560.0,
            height: 1528.0,
            scale: 1.5,
        }
    }

    #[test]
    fn popup_stays_on_a_monitor_at_negative_coordinates() {
        let work = secondary_left();
        let size = (POPUP_WIDTH, POPUP_INITIAL_HEIGHT);
        let (x, y) = place_popup(Some((-1500.0, 400.0)), size, &work);

        assert!(
            x >= work.left && x + size.0 <= work.right(),
            "x {x} left the secondary monitor"
        );
        assert!(x < 0.0, "x {x} was pulled onto the primary monitor");
        assert!(y >= work.top && y + size.1 <= work.bottom());
    }

    #[test]
    fn popup_hangs_below_the_anchor_when_there_is_room() {
        let work = secondary_left();
        let (_, y) = place_popup(
            Some((100.0, 200.0)),
            (POPUP_WIDTH, POPUP_INITIAL_HEIGHT),
            &work,
        );
        assert_eq!(y, 208.0, "expected anchor + 8px spacing");
    }

    #[test]
    fn popup_flips_above_the_anchor_near_the_bottom_edge() {
        let work = secondary_left();
        let (_, y) = place_popup(
            Some((100.0, 1000.0)),
            (POPUP_WIDTH, POPUP_INITIAL_HEIGHT),
            &work,
        );
        assert_eq!(
            y,
            1000.0 - POPUP_INITIAL_HEIGHT - 8.0,
            "expected flip above the anchor"
        );
    }

    #[test]
    fn popup_never_overlaps_the_taskbar_band() {
        let work = primary_hidpi();
        // Anchor below the work area entirely — e.g. a selection over the taskbar.
        let (_, y) = place_popup(Some((800.0, 1600.0)), (660.0, 510.0), &work);
        assert!(
            y + 510.0 <= work.bottom() - work.to_physical(MARGIN),
            "popup bottom {} entered the reserved band below {}",
            y + 510.0,
            work.bottom()
        );
    }

    #[test]
    fn margins_scale_with_monitor_dpi() {
        let work = primary_hidpi();
        // Anchor far left, so the x clamp lands exactly on the left margin.
        let (x, _) = place_popup(Some((-5000.0, 100.0)), (660.0, 510.0), &work);
        assert_eq!(x, 12.0, "8 CSS px at 150% is 12 physical px");
    }

    #[test]
    fn centred_placement_without_an_anchor() {
        let work = secondary_left();
        let (x, y) = place_popup(None, (POPUP_WIDTH, POPUP_INITIAL_HEIGHT), &work);
        assert_eq!(x, -1920.0 + (1920.0 - POPUP_WIDTH) / 2.0);
        assert_eq!(y, (1032.0 - POPUP_INITIAL_HEIGHT) / 2.0);
    }

    #[test]
    fn oversized_popup_is_pinned_to_the_work_area_origin() {
        let work = secondary_left();
        let (x, y) = place_popup(Some((0.0, 0.0)), (4000.0, 4000.0), &work);
        assert_eq!((x, y), (work.left + 8.0, work.top + 8.0));
    }

    #[test]
    fn the_default_width_leaves_the_tab_row_clear_of_the_edge() {
        // Fitting the pills is the floor. The default has to sit far enough
        // above it that the row reads as inset instead of edge to edge.
        let width = popup_width(&primary_hidpi());
        assert!(
            width >= TABS_MIN_WIDTH + 64.0,
            "{width} leaves only {} px after the last tab",
            width - TABS_MIN_WIDTH
        );
    }

    #[test]
    fn the_first_frame_is_tall_enough_that_no_section_covers_another() {
        // The height the monitor actually grants is what the document lays out
        // in, so the clamp is what has to clear the minimum — not the constant.
        for work in [primary_hidpi(), secondary_left()] {
            let height = clamp_popup_height(POPUP_INITIAL_HEIGHT, &work);
            assert!(
                height >= LAYOUT_MIN_HEIGHT,
                "{height} is below the {LAYOUT_MIN_HEIGHT} the editor view needs"
            );
            assert!(
                height >= LAYOUT_TYPICAL_HEIGHT,
                "{height} is below the {LAYOUT_TYPICAL_HEIGHT} an ordinary editor view lays out at"
            );
        }
    }

    #[test]
    fn width_narrows_only_on_a_display_too_small_for_the_default() {
        assert_eq!(popup_width(&primary_hidpi()), POPUP_WIDTH);
        let narrow = WorkArea {
            left: 0.0,
            top: 0.0,
            width: 400.0,
            height: 800.0,
            scale: 1.0,
        };
        assert_eq!(popup_width(&narrow), 384.0);
        // A display that fits the default keeps it rather than tracking width.
        let wide = WorkArea {
            left: 0.0,
            top: 0.0,
            width: 1000.0,
            height: 800.0,
            scale: 1.0,
        };
        assert_eq!(popup_width(&wide), POPUP_WIDTH);
    }

    #[test]
    fn height_is_clamped_to_the_work_area_not_the_full_screen() {
        let work = primary_hidpi();
        // 1528 physical / 1.5 = 1018.67 CSS px of usable height.
        let max_css = work.height_css() * 0.9;
        assert!(clamp_popup_height(5000.0, &work) <= max_css + 0.01);
        assert_eq!(clamp_popup_height(10.0, &work), POPUP_MIN_HEIGHT);
        assert_eq!(clamp_popup_height(400.0, &work), 400.0);
    }

    #[test]
    fn growing_the_popup_pulls_it_back_inside_the_work_area() {
        let work = secondary_left();
        // Placed near the bottom at its initial height, then grown.
        let y = clamp_popup_y(900.0, POPUP_INITIAL_HEIGHT, &work);
        assert_eq!(y, 1032.0 - POPUP_INITIAL_HEIGHT - 8.0);
        // A height that still fits is left untouched.
        assert_eq!(clamp_popup_y(100.0, POPUP_INITIAL_HEIGHT, &work), 100.0);
    }
}
