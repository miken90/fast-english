# Fast English — Project Roadmap

## Phase 1: v1.0 — Core MVP

**Status**: Complete

### Delivered Features
- [x] System tray Windows app
- [x] Text selection detection via UI Automation APIs (polling every 350ms)
- [x] Multi-tier selection: UIA Text Pattern → UIA Value Pattern → clipboard fallback
- [x] Selection debounce (700ms stable selection before trigger)
- [x] Floating popup near selected text (WebView2 UI)
- [x] Global hotkey `Ctrl+Alt+T` to trigger popup (fresh clipboard capture)
- [x] Four writing tools: Enhance Writing, Fix Grammar, Convert Prompt, Translate
- [x] Translate feature supporting multiple source and target languages
- [x] OpenAI integration (gpt-4.1-mini)
- [x] Anthropic integration (claude-3-5-sonnet-latest)
- [x] In-place text replacement via UIA SetValue + clipboard fallback
- [x] Settings UI: provider config, API key, custom prompts
- [x] Connection testing for AI providers
- [x] DPAPI-encrypted API key storage
- [x] Copy feedback with 1.5s auto-dismiss
- [x] Auto-start with Windows (Registry HKCU Run key)
- [x] Portable exe + Inno Setup installer
- [x] Architecture migration from iced to WebView2 (wry/tao)
- [x] Refreshed app icon, branding, and tray icons

### Known Limitations
- Module-level unit tests only; no broad integration/E2E suite
- Popup positioning can be imprecise in some apps

---

## Phase 2: Core UX & Component System

**Status**: Complete (2026-03-19)

### Delivered Features
- [x] Shared CSS component library (`components.css`) — reusable buttons, textareas, status indicators
- [x] Design token centralization (`tokens.css`) — unified color, spacing, shadow definitions
- [x] Three-layer CSS architecture (tokens → base → components → app-specific)
- [x] Keyboard navigation — roving tabindex pattern for tool list
- [x] Accessibility enhancements — aria-live for feedback toasts, processing label, status badges
- [x] Cancel processing flow — `CancelProcessing` IPC message + UI feedback
- [x] Version injection via `CARGO_PKG_VERSION` environment variable
- [x] Resolved CSS token duplication and established single-source-of-truth design system

### Known Limitations
- No unit or integration tests (defer to Phase 3)
- Light theme support planned for v1.1+
- Fluent Midnight brand refresh proposal pending approval

---

## Phase 3: Theme & Polish

**Status**: Complete (2026-03-19)

### Delivered Features
- [x] Light/dark theme system with Auto/Dark/Light toggle (persisted in settings)
- [x] Fluent Midnight visual refresh — backdrop blur, new shadows, accent colors
- [x] Centralized SVG icon module (`icons.js`) — all icons managed via `renderIcon(name, size)`
- [x] Lightning Pen brand icon (tray, taskbar, window)
- [x] Replace Selection confirmation dialog — shows inline bar before replacement
- [x] Settings cancel/reset — Cancel dismisses without saving, Reset restores defaults
- [x] JS state encapsulation — removed global mutable variables, all state in `appState` object
- [x] Universal editor popup refactor — unified four-state flow (`Editor` → `Processing` → `Result`/`Error`) for all tools
- [x] Unified popup IPC/action model — consistent actions (`RunTool`, `CancelProcessing`, `UseOutputAsInput`, `ReplaceResult`, `CopyResult`, `Retry`, language changes)
- [x] Snap sizing contract — mode-based popup heights (`editor`, `editor-translate`, `processing`, `result-compact`, `result-expanded`, `error`) aligned between JS and Rust
- [x] Replace gating update — `Replace` is enabled only in `Result` when `canReplace=true`; editor-level replace is intentionally disabled
- [x] Result view expansion — rich/markdown preview toggle and source snippet collapse/expand with adaptive snap resize
- [x] Result actions expansion — Back, Re-run, Use output as input, Replace, and Copy with inline feedback
- [x] Micro-interactions — button scale on active, smooth swap rotation, copy feedback animation
- [x] Light theme WCAG AA contrast compliance verified

### Known Limitations
- No unit or integration tests (defer to Phase 4)
- backdrop-filter may impact perf on very low-end hardware (acceptable trade-off)

---

## Phase 4: Quality & Reliability

**Status**: Largely delivered (v0.5.2, 2026-08-20). 129 automated tests, up
from none when this phase was written.

### Goals
- [x] Add unit tests for core logic (state transitions, prompt builder, AI clients)
- [x] Structured error types — `src/ai/error.rs` classifies every failure
  (DNS, refused connection, connect timeout, stalled stream, 401/403, 404,
  429 with `Retry-After`, 5xx, non-JSON body, gateway HTML, empty body, HTTP
  200 carrying an error object, blank content) into a message naming the cause
  and the setting to change. `src/ai/response.rs` decides whether a response is
  an answer at all.
- [x] Add structured logging for debugging — `RUST_LOG=fast_english=debug` logs
  response-header latency, `first_token_ms`, character count, and
  `finish_reason` per request.
- [x] Handle edge cases: empty selections fire no request; a truncated or
  late-broken answer is kept with a warning instead of discarded.
- [x] Improve popup positioning accuracy — per-monitor physical-pixel
  placement (`src/ui/popup_monitor.rs`), content-derived sizing.
- [ ] Add integration tests for selection detection and text replacement —
  still absent; both paths touch the live Windows shell.

---

## Phase 5: Enhanced UX

**Status**: In Progress (streaming, diff view, and history delivered 2026-08-17,
v0.5.0; flow, latency, failure handling, and prompts reworked 2026-08-20, v0.5.2)

### Delivered Features (v0.5.0)
- [x] Streaming AI responses — text appears as the model produces it (`src/ai/sse.rs`)
- [x] Diff view for Fix Grammar (on by default) and Enhance Writing (off by
  default) — word-level LCS highlighting in `assets/popup/diff.js`; not
  offered for Translate or Convert Prompt
- [x] Recent Results — last 20 completed results, DPAPI-encrypted at rest
  (`src/history.rs`), reachable from the tray and the popup header

### Remaining Goals
- [ ] Undo support — revert to original text
- [ ] Multiple result variants
- [ ] Custom keyboard shortcuts per tool

### Delivered Features (v0.5.2)
- [x] Dead editor Output card removed — Run goes to the Processing view, which
  owns the streaming surface
- [x] Failure classification with actionable messages (see Phase 4)
- [x] Connect (10s) and per-read (60s) timeouts replacing a single 30s overall
  timeout that had been silently killing any answer longer than 30 seconds
- [x] TLS connection prewarm on popup open, so the first run of a session does
  not pay the handshake on the critical path
- [x] Every tab's prompt rewritten so input is treated as data — a selected
  question is edited or translated, never answered — and Enhance and Fix
  Grammar answer in the input's own language

### Known Limitations (v0.5.2)
- No automated test talks to a live provider. All 129 tests run against a local
  server. Deliberate: a live test would spend real quota on every run
  (decision 2026-08-20). The 0.5.2 popup flow was verified by hand on a test
  build before release; the diff view and history list still have not been
  confirmed rendering.
- `read_timeout` is 60s. A proxy that buffers a long completion and sends
  nothing for over a minute fails with "stopped sending data". Left as-is —
  no evidence yet that it is too tight. Constant in `src/ai/mod.rs`.
- The Inno Setup installer has still never been compiled; the portable zip is
  the shipping format.
- See `docs/release-notes-0.5.2.md`.

---

## Phase 6: More AI Providers & Tools

**Status**: Planned

### Goals
- [ ] Ollama integration for local/offline models
- [ ] Google Gemini integration
- [ ] Custom user-defined writing tools
- [ ] Per-app tool preferences
- [ ] Token usage tracking

---

## Phase 7: Distribution & Polish

**Status**: Planned

### Goals
- [ ] Code signing (Authenticode)
- [ ] Auto-update mechanism
- [ ] Website and landing page
- [ ] Proper app icon design
- [ ] Microsoft Store submission evaluation
