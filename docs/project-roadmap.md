# ToneWriter — Project Roadmap

## Phase 1: v1.0 — Core MVP (Current)

**Status**: Complete

### Delivered Features
- [x] System tray Windows app
- [x] Text selection detection via UI Automation APIs (polling every 350ms)
- [x] Multi-tier selection: UIA Text Pattern → UIA Value Pattern → clipboard fallback
- [x] Floating popup near selected text (iced UI)
- [x] Global hotkey `Ctrl+Alt+T` to trigger popup
- [x] Three writing tools: Enhance Writing, Change Tone, Fix Grammar
- [x] Six tone options: Professional, Casual, Formal, Friendly, Concise, Confident
- [x] OpenAI integration (gpt-4.1-mini)
- [x] Anthropic integration (claude-3-5-sonnet-latest)
- [x] In-place text replacement via UIA SetValue + clipboard fallback
- [x] Settings UI: provider config, API key, custom prompts
- [x] Connection testing for AI providers
- [x] DPAPI-encrypted API key storage
- [x] Portable exe + Inno Setup installer

### Known Limitations
- No unit or integration tests
- No streaming AI responses
- Popup positioning can be imprecise in some apps

---

## Phase 2: Quality & Reliability

**Status**: Planned

### Goals
- [ ] Add unit tests for core logic (state transitions, prompt builder, AI clients)
- [ ] Add integration tests for selection detection and text replacement
- [ ] Structured error types (replace generic error messages)
- [ ] Improve popup positioning accuracy
- [ ] Add structured logging for debugging
- [ ] Handle edge cases: very long text, empty selections, unsupported apps

---

## Phase 3: Enhanced UX

**Status**: Planned

### Goals
- [ ] Streaming AI responses (show text as it arrives)
- [ ] Keyboard shortcuts within the popup
- [ ] History of recent rewrites (local storage)
- [ ] Undo support — revert to original text
- [ ] Multiple result variants
- [ ] Dark mode support

---

## Phase 4: More AI Providers & Tools

**Status**: Planned

### Goals
- [ ] Ollama integration for local/offline models
- [ ] Google Gemini integration
- [ ] Custom user-defined writing tools
- [ ] Per-app tool preferences
- [ ] Token usage tracking

---

## Phase 5: Distribution & Polish

**Status**: Planned

### Goals
- [ ] Code signing (Authenticode)
- [ ] Auto-update mechanism
- [ ] Website and landing page
- [ ] Proper app icon design
- [ ] Microsoft Store submission evaluation
