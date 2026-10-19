---
type: epic
title: "Function explorer frontend"
parent: initiative-geogebra-rs
covers: ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12"]
sources: ["_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md"]
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md", "_bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md", "_bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md"]
after: ["epic-scaffold-tracer", "epic-symbolic-calculus", "epic-asymptote-guard", "epic-rendering-polish", "epic-workspace-persistence"]
risk: medium
refined: true
---

# Function explorer frontend

## Description

Svelte 5 explorer shell integration part only, contract-only via the wasm_bridge ABI: renders DESIGN tokens, the function-bar Analyze loop with char-exact errors under 50 ms, param and point sliders, derivative card, DOM marker buttons synced to canvas geometry at 60 FPS, legend and toggles, and the share row, per EXPERIENCE.md. Engine, render-pipeline, and API budgets stay in epics 1–5. Single-function cut: single primary plus auto-displayed f/f'/f''/tangent only; multi-function set deferred.

## Outcome

For students and lecturers, the explorer delivers the friendly single-function workspace from layout.png on the deployed static bundle, verified by Analyze-to-share working keyboard-first with CALCULATED freshness and 60 FPS slider feedback.

## Requirements

1. R1 (parent R1): Function bar accepts a single primary expression, validates on keystroke, and shows char-exact errors within 50 ms; Enter or Analyze fires WASM analysis, Try chips fill and fire immediately, and errors block Analyze while holding the last good canvas.
2. R2 (parent R2): Parameter sliders render one row per detected parameter under the function bar only when the expression contains parameters; dragging re-renders curves and values eval-only without refiring symbolic analysis.
3. R3 (parent R3): Derivative card shows the symbolic f' readout from WASM with a CALCULATED badge only on fresh values; the f'' line auto-renders toggleable beneath f' in DESIGN purple.
4. R4 (parent R4): Point slider over the current x-domain drives the tangent marker at x0 with live function value, slope to 3 decimals, behavior word, and tangent line at 60 FPS; arrow keys move +-0.1 as the keyboard equivalent.
5. R5 (parent R5): Root markers render as DOM buttons positioned over the canvas from the query_hover overlay shape, opening coordinate tooltips on hover, focus, or click.
6. R6 (parent R6): Extrema and inflection markers render as DOM buttons visually distinct from roots, opening coordinate tooltips; an empty marker layer stays silent with no banner.
7. R7 (parent R7): Poles break curves with gray dashed asymptotes; the point readout at a pole shows an em-dash, never NaN or infinities as finite numbers.
8. R8 (parent R8): Graph canvas mounts the wgpu pipeline with WGSL geometry; the fallback message shows only when WebGPU and WebGL2 adapters both fail, leaving the rest of the workspace usable.
9. R9 (parent R9): Pan by drag and zoom by wheel, buttons, or reset update the domain label and re-render the visible domain without refiring analysis; analysis state survives viewport changes.
10. R10 (parent R10): Curves use the fixed DESIGN mapping of indigo f solid, green f' dashed, amber tangent dashed, purple f'' dashed, gray asymptotes; legend swatches mirror curve style and toggle the same layer, canvas-only re-render, single primary plus auto curves only.
11. R11 (parent R11): Share row appears after the first successful Analyze with a readonly shortlink and Copy confirming inline; failures show an inline retry with analysis untouched.
12. R12 (parent R12): Opening a shortlink restores expression, slider positions, and toggles to the identical session; offline openers see cached expression with sharing disabled and the offline note.
13. R13 (EXPERIENCE.md Accessibility Floor): Focus order, keyboard sliders and markers, aria-live value announcements on release, 44 px targets via hit padding, and prefers-reduced-motion snapping hold across the workspace.

## Constraints

- Client compute: 100% of math and rendering executes client-side in WebAssembly and wgpu with zero server-side math evaluation; Svelte never performs spatial math, only positions tooltips (parent client compute, AD-5).
- Rendering stack: WebGPU via wgpu with automatic WebGL2 fallback; WGSL only; fallback message only when both adapters fail (parent rendering stack, AD-4).
- Security: expressions pass strictly through the AST parser; dynamic code execution is prohibited (parent security, AD-2).
- Payload limit: workspace JSON stays under 10 KB with a version field; API payloads cap at 64 KB; slugs are 8-char URL-safe (parent payload, AD-6).
- Performance budget: 60 FPS under 16 ms during drags and slider moves; sampler plus ribbon under 8 ms each; API under 50 ms; DOM-overlay sync never breaks the budget (parent performance, AD-5 eval-only).
- Architecture decoupling: core_math compiles independently of UI and web dependencies; one-way core_math to render to wasm_bridge; style struct lives in render (parent decoupling, AD-1).

## Done when

- Function bar validates char-exact under 50 ms, Analyze fires WASM analysis, Try chips fill and fire, empty and error states hold last good canvas per EXPERIENCE.md.
- Graph canvas mounts wgpu pipeline, pans and zooms with domain label, breaks at poles with gray dashed asymptotes, shows fallback message only when WebGPU and WebGL2 both fail.
- Analysis markers render as DOM buttons with 44 px hit padding, focus labels, and coordinate tooltips from query_hover, synced to canvas at 60 FPS; canvas carries role=img with text fallback.
- Derivative card shows f' with CALCULATED badge and always auto-renders toggleable f'' dashed purple; point analysis value rows update live with pole showing em-dash, never NaN or infinities as finite.
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
- Decision (2026-10-19): Fixed DESIGN palette wins for single-function mode; free color picker narrows to visibility in this epic.
- Decision (2026-10-19): Remove g/h demo curves in this epic; wire single primary f only.
- Decision (2026-10-19): Time-boxed spike first for DOM-overlay sync at 60 FPS; tracer second.
- Decision (2026-10-19): f'' always auto-renders toggleable dashed purple.
- Decision (2026-10-19): Verification harness left to stories with per-story runnable checks plus a closing accessibility and performance pass.
- Decision (2026-10-19): Planning tracer is entry 2 Analyze loop with single curve and marker buttons (hitl dev-server check); entry 1 is the time-boxed DOM-sync spike per user override of the tracer-first rule; mount handoff is tracer minimal via scaffold pipeline, entry 7 extends the full mount.
- Assumption: Contrast ratios target WCAG AA but are unmeasured; verify before build closes accessibility.
- Assumption: Sub-1024 px widths fall back to horizontal scroll; no mobile reflow in this run.
