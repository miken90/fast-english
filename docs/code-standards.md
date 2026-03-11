# Fast English — Code Standards

## File Organization

### Directory Convention
- Group files by **feature/responsibility**, not by type
- Top-level modules: `ai/`, `selection/`, `editing/`, `ui/`
- Core files at `src/` root: `main.rs`, `app.rs`, `settings.rs`, `persistence.rs`, `hotkey.rs`

### File Naming
- **snake_case** for all Rust source files
- One primary type/module per file
- Keep files under 200 lines where possible

### File Structure (within a file)
```rust
// 1. Imports
use crate::app::AppEvent;
use anyhow::Result;

// 2. Constants
const POLL_INTERVAL: Duration = Duration::from_millis(350);

// 3. Type definitions (structs, enums, traits)
pub struct SelectionContext { ... }

// 4. Implementations
impl SelectionContext { ... }

// 5. Free functions
pub fn detect_selection() -> Option<SelectionContext> { ... }
```

## Naming Conventions

### Types
- **Structs**: PascalCase — `SelectionContext`, `AppSettings`, `ProviderConfig`
- **Enums**: PascalCase, variants in PascalCase — `AIProvider::OpenAI`, `PopupState::Hidden`
- **Traits**: PascalCase — `AIClient`

### Functions and Methods
- **snake_case** — `detect_selection()`, `test_connection()`, `clean_output()`
- **Boolean methods**: prefix with `is_`, `has_`, `should_`

### Constants
- **SCREAMING_SNAKE_CASE** — `POLL_INTERVAL`, `POPUP_WIDTH`

## Rust Patterns

### Concurrency Model
- **tokio** for async runtime (multi-threaded)
- **tokio::sync::mpsc** for cross-component event channel
- **async/await** for AI network calls and selection polling
- COM initialized as STA on main thread

### State Management
```rust
pub enum PopupState {
    Hidden,
    Options(SelectionContext),
    ToneSelection(SelectionContext),
    Processing { context: SelectionContext, tool: WritingToolKind },
    Result { context: SelectionContext, tool: WritingToolKind, output: String },
    Error { context: SelectionContext, message: String },
}
```
- Enum with associated data for state + context
- Iced views match on state enum for rendering

### Trait-Based Abstraction
```rust
#[async_trait]
pub trait AIClient: Send + Sync {
    async fn process(&self, text: &str, instruction: &str) -> Result<String>;
    async fn test_connection(&self) -> Result<String>;
}
```
- Trait objects for swappable components
- Enables adding new AI providers without modifying consumers

### Error Handling
- Use `anyhow::Result` for application-level errors
- Use `tracing` for structured logging
- Map errors to user-visible state (`PopupState::Error`)

## UI Patterns

### Iced Framework
- **Daemon pattern**: `iced::daemon` for popup lifecycle management
- Views driven by `PopupState` enum via pattern matching
- Win32 interop for window management (topmost, positioning)

### System Tray
- `tray-icon` crate for system tray icon and menu
- Events sent via `AppEvent` channel

## Code Quality Guidelines

### General
- Prefer immutable bindings (`let` over `let mut`)
- Use `pub` only when needed; default to private
- Keep functions focused — one responsibility per function
- Use `#[cfg(target_os = "windows")]` for platform-specific code

### File Size
- Target under 200 lines per file
- Extract submodules when files grow too large

### Formatting
- Standard `rustfmt` formatting
- 4-space indentation
