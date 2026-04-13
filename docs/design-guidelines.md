# Fast English — Design Guidelines

> **Last Updated:** 2026-03-19 (Phase 3: Theme & Polish — Complete)  
> **Status:** Active

---

## Design Direction

**"Fluent Midnight"** — Dark-mode-first, Windows 11 Fluent-inspired, layered surfaces with acrylic aesthetics. Includes light theme support with WCAG AA contrast compliance.

See full proposal: `plans/260319-ui-ux-brand-refresh-proposal/reports/design-260319-ui-ux-brand-refresh.md`

---

## Color Tokens (Current — Dark Mode)

| Token | Value | Usage |
|-------|-------|-------|
| `--surface` | `rgba(30, 30, 46, 1)` | Card surfaces |
| `--surface-elevated` | `rgba(37, 37, 54, 1)` | Elevated cards |
| `--mica-bg` | `rgba(26, 26, 38, 1)` | App background |
| `--primary` | `#3b82f6` | Primary actions |
| `--primary-light` | `#60a5fa` | Hover states |
| `--text-primary` | `#f0f0f0` | Headings |
| `--text-secondary` | `#a0a0b0` | Body text |
| `--text-muted` | `#6b6b80` | Hints (needs contrast fix → `#8b8ba0`) |
| `--success` | `#22c55e` | Success |
| `--error` | `#ef4444` | Error |
| `--warning` | `#fbbc2e` | Warning |

## Typography

- **Primary:** Plus Jakarta Sans (400-700)
- **Monospace:** Cascadia Code, Fira Code, Consolas
- **Base size:** 14px body, 13px controls
- **Line height:** 1.5-1.6 body, 1.2 headings

## Spacing Scale

| Name | Value |
|------|-------|
| xs | 4px |
| sm | 8px |
| md | 12px |
| lg | 16px |
| xl | 20px |

## Border Radius

| Name | Value | Usage |
|------|-------|-------|
| popup | 20px | Popup container |
| card | 12px | Cards, sections |
| btn | 10px | Buttons, inputs |

## Component Patterns

### Buttons
- Primary: gradient blue, uppercase, 11px bold
- Secondary: subtle bg, border, uppercase
- Ghost: transparent, border only
- Icon: 26-28px circle, no bg

### Textareas
- Vertically resizable (`resize: vertical`)
- Editor source: 4-line min, 10-line max, autosize on input
- Result output: 80px min, resizable
- Border radius: 12px (card radius)
- Manual resize triggers popup snap resize via IPC

### Accessibility Requirements
- Focus-visible: 2px solid primary, 2px offset
- Min touch target: 44×44px
- Color contrast: WCAG AA minimum (4.5:1)
- `prefers-reduced-motion` respected
- ARIA roles on interactive lists

## Implementation Status

| Feature | Status | Phase |
|---------|--------|-------|
| Tokens & reset | ✅ Complete | Phase 1 |
| Components library | ✅ Complete | Phase 2 |
| Keyboard navigation (roving tabindex) | ✅ Complete | Phase 2 |
| aria-live feedback regions | ✅ Complete | Phase 2 |
| Fluent Midnight dark theme | ✅ Complete | Phase 3 |
| Light theme with WCAG AA contrast | ✅ Complete | Phase 3 |
| Icons.js centralized icon system | ✅ Complete | Phase 3 |
| Theme toggle (Auto/Dark/Light) | ✅ Complete | Phase 3 |
| Replace confirmation dialog | ✅ Complete | Phase 3 |
| Settings cancel/reset buttons | ✅ Complete | Phase 3 |
| Micro-interactions & animations | ✅ Complete | Phase 3 |
| Lightning Pen brand assets | ✅ Complete | Phase 3 |
