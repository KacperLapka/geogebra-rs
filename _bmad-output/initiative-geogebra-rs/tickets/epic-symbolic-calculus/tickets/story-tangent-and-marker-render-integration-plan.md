---
title: 'Tangent and marker render integration (2.6)'
type: 'feature'
ticket: '6'
created: '2026-10-19'
status: 'in-progress'
baseline_revision: '28799b01692d06f08cc0f8857fcd5003867c3429'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - /home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Ticket 2.6 needs the calculus engine on screen — tangent-line evaluation at a draggable `x0` plus root/extrema/inflection marker circles drawn as overlays and hoverable via tooltip, all through the existing `query_hover`/sampler surface with no AD-5 change (R2–R4).

**Approach:** Compute markers and the tangent segment in Rust on the existing `sample_points`/`query_hover` path (additive JSON fields only, no new exports), and draw them in the live 2D canvas with a dedicated `x0` slider, proven by `cargo test` plus a manual `f(x)=x^2` slider-drag check.

## Boundaries & Constraints

**Always:** No new `#[wasm_bindgen]` exports (AD-5); Svelte performs no math — it strokes Rust-computed coordinates verbatim (AD-4); markers viewport-only with pole-jump exclusion; 44-byte vertex/shader contract and `dash_flag=2.0` reservation untouched.

**Never:** No GPU shader changes (overlays ride the live 2D canvas path); no changes to `core_math` solver/diff/simplify behavior; no `Sliders.svelte` generic-flow changes; no workspace-schema changes (`x0` travels as an ordinary session param).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Hover near root | `f=x^2-1`, hover at math `x≈1` | `marker` is `Some` naming the root with coords | No error expected |
| Hover far from markers | `f=x^2-1`, hover mid-gap | `marker` stays `None`, coords still returned | No error expected |
| Markers in samples | `f=x^3-3x` viewport `[-3,3]` | `markers` holds max/min/inflection with `1e-12` coords | No error expected |
| Tangent in samples | `f=x^2`, `x0=1` | `tangent` segment lies on `y=1+2(x-1)` | No error expected |
| Missing/non-finite x0 | no `x0` param yet | `tangent` is `null`, curve + markers unaffected | No error expected |
| Pole form | `f=1/x` | `markers` empty, tangent still computed at finite `x0` | No error expected |
| Dev fallback (no bundle) | `npm run dev` without `pkg/` | curve only (`markers: []`, `tangent: null`), no crash | No error expected |

</frozen-after-approval>

## Code Map

- `crates/wasm_bridge/src/lib.rs:416-464` -- `query_hover_core` (pure, tested) + `query_hover` export: populate `marker` with nearest-marker id within ~12px tolerance, snapping coords to the marker; `HoverResult.marker` field already exists (`:107-112`).
- `crates/wasm_bridge/src/lib.rs:471-485` -- `sample_points` export: add additive `markers` array (`{kind,x,y}`, kinds `root|min|max|inflection`) over the session viewport via `find_roots`/`find_extrema`/`find_inflections` (~200 scan steps to protect the frame budget) plus `tangent` object from the render helper; existing `points`/`params` fields byte-identical.
- `crates/render/src/curve.rs:267-282` -- sampler-path owner: add `tangent_segment(expr, params, x0, viewport)` returning viewport-spanning segment endpoints or `None` on non-finite `x0`/`y0`/slope (uses `core_math::tangent_coeffs`); `render` already depends on `core_math`.
- `frontend/src/lib/wasm.js:24-26` -- no change needed (generic JSON parse passes new fields through); `fallback.js:243-261` needs `markers: []`, `tangent: null` shape parity.
- `frontend/src/App.svelte:47-82` -- owns `refresh()` + slider flow: add `x0` state (default `1.0`), dedicated slider UI, `updateParam("x0")` + resample on drag (same-frame path as `handleParam`); filter `x0` out of the generic slider params so it never renders twice.
- `frontend/src/Canvas.svelte:65-103` -- live 2D path: stroke the tangent segment and draw marker circles from new props, included in the dirty-flag effect.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/render/src/curve.rs` -- add `tangent_segment` + unit tests (on-line segment, `None` on bad `x0`/degenerate viewport) -- Rust owns all overlay math.
- [ ] `crates/wasm_bridge/src/lib.rs` -- populate `query_hover` marker ids + `sample_points` markers/tangent fields + unit tests for every cargo-provable matrix row -- additive JSON only, no new exports.
- [ ] `frontend/src/lib/fallback.js` -- mirror the extended `sample_points` shape with empty overlays -- dev loop never crashes without the bundle.
- [ ] `frontend/src/App.svelte` -- `x0` state, dedicated slider, refresh wiring with `x0` filtered from generic params -- one slider, same-frame resample.
- [ ] `frontend/src/Canvas.svelte` -- draw tangent segment + marker circles, dirty on new props -- overlays follow pan/zoom/hover.
- [ ] Manual check -- `npm run dev` against the linked Rust bundle: `f(x)=x^2`, drag `x0`, confirm tangent tracks at 60 FPS with min marker; `f=x^2-1` shows root markers -- proves the ticket verify end to end.

**Acceptance Criteria:**
- Given `f=x^2-1` with canvas mapped, when `query_hover` lands within tolerance of `x=1`, then `marker` names kind `root` with coords within `1e-12`.
- Given `f=x^3-3x` on `[-3,3]`, when `sample_points` runs, then `markers` contains max/min/inflection entries with `x`/`y` within `1e-12` of theory.
- Given `f=x^2` with `x0=1`, when `sample_points` runs, then `tangent` endpoints satisfy `y=1+2(x-1)` within `1e-12`.
- Given `f=1/x`, when `sample_points` runs, then `markers` is empty while `tangent` is still present for finite `x0`.
- Given the dev fallback backend, when `samplePoints` runs, then it returns `markers: []` and `tangent: null` alongside working points.

## Implementation Notes

- Implemented via `bmad-agent-dev` from this plan as sole source of truth (full route).
- Reported: `TangentSegment`/`tangent_segment` in `render` (+2 tests, exported); `MarkerDto`/`TangentDto`/`collect_markers`/`tangent_for`, `query_hover` marker snap (12px), `sample_points` additive `markers`/`tangent` in `wasm_bridge` (+6 tests); fallback shape parity; App `x0` state + dedicated slider with `x0` filtered from generic params; Canvas tangent stroke + per-kind marker circles.
- Verified by implementer and re-verified here: `cargo test -p core_math -p render -p wasm_bridge` 122/122 pass; scoped `cargo clippy -p render -p wasm_bridge --all-targets -- -D warnings` clean (workspace-wide clippy still fails on pre-existing lints in untouched `core_math/src/diff.rs` and `api`); `npm test` 33/33 pass including new `frontend/tests/overlays.test.js` (fallback shape audit gap, added here).
- HALTED before review/commit per human direction: worktree interleaves this ticket's edits with a concurrent session's uncommitted 4.3/5.x work in the same files (`curve.rs` palette, `wasm_bridge` style-order/`sample_points_for`/style map, multi-curve `App`/`Canvas`, plus `StyleControls`/`share`/`wasm`/`vite.config`/`deferred-work.md` edits outside this ticket). No commit made; step-04 review skipped (polluted diff would file out-of-scope noise). Outstanding: step-04 review + commit after tree separation, and the manual browser check (`wasm-pack` bundle + `npm run dev`, `f(x)=x^2` x0-drag at 60 FPS).

## Plan Change Log

## Review Triage Log

## Design Notes

Marker ids are display strings (`root|min|max|inflection` + coords at 3 decimals), shown verbatim in the existing tooltip suffix. The `x0` slider bounds track the live viewport (`min=x_min`, `max=x_max`); no clamping is needed since out-of-viewport tangents still evaluate and canvas strokes clip automatically.

```json
{"markers": [{"kind": "min", "x": 1.0, "y": -2.0}],
 "tangent": {"x0": 1.0, "y0": 1.0, "slope": 2.0, "from": [-10.0, -19.0], "to": [10.0, 21.0]}}
```

## Verification

**Commands:**
- `cargo test -p core_math -p render -p wasm_bridge` -- expected: all pass, including new tangent/marker wiring tests
- `cargo clippy --all-targets -- -D warnings` -- expected: clean

**Manual checks (if no CLI):**
- Link the Rust bundle (`wasm-pack build` + sync), `npm run dev`, set `f(x)=x^2`, drag the `x0` slider: tangent follows at 60 FPS with the min marker drawn from Rust; `f=x^2-1` shows hoverable root markers.
