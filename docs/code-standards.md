# Fast English — Code Standards

## File Organization

### Directory Convention
- Group files by feature/responsibility (`ai/`, `selection/`, `editing/`, `ui/`).
- Keep core orchestration at `src/` root (`main.rs`, `app.rs`, `settings.rs`, `persistence.rs`, `hotkey.rs`).

### File Naming
- **snake_case** for Rust files.
- One primary module responsibility per file.
- Prefer files under 200 lines; if larger, document reason and plan modularization.

## Naming Conventions

### Types
- Structs/enums/traits: PascalCase.
- Enum variants: PascalCase.
- Keep IPC enum variant names action-oriented (`ToolSelected`, `SaveSettings`, etc.).

### Functions and Methods
- snake_case (`start_ai_request`, `check_webview2_runtime`).
- Boolean helpers prefixed with `is_`, `has_`, `should_`.

### Constants
- SCREAMING_SNAKE_CASE (`POLL_INTERVAL`, `RESULT_HEIGHT`).

## Rust Runtime Patterns

### Concurrency and Threads
- Use tokio runtime for async background work (selection polling, AI HTTP).
- Use `tokio::mpsc` for app/event orchestration.
- For WebView windows, run `tao::EventLoop` on dedicated OS threads.
- Bridge tokio <-> tao using explicit channels (`std::mpsc` + `EventLoopProxy`).

### State Management
- `PopupState` is canonical business/UI state authority.
- WebView frontends render state snapshots provided by Rust; JS must not invent backend state.
- Keep state transitions centralized in runtime coordinator (`ui/popup.rs`).

### Error Handling
- Use `anyhow::Result` for app-level fallible boundaries.
- Convert operational failures into user-visible state (`PopupState::Error`) where relevant.
- Log errors with `tracing` and enough context for debugging.

## CSS Standards

### Organization
- **Layer 1 (tokens.css)**: Design tokens — colors (with light/dark variants), spacing scale, shadows, typography
- **Layer 2 (base.css)**: Global reset, font-face declarations, HTML/body reset, theme detection
- **Layer 3 (components.css)**: Reusable component library (buttons, textareas, status badges, status dots)
- **Layer 4 (popup.css / settings.css)**: App-specific layouts, view styles, animations, Fluent Midnight refresh

Each file documents its requirements and dependencies via CSS comments.

### Naming Conventions
- **CSS classes**: `kebab-case` (e.g., `.status-badge`, `.feedback-toast`)
- **BEM pattern** for component variants: `.button--primary`, `.button--ghost`
- **State prefixes**: `is-`, `has-`, `--active`, `--disabled`, `--loading`
- **Design tokens**: `--{property}-{modifier}` (e.g., `--primary`, `--primary-light`, `--text-muted`)
- **Theme variants**: Use `:root[data-theme="light"]` for light mode overrides (no separate classes)

### Theme System
- All color tokens defined once in `tokens.css` with dark mode as default
- Light theme overrides via CSS selector `:root[data-theme="light"] { --token: value; }`
- JS sets `document.documentElement.dataset.theme` on init (from saved preference or system detect)
- No light/dark class variants on elements — theme is global via `:root` selector

### CSS Reset & Defaults
- Global box-sizing reset via `*` selector.
- Font stack defined on `html` with fallbacks.
- All color tokens inherit from root `:root`.

### Accessibility in CSS
- Focus-visible outlines: `outline: 2px solid var(--primary); outline-offset: 2px`
- Min touch target: 44×44px for interactive elements
- `prefers-reduced-motion` media query respected
- Contrast ratios: WCAG AA minimum (4.5:1 for text) — verified for both light and dark themes

## WebView and IPC Standards

### WebView Module Conventions
- Popup and settings windows live in separate modules (`webview_popup.rs`, `webview_settings.rs`).
- Validate WebView2 runtime before starting window loop.
- Keep non-Windows stubs behind `#[cfg(not(target_os = "windows"))]`.
- Store local HTML/CSS/JS assets in `assets/popup/` and inline them via `include_str!` for offline/runtime-stable loading.
- Include `icons.js` in both popup and settings WebView initialization via `include_str!`.

### Icon System (Phase 3)
- All SVG icons managed through `assets/popup/icons.js` module
- Icons accessed via `renderIcon(name, size)` factory function (e.g., `renderIcon('close', 24)`)
- Icon name map centralized in `icons.js` — updates in one place for all references
- Used in `popup.js` and `settings.js` for dynamic icon rendering
- Eliminates inline SVG strings throughout application code

### Frontend State Management (Phase 3)
- `popup.js`: All mutable state encapsulated in `const appState = {}` object
  - Includes: `selectedToolIndex`, `_translateLanguages`, `_translateFromId`, `_translateToId`, etc.
  - No global mutable variables outside `appState`
  - Accessed throughout as `appState.propertyName`
- `settings.js`: Form state managed similarly with minimal globals
- Benefits: Easier debugging, clearer variable scope, fewer side effects

### IPC Message Format
- Use serde-tagged envelope format:

```json
{
  "type": "MessageKind",
  "payload": { "field": "value" }
}
```

- Define message contracts in a single place (`webview_ipc.rs`).
- JS->Rust payloads must deserialize into typed enums (`JsToRust`).
- Rust->JS updates should be generated from typed state and serialized JSON helpers.

### IPC Safety Rules
- Validate/parse all inbound IPC with typed deserialization and explicit error logging.
- Never pass raw API keys/secrets into JS payloads.
- Escape user/AI text before embedding in HTML-rendered UI state (`escape_html`).
- Pass theme preference as plain string in state; no security implications.

### Confirmation Dialogs & Safety Features (Phase 3)
- Replace Selection confirmation: inline bar with "Replace? [Yes] [No]" before replacement
- Keyboard support: Enter = Yes, Escape = No, with `aria-live="assertive"`
- Settings cancel: close window without saving (no IPC needed)
- Settings reset: JS-only restore of form to defaults (no persistence until Save)

## Settings and Persistence Pattern

- Use `SettingsForm` as mutable UI-edit model.
- Apply form changes back to `AppSettings` via explicit `apply_to_settings` routine.
- Persist API keys only through DPAPI-backed storage.

## Platform-Specific Rules

- Windows-only app behavior is acceptable; gate platform specifics with `#[cfg(target_os = "windows")]`.
- Keep Win32 `unsafe` blocks minimal and document safety assumptions.

## Logging and Security

- Use structured `tracing` logs (`info!`, `debug!`, `warn!`, `error!`).
- Log lifecycle boundaries (startup, WebView runtime checks, IPC parse errors, AI/edit failures).
- Never log secrets or full sensitive user content.

## Quality Gates

- Required verification (PowerShell on Windows path):
  - `cargo check`
  - `cargo fmt --check`
  - `cargo clippy`
- Warnings may exist temporarily, but new code should avoid introducing avoidable lint/format issues.
