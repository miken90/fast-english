# 0002 Recent-result history is encrypted at rest, not plaintext

Date: 2026-08-17

## Status

Accepted

## Context

Recent Results (`src/history.rs`, `persistence::HistoryStore`) keeps the last
20 completed AI results — the user's own writing, both source and output —
so a result is not lost the moment the popup that showed it closes. Settings,
including the API key, already persist under `%APPDATA%/FastEnglish/`, with
the API key protected via Windows DPAPI (`persistence.rs`). History needed the
same choice made explicitly: store it as plain JSON next to `settings.json`
(cheap, simple, matches how most local-first apps keep a recents list), or
reuse the DPAPI path the API key already uses.

## Decision

`history.dat` is written and read through the same DPAPI user-scope
encryption as the API key, via `HistoryStore` in `persistence.rs`. History is
the user's own writing rather than app configuration, so plaintext JSON would
have been a choice to leak it: the DPAPI helpers already existed and cost
nothing extra to reuse here. A missing, truncated, undecryptable, or malformed
file degrades to an empty history rather than failing startup.

## Alternatives Considered

1. Plain JSON next to `settings.json` — rejected. Cheapest to implement and
   matches the existing `settings.json` precedent, but leaves the user's
   selected/rewritten text (which can include anything they were writing,
   including sensitive drafts) readable by anything with filesystem access to
   that user's profile.
2. A separate encryption scheme from the API key (e.g. a passphrase-derived
   key) — rejected as unrequested scope; DPAPI's existing user-scope guarantee
   already matches the threat this addresses, and a second scheme would add
   key-management surface for no additional protection against the actual
   threat model below.

## Consequences

Positive:

- History is not sitting in a plain file for anything on the machine to read;
  it is readable only by the same Windows account.
- Reuses proven DPAPI helpers; no new dependency or key-management code.

Tradeoffs:

- DPAPI protects against another Windows account, a stolen disk, or a
  filesystem-level backup. It does **not** protect against other code running
  as the same user — that thread model is unchanged from the API key's.
- History is capped at 20 entries / 8000 chars per field, re-applied on load
  so the file is not the authority on its own limits; this bounds the file
  size but is a product cap, not a security property.

## Follow-Up

- None planned.
