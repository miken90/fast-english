# Fast English — Codebase Summary

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
│   │   └── mod.rs                         # Module exports
│   ├── selection/                         # Text selection monitoring (UIA)
│   │   └── mod.rs                         # SelectionContext, Rect types
│   ├── editing/                           # Text replacement services
│   │   └── mod.rs                         # Module exports
│   └── ui/                                # All UI components
│       ├── mod.rs                         # Module exports
│       ├── popup*.rs                      # Floating popup (iced daemon)
│       ├── settings_views.rs              # Settings UI
│       └── tray.rs                        # System tray icon
├── resources/                             # Windows resources (manifest, rc)
├── scripts/
│   └── build-release.ps1                  # Release build script
├── installer/
│   └── FastEnglish.iss                    # Inno Setup installer config
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
| `persistence.rs` | JSON file storage in `%APPDATA%/FastEnglish/`, DPAPI-encrypted API keys |

### AI (`src/ai/`)

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports and re-exports |
| `client.rs` | `AIClient` trait definition |
| `openai.rs` | OpenAI Chat Completions API client via reqwest |
| `anthropic.rs` | Anthropic Messages API client via reqwest |
| `prompt.rs` | System/user prompt construction, output cleaning |

### Selection (`src/selection/`)

| File | Purpose |
|------|---------|
| `mod.rs` | `SelectionContext`, `Rect` types, module exports |
| `monitor.rs` | Polling loop (350ms) with 700ms debounce for selection changes |
| `uia_text.rs` | UI Automation Text Pattern detection |
| `uia_value.rs` | UI Automation Value Pattern detection |
| `clipboard_fallback.rs` | Clipboard-based selection detection fallback |

### Editing (`src/editing/`)

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `text_service.rs` | Text replacement via UIA SetValue + clipboard paste fallback |
| `clipboard.rs` | Clipboard read/write, save/restore, simulated Ctrl+V |

### UI (`src/ui/`)

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `popup.rs` | Iced daemon managing popup lifecycle and event bridge (427 LOC — modularization candidate) |
| `popup_window.rs` | Win32 popup window creation and management |
| `popup_views.rs` | Popup UI views (options, tone picker, result, error) |
| `popup_ai.rs` | AI request handling triggered from popup |
| `popup_position.rs` | Popup positioning near selection bounds |
| `settings_views.rs` | Settings UI views |
| `tray.rs` | System tray icon and context menu |

## Lines of Code

| Module | Files | LOC (approx) |
|--------|-------|---------------|
| Core (`src/`) | 5 | 536 |
| AI (`src/ai/`) | 5 (incl. `mod.rs`) | 306 |
| Selection (`src/selection/`) | 5 (incl. `mod.rs`) | 450 |
| Editing (`src/editing/`) | 3 (incl. `mod.rs`) | 110 |
| UI (`src/ui/`) | 8 (incl. `mod.rs`) | 948 |
| **Total** | **26** | **~2,350** |

> ⚠️ `src/ui/popup.rs` is 427 lines — the largest file and a modularization candidate (target <200 lines).

## Key Patterns

- **Event-driven architecture**: `AppEvent` channel bridges background threads into iced UI
- **Trait-based DI**: `AIClient` trait enables swappable AI provider implementations
- **Enum-driven UI**: `PopupState` enum drives popup view rendering
- **Polling-based selection**: Timer-driven UIA polling (350ms) with 700ms debounce for stable selection
- **Multi-tier detection**: UIA Text Pattern → UIA Value Pattern → clipboard fallback
- **Layered text editing**: UIA setValue → clipboard paste fallback chain
- **Settings form pattern**: Separate `SettingsForm` struct for UI editing vs persisted `AppSettings`
- **Copy feedback**: 1.5s auto-dismiss timer subscription after copy action
- **DPAPI encryption**: `CryptProtectData`/`CryptUnprotectData` for API key storage at rest
