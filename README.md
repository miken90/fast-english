# Fast English

A native Windows system tray app that provides AI-powered text rewriting tools. Select text in any app, and a floating popup appears with writing tools to enhance, restyle, or fix your writing.

## Features

- **System tray app** — lives in your system tray, always accessible
- **Global hotkey** — press `Ctrl+Alt+T` to trigger the popup from any application
- **Auto-detection** — automatically detects text selection via Windows UI Automation APIs
- **Writing tools**:
  - **Enhance Writing** — improve clarity and flow
  - **Change Tone** — rewrite in Professional, Casual, Formal, Friendly, Concise, or Confident tone
  - **Fix Grammar** — correct grammar and spelling errors
- **AI providers** — supports OpenAI (gpt-4.1-mini) and Anthropic (claude-3-5-sonnet-latest)
- **In-place replacement** — replaces selected text directly via UI Automation or clipboard fallback
- **Native Rust** — built with Rust, iced UI framework, and Windows APIs

## Requirements

- Windows 10 or later
- An API key from OpenAI or Anthropic

## Installation

### Portable

1. Download `FastEnglish.exe` from Releases
2. Run the executable — it will appear in your system tray
3. Right-click the tray icon to open Settings and configure your AI provider API key

### Installer

1. Download `FastEnglish-Setup-x.x.x.exe` from Releases
2. Run the installer
3. Optionally enable "Start with Windows"

## Usage

1. **Select text** in any application
2. The Fast English popup appears automatically near your selection (or press `Ctrl+Alt+T`)
3. Choose a writing tool:
   - Click **Enhance Writing** for improved clarity
   - Click **Change Tone** and pick a tone
   - Click **Fix Grammar** for corrections
4. Wait for the AI to process
5. Click **Replace** to substitute the original text, or **Copy** to copy to clipboard

### Settings

Right-click the system tray icon → Settings:

- **AI Provider** — select OpenAI or Anthropic, configure base URL, model, and API key; test connection
- **Writing Tools** — customize prompts for each tool

## Development Setup

### Prerequisites

- Rust toolchain (1.75+) via [rustup](https://rustup.rs/)
- Windows 10 SDK
- Visual Studio Build Tools (MSVC)

### Building

```bash
cargo build
```

### Release Build

```powershell
.\scripts\build-release.ps1
```

This builds an optimized release binary and packages it into `dist/`.

### Project Structure

```
src/
├── main.rs                    # Entry point, COM init, runtime setup
├── app.rs                     # State machine (PopupState, AppEvent, WritingToolKind)
├── hotkey.rs                  # Global Ctrl+Alt+T registration
├── settings.rs                # Settings model (AIProvider, ProviderConfig, AppSettings)
├── persistence.rs             # JSON file-based settings & API key storage
├── ai/
│   ├── mod.rs                 # AI module exports
│   ├── client.rs              # AIClient trait
│   ├── openai.rs              # OpenAI API client
│   ├── anthropic.rs           # Anthropic API client
│   └── prompt.rs              # Prompt builder & output cleaner
├── selection/
│   ├── mod.rs                 # Selection types (SelectionContext, Rect)
│   ├── monitor.rs             # Polling-based selection detection
│   ├── uia_text.rs            # UI Automation text pattern detection
│   ├── uia_value.rs           # UI Automation value pattern detection
│   └── clipboard_fallback.rs  # Clipboard-based selection fallback
├── editing/
│   ├── mod.rs                 # Editing module exports
│   ├── text_service.rs        # Text replacement (UIA + clipboard fallback)
│   └── clipboard.rs           # Clipboard read/write operations
└── ui/
    ├── mod.rs                 # UI module exports
    ├── popup.rs               # Iced daemon (popup lifecycle, event bridge)
    ├── popup_window.rs        # Win32 popup window management
    ├── popup_views.rs         # Popup UI views (options, tone picker, result)
    ├── popup_ai.rs            # AI request handling from popup
    ├── popup_position.rs      # Popup positioning near selection
    ├── settings_views.rs      # Settings UI views
    └── tray.rs                # System tray icon and menu
```

## Architecture Overview

Fast English follows a centralized state machine pattern:

- **main.rs** initializes COM, loads settings, starts async runtime, and launches the iced daemon
- **AppEvent** channel bridges background threads (tray, hotkey, selection monitor) into the UI
- **PopupState** enum drives the popup UI rendering
- **Selection monitor** polls via Windows UI Automation APIs every 350ms
- **AI layer** uses a trait-based design (`AIClient`) with swappable provider implementations
- **Text replacement** via UI Automation setValue or clipboard paste fallback

## License

All rights reserved.
