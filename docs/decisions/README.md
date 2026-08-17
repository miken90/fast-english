# Decisions

Decision records preserve lasting product, architecture, data ownership,
security, compatibility, and validation choices that future work must inherit.

Use `docs/templates/decision.md`. Task-local implementation choices remain in
the active execution plan and do not require a separate decision.

An installed consumer begins with no fabricated decisions. Add local decision
documents here as real choices are accepted, then index them in this file.

## Index

- [0001 Manual resize wins for the popup session](0001-manual-resize-wins-for-session.md) — a user's frame-drag wins for that popup session; content-driven auto-sizing resumes on the next hide→show.
