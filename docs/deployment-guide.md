# Fast English — Deployment Guide

## Prerequisites

- Windows 10 or later (Microsoft Edge WebView2 Runtime required)
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

## Release Process

### Release Rules

- Fast English release builds are Windows-only and **must** run via `powershell.exe`
- Default release scope for this repo is **portable only**
- Do **not** include installer artifacts unless the user explicitly asks for installer release support
- Keep release versions in sync across `Cargo.toml`, `Cargo.lock`, `resources/app.manifest`, `scripts/build-release.ps1`, and `installer/FastEnglish.iss`
- Release artifacts are published from `dist/`
- Create GitHub releases as **draft first**, then review assets and notes before publishing
- Generate release notes from `git log <previous-tag>..HEAD` and keep them user-facing

### Version Sync Checklist

Before cutting a release, update these files:

- `Cargo.toml`
- `Cargo.lock`
- `resources/app.manifest`
- `scripts/build-release.ps1`
- `installer/FastEnglish.iss`
- any docs that embed a concrete release filename

### Release Commands

Run all Rust build commands through PowerShell:

```powershell
powershell.exe -Command "cd D:\WORKSPACES\PERSONAL\fast-eng; cargo fmt --check 2>&1"
powershell.exe -Command "cd D:\WORKSPACES\PERSONAL\fast-eng; cargo clippy 2>&1"
powershell.exe -Command "cd D:\WORKSPACES\PERSONAL\fast-eng; cargo build --release --target x86_64-pc-windows-msvc 2>&1"
powershell.exe -Command "cd D:\WORKSPACES\PERSONAL\fast-eng; .\scripts\build-release.ps1 2>&1"
```

If Inno Setup 6 is installed:

```powershell
powershell.exe -Command "cd D:\WORKSPACES\PERSONAL\fast-eng; & 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' .\installer\FastEnglish.iss 2>&1"
```

### GitHub Release Flow

1. Sync versions in all release/version files
2. Run format, lint, release build, and packaging commands
3. Verify `dist/` contains the expected portable `0.3.0` artifacts
4. Draft release notes from `git log v0.2.0..HEAD`
5. Create draft release with `gh release create`
6. Review notes and assets, then publish
7. Only add installer assets when the user explicitly asks for them

## Creating an Installer

The project includes an [Inno Setup](https://jrsoftware.org/isinfo.php) script for creating a Windows installer.

1. Install Inno Setup 6+
2. Build the release binary first (see above)
3. Open `installer/FastEnglish.iss` in Inno Setup Compiler
4. Click Build → Compile
5. Output: `dist/FastEnglish-Setup-0.3.0.exe`

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

### Initialization Failure

1. **WebView2 Runtime missing**: Fast English requires the Microsoft Edge WebView2 Runtime. It is usually pre-installed on modern Windows versions. If missing, [download and install it](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

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
