---
title: 'Tracer bullet: 1/x pole break with vertical dashed overlay (3.1)'
type: 'feature'
ticket: '1'
created: '2026-10-19'
status: 'built'
baseline_revision: '8895ccb613227e293b99635fea202c53a516006a'
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

**Problem:** `f(x)=1/x` has no solver-owned pole-x API and no vertical dashed overlay; the curve already NaN-breaks via `POLE_BREAK` but the `POLE_THRESHOLD`/`POLE_BREAK` 1e6 duplication is undocumented and no end-to-end path proves split plus overlay with no spike or NaN in buffers.

**Approach:** Build the thinnest solver-to-curve path reusing epic-2 Brent output: pole-x detection in the solver pole-skip function, guard-plus-strip-split confirmation in the curve sampler function, and viewport-spanning vertical dashed overlay geometry wired through the bridge to Canvas.

## Boundaries & Constraints

**Always:** Client-compute only in WASM/wgpu, AST-only eval, IEEE754 f64, 60 FPS with 8ms/2000pt budget, `core_math` web-free with one-way deps `core_math <- render <- wasm_bridge` (AD-1..AD-4); reuse `scan_sign_changes`/`brent`/`guard`/`valid`/`expand_ribbon` NaN-split plus `tangent_segment` viewport-span idiom; keep `POLE_THRESHOLD`/`POLE_BREAK` values at 1e6 unchanged and document the duplication; overlay reuses dashed `1.0` path, `2.0` stays reserved; `sample_points` stays additive (existing fields byte-identical).

**Never:** No tan periodic density tuning (story 2), no Horizontal/Slant degree-compare (story 3), no limit-sampling fallback or full overlay wiring with 8ms proof (story 4), no refactor sweep (story 5); no `brent`/`scan`/`find_roots` semantics change, no `CURVE_VERTEX_STRIDE`/layout/`curve.wgsl` edits, no WASM ABI shape break, no grid/palette/workspace changes.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Happy 1/x split + overlay | `f=1/x`, viewport [-1,1]x[-10,10] | Curve has NaN break near x=0, no joined spike; one vertical dashed line at x≈0 spanning viewport | No error expected |
| Pole API | `1/x` over [-1,1] via new solver poles fn | Returns single x within 1e-9 of 0; `x^2` returns empty | Non-finite samples skipped, never emitted |
| NaN-free buffers | Sampler + ribbon output for `1/x` | All finite y within 1e6; ribbon emits 2 verts per valid point, none for NaN, arclength restarts | NaN preserved as break, never uploaded as vertex |
| Non-pole passthrough | `f=x^2`, default viewport | No overlay emitted; curve unchanged | No error; empty overlay list |
| Degenerate viewport | Zero-width viewport or non-finite pole x | Overlay builder returns None; caller emits empty list | Skip overlay, curve untouched |

</frozen-after-approval>

## Code Map

- `crates/core_math/src/solver.rs` -- pole owner; add `find_poles`-style scan returning `Vec<f64>` pole-x using `POLE_THRESHOLD=1e6` magnitude plus sign-jump (inverse of `scan_sign_changes:170-195` pole-skip) reusing `brent:66-165`/`find_roots:202-227` residual pattern; reuse `skips_pole_of_reciprocal:355-361` and `pole_jumps_emit_no_markers:486-508`; do not change `brent`/`scan`/`find_roots` semantics or threshold values.
- `crates/render/src/curve.rs` -- sampler owner; add `vertical_asymptote_segment(x,viewport)` mirroring `tangent_segment:373-402` viewport-span idiom plus doc comment recording `POLE_BREAK=1e6:25` duplication with `solver.rs:13`; reuse `guard:408-414`/`valid:404-406`/`expand_ribbon:140-232` NaN-split and `breaks_at_pole:478-493`; do not change sampler refine-skip, stride, or dash 0/1 paths.
- `crates/wasm_bridge/src/lib.rs` -- ABI owner; add additive `asymptotes:[{x,from,to}]` to `sample_points:645-676` via new `collect_verticals` mirroring `collect_markers:155-186`/`tangent_for:191-208`, reusing `sample_curve` null-for-NaN mapping and `query_hover_core:587` pole filter; do not change existing export signatures or workspace shape.
- `frontend/src/App.svelte`, `frontend/src/Canvas.svelte`, `frontend/src/lib/fallback.js` -- overlay surface; `App` passes through new `asymptotes` field, `Canvas` strokes verticals dashed `[8,6]` verbatim like `tangent:105-119` with NaN guard and dirty-flag wiring, `fallback` returns empty list for shape parity; do not change viewport/hover math.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/core_math/src/solver.rs` -- add pole-x detector plus unit tests for `1/x` single pole at 0 and `x^2`/`sin` empty -- establishes solver contract for overlay.
- [ ] `crates/render/src/curve.rs` -- add vertical segment builder plus duplication doc plus tests for span endpoints, degenerate reject, and NaN-break preservation -- establishes render geometry truth.
- [ ] `crates/wasm_bridge/src/lib.rs` -- add asymptotes DTO plus collector, extend `sample_points` additively with tests for `1/x` overlay present and `x^2` empty -- exposes overlay without breaking ABI.
- [ ] `frontend/src/App.svelte`, `frontend/src/Canvas.svelte`, `frontend/src/lib/fallback.js` -- wire asymptotes prop through to dashed vertical stroke with shape-parity fallback -- proves visible tracer.
- [ ] Edge-case tests -- cover pole API precision, degenerate viewport, ribbon NaN/arc-restart, hidden-curve empty overlay -- locks I/O matrix.

**Acceptance Criteria:**
- Given `f(x)=1/x` on viewport [-1,1], when the solver poles function runs, then it returns one x within 1e-9 of 0 and when `f(x)=x^2` runs, then it returns empty.
- Given `f(x)=1/x` sampled with 257 points, when the sampler output is inspected, then at least one y is NaN and every finite y is within 1e6 in absolute value.
- Given sampler output with a NaN break expanded to a ribbon, when vertex count is checked, then it equals twice the valid-point count with arclength restarting to zero after the break.
- Given `f(x)=1/x` set in a dev session, when `sample_points` is called, then the response contains one asymptote entry at x≈0 with finite viewport-spanning endpoints and existing points/markers fields unchanged.
- Given `f(x)=1/x` typed in `npm run dev`, when the canvas renders, then the curve splits at x=0 with a dashed vertical line and no connecting spike or tooltip spike value.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Quick lens: 0 findings → 0 high, 0 medium, 0 low, 0 false, 0 maybe-false.

## Design Notes

Tracer reuses the derivative dashed `1.0` shader/Canvas path for the vertical line so no `curve.wgsl` or stride change is needed; `dash_flag 2.0` stays reserved for story-4 full wiring. Solver poles are the complement of the existing pole-skip: same 1e6 magnitude plus opposite-sign jump, with Brent residual as second layer per the tan ~76/358 lesson from 2.2. Bridge field is named `asymptotes` (list of `{x,from,to}`) to carry stories 2-4 Horizontal/Slant without a second ABI addition.

## Verification

**Commands:**
- `cargo test -p core_math -p render` -- expected: all pass including new pole-x, segment-span, and NaN-break tests
- `cargo test -p wasm_bridge` -- expected: all pass including new asymptotes additive-shape tests
- `cargo clippy -p core_math -p render -p wasm_bridge -- -D warnings` -- expected: clean
