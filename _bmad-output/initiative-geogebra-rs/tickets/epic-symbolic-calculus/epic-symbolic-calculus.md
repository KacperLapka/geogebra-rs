---
type: epic
title: "Symbolic calculus engine"
parent: initiative-geogebra-rs
covers: ["R3", "R4", "R5", "R6"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md"]
after: ["epic-scaffold-tracer"]
risk: high
refined: true
---

# Symbolic calculus engine

## Description

Implements symbolic first and second derivative computation with mandatory AST simplification, tangent-line rendering at any $x_0$, Brent's method root finding, and local extrema and inflection-point detection — all inside `core_math` and fully exercised by native `cargo test` before any browser integration.

## Outcome

Every calculus analysis capability (derivatives, tangent, roots, extrema, inflection points) works correctly and instantly for all supported differentiable functions, verified by a 250-benchmark test suite passing at 100% (SM-2, R3–R6).

## Requirements

1. R1 (parent R3): Engine computes first and second symbolic derivatives analytically for all supported differentiable functions.
2. R2 (parent R4): Engine computes and displays dynamic tangent lines at any specified coordinate $x_0$, updating synchronously at 60 FPS as $x_0$ varies.
3. R3 (parent R5): Engine identifies all real roots of $f(x)$ within the visible viewport using combined analytical and Brent numerical solvers, displaying hoverable coordinate markers.
4. R4 (parent R6): Engine identifies local minima, local maxima, and inflection points ($f''(x) = 0$ with concavity sign change) within the active viewport, displaying highlighted coordinate markers.

## Constraints

- Client compute: 100% of mathematical computation and rendering must execute client-side in WebAssembly and wgpu with zero server-side math evaluation.
- Rendering stack: Graphics pipeline must target WebGPU via wgpu with automatic WebGL2 fallback on unsupported platforms.
- Security: Mathematical expressions must strictly pass through an AST parser; dynamic code execution (eval) is strictly prohibited.
- Numerical precision: Mathematical evaluation must match IEEE 754 double-precision floats within a tolerance of $\epsilon < 10^{-12}$ for standard operations.
- Performance budget: Continuous 60 FPS (< 16 ms frame time) during active drag/zoom and slider interactions; curve evaluation and vertex buffer updates under 8 ms for 2000 points.
- Architecture decoupling: Core math engine crate must compile independently of UI and web dependencies.

## Done when

- `core_math` computes symbolic `f'(x)` and `f''(x)` with constant-fold simplification for polynomial, trigonometric, exponential, logarithmic functions (R3, AD-3).
- Tangent line at any `x_0` updates synchronously with slider movement at 60 FPS (R4).
- Brent's solver finds all real roots of `f(x)` within the active viewport with precision `ε < 10⁻¹²` (R5, NFR-FUNC-01).
- Local minima, maxima, and inflection points are identified and rendered as markers within the viewport (R6).
- 250-benchmark native test suite passes at 100% (SM-2).

## Boundaries

- Covers only `core_math` and its integration with `render` for marker and tangent overlays.
- Does not cover: asymptote/pole detection (epic 3), multi-curve rendering polish (epic 4), persistence (epic 5).
- Waits on epic-scaffold-tracer for integration only: native core_math stories (entries 1-5, 7) may proceed before tracer finishes; entry 6 integration needs the `wasm_bridge` function-set ABI and canvas render loop to integrate markers.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirements R3, R4, R5, R6
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-2, AD-3

## Notes

- Decision: 2026-10-18 — Refine from parent R3,R4,R5,R6 alone; no extra elicitation, no PRD/UX/architecture run.
- Decision: 2026-10-18 — Reuse current WASM ABI with no AD-5 amendment; tangent via core_math eval plus render overlay, markers via existing query_hover and sampler path.
- Decision: 2026-10-18 — Leave solver contract (viewport scan resolution, POLE_THRESHOLD value, 250-benchmark location) to stories; AD-3 already fixes differentiate-then-simplify and sign-change-then-Brent.
- Decision: 2026-10-18 — Partial gate on epic-scaffold-tracer: native core_math diff/simplify/solver stories may proceed before tracer finishes; only marker/tangent integration waits on working parse-eval-render pipeline and wasm_bridge surface.
- Decision: 2026-10-18 — Tracer bullet is entry 1 (polynomial diff-simplify-tangent eval stubbing marker shape); entry 2 (Brent roots + pole guard) next as least-sure; entries 3-4 split diff into transcendental-chain then rational-quotient hardening; entries 5-7 extrema, render integration, 250-suite; entry 8 refactor sweep. Native-only tracer suffices; render path proved in entry 6.
