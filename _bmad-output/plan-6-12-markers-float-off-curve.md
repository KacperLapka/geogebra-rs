---
title: '6.12 Markers float off the curve'
type: 'bugfix'
ticket: '12'
created: '2026-10-19'
status: 'built'
baseline_revision: '3a2c81c3d794bf8e0bd0ed56d8ebb8f32aed00bf'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/tickets/bug-markers-float-off-the-curve.md'
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/epic-frontend-explorer.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Root and extrema DOM markers float visibly off the indigo curve for `x^3-3x+1` on default domain -5 to 5, misleading analysis though the curve stays readable.

**Approach:** Reproduce via Analyze plus Rust sampler, then fix DOM placement only (same fractions, true dot-centering, skip out-of-range, viewport sync), covered by failing-then-passing tests plus pan/zoom sweep. Curve sampling and hover values stay untouched.

## Boundaries & Constraints

**Always:** Frontend presentation only via `wasm_bridge` ABI (`samplePoints`, `queryHover`, `updateViewport`); engine marker values assumed correct; keep 60 FPS dirty-rAF, 44px hit targets, DESIGN palette, aria labels, tooltips, and Svelte-positions-tooltips-only.

**Never:** No new ABI exports, no solver/sampler math changes, no curve-point changes, no hover-value changes, no style-struct or workspace-schema changes, no canvas-only marker circles, no g/h demo curves, no color picker.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Markers on curve | Analyzed `x^3-3x+1` on -5 to 5, markers shown | Each visible dot centered on curve, tooltip agrees with curve value | No error expected |
| Out-of-range y | Marker y outside current viewport | Skipped from render (hidden), never floats mid-canvas | Silent, no banner |
| Pan/zoom pinned | Drag pan, wheel/buttons zoom, Reset at 60 FPS | Markers stay pinned to curve, domain label updates | Degenerate viewport ignored, last good retained |
| Empty layer | No roots/extrema/inflection | Silent marker layer, workspace usable | No banner, no crash |

</frozen-after-approval>

## Code Map

- `frontend/src/Canvas.svelte:40-51` -- `toScreen` plus `xFrac/yFrac` percentage mapping, `xSpanOk/ySpanOk` guards; reuse as single mapping truth.
- `frontend/src/Canvas.svelte:423-443` -- DOM `button.marker` overlay from `markers`, `left/top %`, `title` tooltip; fix centering/skip/sync here.
- `frontend/src/Canvas.svelte:526-575` -- `.marker-layer/.marker/.dot/.tip` CSS, `translate(-50%,-50%)`, 44px hit-box; verify dot truly centered.
- `frontend/src/Canvas.svelte:80-229` -- Canvas2D `draw()` curves/grid/tangent/hover; curve polyline reference for visual agreement.
- `frontend/src/Canvas.svelte:236-291,341-352` -- `zoomAt/wheel/drag` plus dirty-rAF; ensure DOM re-derives from same `viewport`.
- `frontend/src/App.svelte:69-116` -- `refresh()` pulls `samplePoints(240)` markers/tangent; `262-269 handleViewport` validates then `updateViewport` then `refresh`.
- `frontend/src/lib/wasm.js:24-25` -- `queryHover/samplePoints` JSON wrappers; do not change shape.
- `crates/wasm_bridge/src/lib.rs:168-199,701-738` -- `collect_markers` plus `sample_points` overlay; read-only, engine assumed correct.
- `crates/core_math/src/solver.rs:202,406,442` -- `find_roots/find_extrema/find_inflections`; read-only.

## Tasks & Acceptance

**Execution:**
- [ ] `frontend/src/Canvas.svelte` -- fix overlay placement so marker dots center on curve with out-of-range skip and viewport sync -- duplicate mapping, button-box vs dot centering, missing y-skip, and stale viewport are suspects; keep percentages tied to `toScreen`; check `App.svelte handleViewport → refresh` ordering read-only and touch it only if skew is proven.
- [ ] `frontend/tests/overlays.test.js` + `frontend/tests/explorer.test.js` -- add failing-then-passing geometry tests for centering, tooltip agreement, out-of-range hiding, empty-layer silence, and pan/zoom pinning -- ticket AC2 requires defect plus related-case coverage.
- [ ] Dev repro `x^3-3x+1` Analyze on -5 to 5 -- record which markers/browsers/viewports showed offset, confirm dots centered and tooltips agree, manual pan/zoom sweep (no FPS assertion) -- ticket AC1 plus extra high-risk check.

**Acceptance Criteria:**
- Given analyzed `x^3-3x+1` on default domain, when root and extrema markers are shown, then each visible marker is centered on the curve and its tooltip coordinate agrees with the curve value
- Given the test suite, when it runs, then a test that fails on the defect and passes on the fix covers the reproduction, and one covers each related case the fix touched
- Given the reproduction, when it is run on the current code, then either the expected behavior already holds with evidence recorded in Notes, or the fix above applies -- this supersedes the first two when no change is needed

## Implementation Notes

- Cause: CSS-geometric — `.marker` `padding:16px` + `margin:-16px` shifted every dot ~16px down-right of its `left/top%` point via `translate(-50%,-50%)`; all markers (roots, extrema, inflection) affected on default -5…5. Fixed with an explicit 44px flex-centered box; added `markerOnScreen` `[0,1]` skip for out-of-range.
- Untouched: engine marker values, curve sampling, hover values, `wasm_bridge` ABI, `App.svelte` viewport ordering.
- Repro: geometry + source-pin `node:test` coverage fails pre-fix, passes post-fix. Live `npm run dev` Analyze `x^3-3x+1` plus manual pan/zoom sweep pending human check.

## Plan Change Log

## Review Triage Log

- F1 `low` -- overlays geometry tests mirror `toScreen/xFrac/markerOnScreen` locally so those assertions pass pre- and post-fix; but the `source pins` tests read `Canvas.svelte` and fail on baseline (no `markerOnScreen`, negative margin present), so the suite as a whole keeps failing-then-passing repro coverage. Rejected:low -- repo `node:test` harness is static by convention (no Svelte mount/WASM in unit tests); a mount-based rewrite is complexity beyond a direct correction.
- F2 `low` -- tooltip test asserts `markerTip(m)` equals the same marker's formatted coords, never against sampled curve points, and `CUBIC_MARKERS` are hardcoded; a wrong-y marker would still pass this assertion. Rejected:low -- engine values are assumed correct per frozen boundaries and covered by `core_math`/`wasm_bridge` tests; wiring WASM sampling into `node:test` exceeds a direct correction.
- F3 `low` -- empty-layer test filters `[]` through the local mirror (always `[]`) and the no-banner pin already held pre-fix, so it cannot fail pre-fix. Rejected:low -- it guards regression (negligible user harm) and a failing-then-passing empty-layer test is not required by AC2.
- F4 `low` -- `explorer.test.js` out-of-range pin asserts only `fx >= 0 && fy >= 0`, missing the upper bounds, so an above-viewport leak would still pass that file (caught only by the duplicate `overlays` pin). Routes to patch: trivial assertion extension.
- F5 `medium` -- dev-repro record and live-sweep evidence are absent (`Implementation Notes` empty, diff is code+tests only); implementer reported CSS-geometric cause for all markers but no dev-server pass. Routes to patch for the record part (append notes); the live `npm run dev` sweep stays a human check in presentation.

## Design Notes

Marker truth is fractions, not pixels: `left = (x-xmin)/(xmax-xmin)*100%` with `transform: translate(-50%,-50%)` must place the 12px `.dot` center on the same point `toScreen` strokes. Curve points and hover values are correct and out of scope; fix placement only. Suspects in order: inner-dot offset inside 44px button box, out-of-range y rendered instead of skipped, JS viewport vs Rust viewport skew after pan/zoom. Out-of-range means skip render, not clamped %.

## Verification

**Commands:**
- `cargo test -p core_math solver` -- expected: pass, roots/extrema baseline holds
- `cargo test -p wasm_bridge sample_points` -- expected: pass, markers carried with points
- `npm test` in `frontend/` (`node --test tests/`) -- expected: pass, new geometry tests fail pre-fix pass post-fix
- `npm run dev` Analyze `x^3-3x+1` on -5 to 5 plus manual pan/zoom sweep -- expected: dots centered, tooltips agree, pinned (no FPS assertion)
