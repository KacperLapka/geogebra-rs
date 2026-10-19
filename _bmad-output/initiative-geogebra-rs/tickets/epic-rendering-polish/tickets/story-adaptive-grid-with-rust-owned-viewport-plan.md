---
title: 'Adaptive grid with Rust-owned viewport (4.4)'
type: 'feature'
ticket: '4'
created: '2026-10-19'
status: 'built'
baseline_revision: '55dbe184fdf4f36a1a40888466cc1b3e48b58f8e'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Grid ticks are Rust-computed but only majors are exposed, with no vertex emission and no tick labels; the canvas strokes majors via the 2D fallback without a synchronous Rust-owned update guarantee.

**Approach:** Emit grid geometry synchronously from Rust viewport state (1/2/5 nice steps, max 10, zero-snapped) and render Svelte tick labels as a DOM overlay only, keeping geometry sync on viewport change with labels allowed one frame of lag.

## Boundaries & Constraints

**Always:** Rust owns viewport via `update_viewport` plus tick computation (`grid.rs` 1/2/5 steps, `max_ticks=10`, zero-snapped) plus grid vertex emission; Svelte positions DOM labels from bridge data and never computes ticks; f64 CPU / f32 GPU, WGSL-only GL-translatable, `grid.wgsl` lines-only never text; deps stay `core_math <- render <- wasm_bridge`; NaN-safe validation with no revision bump on reject.

**Never:** No curve sampler, style, palette, dash, or asymptote changes; no full wgpu render-pass or device setup beyond layout plus buffer builder; no breaking change to existing `grid_lines` `{x,y}` shape or workspace JSON; no JS tick math in Canvas/App beyond `toScreen` positioning; no canvas `fillText` text atlas; no `dash_flag 2.0` emission.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Happy pan/zoom | Drag and wheel-zoom in `npm run dev` | Grid lines subdivide synchronously, DOM labels follow within one frame | No error expected |
| Zoom sweep | Span halved 14 times from ±10 | Step never grows, ticks stay 1–11 per axis including 0, minors present | No error expected |
| Degenerate viewport | `x_min>=x_max`, NaN, or infinite bounds | Viewport kept, revision unbumped, grid unchanged | Reject, keep previous |
| Zero snapping | Ticks near 0 at any step | Labels render `"0"`, never `"-0"` or flickering decimals | Snap, no error |
| Minor subdivision | 1/5-mantissa step vs 2-mantissa step | Minors split ÷5, or ÷4 for 2-mantissa, strictly between majors | No error expected |

</frozen-after-approval>

## Code Map

- `crates/render/src/grid.rs` -- tick owner; reuse `grid_ticks`, `tick_step`, `minor_ticks`, `format_tick`, `viewport_ticks`; add vertex emission here; do not change 1/2/5 or NaN-safe semantics.
- `crates/render/src/viewport.rs` -- `Viewport{x_min,x_max,y_min,y_max}` plus `world_to_screen`/`screen_to_world`; reuse as-is; do not add a second viewport truth.
- `crates/render/src/pipeline.rs` -- `Renderer{viewport,uniform,revision}`, `ViewportUniform::from_viewport`, `update_viewport`, `GRID_SHADER` include; add grid vertex layout plus buffer builder; do not change curve 44B layout, probe order, or revision logic.
- `crates/render/shaders/grid.wgsl` -- `ViewportUniform{center,scale}`, `vs_main(world:vec2f)`, gray low-alpha `fs_main`; reuse layout; do not add text or new uniforms.
- `crates/wasm_bridge/src/lib.rs` -- `Session{viewport,viewport_revision}`, `update_viewport`, `grid_lines()->{x,y}`, `gpu_status().viewport_revision`; extend `grid_lines` additively; do not change existing export signatures or `Workspace v1`.
- `frontend/src/App.svelte` -- `viewport`/`grid` state, `handleViewport->updateViewport+refresh()`, `grid=backend.gridLines()`; add label-data flow here; do not compute ticks.
- `frontend/src/Canvas.svelte` -- 2D `draw()` stroking `grid.x/grid.y`, `toScreen`, `wheel`/`down/move/up->onViewport`, dirty-flag rAF loop; add DOM label overlay; do not add tick math.
- `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- `gridLines()` thin passthrough plus JS 1/2/5 mirror; extend both in parity; do not add new JS math elsewhere.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/grid.rs` -- add `grid_vertices(viewport,max_ticks)` emitting `Vec<[f32;2]>` world endpoints for major plus minor lines and `grid_labels(viewport,max_ticks)` returning formatted `{value,text}` via `format_tick` -- establishes sync Rust-owned geometry plus label source.
- [ ] `crates/render/src/pipeline.rs` -- add `grid_vertex_layout()` (`Float32x2` loc 0) plus `grid_vertices_f32()` buffer builder reusing `ViewportUniform::from_viewport`, with naga/stride tests -- wires GPU-ready layout without touching curve contract.
- [ ] `crates/wasm_bridge/src/lib.rs` -- extend `grid_lines()` to `{x,y,x_minor,y_minor,x_labels,y_labels,step_x,step_y}` keeping `x`/`y` byte-identical, with unit tests for minors, labels, and degenerate rejection -- exposes full grid state additively.
- [ ] `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- extend `gridLines()` parsing and fallback mirror (minors plus `format_tick` parity, zero-snapped) -- keeps Rust and fallback backends interchangeable.
- [ ] `frontend/src/App.svelte`, `frontend/src/Canvas.svelte` -- stroke majors plus faint minors synchronously from `backend.gridLines()` in `refresh()`, render tick labels as transparent absolute-positioned DOM overlay via `toScreen` (labels may lag one frame) -- proves sync geometry plus DOM-only labels.
- [ ] Edge-case tests -- cover zoom-sweep monotonicity, degenerate/NaN rejection with no revision bump, `-0` snapping, minor ÷4/÷5 rule -- locks I/O matrix.

**Acceptance Criteria:**
- Given default viewport ±10, when `grid_lines()` is called, then majors use 1/2/5 steps with at most 10 per axis including 0 and minors subdivide strictly between majors.
- Given a drag or wheel-zoom in `npm run dev`, when the viewport changes, then grid lines redraw in the same frame and DOM labels catch up within one frame.
- Given a degenerate or NaN viewport update, when submitted, then the previous viewport and grid are kept and `viewport_revision` does not bump.
- Given ticks near zero at any zoom, when labels render, then they read `"0"` with step-appropriate decimals and do not flicker across frames.
- Given `curve.wgsl` and `grid.wgsl` after the change, when `cargo test shaders_parse_as_wgsl` runs, then both parse and the curve 44B 6-attribute layout test still passes.

## Design Notes

Additive `grid_lines` extension keeps `x`/`y` stable so the 2D fallback and any GPU path consume the same Rust truth. CPU vertex endpoints are world-space `f32` pairs transformed by the existing `ViewportUniform`, so the 2D `toScreen` stroke and the future `grid.wgsl` draw stay identical. DOM labels are plain absolutely-positioned divs (no canvas text, no atlas), which is why one frame of lag is acceptable during continuous drag.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Quick lens: 4 findings → 1 medium, 1 low, 1 low-rejected, 1 false.

- F1 `frontend/src/App.svelte:152-156` handleViewport adopts local viewport before backend rejects — verdict medium, route patch. Evidence: `handleViewport` assigns then calls void `update_viewport` which rejects degenerate/NaN without bump, and `restoreWorkspace` plus `viewport={...doc.viewport}` adopts rejected spans while `gridLines()` returns previous grid, breaking Rust-owned truth.
- F2 `frontend/src/Canvas.svelte:60-70` majors and axes stroke unguarded `toScreen` while minors guard — verdict low, route patch. Evidence: majors/axes feed `moveTo`/`lineTo` directly, minors skip non-finite `t` and `px`/`py`; diverged zero-span viewport from F1 yields Inf/NaN majors where minors skip.
- F3 `crates/render/src/grid.rs:75-78` f32 edge cast without finiteness check — verdict low, rejected (low-reject rule). Evidence: `y0/y1/x0/x1` cast unchecked while per-tick `fx`/`fy` skip non-finite, so ±1e300 f64 yields inf f32, but everyday UI clamps to 1e6 and fix needs new guards/branches.
- F4 plan AC `at most 10 per axis` vs 11 majors at ±10 — verdict false, rejected (fix would edit plan). Evidence: frozen matrix allows 1–11 and `tick_step` span/s<=10 selects step 2 giving 11 ticks `-10…10`; code and tests assert `<=11` per frozen 1/2/5 logic, AC wording alone is imprecise.

## Verification

**Commands:**
- `cargo test -p render -p wasm_bridge` -- expected: all pass including new grid vertex, label, and revision tests
- `cargo test -p render shaders_parse_as_wgsl` -- expected: pass, both shaders valid, curve layout intact
- `cargo test -p render per_frame_cpu_work_stays_far_under_budget` -- expected: pass, viewport path far under 16 ms
- `npm run dev --prefix frontend`, then drag and zoom -- expected: grid subdivides synchronously, labels lag at most one frame
