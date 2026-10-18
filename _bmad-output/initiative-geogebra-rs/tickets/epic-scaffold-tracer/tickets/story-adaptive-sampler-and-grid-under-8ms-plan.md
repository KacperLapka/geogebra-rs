---
title: 'Adaptive sampler and grid under 8ms'
type: 'feature'
ticket: 4
created: '2026-10-18'
status: done
baseline_revision: '1fbf9d082045b63952709199985235b4b4844b6a'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `render/curve.rs` samples uniformly, wasting vertices on flat spans and undersampling high-curvature regions; `render/grid.rs` emits only major 1/2/5 ticks with no log subdivisions, so zoom relabelling jumps.

**Approach:** Add adaptive sub-pixel subdivision to the curve sampler (coarse base pass plus curvature-driven midpoint refinement capped at a 2000-point buffer) and log-scale minor subdivisions plus stable label steps to the grid, keeping full regeneration under 8 ms.

## Boundaries & Constraints

**Always:** f64 math with `POLE_BREAK = 1e6` pole/NaN guards; keep `wasm_bridge::{sample_points, grid_lines}` signatures stable; `PRIMARY | GL` pipeline and `viewport_revision` contract untouched; 100% client-side compute, no `eval()`.

**Never:** Hover/tooltip depth (1.5 owns it); slider/JSON shape (1.6); parser completion (1.3 done); WGSL shader rewrites or surface attach; pushing to remote.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Flat line | `f(x)=2x+1`, default viewport | Near-base sample count, exact endpoints | No error expected |
| High curvature | `sin(50x)`, narrow viewport | Refined midpoints, deviation within sub-pixel tolerance | Cap at 2000 points, stop refining |
| Pole | `1/x` across 0 | NaN break preserved through refinement | Neighbors of pole stay NaN, no spike |
| Zoom sweep | viewport width 20 → 0.002 | Tick step shrinks 1/2/5-decade, minors subdivide, labels stable | Degenerate/empty range returns empty ticks |
| Budget | 2000-pt regen, release profile | Completes < 8 ms measured | No error; test asserts budget |

</frozen-after-approval>

## Code Map

- `crates/render/src/curve.rs` -- owns sampling: replace uniform loop with `sample_curve` (adaptive wrapper, same signature) + `sample_curve_adaptive` with tolerance + `MAX_POINTS = 2000`; reuse `guard`/`POLE_BREAK`, `Viewport`, `Expr::eval`.
- `crates/render/src/grid.rs` -- owns ticks: keep `grid_ticks`/`viewport_ticks` signatures, add decade step + `minor_ticks`/`grid_labels` log subdivisions; reuse `Viewport`.
- `crates/render/src/lib.rs` -- re-export new sampler/grid helpers; no pipeline changes.
- `crates/wasm_bridge/src/lib.rs` -- READ ONLY: `sample_points` (clamps 2..=4000) and `grid_lines` must keep passing unchanged.
- `crates/render/src/pipeline.rs`, `viewport.rs` -- DO NOT CHANGE: 1.2 viewport/revision loop stays as-is.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/curve.rs` -- adaptive midpoint subdivision with screen-x tolerance, pole-aware refinement, 2000-pt cap -- R3 core
- [ ] `crates/render/src/grid.rs` -- log-decade minor subdivisions + stable label formatting -- R4 core
- [ ] `crates/render/src/lib.rs` -- re-export new public helpers -- wiring
- [ ] I/O matrix edge-case unit tests -- budget, pole, zoom-sweep coverage

**Acceptance Criteria:**
- Given `sin(x)*exp(x/10)` on the default viewport, when regenerating 2000 points, then it completes in < 8 ms measured
- Given a high-curvature span, when sampled adaptively, then midpoint deviation from linear interpolation stays within tolerance and flat spans stay near base density
- Given a pole at `1/x`, when refined, then the break remains NaN with no vertical spike
- Given a zoom sweep from width 20 to 0.002, when ticks regenerate, then steps follow stable 1/2/5 decades with minor subdivisions and no label flicker
- Given existing callers, when `sample_points`/`grid_lines` run, then signatures and JSON shapes are unchanged

## Implementation Notes

Built 2026-10-18 on `main` over `1fbf9d0` (1.1,1.2,1.3 in-review), never pushed.

- `crates/render/src/curve.rs` -- `sample_curve` keeps its signature but is now
  an adaptive wrapper (budget `n.clamp(2, MAX_POINTS)`); new
  `sample_curve_adaptive` (explicit tolerance), `sample_points_f32` (GPU-upload
  mirror), `MAX_POINTS/BASE_SAMPLES(257)/MAX_DEPTH(8)`; pole intervals never
  subdivide; output sorted by x, truncated to budget.
- `crates/render/src/grid.rs` -- `grid_ticks`/`viewport_ticks` signatures
  unchanged; new `tick_step`, `minor_ticks` (÷5, ÷4 for 2-mantissa),
  `tick_decimals`, `format_tick` (snaps -0.0), `viewport_minor_ticks`.
- `crates/render/src/lib.rs` -- re-exports new helpers; `pipeline.rs`,
  `viewport.rs`, `wasm_bridge` untouched (bridge contract tests pass as-is).
- Measured (example harness, since removed): smooth 257 pts 0.08 ms debug /
  0.03 ms release; full-cap 2000 pts 1.24 ms debug / 0.26 ms release.
- Clippy: fixed 2 new `manual_clamp` lints to `.clamp()` (house style, matches
  `wasm_bridge`); kept NaN-safe `!(x < y)` guards consistent with existing
  grid/pipeline code.
- `cargo test`: api 1, core_math 32, render 28, wasm_bridge 5 -- all pass.
  `cargo build` clean.

## Plan Change Log

## Review Triage Log

## Design Notes

Adaptive pass: base 257 uniform samples, then recursively bisect intervals whose midpoint deviation from the chord exceeds `tol = viewport.width / (pixels * 2)` mapped through `world_to_screen`-free x-normalized error, max depth 8, hard cap `MAX_POINTS = 2000`. Pole intervals (any non-finite endpoint or |y| > POLE_BREAK) are never bisected. Grid: major step from existing 1/2/5-decade search; minors = step/5 (step 1/2/5 → 5/4/5 minors); labels formatted to decade precision so zoom relabels monotonically.

## Verification

**Commands:**
- `cargo test -p render` -- expected: all curve/grid/pipeline tests pass, budget test green
- `cargo test` -- expected: api 1, core_math 21+, render 19+, wasm_bridge 5, all pass
- `cargo build` -- expected: workspace builds clean, zero warnings in `render`
