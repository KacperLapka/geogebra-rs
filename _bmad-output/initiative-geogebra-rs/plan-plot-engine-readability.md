---
title: 'Refactor plot_engine for readability'
type: 'refactor'
ticket: ''
created: '2026-11-21'
status: 'built'
baseline_revision: 'f4ce94543a958b0447b71c7e382f2928cdf1fd98'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `plot_engine/src/lib.rs` (546 lines) mixes GPU setup, mesh generation, pipelines, rendering, winit events, and WASM startup. Duplicate vertex/tessellation/upload code, dead code, and missing docs obscure the flow; clippy fails and there are no tests.

**Approach:** Behavior-preserving split into focused modules with one `Vertex`, shared helpers, named constants, docs, and geometry tests. Keep the WASM API unchanged; preserve rendering except for the Decisions below.

## Boundaries & Constraints

**Always:** Preserve exports (`updateGraph`, `updateGraphWithRange`, `addOffset`, `zoom`, `run_web`), `[-1, 1]` clip space, 1000 samples, grid `LineList`, curve `TriangleList`, width `0.008`, colors, and grid-then-curve order. Keep behavior unchanged except for the Decisions below. One responsibility per module. No new runtime dependencies. Format, check, clippy `-D warnings`, tests, and wasm check must pass.

**Never:** Change the wgpu/winit/lyon stack or shader semantics. Add panning, axes, labels, or adaptive sampling. Modify `math_engine`, the frontend, or generated `pkg/` output. Rename or remove JS-facing exports.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Startup | New renderer | Grid only; blank plot; resize works | No error expected |
| Update | Valid expression | Curve replaced; one frame rendered | Math error surfaced to JS |
| Invalid input | Unparseable string | Last valid curve remains | `Err` to JS; no panic |
| Empty samples | Zero points | Blank plot | No panic |

## Decisions

- Startup is blank: remove `graph::example`; `Renderer` holds `Option<GraphBuffers>` and draws the curve only when present.
- Updates are atomic: build and validate a new `Mathematician`, commit it to `GraphState`, then mark changed. On failure return `Err` and keep the last valid curve.
- Empty point inputs render blank behind an explicit guard and are covered by a unit test.
- Remove unused `offset`, `is_dragging`, and the empty `CursorMoved` arm.

</frozen-after-approval>

## Code Map

- `crates/plot_engine/src/lib.rs` -- `Vertex` 31–58, `State` 61–78, `new` 83–270, `render` 282–357, `update` 366–391, `App` 393–512, `run` 514–546.
- `crates/plot_engine/src/graph.rs` -- duplicate `MyVertex`; duplicate tessellation in `path_to_vertex` 39–59 and `example` 61–95; dead `str_to_path` 31–37.
- `crates/plot_engine/src/wasm_interface.rs` -- `GraphState` 5–8 misplaced; exports 10–47.
- `crates/plot_engine/Cargo.toml` -- duplicate `wasm-bindgen` 22/28.
- `crates/math_engine/src/lib.rs` -- 1000 samples 175–185, x `[-1,1]`, y `[-10,10]` 196–248; do not modify.
- `test.html` (crate/root) -- untracked harnesses. ADR confirms 1000 samples, `0.008`, colors. Frontend import is stale, out of scope.

## Tasks & Acceptance

**Execution:**
- [x] `crates/plot_engine/src/pipelines.rs` -- add `Vertex`, `Vertex::desc()`, constants, and `create_pipeline(topology, label)` -- one pipeline home.
- [x] `crates/plot_engine/src/grid.rs` -- add `build_grid_vertices()` and `GRID_*` constants -- isolate grid geometry.
- [x] `crates/plot_engine/src/graph.rs` -- drop `MyVertex`, `example`, and duplication; `points_raw_to_path(&[f32]) -> Option<Path>` returns `None` for empty input; add `tessellate_curve(&Path)` and tests -- pure geometry.
- [x] `crates/plot_engine/src/graph_state.rs` -- move `GraphState`, `GRAPH_STATE`, and dirty tracking out of `wasm_interface.rs`; add an atomic replace helper that validates before committing -- correct layering and failed-update behavior.
- [x] `crates/plot_engine/src/renderer.rs` -- move `State` as `Renderer`; split construction and `render()`; add `GraphBuffers::upload`; hold `Option<GraphBuffers>` so startup is blank -- remove duplicate uploads.
- [x] `crates/plot_engine/src/app.rs` -- move `App`, `ApplicationHandler`, window events, and wasm canvas setup; remove `offset`, `is_dragging`, and the empty `CursorMoved` arm -- isolate event-loop concerns.
- [x] `crates/plot_engine/src/wasm_interface.rs` -- keep only `#[wasm_bindgen]` exports delegating to `graph_state` -- thin adapter.
- [x] `crates/plot_engine/src/lib.rs` -- keep modules, crate docs, explicit re-exports, `run()`/`run_web()`; target ~80 lines -- readable top-level flow.
- [x] `crates/plot_engine/Cargo.toml` -- remove the duplicate `wasm-bindgen` dependency -- build hygiene.

**Acceptance Criteria:**
- Given `cargo check`, clippy `-D warnings`, `cargo test`, and the wasm check, when run, then all pass.
- Given `crates/plot_engine/test.html`, when `updateGraph`, `addOffset`, and `zoom` are exercised, then curve operations match the baseline; startup is intentionally blank.
- Given the source, when searching for `MyVertex` and duplicate upload/tessellation blocks, then none remain.
- Given `lib.rs`, when inspected, then it contains no GPU setup, pipeline construction, or winit event handling.
- Given a fresh renderer, when the first frame renders, then only the grid is visible.
- Given an invalid expression, when the update fails, then the last valid curve remains and JS receives `Err`.
- Given zero points, when geometry is built, then nothing is drawn and no panic occurs.

## Verification

**Commands:**
- `cargo fmt -p plot_engine --check` -- expected: no diff.
- `cargo check -p plot_engine` -- expected: success, zero warnings.
- `cargo clippy -p plot_engine --all-targets --no-deps -- -D warnings` -- expected: success.
- `cargo test -p plot_engine` -- expected: all tests pass.
- `cargo check -p plot_engine --target wasm32-unknown-unknown` -- expected: success.

## Implementation Notes

- 2026-11-21: Implemented the module split (`app`, `renderer`, `pipelines`, `grid`, `graph`, `graph_state`), removed duplicate vertex/tessellation/upload code, made startup blank, made updates atomic, and removed the unused pan state.
- Verification is scoped to `plot_engine` because `math_engine` has pre-existing rustfmt and clippy failures and is frozen by this plan.
- Residual risk: non-empty strings that `symb_anafis::diff` accepts but `formulac` cannot parse (for example `"not a function"`) can still panic inside `math_engine::compute_given_function`. Fixing it requires a `math_engine` change or a new direct dependency, both out of scope.

## Plan Change Log

## Review Triage Log

Verdicts: 1 high, 1 medium, 1 low.

- `graph_state.rs:60-61` — high — **defer**: real; `updateGraphWithRange("not a function")` traps the WASM module with `unreachable` and poisons later calls. Pre-existing `math_engine::compute_given_function` panic; the intent forbids modifying `math_engine`. Logged in `deferred-work.md`.
- `graph_state.rs:152-169` — medium — **patch applied**: real; the failed-update test only exercised the whitespace guard. Added a private `replace_function_with` seam and `failed_candidate_keeps_previous_state`, which injects `Err(JsValue::NULL)` and asserts state is unchanged.
- `test.html:547-552` — low — **patch applied**: real; the harness never called `addOffset` and only conditionally reached `updateGraph`. `applyPlot` now calls `updateGraph`, `updateGraphWithRange`, `addOffset`, and `zoom` when available (file remains untracked).
