---
type: epic
title: "Function explorer frontend"
parent: initiative-geogebra-rs
covers: ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md", "_bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md", "_bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md"]
after: ["epic-scaffold-tracer", "epic-symbolic-calculus", "epic-asymptote-guard", "epic-rendering-polish", "epic-workspace-persistence"]
risk: medium
refined: false
---

# Function explorer frontend

## Description

Svelte 5 explorer shell integration part only, contract-only via the wasm_bridge ABI: renders DESIGN tokens, the function-bar Analyze loop with char-exact errors under 50 ms, param and point sliders, derivative card, DOM marker buttons synced to canvas geometry at 60 FPS, legend and toggles, and the share row, per EXPERIENCE.md. Engine, render-pipeline, and API budgets stay in epics 1–5. Single-function cut: single primary plus auto-displayed f/f'/f''/tangent only; multi-function set deferred.

## Outcome

For students and lecturers, the explorer delivers the friendly single-function workspace from layout.png on the deployed static bundle, verified by Analyze-to-share working keyboard-first with CALCULATED freshness and 60 FPS slider feedback.

## Done when

- Function bar validates char-exact under 50 ms, Analyze fires WASM analysis, Try chips fill and fire, empty and error states hold last good canvas per EXPERIENCE.md.
- Graph canvas mounts wgpu pipeline, pans and zooms with domain label, breaks at poles with gray dashed asymptotes, shows fallback message only when WebGPU and WebGL2 both fail.
- Analysis markers render as DOM buttons with 44 px hit padding, focus labels, and coordinate tooltips from query_hover, synced to canvas at 60 FPS; canvas carries role=img with text fallback.
- Derivative card shows f' with CALCULATED badge, f'' line when in scope; point analysis value rows update live with pole showing em-dash, never NaN or infinities as finite.
- Legend and toolbar toggles for f/f'/f''/tangent/markers re-render canvas only without refiring analysis; param sliders re-render eval-only.
- Share row appears after first Analyze, copies 8-char shortlink with inline confirmation, restores expression plus slider position plus toggles; offline and failure states per EXPERIENCE.md.
- Deployed static bundle passes keyboard order, aria-live announcements, and prefers-reduced-motion checks.

## Boundaries

- Covers: `frontend/` Svelte presentation and DOM-overlay sync only.
- Does not cover: core_math parsing, differentiation, solvers (owner: epic-symbolic-calculus); pole detection math and asymptote overlay geometry (owner: epic-asymptote-guard); wgpu ribbon, grid, style structs, perf budgets (owner: epic-rendering-polish); Axum blob store and slug logic (owner: epic-workspace-persistence); Vite and wasm-pack pipeline setup (owner: epic-scaffold-tracer).
- Touch point: wasm_bridge ABI parse_and_validate, set_function, update_param, update_viewport, query_hover — consumed as contract; owner: epic-scaffold-tracer.
- Touch point: marker and tangent data shape via query_hover overlay — consumed; owner: epic-symbolic-calculus.
- Touch point: pole and asymptote overlay shape and break contract — consumed; owner: epic-asymptote-guard.
- Touch point: set_function_style and visibility surface, ribbon and grid pipeline — consumed; owner: epic-rendering-polish.
- Touch point: workspace JSON schema with version, 8-char slug, 64 KB cap, POST and GET contract — consumed; owner: epic-workspace-persistence.
- Must not change: 100% client compute, no eval, f64 math precision, 8 ms sampler and 16 ms frame budgets owned by epics 1–4.
- Single-function cut: multi-curve infra retained opaque in epic-rendering-polish; this epic wires single primary plus auto f/f'/f''/tangent only.
- Waits on epic-scaffold-tracer because: needs the Vite and wasm-pack pipeline, parse-eval-render pipeline, and wasm_bridge surface.
- Waits on epic-symbolic-calculus because: needs the marker and tangent data shape and query_hover overlay contract.
- Waits on epic-asymptote-guard because: needs the pole and asymptote overlay shape and break contract.
- Waits on epic-rendering-polish because: needs the style and visibility surface and ribbon and grid pipeline.
- Waits on epic-workspace-persistence because: needs the workspace JSON schema and share contract.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirements R1, R2, R3, R4, R5, R6, R7, R8, R9, R10, R11, R12
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-4 including 2026-10-19 DOM-buttons amendment, AD-5 including 2026-10-19 eval-only tightening
- design — `_bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md`, full token and component spec
- experience — `_bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md`, component patterns, states, accessibility floor, key flows

## Notes

- Decision (2026-10-19): Tracer bullet is function-bar Analyze loop plus DOM-marker overlay sync; riskiest canvas plus DOM plus query_hover path at 60 FPS first, least-sure part second.
- Decision (2026-10-19): Tree check widened covers to R1–R12 display and integration facets with part-split in description; engine, render, and API budgets stay in epics 1–5; after widened to epics 1–5 for overlay, style, pole, and share contracts.
- Decision (2026-10-19): Single-function cut confirmed for MVP; multi-function set deferred, multi-curve infra retained opaque.
