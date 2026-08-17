//! Tells a user's frame drag apart from the app's own window resizes.
//!
//! Windows reports both through the same `WindowEvent::Resized`, so the only
//! way to know who caused a size change is to remember the size the app just
//! asked for. Everything else is the user dragging the popup's edge.

/// Physical pixel size of the popup's client area.
pub type Size = (u32, u32);

#[derive(Debug)]
pub struct WindowResizeTracker {
    /// The size the app last asked the window manager for, still unconfirmed.
    expected: Option<Size>,
    /// The last size actually reported by the window.
    current: Size,
}

impl WindowResizeTracker {
    pub fn new(initial: Size) -> Self {
        Self {
            expected: None,
            current: initial,
        }
    }

    /// Record a size the app is applying itself, so the resize event it causes
    /// is not mistaken for a user drag.
    pub fn expect(&mut self, size: Size) {
        self.expected = Some(size);
    }

    /// Feed an observed resize. Returns `true` when the user caused it.
    ///
    /// A size equal to the one already on screen is a repeat notification and
    /// counts as nobody's doing. A mismatch while a request is outstanding is
    /// treated as the user's: they can drag the frame before the window manager
    /// applies our size, and letting their size stand is the safer of the two
    /// wrong answers.
    pub fn observe(&mut self, size: Size) -> bool {
        if size == self.current {
            self.expected = None;
            return false;
        }
        let was_expected = self.expected == Some(size);
        self.expected = None;
        self.current = size;
        !was_expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_our_own_size_is_not_a_user_resize() {
        let mut tracker = WindowResizeTracker::new((440, 340));
        tracker.expect((440, 520));
        assert!(!tracker.observe((440, 520)));
    }

    #[test]
    fn dragging_the_frame_is_a_user_resize() {
        let mut tracker = WindowResizeTracker::new((440, 340));
        assert!(tracker.observe((600, 700)));
    }

    #[test]
    fn a_repeated_notification_of_the_current_size_is_ignored() {
        let mut tracker = WindowResizeTracker::new((440, 340));
        assert!(!tracker.observe((440, 340)));

        assert!(tracker.observe((600, 700)));
        assert!(!tracker.observe((600, 700)));
    }

    #[test]
    fn a_confirmed_size_becomes_the_new_baseline() {
        let mut tracker = WindowResizeTracker::new((440, 340));
        tracker.expect((440, 520));
        assert!(!tracker.observe((440, 520)));
        // The app never asked for this one, so it is the user's.
        assert!(tracker.observe((440, 521)));
    }

    #[test]
    fn a_drag_that_beats_our_pending_request_still_counts_as_the_users() {
        let mut tracker = WindowResizeTracker::new((440, 340));
        tracker.expect((440, 520));
        assert!(tracker.observe((820, 900)));
        // The outstanding request is dropped, so its late arrival is the user's
        // size being overwritten — reported, not silently swallowed.
        assert!(tracker.observe((440, 520)));
    }
}
