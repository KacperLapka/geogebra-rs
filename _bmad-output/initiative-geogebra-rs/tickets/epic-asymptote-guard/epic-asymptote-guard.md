---
type: epic
title: "Asymptote and discontinuity guard"
parent: initiative-geogebra-rs
covers: ["R7"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md"]
after: ["epic-symbolic-calculus"]
risk: medium
refined: true
---

# Asymptote and discontinuity guard

## Description

Adds pole/singularity detection to the curve sampler in `render` and asymptote classification to `core_math`, ensuring rational functions and transcendental functions (`tan(x)`, `1/x`, `x/(x-2)`) never draw false connecting lines across discontinuities, and render correct dashed vertical, horizontal, and slant asymptote overlays.

## Outcome

A mathematics educator plots rational and transcendental functions and sees mathematically rigorous rendering — no bogus vertical spikes, no NaN crashes, correct asymptote markers — matching the behaviour expected from production-grade analysis software (R7, NFR-RELI-01).

## Requirements

1. R1 (parent R7): Engine detects vertical, horizontal, and slant asymptotes ($y = mx + b$) without drawing false connecting lines across singularities or propagating NaN values.

## Constraints

- Client compute: 100% of mathematical computation and rendering must execute client-side in WebAssembly and wgpu with zero server-side math evaluation.
- Rendering stack: Graphics pipeline must target WebGPU via wgpu with automatic WebGL2 fallback on unsupported platforms.
- Security: Mathematical expressions must strictly pass through an AST parser; dynamic code execution (eval) is strictly prohibited.
- Numerical precision: Mathematical evaluation must match IEEE 754 double-precision floats within a tolerance of $\epsilon < 10^{-12}$ for standard operations.
- Performance budget: Continuous 60 FPS (< 16 ms frame time) during active drag/zoom and slider interactions; curve evaluation and vertex buffer updates under 8 ms for 2000 points.
- Architecture decoupling: Core math engine crate must compile independently of UI and web dependencies.

## Done when

- Vertical asymptotes detected by pole-magnitude threshold; curve strip split at each pole with no false connecting line (R7, NFR-RELI-01).
- Vertical asymptote dashed overlay rendered at each detected pole (R7).
- Horizontal and slant asymptotes (`y = L` and `y = mx + b`) detected and overlaid for eligible rational functions (R7).
- `f(x) = tan(x)`, `f(x) = 1/x`, `f(x) = (x²-1)/(x-2)` all render correctly without NaN propagation into GPU buffers (NFR-RELI-01).

## Boundaries

- Covers: pole detection in `render/curve.rs` and asymptote classification in `core_math/solver.rs`.
- Does not cover: rendering style polish or multi-curve configuration (epic 4).
- Waits on epic-symbolic-calculus because: needs the Brent solver and sign-change logic to distinguish real zeros from divergent poles.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirement R7
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-3, AD-4

## Notes

- Decision: 2026-10-18 — Refine from parent R7 alone; no extra elicitation, no PRD/UX/architecture run.
- Decision: 2026-10-18 — Settle horizontal/slant scope now: degree-comparison for rationals plus limit-sampling fallback for transcendentals, as two stories.
- Decision: 2026-10-18 — Settle overlay contract now: `solver` returns pole x positions plus Horizontal(y=L) / Slant(m,b); `curve` builds dashed strips from them.
- Decision: 2026-10-18 — Leave POLE_BREAK / POLE_THRESHOLD duplication to first story; `core_math` stays web-free, value documented as 1e6.
- Decision: 2026-10-18 — No spike; uniform sampler plus existing `breaks_at_pole` and `skips_pole_of_reciprocal` tests prove the approach, density tuning goes to stories.
- Decision: 2026-10-18 — Tracer bullet is entry 1 (1/x pole break with vertical overlay using 2.2 Brent output); entry 2 (tan periodic, high-risk) next as least-sure; entry 3 (degree-compare) parallel from 1 on disjoint solver function; entry 4 wires all overlays with 8ms check; entry 5 refactor sweep. Single solver/curve lane, entries 2-3 ordered only by file-shared sequencing via entry 1.
