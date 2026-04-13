# Fast English — UI/UX Optimization & Refactoring Brainstorm

**Date**: 2026-04-13  
**Version**: 0.3.2  
**Scope**: Popup UX improvements + `popup.rs` modularization

---

## Problem Statement

Two interconnected issues:
1. **UI/UX friction** — popup flow feels jumpy (4 full-screen swaps), result view is action-heavy, processing hides context, small text sizes
2. **Maintainability risk** — `popup.rs` at 1138 lines mixes runtime, state machine, AI requests, IPC, window mgmt, Win32 glue

---

## Topic 1: UI/UX Optimization

### A. Continuous Popup Flow (M effort, HIGH impact)

**Problem**: Editor→Processing→Result→Error are 4 separate full-screen swaps. Processing hides source text. Error is a hard context switch.

**Recommendation**:
- **Processing inline**: Keep editor visible, disable inputs, show spinner in header/action row, display "Running Enhance Writing…"
- **Error inline**: Show error banner inside editor/result surface. Reserve full error screen only for unrecoverable failures.
- **Result continuity**: Keep same header structure, clear Source→Output visual relationship

**Trade-off**: Requires rethinking CSS layout but Rust state machine stays unchanged. JS rendering changes are moderate.

### B. Result View Action Simplification (S effort, MEDIUM impact)

**Current**: 7 actions competing — Back, Rerun, Close, Rich, Use as input, Copy, Replace

**Recommended hierarchy**:
| Tier | Actions |
|---|---|
| Primary | Replace, Copy |
| Secondary | Edit (back), Rerun |
| Tertiary | Preview toggle, Use as input, Source snippet |

**Quick wins**:
- Rename `Rich` → `Preview`
- Rename `Use as input` → `Edit output`
- Move Rerun from header to secondary row
- Auto-expand source snippet only when replace disabled

### C. Editor Clarity (S effort, LOW-MEDIUM impact)

- Header: replace generic "Editor" with current tool name
- CTA: tool-specific labels ("Enhance", "Fix Grammar", "Translate", "Convert")
- Disable Run when source empty/whitespace
- Animate translate controls expand/collapse

### D. Typography & Accessibility (S-M effort, MEDIUM impact)

- Increase min text from 9-10px → 11-12px
- Tool pills: either implement full `role="tab"` keyboard semantics (arrow keys, roving tabindex) or drop to regular buttons with `aria-pressed`
- Add `aria-busy="true"` on processing
- Broader `prefers-reduced-motion` support (spinner, transitions, hover transforms)
- Use `ResizeObserver` instead of brittle mouseup-based resize detection

### E. Settings UX (M effort, MEDIUM impact)

**Problem**: Mixed save model — JS sends `SettingsChanged` on every edit BUT explicit Save button exists. Confusing draft/live hybrid.

**Recommendation**: Pure draft model.
- Stop sending `SettingsChanged` per field (except theme preview)
- Keep changes local in JS until Save
- Save success → update snapshot
- Cancel/Escape → restore snapshot + close
- Disable Save until form is dirty
- Show "Unsaved changes" indicator
- Reset to Defaults → confirm dialog

### F. JS/CSS Cleanup (S effort, LOW impact)

**popup.js**:
- Add DOM element cache at init
- Replace inline `style.display` toggles with CSS classes (`.is-hidden`, `.is-expanded`, `.is-preview`)
- Replace `if/else` setState dispatch with view renderer map
- Separate Rust-driven state from ephemeral UI state

**popup.css**:
- Reorganize by component (layout, header, editor, result, error, markdown)
- Introduce state classes instead of context-specific rules
- Normalize spacing tokens between sections

---

## Topic 2: Refactoring (`popup.rs` Modularization)

### Current Structure (1138 LOC, single file)

Mixed concerns:
- Runtime bootstrap + tokio event loop
- Popup state transitions + event handling
- AI request orchestration
- WebView command emission
- Settings IPC
- Win32 message pump
- Tool utility functions

### Recommended Extraction Plan

#### Extract 1: `popup_runtime.rs` (S-M effort)
- `run()` — entry point
- `run_async_event_loop()` — tokio select loop
- Channel setup, shutdown flow
- Win32 message pump helpers (`get_current_thread_id`, `post_quit_to_thread`, `run_win32_message_pump`)

**Result**: Clean bootstrap/orchestrator layer.

#### Extract 2: `popup_controller.rs` (M-L effort)
- `handle_app_event()` — AppEvent dispatch
- `handle_webview_ipc()` — JsToRust dispatch
- `handle_internal_event()` — async result handling
- State transition helpers: `dismiss_popup`, open/close/back/retry logic
- Selection validation, can_replace gating

**Result**: State machine logic isolated from runtime plumbing.

#### Extract 3: `popup_ai.rs` (S effort)
- AI request launch, tool instruction, system override
- Output cleaning per tool
- Max tokens per tool
- Already logically cohesive — easy extract

**Result**: AI concerns isolated, easier to test and add new tools.

#### Extract 4: `popup_view.rs` (S-M effort)
- `show_popup()`, `push_popup_state()`
- Resize/move command emission
- Height calculation for modes
- WebView command helpers

**Result**: Bridge between Rust state and WebView window.

#### Extract 5: `writing_tools.rs` (S effort)
- `map_tool_id_to_kind()`, `default_tool()`
- Tool metadata, instruction lookup
- `popup_tool()`, `popup_context_and_source()`, `popup_state_for_editor()`
- Currently scattered between `popup.rs` and `app.rs`

**Result**: Single place for tool policy.

### Proposed Module Tree

```
src/ui/
├── mod.rs
├── popup_runtime.rs      (~100 LOC)  — bootstrap, event loop, Win32 pump
├── popup_controller.rs   (~400 LOC)  — event handlers, state transitions
├── popup_ai.rs           (~120 LOC)  — AI request orchestration
├── popup_view.rs         (~100 LOC)  — WebView command bridge
├── popup_position.rs     (~240 LOC)  — placement + sizing (existing)
├── webview_popup.rs      (~320 LOC)  — tao/wry host (existing)
├── webview_bridge.rs     (~170 LOC)  — state→JSON serialization (existing)
├── webview_ipc.rs        (~170 LOC)  — IPC contracts (existing)
├── webview_settings.rs   (~15 LOC)   — settings commands (existing)
├── settings_views.rs     (~70 LOC)   — settings form model (existing)
└── tray.rs               (~130 LOC)  — system tray (existing)
```

### AppState Decomposition (M effort)

Split monolithic `AppState` into composed sub-structs:

```rust
struct AppState {
    popup: PopupSession,       // popup_state, selection, visibility, position, copy_feedback
    request: RequestState,     // next_request_id, active_request_id
    settings: AppSettings,     // config
    settings_form: Option<SettingsForm>,
    http_client: reqwest::Client,
    channels: ChannelHandles,  // webview_popup_tx, webview_settings_tx
}
```

**Benefits**: Clearer ownership, easier to test, reduces accidental coupling.

### Helper Extraction (S effort)

Extract repeated patterns:
```rust
fn send_popup(state: &AppState, cmd: WebViewPopupCommand) { ... }
fn send_settings(state: &AppState, cmd: SettingsCommand) { ... }
```

Extract transition helpers:
```rust
fn open_editor(state: &mut AppState, ctx: SelectionContext, tool: WritingToolKind) { ... }
fn transition_to_processing(state: &mut AppState) { ... }
fn transition_to_result(state: &mut AppState, output: String) { ... }
fn transition_to_error(state: &mut AppState, message: String) { ... }
```

---

## Prioritized Action Plan

| # | Action | Effort | Impact | Deps |
|---|--------|--------|--------|------|
| 1 | Extract `popup_ai.rs` | S | Medium | None |
| 2 | Extract `popup_runtime.rs` + Win32 pump | S-M | Medium | None |
| 3 | Extract `popup_view.rs` (WebView bridge) | S-M | Medium | None |
| 4 | Extract `popup_controller.rs` + state helpers | M-L | High | #1-3 |
| 5 | Decompose `AppState` into sub-structs | M | Medium | #4 |
| 6 | Result view action simplification + rename labels | S | Medium | None |
| 7 | Inline processing (keep source visible) | M | High | None |
| 8 | Inline error banner | S-M | Medium | #7 |
| 9 | Editor clarity (tool-specific labels, disable empty) | S | Low-Med | None |
| 10 | Settings draft save model | M | Medium | None |
| 11 | Typography + a11y pass | S-M | Medium | None |
| 12 | JS DOM cache + class-based toggles | S | Low | None |
| 13 | ResizeObserver for textarea resize detection | S-M | Low-Med | None |

**Recommended order**: #1→#2→#3→#4→#5 (refactor sprint), then #6→#7→#8→#9 (UX sprint), then #10→#11→#12→#13 (polish sprint).

---

## Risks & Guardrails

| Risk | Mitigation |
|---|---|
| Refactor breaks event ordering / stale-request handling | Preserve request ID logic exactly; add transition tests |
| UI changes mismatch Rust state machine | Keep PopupState enum unchanged; only change JS rendering |
| Settings save model change creates edge cases | Pick one model (draft), enforce consistently |
| Accessibility regressions on keyboard flow | If keeping tab roles, implement full semantics; otherwise drop to buttons |

---

## Success Criteria

- [ ] `popup.rs` under 200 LOC (orchestrator only)
- [ ] No file in `src/ui/` exceeds 400 LOC
- [ ] Processing shows source text inline
- [ ] Result view has clear primary/secondary action hierarchy
- [ ] `cargo check` passes after each extraction step
- [ ] Settings uses consistent draft save model
