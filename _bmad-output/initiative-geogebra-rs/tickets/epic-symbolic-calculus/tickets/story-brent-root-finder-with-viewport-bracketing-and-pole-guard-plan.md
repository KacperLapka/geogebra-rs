---
title: 'Brent root finder with viewport bracketing and pole guard (2.2)'
type: 'feature'
ticket: '2'
created: '2026-10-19'
status: 'built'
baseline_revision: '16ce70ed9edd7f9be894887c5d23e8fac498c28c'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Ticket 2.2 needs viewport-wide Brent root finding in `core_math` — sign-change bracketing over a viewport scan plus Brent refine with `POLE_THRESHOLD` pole-vs-zero guard per AD-3 (R3) — proven on cubic and trig forms at eps<1e-12 with no marker on pole jumps.

**Approach:** Harden the existing `solver.rs` scan+Brent pipeline in place and extend its native `cargo test` coverage to prove cubic multi-root, trig multi-root, and pole-skip behavior at the ticket's precision.

</frozen-after-approval>

## Implementation Notes

Oneshot route: `brent`, `scan_sign_changes`, and `POLE_THRESHOLD=1e6` already exist in `crates/core_math/src/solver.rs` with line-root and `1/x` pole tests green (34 tests passing); the gap to the ticket verify is only missing cubic/trig multi-root tests at 1e-12 plus pole-jump coverage (e.g. tan), well under 100 lines in a single file — direct implementation with no subagent handoff.

- Decision: keep scope to `core_math` native solver hardening; do not change `Marker` shape, tangent API, WASM ABI, or render sampler (entry 2.6 owns integration).
- Files expected: `crates/core_math/src/solver.rs` (tests only, plus minimal scan/brent fix only if a new test exposes one).
- Decision: fixed `brent` interval-convergence to require `|f(b)|<1e-9` before returning `Some` — pole brackets previously converged to the pole with residual ~1e12 and were reported as roots. Added `find_roots` (`scan`+`brent`+residual filter+1e-9 dedup → `MarkerKind::Root`) and exported it in `lib.rs`; this collapses exact-zero duplicate brackets (cubic `x^3-3x` gave 4 brackets for 3 roots) and drops pole jumps.
- Files changed: `crates/core_math/src/solver.rs` (+~110 lines: brent fix, find_roots, 3 tests), `crates/core_math/src/lib.rs` (export find_roots).
- Surprise: `tan(x)` pole brackets at ±π/2 have endpoint magnitudes ~76/358 and slope ~27k, far below `POLE_THRESHOLD=1e6`, so scan alone cannot skip them — residual filtering in `brent`/`find_roots` is what enforces the pole-vs-zero guard. Verified `cargo test -p core_math` 37 passed, `cargo clippy -p core_math -- -D warnings` clean.

## Plan Change Log

## Review Triage Log

- Slope guard missing (`solver.rs:186`): false — AD-3 parenthetical describes pole signature, not a mandated slope computation; literal `slope>1e6` would miss tan poles (measured slope ~27k) while magnitude+residual correctly drops them; tan/1/x tests prove no pole markers.
- `brent`/`find_roots` don't reference `POLE_THRESHOLD`: false — pipeline uses it in `scan` (`solver.rs:186`); `brent` takes a closure so residual is the correct second layer; probe showed tan pole endpoints ~76/358 <1e6, so residual is required and sufficient.
- Hardcoded `1e-9` vs `eps<1e-12`: false — caller `tol=1e-12` controls interval precision; `1e-9` is pole-rejection acceptance; new tests assert `<1e-12` on both `x` and `y` and pass with residuals ~1e-16.
- Dedup `1e-9` ignores `tol`: low, rejected — distinct viewport roots sit `dx~1e-2` apart; sub-`1e-9` pairs share one bracket and refine to one root anyway; bracket-aware dedup exceeds a simple correction.
- `scan` double-emits exact-zero: false — adjacent brackets sharing an exact-zero sample are correct bracket semantics; aggregation dedup belongs in `find_roots`; direct `scan`+`brent` callers using `brackets[0]` are unaffected.
- `POLE_THRESHOLD` path untested (non-finite guard hits first): low, rejected — pre-existing path also missed by the original `steps=4` test; finite `>1e6` samples need `<1e-6` pole alignment vs `dx~1e-2`, rarely hit; contrived narrow-viewport test adds complexity beyond ticket verify.
- `brent` drops flat true roots (`None` on `interval<tol` with large residual): false — by continuity a simple zero has small `|f|` when the interval is `<1e-12` (e.g. `x^3`: `|x|<1e-12` → `|f|<1e-36`); large residual there means pole/discontinuity, correctly rejected.

## Verification

**Commands:**
- `cargo test -p core_math` -- expected: all pass, including new cubic multi-root, trig multi-root, and pole-skip tests at eps<1e-12
- `cargo clippy -p core_math -- -D warnings` -- expected: clean
