# Execution Plan: UI/UX Optimization & Refactoring

**Team**: solo (sequential execution)  
**Generated**: 2026-04-13  
**Total effort**: ~6 working days

## Execution Order

Phases execute sequentially — each depends on the previous.

```
Phase 01 (2d) ──→ Phase 02 (2.5d) ──→ Phase 03 (1.5d)
  Refactor           Core UX              Polish
```

## Phase 1: Structural Refactor — Steps

| # | Task | Effort | Files |
|---|------|--------|-------|
| 1.1 | Extract `popup_ai.rs` | 2h | `src/ui/popup.rs` → `src/ui/popup_ai.rs` |
| 1.2 | Extract `popup_runtime.rs` | 3h | `src/ui/popup.rs` → `src/ui/popup_runtime.rs` |
| 1.3 | Extract `popup_view.rs` | 3h | `src/ui/popup.rs` → `src/ui/popup_view.rs` |
| 1.4 | Extract `popup_controller.rs` | 5h | `src/ui/popup.rs` → `src/ui/popup_controller.rs` |
| 1.5 | JS/CSS enabling cleanup | 2h | `assets/popup/popup.js`, `assets/popup/popup.css` |
| 1.6 | Verify & measure | 1h | all `src/ui/*.rs` |

**Checkpoint**: `cargo check` + `cargo clippy` pass, all files ≤400 LOC, popup flow unchanged

## Phase 2: Core UX Improvements — Steps

| # | Task | Effort | Files |
|---|------|--------|-------|
| 2.1 | Inline processing | 6h | `popup.js`, `popup.css`, `index.html`, `popup_view.rs` |
| 2.2 | Inline error banner | 3h | `popup.js`, `popup.css`, `index.html` |
| 2.3 | Result action simplification | 2h | `popup.js`, `popup.css`, `index.html` |
| 2.4 | Editor clarity (tool labels) | 1.5h | `popup.js` |
| 2.5 | ResizeObserver upgrade | 2h | `popup.js` |
| 2.6 | Integration test | 1.5h | manual QA |

**Checkpoint**: Processing shows source text, errors inline, result actions clear, popup sizing correct

## Phase 3: Settings & Polish — Steps

| # | Task | Effort | Files |
|---|------|--------|-------|
| 3.1 | Settings draft model | 4h | `settings.js`, `settings.html`, `settings.css`, `popup_controller.rs` |
| 3.2 | Reset confirmation | 0.5h | `settings.js`, `settings.html` |
| 3.3 | Typography pass | 1h | `popup.css`, `components.css` |
| 3.4 | Tool pills a11y | 0.5h | `popup.js` |
| 3.5 | Reduced-motion + aria | 1h | `popup.css`, `popup.js` |
| 3.6 | JS cleanup + verify | 1h | `popup.js`, `cargo check` |

**Checkpoint**: Settings save model consistent, no text <11px, a11y improved, all passes green

## Critical Guardrails

1. **After every Rust extraction**: `powershell.exe cargo check`
2. **Preserve exactly**: `active_request_id` filtering, `ignore_auto_until`, `manual_capture_in_flight`, Win32 shutdown
3. **PopupState enum**: do NOT modify — all UX changes are frontend-only
4. **Settings HTTPS validation**: must still enforce on Save path
