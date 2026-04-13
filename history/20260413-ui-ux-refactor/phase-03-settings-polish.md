# Phase 03 — Settings & Polish

## Context Links
- [Plan](plan.md) | [Brainstorm](brainstorm.md)
- Blocked by: Phase 01, Phase 02
- Blocks: nothing

## Overview
- **Date**: 2026-04-13
- **Priority**: P2
- **Effort**: 1.5 days
- **Status**: ✅ Done

Fix settings save model, accessibility pass, typography improvements, remaining JS/CSS cleanup.

## Key Insights
- Settings currently mixes live `SettingsChanged` updates with explicit Save — confusing UX
- Tool pills use `role="tab"` without full keyboard semantics — simplify to buttons
- Min text sizes 9-10px are too small for readability
- Remaining JS/CSS cleanup is polish, not structural

## Requirements
1. Settings uses pure draft model (changes local until Save)
2. Tool pills use buttons + `aria-pressed` (not tabs)
3. Min text size increased to 11px
4. `prefers-reduced-motion` applied broadly
5. Clean JS/CSS patterns

## Related Code Files
- `assets/popup/settings.js` (301 LOC) — form handling, live updates
- `assets/popup/settings.html` (146 LOC) — form structure
- `assets/popup/settings.css` (346 LOC) — form styles
- `assets/popup/popup.js` — tool switcher, state management
- `assets/popup/popup.css` — typography, animations
- `assets/popup/components.css` — button styles
- `src/ui/popup_controller.rs` (from Phase 1) — SettingsChanged handler
- `src/ui/webview_ipc.rs` — settings IPC contracts

## Implementation Steps

### Step 1: Settings draft model (M)
**settings.js**:
- Remove per-field `SettingsChanged` IPC sends (except theme preview)
- Keep all edits local in JS form state until Save click
- On Save: send single `SaveSettings` IPC with all fields
- On Cancel/Escape: restore form to last saved snapshot, close
- Add dirty-state tracking: `_isDirty` flag
- Show "Unsaved changes" indicator when dirty
- Disable Save button until dirty

**Rust (popup_controller.rs)**:
- Remove or simplify `SettingsChanged` handler — only handle theme preview
- Keep `SaveSettings` handler unchanged

**settings.css**:
- Style for dirty indicator
- Disabled Save button style

**settings.html**:
- Add dirty indicator element

Verify: edit settings → Cancel → verify no changes persisted → Save → verify persisted

### Step 2: Reset confirmation (S)
- "Reset to Defaults" button → show confirmation inline ("Are you sure? [Yes] [No]")
- Use destructive button style (red/warning)

### Step 3: Typography & readability (S)
**popup.css + components.css**:
- Increase all 9px text → 11px (`.char-count`, `.hint-inline`)
- Increase all 10px text → 11px (`.editor-label`, `.btn-toggle-md`)
- Review 11px text — keep or bump to 12px where appropriate
- Ensure contrast meets WCAG AA at new sizes

### Step 4: Tool pills a11y (S)
**popup.js**:
- Remove `role="tab"`, `aria-selected` from tool pills
- Add `aria-pressed="true"/"false"` for active/inactive
- Change to `role="button"` (default for `<button>`)

**popup.css**:
- No changes needed — `.tool-pill.active` styling stays

### Step 5: Broader reduced-motion (S)
**popup.css**:
```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```
- Remove the narrow `#editor-source-text` reduced-motion rule (superseded)
- Keep spinner animation (accessibility exception — users expect loading indication)

### Step 6: Add `aria-busy` + `aria-expanded` (S)
**popup.js**:
- Set `aria-busy="true"` on editor view during processing
- Set `aria-expanded` on Preview toggle and Source snippet toggle (already partial)

### Step 7: Remaining JS cleanup (S)
**popup.js**:
- Replace remaining `if/else` setState dispatch with view renderer map:
  ```js
  const VIEW_RENDERERS = { editor: renderEditor, processing: renderProcessing, ... };
  ```
- Clean up any remaining inline `style.display` not converted in Phase 1
- Ensure DOM cache is used consistently

### Step 8: Final verification (S)
- `powershell.exe cargo check && cargo clippy`
- Manual test: all settings flows, tool switching, accessibility keyboard navigation
- Theme test: dark/light/auto
- Reduced-motion test (Windows accessibility settings)

## Todo List
- [ ] Settings draft model — local-only edits until Save
- [ ] Remove per-field SettingsChanged IPC (except theme)
- [ ] Add dirty-state indicator
- [ ] Disable Save until dirty
- [ ] Reset to Defaults confirmation
- [ ] Increase min text size 9-10px → 11px
- [ ] Tool pills: buttons + aria-pressed (drop role="tab")
- [ ] Broad prefers-reduced-motion support
- [ ] aria-busy on processing
- [ ] aria-expanded on toggles
- [ ] JS renderer map cleanup
- [ ] Final cargo check + clippy
- [ ] Manual smoke test all flows

## Success Criteria
- Settings: Cancel discards changes, Save persists, dirty indicator visible
- No text below 11px anywhere in popup or settings
- Tool pills use semantic button model
- Reduced-motion respected for all animations (except spinner)
- `cargo check` + `cargo clippy` pass
- All flows work in light/dark/auto themes

## Risk Assessment
| Component | Risk | Mitigation |
|---|---|---|
| Settings save model change | MEDIUM | Preserve all persistence logic, only change when IPC fires |
| Typography size changes break layout | LOW | Test all views with longer text content |
| a11y changes break keyboard flow | LOW | Buttons are simpler than tabs, lower risk |

## Security Considerations
- Settings draft model must still validate HTTPS on Save
- API key masking in UI unchanged
- Reset to Defaults must clear API key from DPAPI storage

## Next Steps
→ Phase 4 (roadmap): Unit/integration tests for state transitions and AI clients
