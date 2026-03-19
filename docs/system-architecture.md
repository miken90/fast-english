# Fast English — System Architecture

## High-Level Component Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Fast English (Windows)                            │
│                                                                              │
│  ┌───────────────┐     ┌─────────────────────────┐    ┌──────────────────┐ │
│  │ main.rs       │     │ app.rs                  │    │ settings.rs      │ │
│  │ COM STA init  │────▶│ PopupState + AppEvent   │◀──▶│ persistence.rs   │ │
│  │ runtime setup │     │ tool + translate models │    │ JSON + DPAPI     │ │
│  └───────┬───────┘     └─────────────┬───────────┘    └──────────────────┘ │
│          │                           │                                      │
│          │                  tokio::mpsc AppEvent channel                    │
│          │                           │                                      │
│  ┌───────▼────────┐         ┌────────▼─────────┐         ┌───────────────┐ │
│  │ selection/*     │         │ ui/popup.rs      │         │ ui/tray.rs     │ │
│  │ UIA polling     │────────▶│ WebView runtime   │◀────────│ tray + menu    │ │
│  │ 350ms + debounce│         │ state orchestration│        │ + hotkey flow  │ │
│  └────────────────┘         └───────┬──────────┘         └───────────────┘ │
│                                      │                                        │
│                   ┌──────────────────┴──────────────────┐                     │
│                   │ Rust↔JS bridge (IPC + JSON state)   │                     │
│                   │ webview_bridge.rs + webview_ipc.rs  │                     │
│                   └───────────────┬──────────────────────┘                     │
│                                   │                                            │
│      ┌────────────────────────────┴─────────────────────────────┐              │
│      │ Dedicated tao EventLoop threads (wry WebView)            │              │
│      │ - webview_popup.rs    (floating popup)                   │              │
│      │ - webview_settings.rs (settings window)                  │              │
│      └────────────────────────────┬──────────────────────────────┘              │
│                                   │                                            │
│                         assets/popup/*.html|css|js                             │
│                                                                              │
│  ┌─────────────────────┐                 ┌──────────────────────────────────┐ │
│  │ ai/*                │                 │ editing/*                         │ │
│  │ OpenAI/Anthropic    │────────────────▶│ replace via UIA SetValue         │ │
│  │ prompt cleaning     │                 │ fallback clipboard paste          │ │
│  └─────────────────────┘                 └──────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### Core

| Component | Role |
|---|---|
| `main.rs` | Entry point. COM STA init, settings load, tokio runtime, monitor/tray/hotkey wiring, starts WebView popup runtime. |
| `app.rs` | Shared domain types: `PopupState`, `AppEvent`, `WritingToolKind`, translate constants. |
| `settings.rs` | App settings model (`AppSettings`, provider fields, toggles). |
| `persistence.rs` | Persist settings in `%APPDATA%/FastEnglish/`; API keys protected with DPAPI. |
| `hotkey.rs` | Registers global `Ctrl+Alt+T`, emits `AppEvent::HotkeyPressed`. |

### UI (WebView2)

| Component | Role |
|---|---|
| `ui/popup.rs` | Runtime coordinator. Consumes `AppEvent`, updates `PopupState`, triggers AI/editing, sends commands to WebView threads, handles IPC callbacks from JS. Hotkey toggles popup + supports empty selection. |
| `ui/webview_popup.rs` | Popup window thread (`tao` EventLoop + `wry` WebView), receives show/hide/resize/state commands, forwards JS events to Rust. Includes `icons.js` for SVG icon rendering. |
| `ui/webview_settings.rs` | Settings window thread (`tao` EventLoop + `wry` WebView), handles settings rendering and JS actions. Includes `icons.js`. Passes theme preference from settings state. |
| `ui/webview_bridge.rs` | Serializes Rust popup/settings state to JSON payloads for JS `setState()/setSettings()`, applies HTML escaping for rendered output. Includes theme preference in state. |
| `ui/webview_ipc.rs` | Typed IPC contracts (`JsToRust`, `RustToJs`), JSON serde format, WebView2 runtime preflight check. |
| `ui/settings_views.rs` | Rust-side `SettingsForm` model + HTTPS enforcement + apply/persist logic. Includes theme preference field. |
| `ui/popup_position.rs` | Popup placement + state-dependent window heights. |
| `assets/popup/icons.js` | Centralized SVG icon module with `renderIcon(name, size)` factory. Manages all tool, action, and status icons. |
| `assets/popup/tokens.css` | Design tokens (colors, spacing, shadows, typography). Includes light/dark theme overrides via `:root[data-theme="light"]`. |
| `assets/popup/components.css` | Reusable component library (buttons, textareas, status indicators, labels, feedback toasts). |
| `assets/popup/popup.css` | Popup-specific layouts, animations, Fluent Midnight visual refresh (backdrop blur, shadows). |
| `assets/popup/settings.css` | Settings window layouts and styling. |
| `assets/popup/*.html` | HTML markup with theme toggle (settings), confirmation dialog (popup), version display. |
| `assets/popup/popup.js` | UI orchestration, state management via `appState` object, IPC communication, keyboard nav, resize debouncing, replace confirmation, micro-interactions. |
| `assets/popup/settings.js` | Settings form handling, theme toggle, cancel/reset buttons, icon rendering. |

### AI, Selection, Editing

| Area | Key Modules | Role |
|---|---|---|
| AI | `ai/mod.rs`, `ai/client.rs`, `ai/openai.rs`, `ai/anthropic.rs`, `ai/prompt.rs` | Provider abstraction (shared `reqwest::Client`), API calls, prompt/output handling. `create_ai_client()` factory in `mod.rs`. |
| Selection | `selection/monitor.rs`, `uia_text.rs`, `uia_value.rs`, `clipboard_fallback.rs` | Detect selected text (UIA tiers + clipboard fallback) and emit context events. |
| Editing | `editing/text_service.rs`, `editing/clipboard.rs` | Replace selected text (UIA SetValue first, clipboard paste fallback). |

## State Machine

`PopupState` remains the source of truth for popup behavior:

- `Hidden`
- `Options(context)`
- `TranslateSelection(context)`
- `Processing { context, tool }`
- `Result { context, tool, output }`
- `TranslateResult { context }`
- `Error { context, message }`

WebView rendering is a projection of `PopupState`, not a separate business-state machine.

## IPC Message Flow (Rust ↔ JS)

### Message shape

```json
{
  "type": "ToolSelected",
  "payload": { "kind": "enhance-writing" }
}
```

- Rust receives JS events via `window.ipc.postMessage(...)` and deserializes into `JsToRust`.
- Rust sends UI updates by evaluating scripts like `setState(<json>)`, where JSON comes from `webview_bridge`.

### Typical interaction flow

1. Selection monitor emits `AppEvent::TextSelected(context)`.
2. `ui/popup.rs` updates `PopupState::Options(context)` and sends `WebViewPopupCommand::Show + SetState`.
3. JS user action (for example `ToolSelected`) posts IPC to Rust.
4. Rust transitions to `Processing`, executes AI request, then transitions to `Result`/`Error`.
5. Rust sends fresh `SetState` JSON; JS re-renders view.
6. Replace/copy actions route back through Rust editing/clipboard services.

## Threading Model

- **Main thread (Windows STA)**
  - COM initialization/uninitialization.
  - Startup wiring (settings, monitor spawn, tray, hotkey).
  - Runs `ui::popup::run(...)` lifecycle.

- **Tokio runtime (multi-thread)**
  - Selection polling loop.
  - AI network requests.
  - Async internal event fan-in.

- **WebView UI threads (blocking tao loops)**
  - `webview-popup` thread: popup `tao::EventLoop<WebViewPopupCommand>`.
  - `webview-settings` thread: settings `tao::EventLoop<SettingsCommand>`.
  - Each has a command-forwarder thread (`std::mpsc` receiver → `EventLoopProxy`).

- **Cross-thread bridges**
  - `tokio::mpsc` for app/runtime events.
  - `std::mpsc` for command delivery into tao loops.
  - WebView IPC callbacks (`JsToRust`) re-enter tokio side through unbounded channel.

## Security Considerations

- API keys are stored only in DPAPI-protected storage; not persisted in WebView assets.
- User-facing text injected into HTML contexts is escaped in `webview_bridge::escape_html`.
- IPC payloads are structured typed messages with serde deserialization boundaries.
- HTTPS enforcement: provider base URLs must use `https://`; validated before settings are persisted.
- Theme preference (auto/dark/light) stored as plain text in JSON settings; no security impact.
- No telemetry/analytics pipeline is introduced by WebView cutover.

## Theme System (Phase 3)

- **Settings field**: `theme: String` ("auto", "dark", "light") persisted in JSON config
- **JS-side**: `document.documentElement.dataset.theme` set on init from system preference or saved setting
- **CSS**: `:root[data-theme="light"]` overrides all color tokens for light theme support
- **Updates**: Theme preference IPC sent from settings window to popup window on change
- **Tokens**: Two-variant system in `tokens.css` — dark defaults, light overrides via selector
