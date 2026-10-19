---
type: initiative
title: "Zero-latency mathematical visualization and analytical calculus platform with lightweight workspace sharing"
parent: none
sources: ["_bmad-output/initiative-geogebra-rs/prd-geogebra-rs/prd-geogebra-rs.md"]
companions: ["_bmad-output/initiative-geogebra-rs/glossary.md", "_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md"]
after: []
risk: medium
---

# Zero-latency mathematical visualization and analytical calculus platform with lightweight workspace sharing

## Description

GeoGebra RS delivers an interactive, high-performance mathematical exploration and calculus analysis platform running directly in modern web browsers and desktop environments. By executing 100% of analytical math calculations in client-side WebAssembly and hardware-accelerated wgpu graphics, it eliminates server calculation latency for function plotting, symbolic differentiation, critical point analysis, and dynamic parameter exploration, backed by a lightweight Axum persistence API for instant workspace sharing.

## Outcome

For mathematics students and educators, GeoGebra RS provides zero-latency client-side calculus visualization and workspace sharing, verified by maintaining 60 FPS interaction fluidity (SM-1) and a 100% pass rate on the calculus test suite (SM-2).

## Requirements

1. R1 (prd FR-1, NFR-USAB-01): Mathematical expression parser evaluates polynomials, trigonometric, exponential, logarithmic, and rational functions, reporting localized syntax errors with character index within 50 ms without interrupting the application loop.
2. R2 (prd FR-2): Parser binds user-defined parameters in expressions to reactive UI sliders for real-time recalculation.
3. R3 (prd FR-3): Engine computes first and second symbolic derivatives analytically for all supported differentiable functions.
4. R4 (prd FR-4, Story 2): Engine computes and displays dynamic tangent lines at any specified coordinate $x_0$, updating synchronously at 60 FPS as $x_0$ varies.
5. R5 (prd FR-5, Story 3): Engine identifies all real roots of $f(x)$ within the visible viewport using combined analytical and Brent numerical solvers, displaying hoverable coordinate markers.
6. R6 (prd FR-6, Story 3): Engine identifies local minima, local maxima, and inflection points ($f''(x) = 0$ with concavity sign change) within the active viewport, displaying highlighted coordinate markers.
7. R7 (prd FR-7, Story 4, NFR-RELI-01): Engine detects vertical, horizontal, and slant asymptotes ($y = mx + b$) without drawing false connecting lines across singularities or propagating NaN values.
8. R8 (prd FR-8, NFR-PERF-01): Canvas renders continuous mathematical curves using adaptive sub-pixel sampling, generating vertex buffers for 2000 points in under 8 ms.
9. R9 (prd FR-9, Story 1, NFR-PERF-02, SM-1): Canvas supports continuous 60 FPS interactive panning and zooming with dynamic coordinate grid subdivisions and updated axis labels.
10. R10 (prd FR-10): Multi-curve visualization provides configurable color palettes, line styles (solid, dashed), and visibility toggles for functions and their derivatives.
11. R11 (prd FR-11, Story 5): Client serializes complete workspace state (function expressions, slider parameters, viewport coordinates) into a JSON document under 10 KB.
12. R12 (prd FR-12, Story 5, SM-3): Axum backend exposes REST endpoints (`POST /api/workspaces`, `GET /api/workspaces/:id`) returning 8-character workspace identifiers with response latency under 50 ms.

## Constraints

- Client compute: 100% of mathematical computation and rendering must execute client-side in WebAssembly and wgpu with zero server-side math evaluation (prd Section 1, Section 5).
- Rendering stack: Graphics pipeline must target WebGPU via wgpu with automatic WebGL2 fallback on unsupported platforms (prd Section 5, NFR-RELI-02).
- Security: Mathematical expressions must strictly pass through an AST parser; dynamic code execution (eval) is strictly prohibited (prd NFR-SECU-01).
- Payload limit: Axum API request payload must be bounded to <= 64 KB, and serialized workspace state JSON must stay under 10 KB (prd FR-11, NFR-SECU-02).
- Numerical precision: Mathematical evaluation must match IEEE 754 double-precision floats within a tolerance of $\epsilon < 10^{-12}$ for standard operations (prd NFR-FUNC-01).
- Performance budget: Continuous 60 FPS (< 16 ms frame time) during active drag/zoom and slider interactions; curve evaluation and vertex buffer updates under 8 ms for 2000 points; API persistence under 50 ms (prd NFR-PERF-01, NFR-PERF-02, SM-1, SM-3).
- Architecture decoupling: Core math engine crate must compile independently of UI and web dependencies (prd NFR-MAIN-01).

## Done when

- Formula input parses polynomial, trigonometric, exponential, logarithmic, and rational functions with character-accurate syntax error location within 50 ms (FR-1, NFR-USAB-01).
- Canvas maintains 60 FPS during continuous pan and zoom and slider adjustments across WebGPU and WebGL2 browsers (FR-9, NFR-PERF-02, SM-1).
- First and second symbolic derivatives and tangent lines render synchronously with 100% pass rate across the 250-benchmark calculus test suite (FR-3, FR-4, SM-2).
- Automated detection correctly identifies roots, extrema, inflection points, and asymptotes without false singularity connections (FR-5, FR-6, FR-7, NFR-RELI-01).
- Workspaces serialize into < 10 KB JSON and save/load via Axum REST endpoints in < 50 ms (FR-11, FR-12, SM-3).
- Math engine compiles as an isolated Rust crate decoupled from UI/web dependencies (NFR-MAIN-01).

## Boundaries

- Touch point: Client AST parser & evaluator — used by renderer and calculus engine; owner: epic-parser-engine
- Touch point: wgpu render canvas — used for viewport display; owner: epic-rendering-pipeline
- Touch point: Axum REST API — used for workspace sharing; owner: epic-workspace-persistence
- Tracer path: A user enters a parametric function $f(x) = a \cdot x^2 + b$, manipulates slider $a$ with 60 FPS visual feedback, inspects critical points and the tangent line at $x_0$, and shares the workspace via an Axum shortlink that faithfully recreates the session on a second device.
- Must not change: Math calculation must never be offloaded to backend server infrastructure.
- Out of scope (3D): 3D function plotting ($z = f(x, y)$) is deferred beyond MVP (prd Section 9.2).
- Out of scope (Implicit/Parametric): Implicit curve relations ($x^2 + y^2 = r^2$) and parametric curves are deferred beyond MVP (prd Section 9.2).
- Out of scope (Auth): User accounts, authentication, passwords, or team permissions are not part of MVP (prd Section 9.2).
- Out of scope (General CAS): Arbitrary symbolic algebra simplification and general polynomial factorization are deferred beyond MVP (prd Section 9.2).
- Out of scope (SVG export): High-resolution vector SVG export from WASM engine deferred beyond MVP per Decision 2026-10-18 (prd Section 11 Q2).
- Out of scope (Link expiry): Workspace link expiry/TTL retention deferred beyond MVP; MVP links are permanent per Decision 2026-10-18.

## References

- prd — `_bmad-output/initiative-geogebra-rs/prd-geogebra-rs/prd-geogebra-rs.md`, sections 1–11
- glossary — `_bmad-output/initiative-geogebra-rs/glossary.md`
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-1 through AD-6

## Notes

- Assumption: Target user browsers support WebGPU natively or provide compatible WebGL2 support for wgpu (prd Section 11).
- Assumption: Axum persistence backend utilizes SQLite or lightweight storage without user authentication for workspace state (prd Section 11).
- Assumption: Calculus curriculum focuses primarily on single-variable explicit functions $y = f(x)$ on the Cartesian plane (prd Section 11).
- Decision (2026-10-18): Workspace links are permanent for MVP; 90-day expiry deferred beyond MVP, retires R13 (id never reused). Supersedes 90-day expiry decision.
- Decision (2026-10-18): SVG export deferred beyond MVP as out-of-scope. Supersedes SVG question.
- Decision (2026-10-19): New epic epic-frontend-explorer (id 6) chosen over folding frontend into epic-rendering-polish / epic-workspace-persistence; rule: one source capability delivered and deployed together as one Svelte static bundle, contract-only to Rust via wasm_bridge ABI.
- Decision (2026-10-19): Tracer for epic-frontend-explorer is function-bar Analyze loop + DOM-marker overlay sync first; riskiest canvas-geometry + DOM-buttons + query_hover path at 60 FPS, least-sure part second.
- Decision (2026-10-19): Single-function scope-cut confirmed for MVP (single primary + auto f/f'/f''/tangent); multi-function set deferred, architecture multi-curve infra retained, not forbidden.
- Decision (2026-10-19): Spine finalized via bmad-architecture before slicing (AD-4 DOM-buttons amendment, AD-5 eval-only tightening, 2026-10-19); epic-frontend-explorer References cite spine + DESIGN.md + EXPERIENCE.md.
