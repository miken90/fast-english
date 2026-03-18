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
│   │   ├── openai.rs                      # OpenAI client
│   │   ├── anthropic.rs                   # Anthropic client
│   │   ├── prompt.rs                      # Prompt/output normalization
│   │   └── mod.rs
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
│       ├── settings_views.rs              # Rust settings form model/apply logic
│       ├── tray.rs                        # System tray menu/events
│       ├── popup_ai.rs                    # Shared AI helpers used by popup runtime
│       ├── popup_views.rs                 # Deprecated iced popup stub
│       ├── popup_window.rs                # Deprecated iced window stub
│       └── theme.rs                       # Legacy theme constants (currently unused)
├── assets/
│   └── popup/
│       ├── index.html                     # Popup UI markup
│       ├── popup.css                      # Popup styling
│       ├── popup.js                       # Popup client logic + IPC posts
│       ├── settings.html                  # Settings UI markup
│       ├── settings.css                   # Settings styling
│       └── settings.js                    # Settings client logic + IPC posts
├── resources/
├── scripts/
├── installer/
└── docs/
```

## UI Module Summary (`src/ui/`)

| File | Purpose |
|---|---|
| `popup.rs` | Central runtime loop: consumes `AppEvent`, handles `JsToRust` IPC, orchestrates AI calls, copy/replace flows, and pushes state to WebViews. |
| `webview_popup.rs` | Popup window lifecycle on dedicated thread; show/hide/resize/set-state command handling. |
| `webview_settings.rs` | Settings window lifecycle on dedicated thread; settings/test/save feedback command handling. |
| `webview_bridge.rs` | Converts `PopupState` + settings form into sanitized JSON payloads for JS renderers. |
| `webview_ipc.rs` | Canonical IPC enums and serde boundaries; includes WebView2 runtime preflight helper. |
| `settings_views.rs` | Mutable `SettingsForm` model + persistence apply routine reused by WebView settings flow. |
| `popup_position.rs` | Calculates popup position and state-dependent heights. |
| `tray.rs` | Tray icon/menu creation and menu event mapping to `AppEvent`. |
| `popup_ai.rs` | Provider client creation + tool instruction helpers used by popup runtime. |
| `popup_views.rs` / `popup_window.rs` | Deprecated iced modules retained as compatibility stubs. |

## Runtime Patterns

- **Event-driven core**: `AppEvent` channel drives runtime transitions.
- **Popup state authority**: `PopupState` remains single source of truth; JS mirrors Rust state.
- **Two-thread WebView UI model**: popup and settings each run on dedicated `tao` event-loop threads.
- **Typed IPC contracts**: JSON messages use `{ type, payload }` and deserialize into `JsToRust`.
- **Safe output rendering**: bridge escapes AI output before injecting into JS state payloads.
- **Tiered selection detection**: UIA text -> UIA value -> clipboard fallback.
- **Tiered editing execution**: UIA SetValue -> clipboard paste fallback.

## Current Notes

- `src/ui/popup.rs` is large and a future modularization candidate.
- Build/lint currently complete with warnings (dead code and minor clippy suggestions) but no hard failures.
