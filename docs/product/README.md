# Product Docs

Per-domain product behavior for Fast English. Add a file here when a domain
outgrows the overview; until then the canonical product source is
[`../project-overview-pdr.md`](../project-overview-pdr.md), with per-release
truth in the [release notes](../README.md#release-notes).

Name files after real domains — `writing-tools.md`, `history.md`,
`translation.md` — not after phases or tickets.

## Update rule

When behavior changes:

1. Update the affected product document, or the overview if no domain file
   exists yet.
2. Update the active execution plan when complex work uses one.
3. Add a decision under [`../decisions/`](../decisions/) only when future work
   must inherit a consequential product, architecture, data, security,
   compatibility, or validation choice.

State what was verified and how. A behavior nobody has seen running in the app
is documented as unverified, not as shipped.
