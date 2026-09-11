# Fast English — Design Guidelines

> **Last Updated:** 2026-08-28 (v0.6.0 UI rebuild)
> **Status:** Pointer — the canonical design authority lives elsewhere

## Where the design authority lives

This file used to carry its own copy of the colour, type and spacing tables.
That copy rotted: by v0.6.0 every value in it was wrong. It is now a pointer, so
there is one source of truth instead of two that disagree.

| What you need | Read |
|---|---|
| Colour / type / spacing / radius / shadow tokens | `assets/popup/tokens.css` — the machine source, dark at `:root`, light at `:root[data-theme="light"]` |
| Design rationale, component specs, interaction and a11y rules | [`design/ui-restyle/design-spec.md`](design/ui-restyle/design-spec.md) |
| What each screen contains and how they connect | [`design/ui-restyle/screens.md`](design/ui-restyle/screens.md) — anchor-verified inventory |
| What each screen should look like | [`design/ui-restyle/mockups/`](design/ui-restyle/mockups/) — six static HTML mockups |

**Rule:** when a token value changes, change it in `tokens.css`. Do not restate
values here. `design-spec.md` §5 mirrors the token sheet and says which tokens
were kept versus introduced.

## Direction

**"Fluent Midnight"** — dark-first, Windows 11 Fluent-inspired. Light theme is
supported and must be verified separately; WCAG AA (4.5:1 body text) applies to
both. The system font stack is Segoe UI Variable with Cascadia Code for
monospace — chosen because it is native to the platform. Both windows are opaque
(`with_transparent(false)`), so the mockups' rounded 20px window corners are not
reachable; that deviation is recorded in the spec.

## Non-obvious constraints a designer must respect

These are properties of the implementation, not preferences. Changing them
breaks behaviour:

- **The popup measures itself.** `autosizeEditorSourceTextarea` reads the
  computed `line-height`, padding and border of `#editor-source-text`, and
  `applyMeasuredHeight` measures `#popup-root` to size the native window.
  Changing those properties changes window geometry.
- **`POPUP_INITIAL_HEIGHT`** (`src/ui/popup_position.rs`) must match the real
  chrome height, or the first frame is visibly wrong before the measurement
  lands. It is 326 as of v0.6.0, derived band by band from the rebuilt layout.
- **Exactly one region scrolls per view.** `scrollRegionForCurrentView` hardcodes
  which element that is. Never put a stylesheet `max-height` on one of them, and
  never let a later rule beat `.scroll-region--at-max`'s `overflow-y: auto` —
  both mistakes clip content instead of scrolling it, and both have shipped
  before.
- **Some containers are filled by JS** (`#tool-switcher`, `#history-list`,
  `#result-diff`, …) and cannot carry hand-authored children. Restyling their
  contents means changing the JS that builds them.
- **Minimum interactive target is 32px** (design-spec §6), not the 44px mobile
  figure — this is a desktop popup driven by mouse and keyboard.

## Accessibility baseline

Focus ring on every interactive element via `--focus-ring` (plus a
`forced-colors` outline fallback, because Windows High Contrast suppresses
`box-shadow`); status never carried by colour alone; `prefers-reduced-motion`
respected; disabled controls at 0.45 opacity with no hover response.

The tool pills are plain buttons with `aria-pressed`. They deliberately do **not**
use `role="tab"`: that would oblige roving-tabindex and arrow-key navigation,
which is not implemented.

## Known gap

`applyTheme` writes the raw preference — including `"auto"` — into
`data-theme`, but `tokens.css` themes only on `[data-theme="light"]`. A user on
the default `auto` setting therefore gets the dark palette regardless of their
Windows setting. Pre-existing; QA covers dark and light explicitly.
