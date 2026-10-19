# Walkthrough: 2.1 polynomial derivatives tracer (commit 4f62fd0)

## Block 1: Intent ✅ done

- [x] Visited — accepted unchanged.

Ticket 2.1 (epic-symbolic-calculus) states the intent verbatim:

> Stands up core_math diff.rs/simplify.rs skeleton plus solver marker struct stub; differentiates polynomial f, simplifies f'/f'', evaluates tangent at x0 natively.

Verify, also verbatim from the ticket:

> cargo test differentiates f(x)=x^3+2x, simplifies f'/f'', and evaluates tangent at x0 within eps<1e-12.

Source note: intent pasted verbatim from ticket-tree entry 2.1; the plan's frozen Intent adds that investigation found this scope already in-tree, so the commit proves the verify green and closes one genuine gap (slider-param tangent coverage). Unchanged during review.

## Block 2: Broad strokes ⬅ current

- [ ] Unvisited — say the word when this block is done.

A 17-line test-only addition on top of pre-existing scope: the symbolic differentiation, simplification, tangent helpers, marker stub, and even the ticket's own verify test were already in-tree and green. Open these three in order and the mechanism is clear without reading anything else:

- [new param-tangent test](../../../crates/core_math/src/solver.rs) — the commit's only code change.
- [tangent helpers](../../../crates/core_math/src/solver.rs) — `tangent_coeffs` / `tangent_at`, exercised with and without params.
- [ticket verify test](../../../crates/core_math/src/solver.rs) — pre-existing `x^3+2x` f'/f'' grid plus tangent within 1e-12.

## Block 3: Param-threaded tangent coverage

- [ ] Unvisited — say the word when this block is done.

The one genuine gap: tangent evaluation was only tested with empty params, while the slider flow evaluates with params bound. The new test pins `f(x) = a*x^2` with `a = 2` at `x0 = 1.0`, asserting `(y0, slope) = (2.0, 4.0)` and the tangent line at three offsets, all within 1e-12. Places:

- [new test](../../../crates/core_math/src/solver.rs) — `tangent_with_slider_param_tracks_slope`.
- [helpers under test](../../../crates/core_math/src/solver.rs) — `tangent_coeffs` derives slope via `differentiate(expr, "x").eval`, treating `Param` as constant zero.

## Block 4: Pre-existing scope (proved, not changed)

- [ ] Unvisited — say the word when this block is done.

No changes here; this block exists so the review confirms what the commit relies on: `differentiate` always simplifying before returning, const-fold/identity simplification, `Marker`/`MarkerKind` stub shape, and the verbatim ticket-verify test — all in [solver.rs](../../../crates/core_math/src/solver.rs), [diff.rs](../../../crates/core_math/src/diff.rs), and [simplify.rs](../../../crates/core_math/src/simplify.rs), with the full `core_math` suite (34 tests) green.

## Block 5: Periphery

- [ ] Unvisited — say the word when this block is done.

- [run plan file](../../tickets/epic-symbolic-calculus/tickets/story-tracer-bullet-polynomial-derivatives-with-simplify-and-tange-plan.md) — oneshot plan documenting the verify-and-harden run; no product impact.
