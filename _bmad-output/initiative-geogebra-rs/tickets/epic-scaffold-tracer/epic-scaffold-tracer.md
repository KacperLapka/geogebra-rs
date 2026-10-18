---
type: epic
title: "Project scaffold and tracer bullet"
parent: initiative-geogebra-rs
covers: ["R1", "R2", "R8", "R9"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md"]
after: []
risk: high
refined: true
---

# Project scaffold and tracer bullet

## Description

Restructures the Cargo workspace to the four-crate layout defined in AD-1 (`core_math`, `render`, `wasm_bridge`, `api`), stands up the Svelte 5 + Vite + wasm-pack build pipeline, and delivers the thinnest working path through every system layer: a polynomial expression parsed in Rust, plotted on a wgpu canvas at 60 FPS, and a hovered canvas point returning `(x, y)` from Rust to a Svelte HTML tooltip.

## Outcome

A developer loads the browser app, types a function, sees a live curve, and hovers to read exact coordinates — confirming every layer from Rust AST to GPU canvas to Svelte tooltip is wired and working end-to-end.

## Requirements

1. R1 (parent R1): Mathematical expression parser evaluates polynomials, trigonometric, exponential, logarithmic, and rational functions, reporting localized syntax errors with character index within 50 ms without interrupting the application loop.
2. R2 (parent R2): Parser binds user-defined parameters in expressions to reactive UI sliders for real-time recalculation.
3. R3 (parent R8): Canvas renders continuous mathematical curves using adaptive sub-pixel sampling, generating vertex buffers for 2000 points in under 8 ms.
4. R4 (parent R9): Canvas supports continuous 60 FPS interactive panning and zooming with dynamic coordinate grid subdivisions and updated axis labels.

## Constraints

- Client compute: 100% of mathematical computation and rendering must execute client-side in WebAssembly and wgpu with zero server-side math evaluation.
- Rendering stack: Graphics pipeline must target WebGPU via wgpu with automatic WebGL2 fallback on unsupported platforms.
- Security: Mathematical expressions must strictly pass through an AST parser; dynamic code execution (eval) is strictly prohibited.
- Numerical precision: Mathematical evaluation must match IEEE 754 double-precision floats within a tolerance of ε < 10⁻¹² for standard operations.
- Performance budget: Continuous 60 FPS (< 16 ms frame time) during active drag/zoom and slider interactions; curve evaluation and vertex buffer updates under 8 ms for 2000 points.
- Architecture decoupling: Core math engine crate must compile independently of UI and web dependencies.

## Done when

- Cargo workspace contains `crates/core_math`, `crates/render`, `crates/wasm_bridge`, `crates/api`; `core_math` builds with `cargo test` natively without WASM toolchain (NFR-MAIN-01, AD-1).
- `wasm-pack build` produces a WASM bundle importable by Svelte 5 frontend.
- Typing a polynomial in the formula bar plots a curve on the wgpu canvas within one frame (≤ 16 ms) (R1, R8, R9).
- Moving the mouse over the canvas triggers `query_hover` and a Svelte tooltip displays the math coordinates returned from Rust (AD-4, AD-5).
- A slider parameter `a` bound to `f(x) = a * x^2` updates the curve in real time (R2).

## Boundaries

- Covers: parsing, evaluation, basic single-curve plotting, hover query, slider binding — thin path only.
- Does not cover: symbolic differentiation, calculus markers, asymptote detection, multi-curve styles, persistence (those are epics 2–5).
- Must not change: Cargo workspace crate membership once established (AD-1).
- Owns the workspace JSON schema shape; epic 5 depends on it.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirements R1, R2, R8, R9
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-1 through AD-5

## Notes

- Decision: 2026-10-18 — Restructure by moving `api/` → `crates/api`, `wasm/` → `crates/wasm_bridge` and adding `crates/core_math, crates/render` to meet AD-1; stories assume AD-1 paths.
- Decision: 2026-10-18 — Scaffold `frontend/` with Svelte 5 + Vite + wasm-pack import and minimal `FormulaBar, Canvas, Tooltip, Sliders.svelte`; tracer story proves the pipeline.
- Decision: 2026-10-18 — Lock tracer WASM ABI names now (`parse_and_validate, set_function, update_param, update_viewport, query_hover`); multi-function `id` semantics left to stories.
- Decision: 2026-10-18 — Leave workspace JSON shape (`version:1, functions, params, viewport`) to first story that needs it; Epic 5 waits on it.
- Decision: 2026-10-18 — Leave 60 FPS verification method to stories; declined spike for wgpu/WebGL2 fallback, risk surfaces in tracer story and fails fast.
- Decision: 2026-10-18 — Refine from parent R1,R2,R8,R9 alone; no extra elicitation, no PRD/UX/architecture run.
