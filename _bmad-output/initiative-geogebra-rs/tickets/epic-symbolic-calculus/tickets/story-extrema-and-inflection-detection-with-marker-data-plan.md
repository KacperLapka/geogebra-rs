---
title: 'Extrema and inflection detection with marker data (2.5)'
type: 'feature'
ticket: '5'
created: '2026-10-19'
status: 'built'
baseline_revision: '27f384648274a497b5bd8ad665ebe34958e48b92'
route: 'full'
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

**Problem:** Ticket 2.5 needs extrema and inflection detection in `core_math` — minima/maxima via `f'` zeros and inflection via `f''` sign change through the solver pipeline, with marker coordinates ready for render (R4/R6).

**Approach:** Reuse the `2.2` scan+Brent+filter pattern on the differentiated expressions from `2.3`/`2.4`, classifying `f'` zeros by `f''` sign into `LocalMin`/`LocalMax` and emitting `Inflection` only on true concavity sign change, proven on cubic and sin forms in-viewport.

## Boundaries & Constraints

**Always:** `differentiate`-then-`simplify` before any derivation or evaluation (AD-3); viewport-only markers; pole-jump brackets never emit (reuse `POLE_THRESHOLD` scan + residual filter); `core_math` stays dependency-free with no render/wasm types.

**Never:** No render, WASM ABI, or frontend changes (entry 2.6 owns integration); no changes to `brent`/`scan_sign_changes`/`find_roots` behavior, `diff` arms, or `simplify` folds; no new marker variants.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Cubic extrema | `x^3-3x` on `[-3,3]` | `LocalMax` at `x=-1`, `LocalMin` at `x=+1` with `y=f(x)` | No error expected |
| Sin extrema | `sin(x)` on `[-7,7]` | `LocalMin`/`LocalMax` alternating at `-3π/2..3π/2` | No error expected |
| Cubic inflection | `x^3-3x` on `[-3,3]` | Single `Inflection` at `x=0` | No error expected |
| Sin inflections | `sin(x)` on `[-7,7]` | `Inflection` at each `kπ` in range | No error expected |
| Monotonic, no extrema | `x^3+2x` (`f'=3x^2+2>0`) | Empty extrema list | No error expected |
| Pole-adjacent derivative | `1/x` (`f'=-1/x^2` never zero) | Empty extrema and inflection lists | No error expected |

</frozen-after-approval>

## Code Map

- `crates/core_math/src/solver.rs:17-36` -- `MarkerKind::{Root,LocalMin,LocalMax,Inflection}` + `Marker{kind,x,y}` shape to construct (only `Root` built today at `:223`).
- `crates/core_math/src/solver.rs:202-227` -- `find_roots` pattern to mirror: `scan_sign_changes` + `brent(_,_,tol,100)` + finite/residual (`1e-9`) filter + `1e-9` dedup.
- `crates/core_math/src/solver.rs:66-195` -- `brent` + `scan_sign_changes` (with `POLE_THRESHOLD` skip) reused as-is on `f'`/`f''` closures; do not modify.
- `crates/core_math/src/diff.rs:10-12` -- `differentiate` (always simplified) for deriving `f'` then `f''`; do not touch arms.
- `crates/core_math/src/lib.rs:18` -- add `find_extrema`, `find_inflections` to the solver re-export line.
- `crates/core_math/src/solver.rs:229-400` -- test module precedent (`find_roots` cubic/trig/pole tests) for the new tests.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/core_math/src/solver.rs` -- add `find_extrema` (`f'` zeros classified by `f''` sign: positive→`LocalMin`, negative→`LocalMax`, near-zero→skip) plus shared scan-refine-filter-dedup helper -- reuses the proven 2.2 pipeline on the derived expression.
- [ ] `crates/core_math/src/solver.rs` -- add `find_inflections` (`f''` zeros kept only on concavity sign change across the refined point) -- enforces the epic's `f''=0` WITH sign-change rule.
- [ ] `crates/core_math/src/lib.rs` -- export the two new functions -- makes them usable by render/wasm later.
- [ ] `crates/core_math/src/solver.rs` -- unit-test all six I/O Matrix rows (cubic/sin extrema + inflections, monotonic-empty, pole-empty) -- proves the ticket verify.

**Acceptance Criteria:**
- Given `x^3-3x` on `[-3,3]`, when `find_extrema` runs, then it returns exactly a `LocalMax` near `x=-1` and a `LocalMin` near `x=+1` with `y` equal to `f(x)` within `1e-12`.
- Given `sin(x)` on `[-7,7]`, when `find_extrema` runs, then minima and maxima alternate at odd multiples of `π/2` with coordinates within `1e-12`.
- Given `x^3-3x` on `[-3,3]`, when `find_inflections` runs, then it returns exactly one `Inflection` marker near `x=0`.
- Given `sin(x)` on `[-7,7]`, when `find_inflections` runs, then it returns an `Inflection` marker near each `kπ` in range and none elsewhere.
- Given `x^3+2x` and `1/x`, when both finders run, then both return empty with no pole markers emitted.

## Implementation Notes

## Plan Change Log

## Review Triage Log

## Design Notes

Extrema classification uses the second-derivative test at the refined `f'` zero (`f''>0` minimum, `f''<0` maximum); zeros where `f''` is also ~zero (e.g. `x^3` at `0`) are skipped rather than guessed. Inflection requires `f''` sign difference on the two sides of the refined zero, so tangent-flat points without concavity change never emit.

```rust
let fp = differentiate(expr, "x");
let fpp = differentiate(&fp, "x");
// extrema: zeros of fp, kind by sign of fpp(r)
// inflections: zeros of fpp kept when sign(fpp(r-d)) != sign(fpp(r+d))
```

## Verification

**Commands:**
- `cargo test -p core_math` -- expected: all pass, including new cubic/sin extrema, inflection, and empty-case tests
- `cargo clippy -p core_math -- -D warnings` -- expected: clean
