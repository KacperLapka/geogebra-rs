---
type: epic
title: "Full rendering and multi-curve polish"
parent: initiative-geogebra-rs
covers: ["R8", "R9", "R10"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md", "_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md"]
after: ["epic-scaffold-tracer"]
risk: medium
refined: true
---

# Full rendering and multi-curve polish

## Description

Completes the `render` crate to production quality: quad-ribbon curves with antialiasing and configurable line width, dashed overlay for derivative curves, per-function colour palette and visibility toggles, adaptive grid subdivision, and confirmed 60 FPS on WebGPU and WebGL2 across Chrome, Firefox, and Safari.

## Outcome

A student or lecturer sees multiple styled curves rendered at 60 FPS with clear visual differentiation between `f(x)`, `f'(x)`, and `f''(x)`, on any modern browser without configuration (R8, R9, R10, NFR-PERF-01, NFR-PERF-02, NFR-PORT-01).

## Requirements

1. R1 (parent R8): Canvas renders continuous curves as quad-ribbon triangle strips expanded in the WGSL vertex shader with fragment edge-antialiasing; stroke width clamped [2.5, 6.0] px, default 3.5 px, on WebGPU and WebGL2.
2. R2 (parent R8): Vertex buffer for 2000 points generated in under 8 ms native (`cargo test two_thousand_points_are_fast`) and WASM bench including ribbon expansion; budget split sampler < 8 ms, ribbon-expand + submit < 8 ms, frame < 16 ms.
3. R3 (parent R9): Canvas sustains 60 FPS (p95 < 16 ms) during the shared continuous drag/zoom script on current Chrome (WebGPU), Firefox (WebGPU), Safari (WebGL2 fallback); identical render paths, deltas recorded not branched.
4. R4 (parent R9): Adaptive coordinate grid uses Rust-owned viewport plus 1/2/5 nice ticks (max 10 per axis, zero-snapped); grid geometry updates synchronously on viewport change, Svelte renders only tick labels and tooltips as DOM/SVG overlay (labels may lag one frame).
5. R5 (parent R10): Per-function `FunctionStyle { color linear-sRGB, width_px, dashed, visible }` owned by `render`, keyed by id in bridge Session; Okabe-Ito 8-colour defaults assigned f -> f' -> f'' -> cycle, override via `set_function_style`; `visible = false` skips submission and retains entry; wired via `set_function_style` + `set_function_visibility` (E4-AD-5-AMEND, 5+2 surface).
6. R6 (parent R10): Derivative overlays f'/f'' render as screen-space dashed ribbons (8 px on / 6 px off at 1x, DPR-aware arclength varying, antialiased dash edges); f solid; `dash_flag` 0/1, 2.0 reserved for epic 3 only.

## Constraints

- Client compute: 100% of mathematical computation and rendering must execute client-side in WebAssembly and wgpu with zero server-side math evaluation.
- Rendering stack: Graphics pipeline must target WebGPU via wgpu with automatic WebGL2 fallback on unsupported platforms; all WGSL must compile under the WebGL2 (GLES3) translation, f32 only in shaders, f64 stays CPU-side.
- Numerical precision: Mathematical evaluation must match IEEE 754 double-precision floats within a tolerance of ε < 10⁻¹² for standard operations; sampler and viewport math in f64.
- Performance budget: Continuous 60 FPS (< 16 ms frame time) during active drag/zoom; curve evaluation and vertex buffer updates under 8 ms for 2000 points; identical curve/grid paths on both backends.
- Architecture decoupling: Core math engine crate must compile independently of UI and web dependencies; dependency direction one-way `core_math <- render <- wasm_bridge`; style struct lives in `render`, not `wasm_bridge`; shaders WGSL-only under `render/shaders/`.

## Done when

- Curves rendered as quad-ribbon triangle strips via WGSL vertex shader with antialiasing; minimum 2.5 px stroke width (AD-4, R8).
- Derivative overlays rendered as dashed ribbons using screen-space pattern in fragment shader (R10).
- Per-function configurable colour and visibility toggle wired from Svelte to `wasm_bridge` (R10).
- Adaptive coordinate grid updates logarithmic subdivisions smoothly during zoom (R9).
- Verified 60 FPS on Chrome (WebGPU), Firefox (WebGPU), Safari (WebGL2 fallback) during continuous drag and zoom (NFR-PERF-02, NFR-PORT-01, SM-1).
- Vertex buffer for 2000 points generated in < 8 ms (NFR-PERF-01, R8).

## Boundaries

- Covers: `render` crate shaders, adaptive sampler, grid, and Svelte style controls.
- Does not cover: asymptote overlays (epic 3) or persistence (epic 5).
- Waits on epic-scaffold-tracer because: needs the wgpu pipeline and canvas event loop established there.
- Can run in parallel with epic-symbolic-calculus and epic-asymptote-guard.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirements R8, R9, R10
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-4
- architecture — `_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md`, E4-AD-4-AMEND, E4-DASH, E4-STYLE-STRUCT, E4-PALETTE, E4-AD-5-AMEND, E4-SHADER-CONTRACT, E4-VIEWPORT-GRID, E4-GL-PARITY, E4-PERF-HARNESS, E4-SCOPE

## Notes

- Decision: 2026-10-18 — Refine (not close/drop); epic had refined:false, 0 stories, no implementation evidence for Closure.
- Decision: 2026-10-18 — Amend AD-5 now with `set_function_style` + `set_function_visibility` (single shape, 5+2 surface); fixes parent "exactly four" count.
- Decision: 2026-10-18 — Accept style defaults: 8-colour Okabe-Ito palette in order f -> f' -> f'' -> cycle, width 2.5 min / 3.5 default / 6.0 max, dash 8 on / 6 off screen-space DPR-aware.
- Decision: 2026-10-18 — Accept perf harness: 60 FPS drag/zoom script on current Chrome/Firefox-WebGPU + Safari-WebGL2 (p95 < 16 ms); 8 ms native cargo test + WASM bench incl ribbon expansion.
- Decision: 2026-10-18 — Enrich via bmad-architecture first; delta adopted as companion, spine AD-1..AD-6 read-only, epic 3 owns pole/asymptote overlays via reserved dash_flag 2.0, epic 4 owns style+ribbon+grid+harness.
- Open question: WASM bench harness shape (wasm-bindgen-test vs performance.now script) — deferred to story slicing, build-agent picks.
