---
title: 'Tracer bullet: styled ribbon with shader contract (4.1)'
type: 'feature'
ticket: '1'
created: '2026-10-18'
status: 'built'
baseline_revision: '8e570bd97bfd198667dfc669c2225e7ec062a9e9'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Curves render as 1px hairlines with hardcoded tracer blue and no per-function style; there is no `FunctionStyle` owner, no shader contract, and no visibility toggle for one styled curve.

**Approach:** Stand up `FunctionStyle` in `render`, bring `curve.wgsl` to the E4-SHADER-CONTRACT solid path, expose `set_function_style` + `set_function_visibility` stubs on the bridge Session, and wire thin Svelte controls for a single `f(x)=x^2` ribbon at 3.5px with toggle.

## Boundaries & Constraints

**Always:** Style struct lives in `render`, keyed by id in bridge Session; width clamped [2.5, 6.0]px default 3.5 at ABI and in-shader; `visible=false` skips submission and retains entry; WGSL-only f32 shaders, GL-translatable; deps stay `core_math <- render <- wasm_bridge`; `POLE_BREAK=1e6`/NaN-break semantics read-only; `dash_flag` solid `0.0` only, `2.0` reserved for epic 3.

**Never:** No dashed derivatives (entry 4.2), no 8-colour palette UI or full cycle logic (entry 4.3), no grid changes, no workspace JSON shape change, no existing ABI signature changes, no `grid.wgsl` edits.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Happy styled curve | `f=x^2`, default style, viewport ±10 | 3.5px antialiased ribbon in first palette colour | No error expected |
| Hide curve | `visible=false` for `f`, then toggle back | Canvas skips `f` submission, entry retained; toggle restores ribbon | No error; no delete |
| Width clamp | `set_function_style` with 1.0px and 10.0px | Stored and rendered width clamps to 2.5 and 6.0 | Clamp, no reject |
| Bad color hex | `color_hex="not-a-color"` | Style unchanged, previous colour kept | Reject input, keep old |
| Pole NaN preserved | `f=1/x` with style on | Ribbon breaks at NaN, no spike across pole | NaN break preserved |

</frozen-after-approval>

## Code Map

- `crates/render/src/curve.rs` -- sampler owner; add `FunctionStyle`, defaults, hex→linear-sRGB, ribbon expansion helper; reuse `sample_curve`, `POLE_BREAK`, NaN breaks; do not change sampler semantics.
- `crates/render/shaders/curve.wgsl` -- currently `vec2` LineStrip + hardcoded blue; extend to contract solid path (per-vertex color/width/dash solid, `canvas_px`/`dpr` uniforms, AA); reuse `ViewportUniform` layout; do not touch `grid.wgsl`.
- `crates/render/src/pipeline.rs` -- `curve_vertex_layout()` stride 8 single attr + `CURVE_SHADER` include + naga test; update layout to 7-attr contract; do not change probe order, `ViewportUniform`, revision logic.
- `crates/wasm_bridge/src/lib.rs` -- `Session{functions,formulas,params,viewport}` + JSON-string ABI; add `styles: HashMap<String,FunctionStyle>`, `set_function_style` + `set_function_visibility`; reuse `set_function_core`, `active_expr`, ident checks; do not change existing export signatures or `Workspace v1`.
- `frontend/src/Canvas.svelte` -- 2D fallback `draw()` hardcodes `#2563eb`/2px; parameterize to style color/width/visibility; reuse `toScreen`, dirty-flag loop; do not change viewport/hover math.
- `frontend/src/App.svelte`, `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- `setFunction/updateParam` wrapper pattern; add thin style wiring + `StyleControls.svelte` (color swatch, width display, visibility checkbox); do not change `Workspace` shape or mount path.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/curve.rs` -- add `FunctionStyle{color:[f32;4],width_px:f32,dashed:bool,visible:bool}`, `default_style()`, `for_function_index(0)` returning tracer-blue linear-sRGB, `clamp_width`, `hex_to_linear`, `expand_ribbon` stub emitting per-vertex attrs -- establishes render-owned style truth.
- [ ] `crates/render/shaders/curve.wgsl` -- implement contract solid path: `position_math,normal_dir,color,width_px,dash_flag,arclength_px` inputs, viewport+`canvas_px`/`dpr` uniforms, width clamp [2.5,6.0], edge AA, solid branch only -- creates shader side of contract.
- [ ] `crates/render/src/pipeline.rs` -- update `curve_vertex_layout()` to 7-attribute stride matching shader locations, update `vertex_layout_matches_shader_input` + `shaders_parse_as_wgsl` tests -- keeps CPU/GPU layout in sync.
- [ ] `crates/wasm_bridge/src/lib.rs` -- add `styles` map with auto-default on `set_function`, implement `set_function_style(id,color_hex,width_px,dashed)` with hex validation + clamp and `set_function_visibility(id,visible)` retaining entry, unit tests -- exposes E4-AD-5-AMEND surface.
- [ ] `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- add `setFunctionStyle`/`setFunctionVisibility` wrappers with JSON parse boundary and fallback parity -- keeps Rust/fallback backends interchangeable.
- [ ] `frontend/src/StyleControls.svelte`, `frontend/src/App.svelte`, `frontend/src/Canvas.svelte` -- new thin controls (first-colour swatch, width label, visibility checkbox) wired to bridge, `draw()` uses style color/3.5px/skips when hidden -- proves end-to-end tracer.
- [ ] Edge-case tests -- cover width clamp, bad hex rejection, visibility retain/restore, NaN-break preservation in ribbon path -- locks I/O matrix.

**Acceptance Criteria:**
- Given `f(x)=x^2` with default style, when dev runs `cargo test` and `npm run dev`, then canvas shows a 3.5px antialiased ribbon in first palette colour.
- Given visibility unchecked for `f`, when frame renders, then `f` is skipped but its style entry is retained, and when re-checked, then ribbon returns without re-entering formula.
- Given `set_function_style` called with width 1.0 and 10.0, when style is read back, then widths are 2.5 and 6.0 respectively.
- Given sampler output contains NaN at a pole, when ribbon expansion runs, then the break is preserved with no joined spike.
- Given `curve.wgsl` updated, when `cargo test shaders_parse_as_wgsl` runs, then both shaders parse and the vertex layout test matches the contract stride.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Quick lens: 10 findings → 1 medium, 6 low, 1 false, 1 low-rejected, 1 deferred (api/routes sibling 5.1, not this change).

- F1 `crates/render/src/curve.rs:106` CurveVertex missing `#[repr(C)]` — verdict medium, route patch. Evidence: struct has no repr, pipeline assumes C offsets 0/8/16/32/36/40 stride 44; size assert passes but order/padding not guaranteed, future GPU misalignment.
- F2 `crates/render/shaders/curve.wgsl:27` CanvasUniform binding without Rust provider — verdict low, rejected (low-reject rule). Evidence: no CanvasUniform/bind-group in `render/src`, but no pipeline creation exists yet (`init_gpu` only acquires device); 2D fallback draws, shader parses; everyday tracer use never hits it and provider needs new uniform infrastructure beyond tracer.
- F3 `crates/wasm_bridge/src/lib.rs:424` sample_points ignores visibility + F4 `crates/render/src/curve.rs:125` expand_ribbon ignores visible — grouped, verdict low, route patch. Evidence: Canvas skips draw so AC holds visually, but Rust still samples/expands hidden curves; fix is early-return empty when `!visible`.
- F5 `crates/api/src/routes.rs:30` flat workspace validation — verdict medium (for its callers), route defer (not this change). Evidence: change comment marks 5.1 tracer contract, 4.1 implementer touched only render/bridge/frontend; sibling work in same diff since baseline.
- F6 `crates/wasm_bridge/src/lib.rs:494` is_ident Unicode vs fallback ASCII — verdict low, route patch. Evidence: Rust `is_alphanumeric` accepts `café`, fallback `/^[A-Za-z_][A-Za-z0-9_]*$/` rejects; function ids are `f`/`g` in practice.
- F7 `frontend/src/lib/fallback.js:217` bad-hex inserts default vs Rust get-only — verdict low, route patch. Evidence: same bad-hex call on unknown id stores entry in fallback, none in Rust; fix fallback to not insert on bad hex.
- F8 bare `2563eb` accepted without `#` — verdict false. Evidence: both Rust and fallback consistently accept bare, App always sends `#`, no AC or parity rule broken; strict-`#` requirement invented by reviewer.
- F9 `frontend/src/lib/wasm.js:42` stale-bundle sanity misses new exports — verdict low, route patch. Evidence: check only covers `parse_and_validate`, `wrapRust` calls new fns unconditionally; current bundle rebuilt so only stale deploys hit it.
- F10 `frontend/src/StyleControls.svelte:24` width label NaN — verdict low, route patch. Evidence: App always seeds 3.5 and preserves via `??`, so undefined never occurs in practice; default in control is one-line hardening.

## Design Notes

Tracer keeps first colour equal to existing tracer blue `#2563eb` (linearized in `render`) so the 2D fallback and GPU path stay identical; full 8-colour Okabe-Ito cycle defers to entry 4.3 to avoid palette-shape collision. Solid path only (`dash_flag 0.0`); `arclength_px` varying is emitted but dash branch lands in 4.2. Width clamps twice (ABI + shader) per E4 conventions. New `StyleControls.svelte` reuses `Sliders.svelte` forward pattern, minimal for one curve.

## Verification

**Commands:**
- `cargo test -p render -p wasm_bridge` -- expected: all pass including new style/ribbon tests
- `cargo test -p render two_thousand_points_are_fast` -- expected: pass, <8ms budget intact
- `cargo test -p render shaders_parse_as_wgsl` -- expected: pass, contract shader valid
- `npm run dev --prefix frontend` then type `x^2` -- expected: 3.5px ribbon in first colour, visibility toggle hides/restores
