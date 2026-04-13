---
title: "UI/UX Optimization & Refactoring"
description: "Split popup.rs, inline processing/error, simplify result actions, settings draft model"
status: done
priority: P1
effort: 6d
branch: main
tags: [refactor, ui-ux, popup, webview]
created: 2026-04-13
mode: hard
blockedBy: []
blocks: []
---

# UI/UX Optimization & Refactoring

**Source**: [brainstorm.md](brainstorm.md)

## Phases

| Phase | Title | Effort | Status |
|---|---|---|---|
| 01 | [Structural Refactor](phase-01-structural-refactor.md) | 2d | ✅ Done |
| 02 | [Core UX Improvements](phase-02-core-ux.md) | 2.5d | ✅ Done |
| 03 | [Settings & Polish](phase-03-settings-polish.md) | 1.5d | ✅ Done |

## Scope

**In scope**: popup.rs modularization, inline processing/error, result simplification, editor clarity, settings draft model, basic a11y, JS cleanup  
**Out of scope**: AppState decomposition (deferred), streaming AI, new tools, tests (Phase 4 roadmap)

## Key Decisions

- Rust `PopupState` enum stays unchanged — UX changes are frontend-only reinterpretation
- Tool pills → regular buttons + `aria-pressed` (not full tab semantics)
- `AppState` decomposition deferred — file extraction gives most value at lower risk
- Every extraction step verified via `powershell.exe cargo check`
