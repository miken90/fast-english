# Fast English

A native Windows system tray app that provides AI-powered text rewriting tools. Select text in any app, and a floating popup appears with writing tools to enhance, restyle, or fix your writing.

## Features

- **System tray app** — lives in your system tray, always accessible
- **Global hotkey** — press `Ctrl+Alt+T` to trigger the popup from any application (toggles on/off; opens only when manual capture finds text or bounds)
- **Auto-detection** — automatically detects text selection via Windows UI Automation APIs
- **Writing tools**:
  - **Enhance Writing** — improve clarity and flow
  - **Change Tone** — rewrite in Professional, Casual, Formal, Friendly, Concise, or Confident tone
  - **Fix Grammar** — correct grammar and spelling errors
  - **Translate** — translate between 15 languages with editable input and re-translate support
  - **Convert Prompt** — convert text into structured AI prompts with markdown formatting and rich preview
- **AI providers** — supports OpenAI-compatible APIs (gpt-4.1-mini default) via configurable base URL
- **In-place replacement** — replaces selected text directly via UI Automation or clipboard fallback
- **Native Rust** — built with Rust, WebView2 (wry/tao), and Windows APIs

`FastEnglish.exe --hub` is used by the my-utilities hub: no tray icon, single instance, settings opened in the hub.

## Requirements

- Windows 10 or later
- An API key from OpenAI or Anthropic

## Installation

### Portable

1. Download `FastEnglish-0.3.0-portable.zip` from Releases
2. Extract and run `FastEnglish.exe` — it will appear in your system tray
3. Right-click the tray icon to open Settings and configure your AI provider API key

### Installer

Installer builds are not included by default. They are opt-in for this repo and only published when explicitly requested.

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
│   ├── mod.rs                 # AI module exports + create_ai_client()
│   ├── client.rs              # AIClient trait
│   ├── openai.rs              # OpenAI API client (shared reqwest::Client)
│   ├── anthropic.rs           # Anthropic API client
│   └── prompt.rs              # Prompt builder & output cleaner
├── selection/
│   ├── mod.rs                 # Selection types (SelectionContext, Rect, Tier)
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
    ├── popup.rs               # WebView runtime, state coordinator, IPC handling
    ├── webview_popup.rs       # WebView2 popup window
    ├── webview_settings.rs    # WebView2 settings window
    ├── webview_bridge.rs      # Rust↔JS IPC bridge (state → JSON)
    ├── webview_ipc.rs         # IPC message types
    ├── popup_position.rs      # Popup placement + state-dependent heights
    ├── settings_views.rs      # Settings form model + HTTPS validation
    └── tray.rs                # System tray icon and menu (embedded PNG)

assets/
├── popup/
│   ├── index.html             # Popup HTML
│   ├── popup.js               # Popup JavaScript logic + inline MD parser
│   ├── popup.css              # Popup styles
│   ├── settings.html          # Settings HTML
│   ├── settings.js            # Settings JavaScript
│   └── settings.css           # Settings styles

resources/
├── app.ico                    # Application icon
├── app.rc                     # Windows resource file
├── tray_dark.png              # Dark mode tray icon
└── tray_light.png             # Light mode tray icon
```

## Architecture Overview

Fast English uses a hybrid Rust + WebView2 architecture:

- **main.rs** initializes COM, loads settings, starts async runtime, and launches the WebView2 UI
- **AppEvent** channel bridges background threads (tray, hotkey, selection monitor) into the UI
- **PopupState** enum drives the UI state, mirrored to JS via the Rust↔JS bridge
- **WebView2** (wry/tao) hosts the frontend using standard web technologies (HTML/CSS/JS)
- **Selection monitor** polls via Windows UI Automation APIs every 350ms
- **AI layer** uses a trait-based design (`AIClient`) with a shared `reqwest::Client` instance
- **Text replacement** via UI Automation setValue or clipboard paste fallback
- **Hotkey** toggles popup; opens after manual capture only when text or bounds are found (tray “Open Popup” can still open an empty popup)
- **HTTPS enforcement** validates provider base URLs before persisting settings

## License

All rights reserved.
