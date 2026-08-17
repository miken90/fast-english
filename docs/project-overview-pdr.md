# Fast English — Product Development Requirements

## Vision

Fast English eliminates the friction of rewriting text by bringing AI-powered writing tools directly to the user's cursor. Instead of switching to a browser or chat interface, users select text in any Windows app and get instant rewrites in a lightweight floating popup.

## Target Users

### Primary: Knowledge Workers
- Writers, editors, marketers, and professionals who produce written content daily
- Value speed and staying in their current workflow context
- Need quick writing improvements without leaving their app

### Secondary: Non-Native English Speakers
- Need grammar and spelling corrections integrated into their workflow
- Want quick polishing without leaving their writing app

### Tertiary: Developers and Technical Writers
- Write documentation, comments, PRs, and emails
- Need concise, clear communication but spend minimal time polishing prose

## Core Features

### 1. Text Selection Detection
- Monitors focused application for text selection via Windows UI Automation APIs
- Polls every 350ms for selection changes
- 700ms debounce — waits for stable selection before triggering popup
- Multi-tier detection: UIA Text Pattern → UIA Value Pattern → clipboard fallback

### 2. Floating Popup Interface
- Appears near selected text automatically or via `Ctrl+Alt+T` hotkey
- Universal four-state flow: `Editor` → `Processing` → `Result`/`Error`
- Snap-sized popup modes by state: editor, editor-translate, processing, result-compact/result-expanded, error
- Answers stream in as the model produces them, instead of waiting for the full response
- Result supports Replace (when editable), Copy, Re-run, Use output as input, and optional rich/markdown preview
- Fix Grammar opens with a word-level diff (removed/added highlighting); Enhance Writing offers the same diff off by default; Translate and Convert Prompt do not offer a diff
- Dismisses on outside click or Escape key

### 2b. Recent Results
- Last 20 completed results kept, DPAPI-encrypted at rest, reachable from the tray or the popup header
- Opening an entry restores the result view (Copy, Edit output, diff); Replace is disabled since the original selection no longer exists

### 3. Writing Tools
| Tool | Description |
|------|-------------|
| Enhance Writing | Improve clarity, flow, and readability |
| Fix Grammar | Correct grammar and spelling |
| Convert Prompt | Transform text into different formats or styles |
| Translate | Translate text between languages |

### 4. AI Provider Integration
- Supports OpenAI and Anthropic APIs
- Configurable base URL, model, and API key per provider
- Connection testing from Settings
- Trait-based design allows adding new providers

### 5. In-Place Text Replacement
- Primary: UI Automation `SetValue` for direct replacement
- Fallback: Clipboard-based paste via simulated `Ctrl+V` keystroke
- Preserves user's clipboard by saving/restoring contents

### 6. Settings & Customization
- Choose AI provider and configure credentials
- Settings stored as JSON in user's app data directory
- API keys stored securely via Windows DPAPI encryption
- Auto-start with Windows (toggle in settings, uses Registry HKCU Run key)

## Technical Constraints

- Windows 10+ only (UI Automation, WebView2 runtime dependency)
- AI features require user-provided API key (no bundled keys)
- System tray app (no main window)

## Future Roadmap Ideas

### Short-Term
- Add integration/E2E tests (unit tests exist; broad suite still pending)
- Keyboard shortcuts within the popup
- Undo support — revert to original text

### Medium-Term
- Additional AI providers (Ollama for local models, Google Gemini)
- Custom user-defined writing tools

### Long-Term
- Installer with auto-update mechanism
- Multi-language support (UI and writing tools)
- Context-aware suggestions
