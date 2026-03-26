# Fast English — Codebase Summary

## Overview

- **Language**: Rust (2021 edition)
- **Platform**: Windows 10+
- **UI Stack**: WebView2 via `wry` + `tao` (WebView popup + settings windows)
- **Build system**: Cargo
- **Key dependencies**: `windows`, `uiautomation`, `wry`, `tao`, `tray-icon`, `global-hotkey`, `reqwest`, `tokio`
- **Tests**: limited module-level tests (for IPC serde); no broad integration/E2E suite yet

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
│       ├── settings_views.rs              # Settings form model + HTTPS validation
│       └── tray.rs                        # System tray menu/events
├── assets/
│   └── popup/
│       ├── index.html                     # Popup UI markup
│       ├── tokens.css                     # Design tokens (colors, spacing, shadows)
│       ├── base.css                       # Global reset + font faces
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
| `popup.rs` | Central runtime loop: consumes `AppEvent`, handles `JsToRust` IPC, orchestrates AI calls (shared HTTP client), and pushes universal popup states to WebViews (`Editor` → `Processing` → `Result`/`Error`). Also handles hotkey toggle + empty selection fallback and mode-based snap resize handling. |
| `webview_popup.rs` | Shared WebView UI host on one `tao` EventLoop thread for both popup and settings windows; handles popup show/hide/resize/move/set-state commands and forwards JS IPC to Rust. |
| `webview_settings.rs` | Settings command model (`SettingsCommand`) used by shared WebView host; drives settings show/hide/test/save feedback updates. |
| `webview_bridge.rs` | Converts `PopupState` + settings form into sanitized JSON payloads for JS renderers, including `canReplace` state and popup min-height hints by state/tool. |
| `webview_ipc.rs` | Canonical IPC enums and serde boundaries; includes WebView2 runtime preflight helper, unified popup actions, and `ResizePopup { height, mode }`/`MovePopup` contracts. |
| `settings_views.rs` | Mutable `SettingsForm` model + HTTPS enforcement + persistence apply routine. |
| `popup_position.rs` | Calculates popup anchor placement and snap height constants (`EDITOR_MIN_HEIGHT`, `EDITOR_TRANSLATE_HEIGHT`, `PROCESSING_MIN_HEIGHT`, `RESULT_MIN_HEIGHT`, `RESULT_EXPANDED_HEIGHT`, `ERROR_MIN_HEIGHT`). |
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

## CSS Architecture (Phase 2)

Three-layer CSS organization for maintainability and reusability:

1. **tokens.css** — Design tokens (colors, spacing, shadows, typography scale)
2. **base.css** — Global reset, font faces, root variables
3. **components.css** — Reusable component library (buttons, textareas, status dots, badges)
4. **popup.css** / **settings.css** — App-specific styles (views, layout, animations)

All HTML files load tokens → base → components, then app-specific CSS for overrides.

## Current Notes

- `src/ui/popup.rs` is large (~860 lines) and a future modularization candidate.
- Build/lint complete with warnings (dead code and minor clippy suggestions) but no hard failures.
- Dead files (popup_ai, popup_views, popup_window, theme) and iced dependency removed in cleanup.
- Shared `reqwest::Client` via `Arc` used across AI providers.
- Universal editor popup refactor consolidated tool UX under one flow (`Editor`/`Processing`/`Result`/`Error`) with shared action handling.
- Replace behavior is intentionally gated by `canReplace` in result state; copy/rerun paths remain available when replace is blocked.
- Result view includes iterative workflow support via `UseOutputAsInput` and re-run actions.
- Phase 2: Keyboard navigation (roving tabindex), aria-live attributes, cancel processing flow implemented.
