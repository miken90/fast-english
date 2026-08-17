# Fast English — Codebase Summary

## Overview

- **Language**: Rust (2021 edition)
- **Platform**: Windows 10+
- **UI Stack**: WebView2 via `wry` + `tao` (WebView popup + settings windows)
- **Build system**: Cargo
- **Key dependencies**: `windows`, `uiautomation`, `wry`, `tao`, `tray-icon`, `global-hotkey`, `reqwest`, `tokio`
- **Tests**: module-level unit tests; no broad integration/E2E suite yet. Count with `find src -name '*.rs' | xargs grep -c '#\[test\]'`

## Directory Structure

```text
fast-english/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── src/
│   ├── main.rs                            # Entry point, COM init, runtime wiring
│   ├── app.rs                             # PopupState, AppEvent, writing tool and translation models
│   ├── hotkey.rs                          # Global Ctrl+Alt+T registration
│   ├── settings.rs                        # Settings model
│   ├── persistence.rs                     # JSON storage + DPAPI key storage
│   ├── ai/
│   │   ├── client.rs                      # AIClient trait
│   │   ├── openai.rs                      # OpenAI client (shared reqwest::Client)
│   │   ├── anthropic.rs                   # Anthropic client
│   │   ├── prompt.rs                      # Prompt/output normalization
│   │   └── mod.rs                         # create_ai_client() factory
│   ├── selection/
│   │   ├── monitor.rs                     # Polling + debounce + manual capture
│   │   ├── uia_text.rs                    # UIA text pattern detection
│   │   ├── uia_value.rs                   # UIA value pattern detection
│   │   ├── clipboard_fallback.rs          # Ctrl+C fallback capture
│   │   └── mod.rs
│   ├── editing/
│   │   ├── text_service.rs                # Replace selection flow
│   │   ├── clipboard.rs                   # Clipboard helpers
│   │   └── mod.rs
│   └── ui/
│       ├── mod.rs
│       ├── popup.rs                       # WebView runtime/state coordinator
│       ├── webview_popup.rs               # Popup WebView thread (tao+wry)
│       ├── webview_settings.rs            # Settings WebView thread
│       ├── webview_bridge.rs              # Popup/settings state -> JSON bridge
│       ├── webview_ipc.rs                 # Typed Rust<->JS IPC contracts
│       ├── popup_position.rs              # Popup placement utilities
│       ├── window_resize_tracker.rs       # Tells a user frame-drag apart from an app-initiated resize
│       ├── settings_views.rs              # Settings form model + HTTPS validation
│       └── tray.rs                        # System tray menu/events
├── assets/
│   └── popup/
│       ├── index.html                     # Popup UI markup
│       ├── tokens.css                     # Design tokens (colors, spacing, shadows)
│       ├── base.css                       # Global reset + root variables (system font stack)
│       ├── components.css                 # Shared component library (buttons, textareas, status)
│       ├── popup.css                      # Popup-specific styling
│       ├── popup.js                       # Popup client logic + IPC + roving tabindex
│       ├── settings.html                  # Settings UI markup
│       ├── settings.css                   # Settings-specific styling
│       └── settings.js                    # Settings client logic + IPC posts
├── resources/
├── scripts/
├── installer/
└── docs/
```

## UI Module Summary (`src/ui/`)

| File | Purpose |
|---|---|
| `popup.rs` | Thin re-export hub (3 LOC) — delegates to focused submodules below. |
| `popup_ai.rs` | AI request orchestration (126 LOC) — tool instructions, max-token selection, request launch. |
| `popup_runtime.rs` | `run()` entry, async event loop, `AppState`, Win32 message pump (222 LOC). |
| `popup_view.rs` | Show/dismiss/push popup state, height calculation (83 LOC). |
| `popup_controller.rs` | Event handlers, IPC dispatch, state transitions (741 LOC). |
| `webview_popup.rs` | Shared WebView UI host on one `tao` EventLoop thread for both popup and settings windows; handles popup show/hide/resize/move/set-state commands and forwards JS IPC to Rust. Feeds every `WindowEvent::Resized` through `window_resize_tracker` to tell a user drag apart from the app's own `set_inner_size`. |
| `window_resize_tracker.rs` | Distinguishes a user's frame-drag from an app-initiated resize by remembering the size the app last requested; everything else is the user's. |
| `webview_settings.rs` | Settings command model (`SettingsCommand`) used by shared WebView host; drives settings show/hide/test/save feedback updates. |
| `webview_bridge.rs` | Converts `PopupState` + settings form into sanitized JSON payloads for JS renderers, including `canReplace` state and theme preference. Carries no height: the WebView measures its own content and asks for a size via `ResizePopup`. |
| `webview_ipc.rs` | Canonical IPC enums and serde boundaries; includes WebView2 runtime preflight helper, unified popup actions, and `ResizePopup { height }`/`PopupDragStart`/`PopupDragMove` contracts (drag carries no coordinates; the OS cursor is tracked instead). |
| `settings_views.rs` | Mutable `SettingsForm` model + HTTPS enforcement + persistence apply routine. |
| `popup_position.rs` | Calculates popup anchor placement and content-driven sizing constants (`POPUP_WIDTH`, `POPUP_INITIAL_HEIGHT`, `POPUP_MIN_HEIGHT`). |
| `tray.rs` | Tray icon/menu creation and menu event mapping to `AppEvent`. |

## Runtime Patterns

- **Event-driven core**: `AppEvent` channel drives runtime transitions.
- **Universal editor state flow**: `PopupState` now centers on `Editor`, `Processing`, `Result`, and `Error`; JS strictly mirrors Rust state.
- **Shared WebView UI host model**: one `tao` event-loop thread hosts both popup and settings windows; command-forwarder threads route popup/settings channels into that loop.
- **Unified IPC/action model**: JSON messages use `{ type, payload }` and route actions like `RunTool`, `CancelProcessing`, `UseOutputAsInput`, `ReplaceResult`, `CopyResult`, and `Retry` into `JsToRust`.
- **Replace gating**: replace is only enabled in `Result` when `canReplace=true`; editor-level replace is disabled.
- **Result action set**: Back to editor, Re-run, Use output as input, Replace, and Copy (with inline feedback).
- **Safe output rendering**: bridge escapes AI output before injecting into JS state payloads.
- **Tiered selection detection**: UIA text -> UIA value -> clipboard fallback.
- **Tiered editing execution**: UIA SetValue -> clipboard paste fallback.
- **Manual-resize policy**: a user's frame-drag wins for that popup session (`.popup-fill-window` mode, `window.setManualWindowSize(true)`); content-driven auto-sizing resumes on the next hide→show. See [`docs/decisions/0001-manual-resize-wins-for-session.md`](decisions/0001-manual-resize-wins-for-session.md).

## CSS Architecture (Phase 2)

Three-layer CSS organization for maintainability and reusability:

1. **tokens.css** — Design tokens (colors, spacing, shadows, typography scale)
2. **base.css** — Global reset, root variables (system font stack, no `@font-face`)
3. **components.css** — Reusable component library (buttons, textareas, status dots, badges)
4. **popup.css** / **settings.css** — App-specific styles (views, layout, animations)

All HTML files load tokens → base → components, then app-specific CSS for overrides.

## Current Notes

- `src/ui/popup.rs` has been split into 5 focused modules: `popup.rs` (re-export), `popup_ai.rs`, `popup_runtime.rs`, `popup_view.rs`, `popup_controller.rs`.
- Build/lint complete with warnings (dead code and minor clippy suggestions) but no hard failures.
- Dead files (popup_ai, popup_views, popup_window, theme) and iced dependency removed in cleanup.
- Shared `reqwest::Client` via `Arc` used across AI providers.
- Universal editor popup refactor consolidated tool UX under one flow (`Editor`/`Processing`/`Result`/`Error`) with shared action handling.
- Replace behavior is intentionally gated by `canReplace` in result state; copy/rerun paths remain available when replace is blocked.
- Result view includes iterative workflow support via `UseOutputAsInput` and re-run actions.
- Phase 2: Keyboard navigation (roving tabindex), aria-live attributes, cancel processing flow implemented.

## Reliability Improvements (Hotkey Race Prevention)

- **RT-2 (popup_visible guard)**: Manual capture results are ignored if popup is already visible, preventing stale async results from reopening dismissed popups.
- **RT-4 (strict text requirement)**: Hotkey path rejects bounds-only captures (e.g., UIA caret position with empty text); tray "Open Popup" still allows empty results.
- **RT-5 (clipboard restore logging)**: `ClipboardRestoreGuard` logs warnings on restore failure. Non-text clipboard formats are not backed up; only plain text is restored.
- **Blocking I/O safety**: Clipboard capture (with ~1.88s max blocking I/O) now runs via `tokio::task::spawn_blocking()` to prevent blocking worker threads during manual hotkey trigger.
