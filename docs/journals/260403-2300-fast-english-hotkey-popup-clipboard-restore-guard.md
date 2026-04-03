# Hotkey Routing and Clipboard Restore Guard Fix Landed

**Date**: 2026-04-03 23:00
**Severity**: High
**Component**: Hotkey pipeline, popup trigger flow, clipboard fallback
**Status**: Resolved

## What Happened
We fixed a behavior leak where popup opening logic mixed hotkey and tray intents, and clipboard restoration could be skipped on early returns. The result was inconsistent popup behavior and fragile clipboard state handling during fallback capture.

## The Brutal Truth
This hurt because we were carrying ambiguous event semantics in a sensitive path: one signal (`OpenPopupRequested`) was overloaded for two user intents. That made debugging noisy and confidence low. The clipboard path was worse emotionally: we *thought* restore was covered, but early-return branches proved that confidence was premature.

## Technical Details
- `src/hotkey.rs`: now forwards only matching hotkey id + pressed state; added unit test for dispatch correctness.
- `src/ui/popup.rs`: split `HotkeyPressed` from `OpenPopupRequested`; hotkey path now opens only after successful manual capture, tray path may still open empty by design; added gating helper + unit tests.
- `src/selection/clipboard_fallback.rs`: added restore guard so clipboard text restoration executes even on early return paths.
- Docs updated: `README.md`, `docs/codebase-summary.md`.
- Verification: `cargo build` passed; `cargo test` passed with **8 tests**.

## What We Tried
1. Kept shared open event and patched conditions inline — rejected; still conflated tray vs hotkey intent.
2. Added targeted branching in popup handler only — partial fix, but signal ambiguity persisted upstream.
3. Final approach: separate event semantics + explicit gating helper + guard-based clipboard restoration.

## Root Cause Analysis
We encoded two different intents behind one event and trusted branch discipline to keep behavior correct. That was the core design mistake. Clipboard restore logic also relied on control-flow assumptions instead of structural guarantees (guard pattern), so early exits could bypass restoration.

## Lessons Learned
- Never overload UI events with mixed intent; model intent explicitly at the type/event level.
- For resource restoration (clipboard, temp state), prefer RAII/guard patterns over “remember to restore before return.”
- Unit tests around event routing should be mandatory whenever trigger semantics are changed.

## Next Steps
- Owner: core app maintainers (main branch).
- By next patch cycle: evaluate rich clipboard preservation path; current restore remains text-only.
- Add focused test coverage for tray-open-empty expectation to prevent future regressions.
- Docs impact: **minor**, already reflected in README and codebase summary.

