# Hotkey Manual Capture Reliability: Red-Team Findings Applied

**Date**: 2026-04-04 10:43
**Severity**: High
**Component**: Hotkey trigger, manual capture pipeline, clipboard fallback, popup async flow
**Status**: Resolved

## What Happened

We shipped 7 red-team findings (RT-1 through RT-7) that fixed critical race conditions and reliability gaps in the hotkey-triggered manual capture flow. The root issue: concurrent capture attempts when key-repeat collides with in-flight captures, async clipboard operations blocking Tokio, and stale popup reopen on dismissed-then-completed capture scenarios.

## The Brutal Truth

This is absolutely maddening because the race window was real and easy to hit during normal typing after a hotkey trigger. Spamming the hotkey key-repeats could spawn multiple concurrent captures, each fighting for clipboard access and UI state. The code review catch—that `None` capture return left the in-flight guard stuck forever—is the kind of bug that would have created cascading user reports ("app broken, won't respond to hotkey anymore") and looked unfixable until someone traced the state machine. We got lucky the reviewer caught it before shipping.

## Technical Details

**RT-1: Manual capture in-flight guard**
- Added `manual_capture_in_flight: bool` atomic flag in `src/ui/popup.rs`
- Key-repeat during capture drops the event; guards `once_cell::sync::Lazy` spawn of `spawn_text_capture()`
- Prevents concurrent spawns when user hammers hotkey

**RT-2: Popup visibility check before async reopen**
- Capture completion stores result async; if popup dismissed before completion fires, stale result tried to reopen it
- Added `popup_visible` check in completion handler; dismissal sets flag to false
- Prevents phantom popup reopening after user explicitly closes it

**RT-3: Blocking clipboard I/O off Tokio thread**
- `src/selection/clipboard_fallback.rs` clipboard operations were running on Tokio worker threads
- Wrapped in `tokio::task::spawn_blocking()` — clipboard access is synchronous-only and can stall
- Eliminates thread pool starvation on systems with slow clipboard backends

**RT-4: Strict text-only hotkey path**
- `src/hotkey.rs` now requires `.as_deref() == Some("text")` from UIA caret position
- Bounds-only carets (no text attribute) are rejected early; prevents false-positive "capture" on non-text fields
- Reduces noise captures and respects actual text selection intent

**RT-5: Clipboard format destruction documented**
- `clip.clear()` in fallback destroys all clipboard formats, not just text
- Documented in code comments; users lose images, rich text, etc. during fallback
- Trade-off: necessary evil but now explicit

**RT-7: ClipboardRestoreGuard Drop logs restore failures**
- Guard Drop impl now `warn!()` instead of silently swallowing `Err`
- Makes clipboard restore failures visible in logs for debugging clipboard backend issues

**Code Review Catch (Pre-Ship)**
- **H1**: `HotkeyPressed` event sent completion result only on `Some(text)` capture; `None` capture never cleared `manual_capture_in_flight` flag, leaving it permanently stuck
- **H2**: `JoinError` from `spawn_text_capture()` join was silently swallowed; propagated to explicit error logging

**Verification**
- `cargo build`: clean
- `cargo test`: 8/8 pass
- Files modified: `src/ui/popup.rs`, `src/hotkey.rs`, `src/selection/monitor.rs`, `src/selection/clipboard_fallback.rs`, docs, README

## What We Tried

1. Inline key-repeat filtering at hotkey event level — incomplete; capture already spawned by then
2. Async-await refactor to serialize captures — too invasive; broke tray/timer triggers
3. **Final**: In-flight guard + visibility check + blocking I/O segregation + strict text validation + explicit logging

## Root Cause Analysis

Core mistakes: (1) Treated key-repeat as benign; expected app to handle concurrent captures gracefully without guards; (2) Assumed clipboard operations were cheap and could run on Tokio threads; (3) Async capture completion didn't verify popup state before reopening; (4) Code review process initially missed the `None` capture state machine gap, caught only in pre-ship verification.

## Lessons Learned

- **Guard state machine semantics rigorously**: All paths through completion handler must clear state, even `None` / error paths. Test both success and failure branches explicitly.
- **Never assume input repetition is rare**: Key-repeat at OS level hits immediately on hold; design for concurrent attempts, not "it won't happen often."
- **Segregate blocking I/O from async runtime**: Clipboard, file, network ops belong in `spawn_blocking()`, not worker threads.
- **Async completion handlers are state machines**: Every branch must restore invariants; add unit tests for each completion branch, not just the happy path.
- **Code review must trace state through all error paths**: H1 would have caught in review if we explicitly tested "capture returns None, what's the state?"

## Next Steps

- **Owner**: Core app maintainers (main branch).
- **Monitor logs**: Watch for `warn!()` clipboard restore failures in next release; may indicate platform clipboard backend issues.
- **Add branch coverage**: Extend tests to cover `None` capture, `JoinError`, popup-dismissed-during-async scenarios.
- **Docs impact**: **Minor** — technical decisions documented in code comments and README; no user-facing change needed.
