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
| `app.rs` | Shared domain types: `PopupState`, `AppEvent`, `WritingToolKind`, tone/translate constants. |
| `settings.rs` | App settings model (`AppSettings`, provider fields, toggles). |
| `persistence.rs` | Persist settings in `%APPDATA%/FastEnglish/`; API keys protected with DPAPI. |
| `hotkey.rs` | Registers global `Ctrl+Alt+T`, emits `AppEvent::HotkeyPressed`. |

### UI (WebView2)

| Component | Role |
|---|---|
| `ui/popup.rs` | Runtime coordinator. Consumes `AppEvent`, updates `PopupState`, triggers AI/editing, sends commands to WebView threads, handles IPC callbacks from JS. |
| `ui/webview_popup.rs` | Popup window thread (`tao` EventLoop + `wry` WebView), receives show/hide/resize/state commands, forwards JS events to Rust. |
| `ui/webview_settings.rs` | Settings window thread (`tao` EventLoop + `wry` WebView), handles settings rendering and JS actions. |
| `ui/webview_bridge.rs` | Serializes Rust popup/settings state to JSON payloads for JS `setState()/setSettings()`, applies HTML escaping for rendered output. |
| `ui/webview_ipc.rs` | Typed IPC contracts (`JsToRust`, `RustToJs`), JSON serde format, WebView2 runtime preflight check. |
| `ui/settings_views.rs` | Rust-side `SettingsForm` model + apply/persist logic used by WebView settings flow. |
| `assets/popup/*` | Offline popup/settings frontend assets (HTML/CSS/JS) inlined into WebView at runtime. |
| `ui/popup_views.rs`, `ui/popup_window.rs` | Deprecated iced UI stubs set for removal. |

### AI, Selection, Editing

| Area | Key Modules | Role |
|---|---|---|
| AI | `ai/client.rs`, `ai/openai.rs`, `ai/anthropic.rs`, `ai/prompt.rs` | Provider abstraction, API calls, prompt/output handling. |
| Selection | `selection/monitor.rs`, `uia_text.rs`, `uia_value.rs`, `clipboard_fallback.rs` | Detect selected text (UIA tiers + clipboard fallback) and emit context events. |
| Editing | `editing/text_service.rs`, `editing/clipboard.rs` | Replace selected text (UIA SetValue first, clipboard paste fallback). |

## State Machine

`PopupState` remains the source of truth for popup behavior:

- `Hidden`
- `Options(context)`
- `ToneSelection(context)`
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
- All provider traffic remains HTTPS to configured endpoints.
- No telemetry/analytics pipeline is introduced by WebView cutover.
