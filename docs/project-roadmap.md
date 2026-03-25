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
- No unit or integration tests
- No streaming AI responses
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
- [x] Replace gating update — `Replace` is enabled only in `Result` when `canReplace=true`; editor-level replace is intentionally disabled
- [x] Result actions expansion — Back, Re-run, Use output as input, Replace, and Copy with inline feedback
- [x] Micro-interactions — button scale on active, smooth swap rotation, copy feedback animation
- [x] Light theme WCAG AA contrast compliance verified

### Known Limitations
- No unit or integration tests (defer to Phase 4)
- backdrop-filter may impact perf on very low-end hardware (acceptable trade-off)

---

## Phase 4: Quality & Reliability

**Status**: Planned

### Goals
- [ ] Add unit tests for core logic (state transitions, prompt builder, AI clients)
- [ ] Add integration tests for selection detection and text replacement
- [ ] Structured error types (replace generic error messages)
- [ ] Improve popup positioning accuracy
- [ ] Add structured logging for debugging
- [ ] Handle edge cases: very long text, empty selections, unsupported apps

---

## Phase 5: Enhanced UX

**Status**: Planned

### Goals
- [ ] Streaming AI responses (show text as it arrives)
- [ ] History of recent rewrites (local storage)
- [ ] Undo support — revert to original text
- [ ] Multiple result variants
- [ ] Dark/light mode theme switching
- [ ] Custom keyboard shortcuts per tool

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
