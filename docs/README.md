# Documentation Map

Fast English is a Windows-only tray app (Rust + tao/wry over WebView2) that
rewrites selected text through an OpenAI-compatible endpoint. Start with the
smallest authoritative surface below.

## Product

- [`project-overview-pdr.md`](project-overview-pdr.md): what the product is,
  who it is for, and the four writing tools it ships.
- [`project-roadmap.md`](project-roadmap.md): phases, what is delivered, and
  the known limitations of each release.
- [`product/`](product/): per-domain product behavior, once a domain outgrows
  the overview.

## Engineering

- [`system-architecture.md`](system-architecture.md): the canonical
  architecture — process model, IPC, AI layer, timeout and failure policy,
  window sizing. This repo predates the harness, so it keeps this filename
  instead of a generic `ARCHITECTURE.md`.
- [`codebase-summary.md`](codebase-summary.md): module map and where each
  concern lives.
- [`code-standards.md`](code-standards.md): conventions this repo actually
  follows.
- [`design-guidelines.md`](design-guidelines.md): design direction, the implementation constraints a designer must respect, and pointers to the canonical token/spec/mockup sources under `design/ui-restyle/`.
- [`patterns/encoding-invariants.md`](patterns/encoding-invariants.md):
  invariants worth enforcing mechanically rather than by review.

## Process

- [`WORKFLOW.md`](WORKFLOW.md): request shape, planning, validation, and
  completion.
- [`deployment-guide.md`](deployment-guide.md): build, package, and release —
  including the Windows-only build rule and the version source of truth.
- [`decisions/`](decisions/): lasting choices future work must inherit.
- [`plans/`](plans/): working memory for work that needs it, active and
  completed.
- [`templates/`](templates/): decision, plan, runbook, and harness-improvement
  structures.

## Release Notes

One file per shipped version, newest first:
[0.6.0](release-notes-0.6.0.md) ·
[0.5.2](release-notes-0.5.2.md) ·
[0.5.1](release-notes-0.5.1.md) ·
[0.5.0](release-notes-0.5.0.md) ·
[0.4.0](release-notes-0.4.0.md) ·
[0.3.0](release-notes-0.3.0.md)

Each states plainly what was verified and what was not. Read the newest one
before assuming a feature works in the running app.

## Authority

Code, tests, and observable application behavior are authoritative. Where a
document disagrees with the code, the code wins and the document is a bug.
