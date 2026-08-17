# 0001 Manual resize wins for the popup session

Date: 2026-08-17

## Status

Accepted

## Context

The popup was sized purely from its own content. Dragging the window frame
grew the OS window but the document kept its old height, leaving an empty
band; and if content-driven sizing had also been left running after a drag it
would have snapped the window back and overwritten the size the user just
chose. Both directions had to be reconciled: who wins when the user's drag and
the app's content-driven sizing disagree.

## Decision

For the lifetime of one popup session, a user's dragged size wins. Once
`window_resize_tracker` attributes a `WindowEvent::Resized` to the user
(`src/ui/window_resize_tracker.rs`), the popup switches into
`.popup-fill-window` mode (`window.setManualWindowSize(true)`, `assets/popup/popup.js`),
content fills whatever size the user chose, and the measurement pass that
would resize the window from content is disabled. Auto-sizing resumes only on
the next hide→show of the popup.

## Alternatives Considered

1. Keep content-driven auto-sizing always on and ignore frame drags — rejected,
   it silently overwrites the user's explicit action.
2. Persist the user's chosen size across sessions — rejected as unrequested
   scope; not needed to fix the reported defect.

## Consequences

Positive:

- The user's resize is never fought or reverted mid-session.
- Content can grow into a resized window instead of being clipped.

Tradeoffs:

- A monitor DPI/scale change also raises `WindowEvent::Resized` and is
  misread as a user drag; harmless and self-healing on the next show, but not
  distinguished from a real drag.
- The user's chosen size does not persist past hide→show.

## Follow-Up

- None planned.
