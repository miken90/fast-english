# ToneWriter — Codebase Summary

## Overview

- **Language**: Rust (2021 edition)
- **UI Framework**: iced 0.13
- **Platform**: Windows 10+
- **Build system**: Cargo
- **Key dependencies**: windows (Win32 APIs), uiautomation, iced, tray-icon, global-hotkey, reqwest, tokio
- **Tests**: None currently

## Directory Structure

```
fast-english/
├── Cargo.toml                             # Rust package manifest
├── Cargo.lock                             # Dependency lock file
├── build.rs                               # Build script (resource embedding)
├── src/
│   ├── main.rs                            # Entry point, COM init, runtime
│   ├── app.rs                             # State machine, events, tool types
│   ├── hotkey.rs                          # Global Ctrl+Alt+T registration
│   ├── settings.rs                        # Settings model
│   ├── persistence.rs                     # JSON file-based storage
│   ├── ai/                                # AI provider clients & prompts
│   ├── selection/                         # Text selection monitoring (UIA)
│   ├── editing/                           # Text replacement services
│   └── ui/                                # All UI components
│       ├── popup*.rs                      # Floating popup (iced daemon)
│       ├── settings_views.rs              # Settings UI
│       └── tray.rs                        # System tray icon
├── resources/                             # Windows resources (manifest, rc)
├── scripts/
│   └── build-release.ps1                  # Release build script
├── installer/
│   └── ToneWriter.iss                     # Inno Setup installer config
└── docs/                                  # Project documentation
```

## Module Descriptions

### Core (`src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Entry point; COM init (STA), settings load, tokio runtime, iced daemon launch |
| `app.rs` | `PopupState` enum, `AppEvent` channel events, `WritingToolKind`, `ToneOption` |
| `hotkey.rs` | Registers `Ctrl+Alt+T` via `global-hotkey` crate |
| `settings.rs` | `AIProvider`, `ProviderConfig`, `AppSettings` models |
| `persistence.rs` | JSON file storage in `%APPDATA%/ToneWriter/`, DPAPI-encrypted API keys |

### AI (`src/ai/`)

| File | Purpose |
|------|---------|
| `client.rs` | `AIClient` trait definition |
| `openai.rs` | OpenAI Chat Completions API client via reqwest |
| `anthropic.rs` | Anthropic Messages API client via reqwest |
| `prompt.rs` | System/user prompt construction, output cleaning |

### Selection (`src/selection/`)

| File | Purpose |
|------|---------|
| `monitor.rs` | Polling loop (350ms) with debounce for selection changes |
| `uia_text.rs` | UI Automation Text Pattern detection |
| `uia_value.rs` | UI Automation Value Pattern detection |
| `clipboard_fallback.rs` | Clipboard-based selection detection fallback |

### Editing (`src/editing/`)

| File | Purpose |
|------|---------|
| `text_service.rs` | Text replacement via UIA SetValue + clipboard paste fallback |
| `clipboard.rs` | Clipboard read/write, save/restore, simulated Ctrl+V |

### UI (`src/ui/`)

| File | Purpose |
|------|---------|
| `popup.rs` | Iced daemon managing popup lifecycle and event bridge |
| `popup_window.rs` | Win32 popup window creation and management |
| `popup_views.rs` | Popup UI views (options, tone picker, result, error) |
| `popup_ai.rs` | AI request handling triggered from popup |
| `popup_position.rs` | Popup positioning near selection bounds |
| `settings_views.rs` | Settings UI views |
| `tray.rs` | System tray icon and context menu |

## Key Patterns

- **Event-driven architecture**: `AppEvent` channel bridges background threads into iced UI
- **Trait-based DI**: `AIClient` trait enables swappable AI provider implementations
- **Enum-driven UI**: `PopupState` enum drives popup view rendering
- **Polling-based selection**: Timer-driven UIA polling (350ms) with debounce
- **Multi-tier detection**: UIA Text Pattern → UIA Value Pattern → clipboard fallback
- **Layered text editing**: UIA setValue → clipboard paste fallback chain
