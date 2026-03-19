# Fast English — Design Guidelines

> **Last Updated:** 2026-03-19 (Phase 2: Core UX & Component System)  
> **Status:** Active (brand refresh pending v1.1+)

---

## Design Direction

**"Fluent Midnight"** — Dark-mode-first, Windows 11 Fluent-inspired, layered surfaces with acrylic aesthetics.

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

### Accessibility Requirements
- Focus-visible: 2px solid primary, 2px offset
- Min touch target: 44×44px
- Color contrast: WCAG AA minimum (4.5:1)
- `prefers-reduced-motion` respected
- ARIA roles on interactive lists

## Known Issues (Phase 2 Resolution)

1. ✅ **RESOLVED** — Tokens duplicated across popup.css and settings.css → Centralized in tokens.css + components.css layer
2. ✅ **RESOLVED** — No shared CSS file → Created components.css with reusable patterns
3. ⏳ No light theme support (planned v1.1+)
4. ⏳ Brand refresh (Fluent Midnight proposal pending approval)
5. **Keyboard Navigation**: Roving tabindex pattern implemented in tool list (Phase 2)
6. **Accessibility**: aria-live regions added to feedback toasts and processing label (Phase 2)
