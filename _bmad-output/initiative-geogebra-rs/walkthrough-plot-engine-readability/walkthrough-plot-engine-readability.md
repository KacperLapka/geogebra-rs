# Walkthrough: plot_engine readability refactor

Target: commit `98cfc266c9a6123112f4eaea4ae66c808ff29ace` — `refactor(plot_engine): improve readability`
Plan: [plan-plot-engine-readability.md](../plan-plot-engine-readability.md)

Status: complete · Current block: none

Legend: `[ ]` means unvisited, in progress, or reopened; `[x]` means accepted by the reviewer.

## Block 1 — Intent

- [x] Accepted
- Changed during review: no

Source: the approved plan's Intent section.

**Problem:** [`plot_engine/src/lib.rs`](../../../crates/plot_engine/src/lib.rs) was 546 lines mixing GPU setup, mesh generation, pipelines, rendering, winit events, and WASM startup. Duplicate vertex/tessellation/upload code, dead code, and missing docs obscured the flow; clippy failed and there were no tests.

**Approach:** Behavior-preserving split into focused modules with one `Vertex`, shared helpers, named constants, docs, and geometry tests. Keep the WASM API unchanged; preserve rendering except for the approved decisions: blank startup, atomic failed updates, blank empty input, and removal of the unused pan state.

## Block 2 — Broad strokes

- [x] Accepted
- Changed during review: no

The commit turns one monolithic renderer into seven focused modules plus a thin WASM adapter. A reader can follow the flow through five entry points:

1. [lib.rs:16](../../../crates/plot_engine/src/lib.rs#L16) — module wiring, crate docs, exports, and native/web startup.
2. [app.rs:46](../../../crates/plot_engine/src/app.rs#L46) — winit event loop and window events.
3. [renderer.rs:63](../../../crates/plot_engine/src/renderer.rs#L63) — GPU resources, surface setup, and frame drawing.
4. [graph_state.rs:13](../../../crates/plot_engine/src/graph_state.rs#L13) — the active function and atomic updates.
5. [graph.rs:30](../../../crates/plot_engine/src/graph.rs#L30) — normalized samples into tessellated curve geometry.

[pipelines.rs](../../../crates/plot_engine/src/pipelines.rs) and [grid.rs](../../../crates/plot_engine/src/grid.rs) are supporting modules for the shared vertex format and the static grid.

## Slices

### Slice 1 — Module split and dependency direction

- [x] Accepted
- Changed during review: no

`lib.rs` is now 67 lines of crate docs, module declarations, re-exports, and startup. The event loop lives in `app.rs`, GPU resources and drawing in `renderer.rs`, the graph model in `graph_state.rs`, and JS exports in `wasm_interface.rs`. Dependencies flow `app -> renderer -> {pipelines, grid, graph, graph_state}` and `wasm_interface -> graph_state`; `lib.rs` only wires them, so the old GPU/event/state tangle is gone.

References: [lib.rs:16](../../../crates/plot_engine/src/lib.rs#L16), [app.rs:19](../../../crates/plot_engine/src/app.rs#L19), [renderer.rs:63](../../../crates/plot_engine/src/renderer.rs#L63), [graph_state.rs:13](../../../crates/plot_engine/src/graph_state.rs#L13), [wasm_interface.rs:1](../../../crates/plot_engine/src/wasm_interface.rs#L1).

## Periphery

- [x] Accepted
- Changed during review: no

- [Cargo.toml](../../../crates/plot_engine/Cargo.toml) — removed the duplicate `wasm-bindgen` target dependency.
- [lib.rs:1](../../../crates/plot_engine/src/lib.rs#L1) — crate and module documentation, plus explicit re-exports.
- [test.html](../../../crates/plot_engine/test.html) — manual harness updated to exercise `updateGraph`, `updateGraphWithRange`, `addOffset`, and `zoom`; remains untracked.
- [deferred-work.md](../deferred-work.md) — three follow-ups: the `math_engine` parse trap, offset/zoom atomicity, and frontend/ADR drift.
