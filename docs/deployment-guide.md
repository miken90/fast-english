# Fast English — Deployment Guide

## Prerequisites

- Windows 10 or later
- Rust toolchain (1.75+) via [rustup](https://rustup.rs/)
- Visual Studio Build Tools with MSVC and Windows 10 SDK

## Building from Source

### Debug Build

```bash
cargo build
```

Binary at `target/debug/fast-english.exe`.

### Release Build

```bash
cargo build --release
```

Binary at `target/release/fast-english.exe`.

### Automated Release Build

```powershell
.\scripts\build-release.ps1
```

This builds an optimized release binary targeting x86_64-pc-windows-msvc and packages it into `dist/`.

## Creating an Installer

The project includes an [Inno Setup](https://jrsoftware.org/isinfo.php) script for creating a Windows installer.

1. Install Inno Setup 6+
2. Build the release binary first (see above)
3. Open `installer/FastEnglish.iss` in Inno Setup Compiler
4. Click Build → Compile
5. Output: `dist/FastEnglish-Setup-1.0.0.exe`

## Installation

### Portable (No Install)

1. Download or build `FastEnglish.exe`
2. Run it from any location
3. The app appears in the system tray

### Via Installer

1. Run `FastEnglish-Setup-x.x.x.exe`
2. Follow the wizard
3. Optionally enable "Start Fast English when Windows starts"
4. The app launches and appears in the system tray

## Configuration

### AI Provider Setup

1. Right-click the Fast English system tray icon → **Settings**
2. Select your provider (OpenAI or Anthropic)
3. Enter your API key
4. (Optional) Customize the base URL or model
5. Click **Test Connection** to verify

### Default Provider Configuration

| Provider | Default Base URL | Default Model |
|----------|-----------------|---------------|
| OpenAI | `https://api.openai.com/v1` | `gpt-4.1-mini` |
| Anthropic | `https://api.anthropic.com/v1` | `claude-3-5-sonnet-latest` |

### Settings Storage

Settings are stored as JSON in `%APPDATA%/FastEnglish/settings.json`. API keys are encrypted via Windows DPAPI.

## Troubleshooting

### Popup doesn't appear when I select text

1. **Check the system tray**: Ensure Fast English is running (icon visible)
2. **Try the hotkey**: Select text, then press `Ctrl+Alt+T`
3. **Unsupported app**: Some apps don't expose text selection via UI Automation APIs
4. **Restart the app**: Right-click tray icon → Quit, then relaunch

### "Replace" doesn't work in some apps

1. **UIA SetValue not supported**: Some apps don't support UI Automation value setting. The app falls back to clipboard paste.
2. **Read-only fields**: If the text field is read-only, replacement will fail. Use "Copy" instead.

### AI request fails

1. **Check API key**: Open Settings and verify the API key is correct
2. **Test connection**: Click "Test Connection" in Settings
3. **Check network**: Ensure you can reach the AI provider's API endpoint
4. **Rate limits**: You may have hit the provider's rate limit. Wait and retry.

### Hotkey `Ctrl+Alt+T` doesn't work

1. **Conflict**: Another app may have registered the same hotkey
2. **Restart**: Quit and relaunch Fast English

## Uninstalling

### If installed via installer
- Use Windows Settings → Apps → Fast English → Uninstall

### If using portable version
- Just delete the exe
- Optionally remove settings: delete `%APPDATA%/FastEnglish/`
