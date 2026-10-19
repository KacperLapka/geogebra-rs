---
title: 'Tracer bullet: polynomial derivatives with simplify and tangent eval (2.1)'
type: 'feature'
ticket: '1'
created: '2026-10-19'
status: 'built'
baseline_revision: 'a8fa8667f8a49cc2b0b4bd57a45666d0c8d91b6d'
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

**Problem:** Ticket 2.1 needs native polynomial differentiation with mandatory simplification plus tangent evaluation at x0 in `core_math`, but investigation found this exact scope already present in-tree, so the work is proving its verify green and closing genuine gaps rather than building fresh modules.

**Approach:** Run the ticket's verify (`x^3+2x` f'/f'' plus tangent within eps<1e-12) and the full `core_math` suite; add only minimal hardening tests for real gaps found, touching nothing outside `core_math`.

</frozen-after-approval>

## Implementation Notes

Oneshot route: investigation found `diff.rs`, `simplify.rs`, and the solver `Marker`/tangent helpers already implemented with the ticket's verify test in place and the suite green, so the expected diff is zero or one small test (well under 100 lines, single crate) — direct implementation with no subagent handoff.

- Decision: kept scope to the ticket's polynomial tracer; transcendental `diff_call` arms and Brent hardening in-tree belong to entries 2.3/2.2 and were not touched.
- Files changed: `crates/core_math/src/solver.rs` (+17 test lines only).
- Surprise: the ticket's verify test (`tracer_bullet_polynomial_derivatives_simplify_and_tangent`) already existed verbatim and passed; the one genuine gap was tangent evaluation with slider params (only empty-params coverage), closed with `tangent_with_slider_param_tracks_slope` (`a*x^2`, a=2, eps 1e-12).
- Left alone: pre-existing `cargo fmt` diffs in `solver.rs` signatures and `lib.rs` re-export line — not this ticket's scope.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `cargo test -p core_math tracer_bullet_polynomial_derivatives_simplify_and_tangent` -- expected: pass, f'/f'' grid and tangent within 1e-12
- `cargo test -p core_math` -- expected: all pass, no regressions
- `cargo clippy -p core_math -- -D warnings` -- expected: clean
