# Fast English — System Architecture

The canonical architecture document: current product, code, state, update, and
ownership boundaries. This repo predates the installed harness, so it keeps this
filename rather than the harness's generic `ARCHITECTURE.md`; there is no second
copy.

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
│      │ Shared tao EventLoop host (wry WebView)                  │              │
│      │ - webview_popup.rs    (UiCommand event loop host)        │              │
│      │ - webview_settings.rs (SettingsCommand contract)          │              │
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
| `persistence.rs` | Persist settings in `%APPDATA%/FastEnglish/`; API keys and `history.dat` (recent results) protected with DPAPI. |
| `history.rs` | Recent-result entry shape (tool, source, output, timestamp, translate pair) and caps: 20 entries, 8000 chars/field, re-applied on load. |
| `hotkey.rs` | Registers global `Ctrl+Alt+T`, emits `AppEvent::HotkeyPressed`. |

### UI (WebView2)

| Component | Role |
|---|---|
| `ui/popup.rs` | Runtime coordinator. Consumes `AppEvent`, updates `PopupState`, triggers AI/editing, sends commands to WebView threads, handles IPC callbacks from JS. Implements universal editor transitions (`Editor`/`Processing`/`Result`/`Error`) and action handling (`UseOutputAsInput`, `CancelProcessing`, `ReplaceResult`, `CopyResult`, `Retry`). |
| `ui/webview_popup.rs` | Unified WebView UI host: one `tao` EventLoop thread runs both popup and settings windows (`wry`), receives popup/settings commands, forwards JS events to Rust, and inlines CSS/JS assets (including `icons.js`). Routes `WindowEvent::Resized` through `window_resize_tracker` to tell a user drag apart from an app-initiated resize. |
| `ui/window_resize_tracker.rs` | Tells a user's frame-drag apart from the app's own `set_inner_size` calls — both surface as the same `WindowEvent::Resized`, so it remembers the size the app last requested and treats any mismatch as the user's. |
| `ui/webview_settings.rs` | Settings command contract (`SettingsCommand`) for the shared WebView host; handles settings rendering and feedback updates. |
| `ui/webview_bridge.rs` | Serializes Rust popup/settings state to JSON payloads for JS `setState()/setSettings()`, applies HTML escaping for rendered output, includes theme preference. Sends no height — sizing travels the other way, from the WebView's own measurement through `ResizePopup`. |
| `ui/webview_ipc.rs` | Typed IPC contracts (`JsToRust`, `RustToJs`), JSON serde format, WebView2 runtime preflight check. Includes `ResizePopup { height }` and move/translate actions used by universal editor flow. |
| `ui/settings_views.rs` | Rust-side `SettingsForm` model + HTTPS enforcement + apply/persist logic. Includes theme preference field. |
| `ui/popup_position.rs` | Popup placement + content-driven sizing constants (`POPUP_WIDTH`, `POPUP_INITIAL_HEIGHT`, `POPUP_MIN_HEIGHT`). |
| `assets/popup/icons.js` | Centralized SVG icon module with `renderIcon(name, size)` factory. Manages all tool, action, and status icons. |
| `assets/popup/tokens.css` | Design tokens (colors, spacing, shadows, typography). Includes light/dark theme overrides via `:root[data-theme="light"]`. |
| `assets/popup/components.css` | Reusable component library (buttons, textareas, status indicators, labels, feedback toasts). |
| `assets/popup/popup.css` | Universal popup layouts and action rows for editor/result/error states, vertically resizable textareas, animations, and Fluent Midnight visual refresh. |
| `assets/popup/settings.css` | Settings window layouts and styling. |
| `assets/popup/*.html` | HTML markup for the universal popup sections (`Editor`, `Processing`, `Result`, `Error`) plus settings theme/version UI. |
| `assets/popup/popup.js` | UI orchestration, `appState`-based rendering for universal views, unified IPC actions, textarea autosize + manual resize detection, resize debouncing, streamed-text appending, the Recent Results list view, and result actions (Back/Re-run/Use output as input/Replace/Copy). |
| `assets/popup/diff.js` | Word-level LCS diff, no dependency/build step: common prefix/suffix trim before the quadratic step, whitespace-preserving word split (keeps Vietnamese diacritics intact), and a `MAX_DIFF_TOKENS` (800/side) size guard that falls back to plain text with an inline note. Toggled per tool from `popup.js`. |
| `assets/popup/settings.js` | Settings form handling, theme toggle, cancel/reset buttons, icon rendering. |

### AI, Selection, Editing

| Area | Key Modules | Role |
|---|---|---|
| AI | `ai/mod.rs`, `ai/client.rs`, `ai/openai.rs`, `ai/prompt.rs`, `ai/sse.rs`, `ai/response.rs`, `ai/error.rs` | Provider abstraction (shared `reqwest::Client` built once by `build_http_client()`), streaming (`process_streaming`, default falls back to non-streaming) and non-streaming API calls, byte-level SSE decoding, script-density token budgeting, response validation (`response.rs`) and user-facing failure messages (`error.rs`). `create_ai_client()` factory in `mod.rs`. |
| Selection | `selection/monitor.rs`, `uia_text.rs`, `uia_value.rs`, `clipboard_fallback.rs` | Detect selected text (UIA tiers + clipboard fallback) and emit context events. |
| Editing | `editing/text_service.rs`, `editing/clipboard.rs` | Replace selected text (UIA SetValue first, clipboard paste fallback). |

## State Machine

`PopupState` remains the source of truth for popup behavior:

- `Hidden`
- `Editor { context, tool, source_text }`
- `Processing { context, tool, source_text }`
- `Result { context, tool, source_text, output, can_replace }`
- `Error { context, tool, source_text, message }`

WebView rendering is a projection of `PopupState`, not a separate business-state machine. The universal popup always moves through this shared editor pipeline (including Translate as a tool mode inside `Editor`).

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
2. `ui/popup.rs` sets `PopupState::Editor { ... }` and sends `WebViewPopupCommand::Show + SetState`.
3. JS actions (`ToolSelected`, `UpdateSourceText`, `RunTool`) post IPC to Rust.
4. Rust transitions to `Processing`, executes AI request, then transitions to `Result`/`Error`.
5. Rust sends fresh `SetState` JSON; JS re-renders the active universal view.
6. Result/Error actions route through unified IPC: `BackToEditor`, `UseOutputAsInput`, `ReplaceResult`, `CopyResult`, `Retry`, `CancelProcessing`. `OpenHistory`/`OpenHistoryEntry`/`ClearHistory` route Recent Results independently of the current popup state.
7. Replace is gated by `can_replace`; when false, `ReplaceResult` stays disabled and copy/rerun flows remain available.

## Threading Model

- **Main thread (Windows STA)**
  - COM initialization/uninitialization.
  - Startup wiring (settings, monitor spawn, tray, hotkey).
  - Runs `ui::popup::run(...)` lifecycle.

- **Tokio runtime (multi-thread)**
  - Selection polling loop.
  - AI network requests.
  - Async internal event fan-in.

- **WebView UI thread (blocking tao loop)**
  - One `webview-ui` thread runs `tao::EventLoop<UiCommand>` and hosts both popup and settings windows.
  - Two command-forwarder threads bridge `std::mpsc` receivers into the same `EventLoopProxy` (`UiCommand::Popup` / `UiCommand::Settings`).

- **Cross-thread bridges**
  - `tokio::mpsc` for app/runtime events.
  - `std::mpsc` for command delivery into tao loops.
  - WebView IPC callbacks (`JsToRust`) re-enter tokio side through unbounded channel.

## Security Considerations

- API keys and recent-result history (`history.dat`) are stored only in DPAPI-protected storage, scoped to the Windows account; not persisted in WebView assets. DPAPI protects against another account, a stolen disk, or a backup — not against other code running as the same user.
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

## Reliability Patterns

### Manual Capture Race Prevention (RT-2 & RT-4)
- **Concurrent capture guard**: `manual_capture_in_flight` bool prevents overlapping manual capture requests (hotkey or tray "Open Popup")
- **Stale result rejection**: Manual capture results are ignored if `popup_visible=true`, preventing dismissed popups from being reopened by delayed async results
- **Strict hotkey filtering**: Hotkey path requires actual text content; bounds-only captures (empty UIA caret position) are rejected via `should_open_manual_capture_result(false)`. Tray "Open Popup" allows empty results via `allow_empty=true`.

### Manual Resize Handoff
- **Policy**: a user's frame-drag wins for that popup session; content-driven auto-sizing resumes only on the next hide→show. Recorded as a decision in [`docs/decisions/0001-manual-resize-wins-for-session.md`](decisions/0001-manual-resize-wins-for-session.md).
- **Ambiguity**: a monitor DPI/scale change also raises `WindowEvent::Resized`, so `window_resize_tracker` reads it as a user drag. Self-healing on the next show; not distinguished from a real drag.

### Failure Reporting
- **Every failure is a sentence, never a status code**: `ai/error.rs` classifies transport failures (unreachable host, DNS, connect timeout, stall, connection dropped mid-answer) and HTTP statuses (401/403 key, 404 base URL or model, 429 with `Retry-After`, 4xx context-length, 5xx) into messages that name the cause and the setting to change. HTML gateway pages and oversized bodies are described, never pasted.
- **A success status is not a success**: `ai/response.rs` rejects a 200 carrying an `error` object, an empty `choices` array, and blank content.
- **Incomplete is not the same as failed**: an answer that hit the model's token limit (`finish_reason: length`), or a stream that broke after at least `MIN_SALVAGEABLE_CHARS` (20) characters had arrived, is shown as a normal `Result` carrying a `notice` — the text is what the user asked for and is theirs to keep. The result view renders the notice as a warning above the output so a cut-short answer is never pasted unknowingly. A stream that dies with nothing, or with only a fragment, stays an error: there is nothing to show instead of the reason.
- **Empty input never becomes a request**: `start_ai_request` fails whitespace-only source text locally.
- **Timeout policy**: `build_http_client()` sets a connect timeout (10s) and a per-read timeout (60s), and deliberately no overall request timeout — an overall one caps how long a legitimate long completion may take, which is what the previous 30s `timeout()` did. A stalled proxy now fails with a message; a slow-but-alive stream runs to completion.
- **Connection prewarm**: showing the popup warms a pooled connection to the configured endpoint (at most once per `PREWARM_INTERVAL`), so the first run of a session does not pay for the TLS handshake on the critical path. `pool_idle_timeout` is 300s so later runs reuse it.

### Streaming Request Safety
- **Request-id guard**: `accepts_stream_chunk`/`accepts_response` reject a chunk or final response whose request id no longer matches the active one, so a stale stream from a superseded or cancelled request cannot paint into a live view.
- **Cancel aborts the task**: `abort_active_request()` runs before a new request id is issued, so cancelling stops the HTTP body being read instead of leaving a task that writes into a dead view.
- **One streaming surface**: streamed text is painted only by the Processing view (`#processing-stream-output`). The editor view has no output region — running a tool always moves to the Processing screen, which is what makes first-token text visible without a second box that is empty whenever nothing is streaming.
- **Manual-resize interaction**: every sizing path checks `_manualWindowSize` before resizing, so a stream growing the result region never resizes a window the user has already dragged.
- **History gating**: `records_history` only accepts a completed, successful response tied to the still-active request id — never a chunk, and never a cancelled or failed request.

### Clipboard Capture Thread Safety (RT-5)
- **Blocking I/O isolation**: Clipboard capture (includes ~1.88s max polling with `std::thread::sleep`) runs in `tokio::task::spawn_blocking()` to avoid blocking Tokio worker threads
- **Format loss documentation**: `ClipboardRestoreGuard` backs up and restores only plain text; non-text formats (images, rich text, HTML) are not preserved across clipboard intercept
- **Restore failure logging**: Guard logs `warn!` if clipboard text restoration fails, preventing silent data loss
