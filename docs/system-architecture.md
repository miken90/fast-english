# Fast English — System Architecture

## High-Level Component Diagram

```
┌─────────────────────────────────────────────────────┐
│                   Fast English App                    │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ main.rs  │→ │ app.rs   │  │ settings.rs      │  │
│  │ COM init │  │ State    │  │ persistence.rs   │  │
│  │ Runtime  │  │ Machine  │  │ (JSON + DPAPI)   │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
│       │              ↕                              │
│  ┌────┴────────────────────────────────────────┐    │
│  │         AppEvent Channel (tokio mpsc)        │    │
│  └────┬────────┬────────┬──────────┬───────────┘    │
│       ↓        ↓        ↓          ↓                │
│  ┌────────┐ ┌──────┐ ┌────────┐ ┌──────────────┐   │
│  │ tray   │ │hotkey│ │monitor │ │ popup (iced)  │   │
│  │        │ │Ctrl+ │ │UIA poll│ │ views, AI,    │   │
│  │        │ │Alt+T │ │ 350ms  │ │ positioning   │   │
│  └────────┘ └──────┘ └────────┘ └──────────────┘   │
│                          ↕              ↕           │
│                    ┌──────────┐  ┌─────────────┐    │
│                    │selection │  │ editing     │    │
│                    │ UIA Text │  │ UIA SetValue│    │
│                    │ UIA Value│  │ Clipboard   │    │
│                    │ Clipboard│  │ fallback    │    │
│                    └──────────┘  └─────────────┘    │
│                                        ↕           │
│                                 ┌─────────────┐    │
│                                 │ ai/         │    │
│                                 │ OpenAI      │    │
│                                 │ Anthropic   │    │
│                                 │ PromptBuilder│    │
│                                 └─────────────┘    │
└─────────────────────────────────────────────────────┘
         ↕                    ↕              ↕
   ┌──────────┐        ┌──────────┐   ┌──────────┐
   │ Win32    │        │ OpenAI   │   │Anthropic │
   │ UIA APIs │        │ API      │   │ API      │
   └──────────┘        └──────────┘   └──────────┘
```

## Component Responsibilities

### Core

| Component | Role |
|-----------|------|
| **main.rs** | Entry point. COM init (STA), settings load, tokio runtime, selection monitor start, hotkey/tray setup, iced daemon launch. |
| **app.rs** | Defines `PopupState` enum (state machine), `AppEvent` (cross-component events), `WritingToolKind`, `ToneOption`. |
| **settings.rs** | Data models: `AIProvider`, `ProviderConfig`, `AppSettings`. |
| **persistence.rs** | JSON file storage in `%APPDATA%/FastEnglish/`. API keys encrypted via Windows DPAPI. |
| **hotkey.rs** | Registers global `Ctrl+Alt+T` via `global-hotkey` crate. Sends `AppEvent::HotkeyPressed`. |

### UI

| Component | Role |
|-----------|------|
| **tray.rs** | Creates system tray icon with context menu (Settings, Quit). |
| **popup.rs** | Iced daemon managing popup lifecycle and bridging `AppEvent` into iced `Message`. |
| **popup_window.rs** | Win32 window creation for popup (topmost, no taskbar). |
| **popup_views.rs** | Iced views for each popup state (options, tone picker, processing, result, error). |
| **popup_ai.rs** | Handles AI requests from the popup (creates client, processes, cleans output). |
| **popup_position.rs** | Calculates popup position near selection bounds with screen-edge detection. |
| **settings_views.rs** | Iced settings views for provider config and writing tools. |

### AI

| Component | Role |
|-----------|------|
| **client.rs** | `AIClient` trait: `process(text, instruction)` and `test_connection()`. |
| **openai.rs** | OpenAI Chat Completions API client via reqwest. |
| **anthropic.rs** | Anthropic Messages API client via reqwest. |
| **prompt.rs** | Constructs system/user prompts. Strips AI commentary from output. |

### Selection

| Component | Role |
|-----------|------|
| **monitor.rs** | Async polling loop (350ms) with debounce. Detects selection changes, sends `AppEvent::TextSelected`. |
| **uia_text.rs** | UI Automation Text Pattern — reads selected text from focused element. |
| **uia_value.rs** | UI Automation Value Pattern — reads value and detects selection via clipboard snapshot. |
| **clipboard_fallback.rs** | Simulates Ctrl+C to capture selection when UIA methods fail. |

### Editing

| Component | Role |
|-----------|------|
| **text_service.rs** | Replaces selected text. Tries UIA SetValue first, falls back to clipboard paste. |
| **clipboard.rs** | Clipboard read/write, save/restore around paste operations, simulated Ctrl+V. |

## State Machine — PopupState

```
Hidden → Options(ctx)          [Selection detected or Ctrl+Alt+T]
Options → ToneSelection(ctx)   [User clicks "Change Tone"]
Options → Processing{ctx,tool} [User clicks "Enhance" or "Fix Grammar"]
Options → Hidden               [Dismiss]
ToneSelection → Processing     [User selects a tone]
ToneSelection → Options        [Back]
ToneSelection → Hidden         [Dismiss]
Processing → Result{ctx,output}[AI response received]
Processing → Error{ctx,msg}    [AI request failed]
Result → Hidden                [Replace / Copy / Dismiss]
Result → Options               [Try again]
Error → Options                [Try again]
Error → Hidden                 [Dismiss]
```

## Data Flow

### Text Rewrite Flow

1. User selects text in any app
2. Selection monitor polls UIA (every 350ms), detects selection
3. `AppEvent::TextSelected(ctx)` sent via channel
4. Iced daemon receives event, transitions to `PopupState::Options`
5. Popup window shown near selection bounds
6. User clicks a writing tool
7. `PopupState::Processing` — AI request fires via reqwest
8. AI response received → `PopupState::Result`
9. User clicks Replace → `text_service` replaces text via UIA or clipboard

### Selection Detection Pipeline

```
Timer fires (every 350ms)
  → Tier 1: UIA Text Pattern (get selected text range)
  → Tier 2: UIA Value Pattern (get element value + clipboard snapshot)
  → Tier 3: Clipboard fallback (simulate Ctrl+C)
  → Debounce: wait 700ms for stable selection
  → Send AppEvent::TextSelected(context)
```

### Text Editing Pipeline

```
text_service::replace(context, new_text)
  → Try: UIA SetValue on focused element
    → Success? Done.
    → Failure? Continue to fallback.
  → Fallback: Clipboard paste
    → Save current clipboard
    → Write new text to clipboard
    → Simulate Ctrl+V
    → Restore original clipboard
```

## Threading Model

- **Main thread**: Win32 message loop (STA COM), tray icon, hotkey listener
- **Tokio runtime**: Selection monitor polling, AI network requests
- **Iced daemon**: UI rendering and event processing (runs on main thread)
- **AppEvent channel**: tokio mpsc bridges background threads to UI

## Security Considerations

- API keys encrypted via Windows DPAPI before storage
- No API keys bundled in the app; user must provide their own
- Network calls use HTTPS to AI provider endpoints
- No analytics or telemetry; all data stays local
