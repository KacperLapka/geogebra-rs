---
title: 'Rational quotient-power rules with simplification hardening (2.4)'
type: 'feature'
ticket: '4'
created: '2026-10-19'
status: 'built'
baseline_revision: '9454a516336e133e85f854b4caaa77f2f324e53e'
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

**Problem:** Ticket 2.4 needs rational and power differentiation hardened in `core_math` — quotient/power rules with identity elimination, sign normalization, and constant folding for rational forms — proven on rational and power forms including second derivatives with no unbounded AST growth on second derivation (R1, AD-3).

**Approach:** Add the missing span-insensitive rational folds (`x-x→0`, `x/x→1`) to `simplify.rs` so quotient-rule output collapses, and extend native `cargo test` coverage for rational/power first and second derivatives plus AST-growth bounds.

</frozen-after-approval>

## Implementation Notes

Oneshot route: quotient rule (`diff.rs:47-60`), `diff_pow` const-exp/const-base/log-diff (`diff.rs:72-113`), and constant/identity folding (`simplify.rs:89-162`) already exist with the suite green at 40 tests; probes showed numerics exact but rational ASTs grow ~3x per derivation (`(x^2+1)/(x+1)`: 9→19→47→135 nodes) because `x-x` and `x/x` never fold (spans defeat derived `PartialEq`). Two small folds plus tests fit well under 100 lines across two files — direct implementation with no subagent handoff.

- Decision: keep scope to `core_math` `simplify.rs` folds plus `diff.rs`/`simplify.rs` tests; do not touch transcendental arms (2.3), solver/tangent (2.1/2.2), WASM ABI, or render.
- Files expected: `crates/core_math/src/simplify.rs` (span-insensitive `x-x→0`, `x/x→1` + tests), `crates/core_math/src/diff.rs` (rational/power derivative tests only).
- Decision: added `same_shape` span-insensitive structural equality plus `x-x→0` (Sub) and `x/x→1` (Div) folds after the existing zero/one checks. Literal `0/0` still folds to `NaN` via IEEE constant folding (number-number inputs return before the shape checks); `0/non-number` keeps the pre-existing `→0` fold. Quotient/power numerics were already exact; the folds collapse degenerate quotient-rule output (`d(x/x)` is now exactly `0`).
- Files changed: `crates/core_math/src/simplify.rs` (+~50 impl lines, +1 test), `crates/core_math/src/diff.rs` (+2 tests).
- Surprise: my hand-check of `(x^2+1)/(x+1)'` at 1.5 said 0.64 but the code's 0.68 is right (`[2·1.5·2.5−3.25]/6.25`); growth probe showed `9→19→47→135` nodes justifying the folds, though realistic quotients still grow via non-literal patterns like `(x+1)−x` left unfolded by design. Verified `cargo test -p core_math` 43 passed, `cargo clippy -p core_math -- -D warnings` clean.

## Plan Change Log

## Review Triage Log

- Growth bound `<60` vacuous / folds unproven (`diff.rs`): false — fold-proof lives in sibling structural assertions (`d(x/x)` is exactly `0`, `(x+1)/(x+1)→1`); the node bound is a termination/regression guard, and second derivation terminates finite (47 nodes).
- Const-base/general power arms untested (`diff_pow`): medium, patched — added `2^x` and `x^x` numeric cases at `1e-12` (probe-verified exact); no impl change needed.
- `x-x→0` unsound at singularities (`simplify.rs`): false — generic-point convention shared with pre-existing `0/x→0` and `x*0→0` folds; ticket-mandated identity elimination; `scan`/`brent` guard non-finite and no reachable caller harm was shown.
- `x/x→1` removes holes/poles (`simplify.rs`): false — same convention; only degenerate identical-quotient inputs collapse while genuine poles (`1/x`) never match the shape; ticket-mandated and no reachable render/solver harm shown.
- `0/0→0` convention claim vs `NaN` code (`simplify.rs`): low, patched doc only — literal `0/0` correctly folds to IEEE `NaN` before shape checks; note corrected, code untouched; no test asserted the wrong claim.

## Verification

**Commands:**
- `cargo test -p core_math` -- expected: all pass, including new rational/power first- and second-derivative tests and AST-growth bound tests
- `cargo clippy -p core_math -- -D warnings` -- expected: clean
