---
title: 'Screen-space dashed derivatives (4.2)'
type: 'feature'
ticket: '2'
created: '2026-10-19'
status: 'built'
baseline_revision: '16ce70ed9edd7f9be894887c5d23e8fac498c28c'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick', 'quick']
review_loop_iteration: 1
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The 4.1 ribbon path is solid-only (`dash_flag` hardcoded `0.0`, shader ignores `arclength_px`); there is no visual differentiation for derivative overlays `f'` / `f''` per R6.

**Approach:** Emit `style.dashed` as `dash_flag 1.0` through the existing 44-byte vertex contract, implement the 8px-on / 6px-off DPR-aware screen-space dash with antialiased ends in `curve.wgsl`, and mirror it in the 2D fallback so `f` stays solid while derivative styles render dashed.

## Boundaries & Constraints

**Always:** `dash_flag` is `0.0` solid / `1.0` dashed only; arclength stays CSS-px at 1x on CPU, scaled by `dpr` in-shader; edge AA and width clamp [2.5, 6.0] preserved; WGSL-only f32, GL-translatable; deps stay `core_math <- render <- wasm_bridge`; NaN/pole break + arc-reset semantics preserved; `visible=false` still expands to nothing.

**Never:** No `dash_flag 2.0` (reserved for epic 3 asymptote overlays); no vertex layout / stride / location changes; no `grid.wgsl` edits; no 8-colour palette cycle (entry 4.3); no derivative math / `f'` sampling (symbolic-calculus epic owns it); no `Workspace` shape or existing ABI signature changes.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Happy solid | `f=x^2`, `dashed=false`, viewport ±10 | Continuous 3.5px solid antialiased ribbon | No error expected |
| Happy dashed | Same curve, `dashed=true` (simulating `f'`) | Same ribbon with 8px-on / 6px-off screen-space dashes, AA ends | No error expected |
| Pole break | `f=1/x`, dashed on, NaN at pole | Break preserved, arclength restarts, no dash joining across pole | NaN break preserved |
| HiDPI | `dpr=2`, dashed on | Dash measures 8/6 CSS px (16/12 device px), edges still AA | Clamp `max(dpr,1.0)` |
| Reserved flag | Any style input | Emitted `dash_flag` is only 0.0 or 1.0, never 2.0 | No 2.0 emitted |

</frozen-after-approval>

## Code Map

- `crates/render/src/curve.rs` -- ribbon owner; `expand_ribbon` hardcodes `dash_flag 0.0` (lines 206,214), arclength in CSS px with NaN reset (157-198); reuse `clamp_width`, `FunctionStyle.dashed`, `CurveVertex`, `CURVE_VERTEX_STRIDE`; do not change sampler semantics or `POLE_BREAK`.
- `crates/render/shaders/curve.wgsl` -- solid path only; `dash_flag`/`arclength_px` accepted but unused (62-65), `VertexOut` has only `color/u_px/half_px`; reuse `CanvasUniform.dpr`, `half_device` idiom, width clamp; do not change `ViewportUniform`, binding layout, or touch `grid.wgsl`.
- `crates/render/src/pipeline.rs` -- `curve_vertex_layout()` 6 attrs stride 44 matches shader; reuse layout tests; do not change stride/offsets/locations.
- `crates/wasm_bridge/src/lib.rs` -- `set_function_style(id,hex,width,dashed)` + `StyleDto` already generic; reuse as-is, no ABI change, no `Workspace` shape change (no new fields, style persistence belongs to epic 5).
- `frontend/src/Canvas.svelte` -- `draw()` solid-only (66-85), no `setLineDash`; reuse `toScreen`, dirty-flag loop; do not change viewport/hover math.
- `frontend/src/App.svelte`, `frontend/src/StyleControls.svelte` -- style state `{color,widthPx,visible}` with `handleStyle` hardcoding `false`; reuse wrapper pattern; do not change `Workspace` shape.
- Do not touch: `crates/render/src/pipeline.rs` layout, `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js`, `frontend/src/lib/share.js`, `frontend/vite.config.js`, `restoreFromSlug` persistence, or any file outside the five tasks below.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/curve.rs` -- map `style.dashed` to `dash_flag` 1.0/0.0 in `expand_ribbon`, keep arclength CSS-px + NaN reset + `visible` early-return -- connects style flag to GPU contract.
- [ ] `crates/render/shaders/curve.wgsl` -- forward `dash_flag` + `arclength_px*dpr` to fragment, apply 14px-period (8 on / 6 off) discard with ~1px AA fade at dash ends combined with edge AA -- creates R6 dash rendering.
- [ ] `frontend/src/Canvas.svelte` -- honor `style.dashed` via `ctx.setLineDash([8,6])` (solid resets to `[]`), reset after stroke -- keeps 2D fallback in parity with GPU.
- [ ] `frontend/src/App.svelte`, `frontend/src/StyleControls.svelte` -- add `dashed` to style state, pass through `setFunctionStyle` instead of hardcoded `false`, thin demo toggle; do not add `doc.style` persistence or touch `restoreFromSlug` -- proves f-solid vs f-prime-dashed in dev without building derivative math.
- [ ] Edge-case tests -- cover dashed/solid flag mapping, arclength reset on NaN, reserved `2.0` never emitted, shader still parses under naga -- locks I/O matrix.

**Acceptance Criteria:**
- Given `f(x)=x^2` with default style, when dev runs `npm run dev`, then the curve renders as a solid antialiased ribbon.
- Given the dashed toggle enabled for the same curve, when dev runs `npm run dev`, then the curve renders with a repeating 8px-on / 6px-off screen-space dash with smooth ends.
- Given a pole curve with NaN samples and dashed on, when the ribbon expands, then no quad joins across the break and dashing restarts after it.
- Given any style input, when vertices are emitted, then every `dash_flag` is 0.0 or 1.0 and none is 2.0.
- Given `curve.wgsl` updated, when `cargo test -p render` runs, then all tests pass including new dash tests and the shader-parse test.

## Implementation Notes

## Plan Change Log

- Loopback 1 (quick review F1/F3/F4/F5): fixed Design Notes to scale dash period/on/fades by dpr (was fixed 14.0/8.0 device-px, which shrank dashes in CSS px at dpr 2); constrained Code Map + App task to forbid `Workspace` shape changes, `doc.style` persistence, and untasked files (`wasm.js`, `fallback.js`, `share.js`, `vite.config.js`, `pipeline.rs` layout, `restoreFromSlug`). Avoids re-deriving HiDPI-shrink shader, workspace-shape creep, and Epic 5/infra bundling. KEEP: `style.dashed` → `dash_flag` 1.0/0.0 mapping with NaN arc-reset, `VertexOut dash_flag` + `arc_dev` varyings with `<1.5` reserved-2.0 gate, `Canvas.setLineDash([8,6])` parity, Svelte dashed toggle, and the three new ribbon tests.

## Review Triage Log

Quick lens: 5 findings → 1 medium, 3 low, 1 false.

- F1 `crates/render/shaders/curve.wgsl:69,81` HiDPI dash uses fixed 14/8 device-px thresholds on dpr-scaled arclength — verdict medium, route bad_plan. Evidence: `out.arc_dev = arclength_px * dpr` with `fract(arc_dev/14.0)`, so at dpr 2 the dash is 4/3 CSS px, contradicting the matrix HiDPI row (8/6 CSS px); plan Design Notes prescribed the fixed thresholds.
- F2 `frontend/src/App.svelte:194-203` restore drops dashed flag — verdict false. Evidence: current `App.svelte:202,204` reads `loadedStyle.dashed` and preserves it in the rebuilt style; the bad outcome does not occur.
- F3 `crates/wasm_bridge/src/lib.rs:121-124,503-518` `Workspace.style` changes workspace JSON shape — verdict low, route bad_plan. Evidence: frozen Never forbids Workspace shape changes and Code Map said reuse as-is; additive optional field still erodes epic 5 ownership of the shape.
- F4 `frontend/src/lib/wasm.js:40` bundle URL resolves against origin, discarding subpaths — verdict low, route bad_plan. Evidence: with `base: './'` a subpath deploy needs the app subpath, which origin-resolution drops; file is outside the plan task list and the one-line revert would re-break the `/w/<slug>` view, so the fix belongs to a coherent plan, not a patch.
- F5 `frontend/vite.config.js:8`, `frontend/src/lib/share.js`, `frontend/src/lib/fallback.js` Epic 5 / infra changes bundled into 4.2 — verdict low, route bad_plan. Evidence: SPA fallback, style persistence helpers, and fallback serialization are outside plan Tasks lines 57-61; multi-file surface removal is not a trivial patch.
- Loopback 1 re-review (scoped 5-file diff): 4 findings → 4 low, all route patch.
- F6 `frontend/src/Canvas.svelte:69-88` 2D dash phase continues across NaN breaks while GPU restarts arclength — verdict low, route patch. Evidence: loop sets `pen=false` and `moveTo`s without resetting `lineDashOffset`, while `curve.rs` resets `arc` on non-finite samples; phases diverge after a pole on dashed curves.
- F7 `frontend/src/App.svelte:102-106` `handleDashed` drops bridge-returned width — verdict low, route patch. Evidence: spreads old state and overwrites only `dashed`, while `handleStyle:99` syncs `widthPx` from `res`; unreachable via current UI (no width input) but one-line sync makes it consistent.
- F8 `crates/render/src/curve.rs` + `pipeline.rs` tests — no test locks HiDPI dash scaling (matrix HiDPI row) — verdict low, route patch. Evidence: new tests cover flag/arc-reset/reserved plus shader parse only; a revert to fixed 14.0/8.0 thresholds would still pass; smallest fix is a small scaling-lock test, no public surface.
- F9 `frontend/src/Canvas.svelte:67` fallback `lineWidth` unclamped while all other layers clamp — verdict low, route patch. Evidence: direct assignment from `style.widthPx` vs in-shader/ABI/fallback clamps; unreachable via current UI but one-line clamp closes it.

## Design Notes

Dash pattern stays 8/6 CSS px at any DPR: `dpr = max(dpr,1.0)`, `dev = arclength_px * dpr`, `period = 14.0 * dpr`, `on = 8.0 * dpr`. `t = fract(dev / period) * period`; solid when `dash_flag < 0.5`, else keep when `t < on`, 1-device-px AA fade `on..on+1*dpr`, discard `on+1*dpr..period-1*dpr`, 1-device-px fade-in `period-1*dpr..period`. CPU keeps arclength in CSS px so the same buffer is correct at any DPR; never compare dpr-scaled arclength against fixed 14.0/8.0 thresholds.

## Verification

**Commands:**
- `cargo test -p render -p wasm_bridge` -- expected: all pass including new dash-flag/arclength/2.0-reserved tests
- `cargo test -p render shaders_parse_as_wgsl` -- expected: pass, dashed shader still valid WGSL
- `cargo test -p render two_thousand_points_are_fast` -- expected: pass, <8ms budget intact
- `npm run dev --prefix frontend` then toggle dashed -- expected: `f` solid by default, dashed on toggle, no console errors
