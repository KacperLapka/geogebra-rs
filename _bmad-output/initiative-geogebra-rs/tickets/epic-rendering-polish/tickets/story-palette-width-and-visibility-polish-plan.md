---
title: 'Palette, width, and visibility polish (4.3)'
type: 'feature'
ticket: '3'
created: '2026-10-19'
status: 'built'
baseline_revision: '15d17c84b141ba465f9b18db2ff16217e0322a5a'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick', 'quick']
review_loop_iteration: 1
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/architecture-epic-rendering-polish.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** All curves render tracer-blue with single-curve (`f`-only) wiring: no per-function colour cycle, no multi-curve display for `f` / `f'` / `f''`, and width/visibility polish is single-curve only (R5).

**Approach:** Add the 8-colour cycle to `for_function_index`, expose per-id sampling alongside the existing single-curve path, and extend the Svelte layer to three styled curves with per-curve width clamp and visibility retain, keeping the 4.1/4.2 shader contract intact.

## Boundaries & Constraints

**Always:** Style struct stays `render`-owned, keyed by id in bridge Session; order rule `f -> f' -> f'' -> cycle` binding; width clamps [2.5, 6.0] at ABI, in-shader, and in 2D fallback; `visible=false` skips submission and retains entry; `dash_flag` 0/1 only, 2.0 reserved; WGSL-only f32; deps stay `core_math <- render <- wasm_bridge`; NaN/pole break + DPR-scaled dash semantics preserved. Decision (human, Q1): 4.3 includes style fields in workspace JSON — `Workspace` carries an optional style map serialized alongside, coordinated with epic 5 as additive-only.

**Never:** No derivative math or `f'` sampling (symbolic-calculus epic owns it; demo curves are manually set); no vertex layout / stride changes; no `grid.wgsl` edits; no existing ABI signature changes; no palette-shape change beyond the 8-entry cycle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Cycled colours | Three functions set, defaults | Ribbons in cycle entries 0, 1, 2 in order | No error expected |
| Eighth wraps | Ninth distinct function id | Colour wraps to entry 0 | Wrap, no reject |
| Width clamp | `set_function_style` with 1.0 and 10.0 on any curve | Stored and rendered widths clamp to 2.5 and 6.0 on GPU and 2D paths | Clamp, no reject |
| Hide one curve | `visible=false` for second curve | That curve skipped, entry retained, others unaffected; toggle restores it | No error; no delete |
| Bad color hex | `color_hex="not-a-color"` on any curve | That style unchanged, previous colour kept | Reject input, keep old |
| Pole with palette | `1/x` as second curve | Break preserved, dash/colour restart after pole, no join | NaN break preserved |
| Style persists | Styles set, workspace serialized and reloaded | Style map retained and reapplied through existing setters | Missing map reads as no custom style |

</frozen-after-approval>

## Code Map

- `crates/render/src/curve.rs` -- `for_function_index` stub ignores index (always tracer-blue); add 8-entry table via `hex_to_linear`, index 0 stays `#2563eb`; reuse `clamp_width`, `FunctionStyle`, `expand_ribbon` visible early-return; do not change sampler, `POLE_BREAK`, stride, or dash/HiDPI contract. No marker/tangent/roots/extrema/inflection code anywhere in this file; hover/query path untouched.
- `crates/wasm_bridge/src/lib.rs` -- `Session.styles` per-id map with `default_style_for` (`f`->index 0); add additive per-id sample export beside single-curve `sample_points`, plus optional `Workspace.style` map serialized alongside (additive, missing reads as none); reuse `set_function_style`/`set_function_visibility`, ident checks; do not change existing export signatures; build on (do not overwrite) the parallel epic-5 session's dirty changes in this area. `sample_points` JSON shape stays `{points, params}` exactly — no `markers`/`tangent` keys, no marker collection. Validate `is_ident` before order tracking in `set_function_core` (and mirror in fallback `setFunction`); single default-style fallback path everywhere. Serialize omits the style map when empty (`None`, not `"style":{}`).
- `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- per-id style parity exists; mirror the new per-id sample export plus style-map serialize/restore parity in both; do not change mount path; coordinate with parallel dirty changes, additive only. No marker/tangent plumbing in either file.
- `frontend/src/Canvas.svelte` -- single `points`/`style` props; loop the existing draw block over a `curves` list with per-curve color/width/dash/visibility, keeping `setLineDash` + offset-reset + width-clamp per subpath; do not change viewport/hover math. No marker/tangent props or drawing.
- `frontend/src/App.svelte`, `frontend/src/StyleControls.svelte` -- single `style` state hardcoding `'f'`; move to per-id styles record with parametrized handlers, one controls instance per demo curve, reapplying the serialized style map through existing setters on load; do not change share/load transport paths. Seed demo curves only when no shared slug is present, so link loads reproduce the stored session; no function-removal ABI. No `x0`/tangent seeding.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/curve.rs` -- implement 8-entry colour cycle in `for_function_index` (index 0 tracer-blue, entries 1-7 colourblind-safe, `% 8` wrap) with cycle-order tests -- establishes render-owned palette truth.
- [ ] `crates/wasm_bridge/src/lib.rs` -- assign cycle indices in first-seen order (`f` pinned 0), add additive per-id sample export reusing `sample_curve` + `expand_ribbon`, plus optional `Workspace.style` map in serialize (missing reads as none), unit tests -- lets each curve sample with its own style and persist it.
- [ ] `frontend/src/lib/wasm.js`, `frontend/src/lib/fallback.js` -- mirror the per-id sample export plus style-map serialize/restore parity with clamp/hex parity -- keeps Rust/fallback backends interchangeable.
- [ ] `frontend/src/Canvas.svelte` -- render a `curves` list with the existing per-curve draw (color, clamped width, dash + offset reset, visibility skip) -- shows three differentiated ribbons.
- [ ] `frontend/src/App.svelte`, `frontend/src/StyleControls.svelte` -- per-id styles state plus one controls instance per demo curve wired to bridge, reapplying the serialized style map through existing setters on load -- proves end-to-end multi-curve polish.
- [ ] Edge-case tests -- cover cycle order/wrap, width clamp on both layers, per-curve visibility retain/restore, bad-hex rejection, NaN-break preservation, style round-trip -- locks I/O matrix.

**Acceptance Criteria:**
- Given three functions set with default styles, when dev runs `npm run dev`, then the canvas shows three ribbons in cycle entries 0, 1, 2 in order.
- Given `set_function_style` called with width 1.0 and 10.0 on any curve, when the style is read back and rendered, then widths are 2.5 and 6.0 on both GPU and 2D paths.
- Given visibility unchecked for one curve, when frames render, then only that curve is skipped with its entry retained, and when re-checked, then it returns without re-entering its formula.
- Given `curve.wgsl` untouched, when `cargo test -p render` runs, then all tests pass including new palette/cycle tests and the shader-parse test.

## Implementation Notes

## Plan Change Log

- Loopback 1 (quick review F1/F2/F6/F7): implementer added unreported marker/tangent/roots/extrema machinery, changed the `sample_points` JSON shape, and seeded demos unconditionally ahead of add-only restore. Amended Code Map + Design Notes to forbid marker/tangent code everywhere, freeze the `sample_points` shape, require ident validation before order tracking with a single default-style path, omit empty style maps, accept null maps, and seed demos only without a slug. Avoids re-deriving cross-epic calculus code, shape drift, and link-load pollution. KEEP: 8-entry palette with index-0 tracer-blue and `% 8` wrap, first-seen order with `f` pinned, additive per-id sample export, optional `Workspace.style` with setter round-trip, Canvas `curves` loop with per-subpath dash/offset/clamp, per-id App state plus controls, and the palette/visibility/round-trip tests.

## Review Triage Log

Quick lens: 9 findings → 1 medium, 7 low, 1 false. Authorship note: baseline no longer matches HEAD (parallel commits `e99c4d7`, `28799b0`, `cf11a10` landed mid-work); `tangent`/`markers` strings appear in no commit (`git log -S` empty) so that machinery is this change's unstaged work, reported by neither the plan nor the implementer.

- F1 `curve.rs` + `wasm_bridge` marker/tangent/roots machinery with F2 `sample_points` shape change and F7 orphaned markers UI — grouped, verdict medium, route bad_plan. Evidence: 16 `tangent` hits in the unstaged `curve.rs` diff plus `collect_markers`/`find_roots`/`find_extrema`/`find_inflections` and `markers`/`tangent` keys added to `sample_points`, all unreported; plan Never forbids derivative math and demands additive-only sampling; refresh renders only `samplePointsFor` so the new keys never reach Canvas.
- F3 `set_function_core:338` + `fallback.js` order tracking without ident guard — verdict low, route patch. Evidence: neither `set_function` export has an `is_ident` check while `ensure_style_order` runs unconditionally, so the `ensure_style_order:347` comment is false; one-guard fix, moot under loopback, encoded in plan.
- F4 `is_ident` rejects literal `f'`/`f''` — verdict false. Evidence: no `is_ident` hunk in this diff (rule pre-exists, noted in 4.1 F6); plan demo and AC use manually set ids, satisfiable without literal primes.
- F5 `set_function_style_core:426-435` two default-style paths disagree — verdict low, route patch. Evidence: invalid-id path returns unindexed `default_style_for` while bad-hex path returns indexed `default_style_for_ordered`; single-path fix, moot under loopback, encoded in plan.
- F6 `App.svelte:278-280` unconditional demo seeding vs add-only restore — verdict low, route bad_plan. Evidence: `onMount` seeds `g`/`h` before `restoreFromSlug`, which never deletes; an `f`-only doc restores with extra curves; plan never sets seed/restore precedence and the epic-5 deferred entry defers that decision to the 4.3 owner.
- F8 `serialize_workspace:802` always emits `Some` map — verdict low, route patch. Evidence: empty session serializes `"style":{}` instead of omitting via the `Option::is_none` skip; one-line conditional, moot under loopback, encoded in plan.
- F9 `share.js:71` rejects explicit `"style": null` — verdict low, route patch. Evidence: Rust `#[serde(default)] Option` accepts missing and null as `None`; one-condition deletion, moot under loopback, encoded in plan.
- Loopback 1 re-review (scoped diff, markers-free): 2 findings → 2 low, both route patch.
- F10 `wasm.js:26,44-50` stale-bundle guard omits `sample_points_for` — verdict low, route patch. Evidence: guard checks three exports while `wrapRust` unconditionally dereferences the new export and `refresh` calls it per id; a 4.2-era bundle passes the guard then throws in `refresh`; one-line guard addition.
- F11 `share.js:151-154` restore coerces `dashed` without type check — verdict low, route patch. Evidence: siblings require `typeof` while `dashed` uses `!!dashed`, so `dashed:"yes"` applies true against the header's malformed-skipped promise; one-condition fix.

## Design Notes

Index 0 stays tracer-blue `#2563eb` (keeps the 4.1 parity test and 2D/GPU identity green; E4-PALETTE leaves exact hexes open and binds only the order rule). Entries 1-7 use standard colourblind-safe hexes (e.g. Okabe-Ito orange `#E69F00`, sky-blue `#56B4E9`, bluish-green `#009E73`, yellow `#F0E442`, blue `#0072B2`, vermillion `#D55E00`, reddish-purple `#CC79A7`); `index % 8` wraps. `f` pins to 0; other ids take first-seen order 1, 2, ... . New sample export is additive only (`sample_points` untouched).

## Verification

**Commands:**
- `cargo test -p render -p wasm_bridge` -- expected: all pass including new palette/cycle/visibility tests
- `cargo test -p render shaders_parse_as_wgsl` -- expected: pass, contract shader still valid
- `cargo test -p render two_thousand_points_are_fast` -- expected: pass, <8ms budget intact
- `npm run build --prefix frontend` -- expected: success, no errors
- `npm run dev --prefix frontend` then set three functions -- expected: cycled colours, clamp holds, hidden curve retains style on toggle-back
