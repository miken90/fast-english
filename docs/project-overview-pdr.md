# Fast English — Product Development Requirements

## Vision

Fast English eliminates the friction of rewriting text by bringing AI-powered writing tools directly to the user's cursor. Instead of switching to a browser or chat interface, users select text in any Windows app and get instant rewrites in a lightweight floating popup.

## Target Users

### Primary: Knowledge Workers
- Writers, editors, marketers, and professionals who produce written content daily
- Frequently need to adjust tone for different audiences (email to boss vs. message to teammate)
- Value speed and staying in their current workflow context

### Secondary: Non-Native English Speakers
- Need grammar and spelling corrections integrated into their workflow
- Benefit from tone adjustment to match professional norms in English
- Want quick polishing without leaving their writing app

### Tertiary: Developers and Technical Writers
- Write documentation, comments, PRs, and emails
- Need concise, clear communication but spend minimal time polishing prose

## Core Features

### 1. Text Selection Detection
- Monitors focused application for text selection via Windows UI Automation APIs
- Polls every 350ms for selection changes
- Multi-tier detection: UIA Text Pattern → UIA Value Pattern → clipboard fallback

### 2. Floating Popup Interface
- Appears near selected text automatically or via `Ctrl+Alt+T` hotkey
- Three-phase flow: tool selection → (optional tone selection) → result display
- Provides Replace (in-place) and Copy actions on results
- Dismisses on outside click or Escape key

### 3. Writing Tools
| Tool | Description | Options |
|------|-------------|---------|
| Enhance Writing | Improve clarity, flow, and readability | — |
| Change Tone | Rewrite in a specific tone | Professional, Casual, Formal, Friendly, Concise, Confident |
| Fix Grammar | Correct grammar and spelling | — |

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

## Technical Constraints

- Windows 10+ only (UI Automation, iced UI framework)
- AI features require user-provided API key (no bundled keys)
- System tray app (no main window)

## Future Roadmap Ideas

### Short-Term
- Add unit and integration tests
- Streaming AI responses for perceived speed improvement
- Keyboard shortcuts within the popup

### Medium-Term
- Additional AI providers (Ollama for local models, Google Gemini)
- Custom user-defined writing tools
- Usage history (local only)

### Long-Term
- Installer with auto-update mechanism
- Multi-language support (UI and writing tools)
- Context-aware suggestions
