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
| `popup.rs` | Central runtime loop: consumes `AppEvent`, handles `JsToRust` IPC, orchestrates AI calls (shared HTTP client), copy/replace flows, and pushes state to WebViews. Also handles hotkey toggle + empty selection fallback. |
| `webview_popup.rs` | Popup window lifecycle on dedicated thread; show/hide/resize/set-state command handling. |
| `webview_settings.rs` | Settings window lifecycle on dedicated thread; settings/test/save feedback command handling. |
| `webview_bridge.rs` | Converts `PopupState` + settings form into sanitized JSON payloads for JS renderers. |
| `webview_ipc.rs` | Canonical IPC enums and serde boundaries; includes WebView2 runtime preflight helper. |
| `settings_views.rs` | Mutable `SettingsForm` model + HTTPS enforcement + persistence apply routine. |
| `popup_position.rs` | Calculates popup position and state-dependent heights. |
| `tray.rs` | Tray icon/menu creation and menu event mapping to `AppEvent`. |

## Runtime Patterns

- **Event-driven core**: `AppEvent` channel drives runtime transitions.
- **Popup state authority**: `PopupState` remains single source of truth; JS mirrors Rust state.
- **Two-thread WebView UI model**: popup and settings each run on dedicated `tao` event-loop threads.
- **Typed IPC contracts**: JSON messages use `{ type, payload }` and deserialize into `JsToRust`.
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
- Phase 2: Keyboard navigation (roving tabindex), aria-live attributes, cancel processing flow implemented.
