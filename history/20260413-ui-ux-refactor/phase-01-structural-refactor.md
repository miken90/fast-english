# Phase 01 — Structural Refactor

## Context Links
- [Plan](plan.md) | [Brainstorm](brainstorm.md)
- Blocked by: nothing
- Blocks: Phase 02, Phase 03

## Overview
- **Date**: 2026-04-13
- **Priority**: P1
- **Effort**: 2 days
- **Status**: ✅ Done

Split `popup.rs` (1138 LOC) into focused modules. Extract safe seams first. Enable Phase 2 UX changes with minimal JS/CSS prep work.

## Key Insights
- popup.rs mixes 6+ concerns: runtime, state machine, AI, window mgmt, settings IPC, Win32 glue
- AppState decomposition deferred — file extraction gives 80% of maintainability benefit
- Preserve request ID stale-response filtering, `ignore_auto_until`, `manual_capture_in_flight` exactly
- JS class-based toggles are enabling work for Phase 2 inline states

## Requirements
1. Split `popup.rs` into 4-5 focused modules
2. No file in `src/ui/` exceeds 400 LOC
3. Add enabling JS/CSS cleanup for Phase 2
4. `cargo check` passes after each extraction

## Architecture

### Target module tree
```
src/ui/
├── popup_runtime.rs      (~100 LOC) — run(), event loop, Win32 pump
├── popup_controller.rs   (~400 LOC) — event handlers, state transitions
├── popup_ai.rs           (~120 LOC) — AI request orchestration
├── popup_view.rs         (~100 LOC) — WebView command bridge
├── popup_position.rs     (existing, unchanged)
├── webview_popup.rs      (existing, unchanged)
├── webview_bridge.rs     (existing, unchanged)
├── webview_ipc.rs        (existing, unchanged)
└── ...
```

### Helper patterns to extract
```rust
fn send_popup(state: &AppState, cmd: WebViewPopupCommand) { ... }
fn send_settings(state: &AppState, cmd: SettingsCommand) { ... }
fn open_editor(state: &mut AppState, ctx: SelectionContext, tool: WritingToolKind) { ... }
fn transition_to_processing(state: &mut AppState) { ... }
fn transition_to_result(state: &mut AppState, output: String) { ... }
fn transition_to_error(state: &mut AppState, message: String) { ... }
```

## Related Code Files
- `src/ui/popup.rs` (1138 LOC — primary target)
- `src/ui/mod.rs` (8 LOC — add new module declarations)
- `src/app.rs` (183 LOC — WritingToolKind, PopupState)
- `assets/popup/popup.js` (570 LOC — enabling cleanup)
- `assets/popup/popup.css` (474 LOC — state classes)

## Implementation Steps

### Step 1: Extract `popup_ai.rs` (S)
Move from `popup.rs`:
- AI request launch logic (likely in handle_webview_ipc RunTool branch)
- `max_tokens_for_tool()`
- Tool instruction/system-override helpers (if any in popup.rs)
- Keep AI trait/client in `src/ai/` — this is only the orchestration bridge

Verify: `powershell.exe cargo check`

### Step 2: Extract `popup_runtime.rs` (S-M)
Move:
- `run()` function (entry point)
- `run_async_event_loop()` (tokio select loop)
- `AppState` struct definition
- `InternalEvent` enum
- `get_current_thread_id()`, `post_quit_to_thread()`, `run_win32_message_pump()`

Keep in popup_runtime: channel setup, spawning event loop thread, startup wiring.
Re-export `AppState` and `InternalEvent` for other modules.

Verify: `powershell.exe cargo check`

### Step 3: Extract `popup_view.rs` (S-M)
Move:
- `show_popup()` — positioning + WebViewPopupCommand::Show
- `push_popup_state()` — serialize state → JSON → SetState
- `dismiss_popup()` — hide + reset state
- `calculate_height_for_mode()`
- Helper: `send_popup()`, `send_settings()`

Verify: `powershell.exe cargo check`

### Step 4: Extract `popup_controller.rs` (M)
Move:
- `handle_app_event()`
- `handle_webview_ipc()`
- `handle_internal_event()`
- State transition helpers
- `normalize_text()`, `default_tool()`, `context_can_replace()`
- `popup_tool()`, `popup_context_and_source()`, `popup_state_for_editor()`
- `update_translate_tool()`, `find_translate_language()`, `map_tool_id_to_kind()`
- `should_open_manual_capture_result()`

This is the largest extraction — most of the business logic.

Verify: `powershell.exe cargo check`

### Step 5: JS/CSS enabling cleanup (S)
**popup.js**:
- Replace inline `style.display = 'none'` with class toggles (`.is-hidden`)
- Add DOM element cache object at DOMContentLoaded
- Separate Rust-driven state from ephemeral UI state in appState

**popup.css**:
- Add state classes: `.is-hidden`, `.is-processing`, `.is-expanded`, `.is-preview`
- Prepare layout structure for inline processing (editor stays visible)

Verify: manual test popup flow still works

### Step 6: Verify & measure (S)
- Confirm all files under 400 LOC
- Run `powershell.exe cargo check && cargo clippy`
- Manual smoke test: open popup, run tool, see result, dismiss

## Todo List
- [ ] Extract `popup_ai.rs` + verify
- [ ] Extract `popup_runtime.rs` + verify
- [ ] Extract `popup_view.rs` + verify
- [ ] Extract `popup_controller.rs` + verify
- [ ] Update `src/ui/mod.rs` with new module declarations
- [ ] JS class-based state toggles
- [ ] CSS state classes
- [ ] DOM cache in popup.js
- [ ] All files ≤400 LOC
- [ ] `cargo check` + `cargo clippy` pass
- [ ] Manual smoke test

## Success Criteria
- `popup.rs` no longer exists (or is ≤50 LOC re-export)
- 4-5 new modules, each ≤400 LOC
- All existing functionality preserved
- `cargo check` passes
- JS popup flow unchanged (no visual regressions)

## Risk Assessment
| Component | Risk | Mitigation |
|---|---|---|
| Event ordering regression | MEDIUM | Preserve exact event dispatch order during extraction |
| Stale async response bugs | HIGH | Keep `active_request_id` filtering logic intact and co-located |
| Circular dependencies | LOW | Extract in order: ai → runtime → view → controller |
| JS cleanup breaks flow | LOW | Class toggles are additive, keep old code until verified |

## Security Considerations
- No changes to DPAPI, API key handling, or HTTPS enforcement
- Extracted modules must not expose internal state publicly

## Next Steps
→ Phase 02: Core UX Improvements (inline processing, result simplification)
