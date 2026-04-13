# Phase 02 — Core UX Improvements

## Context Links
- [Plan](plan.md) | [Brainstorm](brainstorm.md)
- Blocked by: Phase 01
- Blocks: Phase 03

## Overview
- **Date**: 2026-04-13
- **Priority**: P1
- **Effort**: 2.5 days
- **Status**: ✅ Done

Transform the popup from 4 full-screen swaps into a continuous flow. Processing and recoverable errors become inline overlays. Result actions get clear hierarchy.

## Key Insights
- `PopupState` enum stays unchanged in Rust — changes are frontend-only reinterpretation
- Processing removing source text is the #1 friction point
- Result view has 7 competing actions — needs clear primary/secondary/tertiary hierarchy
- `ResizeObserver` may be needed once content becomes dynamic with inline states
- Oracle flagged: inline states are a **render-model change**, not just CSS

## Requirements
1. Processing state keeps editor/source visible with disabled controls + inline spinner
2. Recoverable errors show as banner inside editor surface
3. Result actions simplified with clear hierarchy
4. Editor shows tool-specific CTA labels
5. Popup sizing adapts correctly with inline content

## Architecture

### Inline Processing Model
```
┌──────────────────────────┐
│ Header: [tool] ● Running │  ← spinner in header
├──────────────────────────┤
│ [Tool pills — disabled]  │
│                          │
│ Source: [textarea]       │  ← visible but disabled
│ 42 chars                 │
│                          │
│ [Cancel]  Running...     │  ← action row
└──────────────────────────┘
```

Rust still sends `PopupState::Processing` → JS receives it but renders it as an overlay on the editor view instead of a full-screen swap.

### Inline Error Model
```
┌──────────────────────────┐
│ Header: [tool] ● Error   │
├──────────────────────────┤
│ ⚠ Something went wrong   │  ← error banner
│   [Retry] [Dismiss]      │
│                          │
│ Source: [textarea]       │  ← still visible
│ 42 chars                 │
└──────────────────────────┘
```

### Result Action Hierarchy
```
┌──────────────────────────┐
│ [← Edit]  ● Result  [×] │
├──────────────────────────┤
│ Output: [textarea]       │
│ 357 chars                │
│                          │
│ Source snippet ▼         │
│                          │
│ [Preview] [Edit output]  │  ← secondary
│           [Copy] [Replace]│  ← primary
└──────────────────────────┘
```

## Related Code Files
- `assets/popup/popup.js` — setState, renderProcessing, renderError, showView
- `assets/popup/popup.css` — view layouts, processing-view, error-view-inner
- `assets/popup/index.html` — view sections structure
- `src/ui/popup_view.rs` (new from Phase 1) — push_popup_state, height calculation
- `src/ui/webview_bridge.rs` — popup_state_to_json

## Implementation Steps

### Step 1: Inline processing (M)
**popup.js**:
- Modify `setState()` for `processing` view: instead of `showView('processing')`, keep editor visible
- Add `.is-processing` class to editor view — disables inputs, shows spinner in header
- Show tool name + "Running…" in action row or header
- Cancel button replaces Run button
- On result/error, remove `.is-processing` and transition normally

**popup.css**:
- `.view.is-processing .editor-textarea` — `pointer-events: none; opacity: 0.5`
- `.view.is-processing .tool-pill` — disabled appearance
- `.view.is-processing .popup-header .spinner-inline` — small inline spinner

**popup.js** resize:
- Processing no longer needs separate snap height — editor height preserved
- Remove `processing` from `resolveSnapMode()` or map it to `editor`

**Rust** (minimal):
- `calculate_height_for_mode("processing")` → use editor height instead of fixed 155px
- Or: frontend stops sending ResizePopup for processing state

Verify: manual test — trigger tool, see source visible during processing

### Step 2: Inline error banner (S-M)
**popup.js**:
- When `setState()` receives `error` view: check if previous view was `editor` or `processing`
- If yes → keep editor visible, show error banner overlay
- Banner contains: icon, message, [Retry] [Dismiss] buttons
- Full error screen reserved only for startup/unrecoverable errors (no previous editor state)

**popup.css**:
- `.error-banner` — fixed-position or absolute within editor, warning style
- Dismiss removes banner, returns to editor
- Retry sends existing `Retry` IPC

**index.html**:
- Add `<div class="error-banner" id="editor-error-banner">` inside `view-editor`

Verify: trigger error (e.g., bad API key), see banner + source visible

### Step 3: Result action simplification (S)
**popup.js + popup.css**:
- Rename: `Rich` → `Preview`, `Use as input` → `Edit output`
- Move Rerun from header to secondary actions near output
- Reorder footer: left=[Edit, Rerun], right=[Preview, Edit output | Copy, Replace]
- Or simpler: left=[Edit], right=[Copy, Replace], secondary row=[Preview, Edit output, Rerun]

**index.html**:
- Restructure `action-row--result` with `action-group-primary` / `action-group-secondary`

Verify: visual check result actions, keyboard flow

### Step 4: Editor clarity (S)
**popup.js**:
- Header label: replace "Editor" with current tool title (e.g., "Enhance Writing")
- Run button: tool-specific label ("Enhance", "Fix Grammar", "Translate", "Convert")
- Disable Run when source is empty/whitespace only

**popup.css**:
- Style for disabled Run button

Verify: switch tools, check labels change

### Step 5: ResizeObserver upgrade (S-M)
**popup.js**:
- Replace mouseup-based resize detection with `ResizeObserver` on textareas
- Single observer triggers `requestSnapResizeForCurrentView()`
- Remove `_lastEditorH` / `_lastResultH` mouseup listeners

Verify: resize textarea, popup window follows

### Step 6: Integration test (S)
- Full flow: open popup → type text → run tool → see inline processing → result → copy → dismiss
- Error flow: bad API → see inline error → retry → dismiss
- Resize: resize textarea in editor and result → popup adapts
- Theme: test light/dark/auto

## Todo List
- [ ] Inline processing — keep editor visible with disabled controls
- [ ] Inline spinner in header during processing
- [ ] Cancel replaces Run during processing
- [ ] Processing height uses editor height (not fixed 155px)
- [ ] Inline error banner in editor view
- [ ] Full error screen only for unrecoverable errors
- [ ] Rename `Rich` → `Preview`
- [ ] Rename `Use as input` → `Edit output`
- [ ] Move Rerun to secondary actions
- [ ] Header shows tool name instead of "Editor"
- [ ] Run button shows tool-specific label
- [ ] Disable Run when source empty
- [ ] Replace mouseup resize with ResizeObserver
- [ ] Manual full-flow smoke test

## Success Criteria
- Processing shows source text (never hidden)
- Recoverable errors appear as inline banner
- Result view has clear action hierarchy
- Editor shows tool-specific labels
- Popup sizing works correctly with inline states
- No visual regressions in existing flows

## Risk Assessment
| Component | Risk | Mitigation |
|---|---|---|
| Inline processing render model | MEDIUM | Keep PopupState unchanged, reinterpret in JS only |
| Popup sizing with inline content | MEDIUM | Test with short/long text, all tool types |
| Error banner state management | LOW | Banner is ephemeral UI state, not Rust state |
| ResizeObserver browser support | LOW | WebView2 supports it natively |

## Security Considerations
- No changes to AI request handling, credentials, or IPC contracts

## Next Steps
→ Phase 03: Settings & Polish (draft model, a11y, remaining cleanup)
