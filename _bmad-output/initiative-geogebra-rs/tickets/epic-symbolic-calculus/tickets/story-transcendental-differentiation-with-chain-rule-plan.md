---
title: 'Transcendental differentiation with chain rule (2.3)'
type: 'feature'
ticket: '3'
created: '2026-10-19'
status: 'built'
baseline_revision: '7aff42a462e88adee8c4debab3482588bc2b07a3'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Ticket 2.3 needs transcendental differentiation in `core_math` — sin/cos, exp, ln rules plus chain rule and constant folding for nested forms — proven on compositions including second derivatives with simplified canonical output (R1, AD-3).

**Approach:** Prove the existing `diff.rs` chain-rule pipeline against the ticket verify and add only the missing native `cargo test` coverage for nested compositions, second derivatives, and canonical simplification.

</frozen-after-approval>

## Implementation Notes

Oneshot route: `diff_raw` chain rule (`diff.rs:63-67`), `diff_call` arms for Sin/Cos/Tan/Exp/Ln/Log/Sqrt (`diff.rs:116-142`), and `differentiate`-then-`simplify` invariant (`diff.rs:10-12`) already exist with the suite green; the gap to verify is only composition/second-derivative/canonical-output tests, well under 100 lines in a single file — direct implementation with no subagent handoff.

- Decision: keep scope to `core_math` native diff tests; do not touch `diff_pow`, solver, tangent, WASM ABI, or render (other entries own those).
- Files expected: `crates/core_math/src/diff.rs` (tests only, plus minimal impl fix only if a new test exposes one).
- Decision: no impl fix needed — probe showed `sin(x^2)`, `exp(sin(x))`, `ln(x^2+1)`, `cos`, `sin(cos(x))` match analytic `std` references exactly (0 or ~1e-16), second derivatives exact, and `exp(0)*x` folds to `Number(1.0)`. Added 3 tests only (`diff.rs` +~85 lines).
- Files changed: `crates/core_math/src/diff.rs` (tests only).
- Surprise: none in impl; my first probe lambda for `sin(cos(x))` used `sin(cos)` instead of `cos(cos)` (diff 0.18) — corrected to `-sin(x)·cos(cos(x))` in the test. Verified `cargo test -p core_math` 40 passed, `cargo clippy -p core_math -- -D warnings` clean.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `cargo test -p core_math` -- expected: all pass, including new sin/exp/ln composition, chain-rule, second-derivative, and canonical-simplification tests
- `cargo clippy -p core_math -- -D warnings` -- expected: clean
