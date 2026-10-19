---
title: 'Function explorer frontend - epic 6'
type: 'feature'
ticket: ''
created: '2026-10-19'
status: 'built'
baseline_revision: '29c37fb3dc496fa5c44cdb8c7239a4ee3f324a5d'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
followup_review_recommended: true
context:
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md'
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md'
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md'
warnings:
  - 'multiple-goals'
deferred:
  - summary: >-
      Tangent/marker toggles do not round-trip through workspace JSON.
    evidence: |-
      handleToggle persists only f/fp/fpp via setFunctionVisibility; serializeWorkspace style map carries only those ids. toggles.tangent/markers are canvas-only props (App.svelte showTangent/showMarkers) never serialized, so restore defaults them on. Verified in App.svelte handleToggle/refresh, share.js restoreWorkspace, fallback.js serializeWorkspace.
      Fix excluded by intent: Boundaries Never forbids changing wasm_bridge ABI shapes and Axum store; schema owned by epic-workspace-persistence.
    location: >-
      frontend/src/App.svelte:268
    severity: medium
  - summary: >-
      Symbolic f'/f'' display strings are derived in JS mirror instead of Rust.
    evidence: |-
      derive.js diffRaw/diffPow/simplify/printBackend installs fp/fpp via setFunction; sampling stays in WASM but differentiation duplicates core_math/diff.rs. wasm_bridge exposes no symbolic endpoint (grep differentiate: none), so R3 readout has no contract. Divergence risk on new functions (e/pi implicit-star handling differs from parser.rs).
      Fix (Rust symbolic endpoint) excluded by intent: Never forbids new ABI exports; owner epic-symbolic-calculus.
    location: >-
      frontend/src/lib/derive.js:227
    severity: medium
---

<intent-contract>

## Intent

**Problem:** The explorer shell exists as a tracer prototype with debounced formula input, demo f/g/h curves, canvas-only markers, and no DESIGN/EXPERIENCE compliance; students cannot run the friendly single-function Analyze-to-share workspace from layout.png keyboard-first at 60 FPS.

**Approach:** Build frontend/ presentation only contract-only via wasm_bridge ABI in tickets.toml order (spike 1 DOM-sync, tracer 2 Analyze loop, entries 3-10 features, 11 sweep), wiring single primary f plus auto f/f'/f''/tangent with DOM marker buttons, fixed DESIGN palette, and share row on the static bundle.

## Boundaries & Constraints

**Always:** 100% client compute in WASM/wgpu, zero server math; Svelte positions tooltips only, never spatial math; AST parser only, no eval; f64 precision; eval-only slider/viewport updates, symbolic only on Analyze; fixed DESIGN palette indigo f solid, green f' dashed, amber tangent dashed, purple f'' dashed, gray asymptotes; 60 FPS under 16ms, sampler/ribbon under 8ms each; workspace JSON under 10KB with version 1, API 64KB cap, 8-char slug; wgpu with WebGL2 fallback, fallback message only on double failure.

**Never:** Do not change core_math, render style structs/pipeline/budgets, wasm_bridge ABI shapes, Axum blob store/slug logic, Vite/wasm-pack pipeline setup; do not add multi-function sets, free color picker, server-side math, or canvas-only markers; do not re-fire symbolic analysis on slider pan/zoom/toggle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Analyze happy path | Type e^(0.5x), Enter/Analyze | Indigo f curve, marker buttons with tooltips, derivative card with CALCULATED | No error expected |
| Char-exact error | Mistyped char | Caret error in under 50ms, Analyze disabled, last good canvas held | Clear within 50ms of fix |
| Pole readout | 1/x at x=0 | Curve break with gray dashed asymptote, value row shows em-dash | Never NaN/Infinity as finite |
| Empty markers | Marker-free view | Silent layer, no banner | No error expected |
| Share fail/offline | POST fail or offline | Inline retry with analysis untouched; offline cached expression with disabled share + note | Retry preserves analysis |
| All layers off | Every toggle off | Canvas hint state, analysis retained | No error expected |
| Double GPU fail | WebGPU and WebGL2 fail | Fallback message only then, rest of workspace usable | Workspace stays usable |

</intent-contract>

## Code Map

- `frontend/src/App.svelte` -- shell + all $state, applyFormula/handleParam/handleX0/handleViewport/handleShare/restoreFromSlug, DEMO_CURVES f/g/h at 42-46 to remove, refresh() at 53-100 via samplePointsFor/samplePoints
- `frontend/src/FormulaBar.svelte` -- bar shell with 120ms debounce to replace with under-50ms parseAndValidate + Analyze/clear/Try chips
- `frontend/src/Sliders.svelte` -- generic param slider with forward-guard + eval-only updateParam-refresh loop to reuse for R2/R4
- `frontend/src/Canvas.svelte` -- 2D canvas dirty-rAF loop, toScreen/xFrac/yFrac overlay positioning, wheel zoom + drag pan, NaN-break, MARKER_COLORS canvas circles to replace with DOM buttons
- `frontend/src/Tooltip.svelte` -- queryHover formatter with toFixed(3)/em-dash to reuse for marker tooltips
- `frontend/src/StyleControls.svelte` -- demo free color-picker to narrow/replace with fixed-palette legend toggles
- `frontend/src/lib/wasm.js` -- wrapRust/getBackend contract (parseAndValidate/setFunction/updateParam/updateViewport/queryHover/samplePoints/gridLines/serializeWorkspace/gpuStatus) to consume as-is
- `frontend/src/lib/share.js` -- SLUG_RE/slugFromPath/saveWorkspace/loadWorkspace/restoreWorkspace ABI-only recipe to reuse, microcopy to align to Copied./retry
- `frontend/src/lib/fallback.js` -- JS dev backend mirroring tracer path + style/grid parity to keep in sync
- `frontend/src/app.css` -- hardcoded colors to replace with DESIGN tokens (page #F7F7FC, primary #4F46E5, curves indigo/green/amber/purple/gray)
- `crates/wasm_bridge/src/lib.rs` -- READ-ONLY contract: parse_and_validate:518, set_function:528, update_param:557, update_viewport:577, query_hover:677, sample_points:701, serialize_workspace:825
- `crates/api/src/routes.rs` + `store.rs` -- READ-ONLY share contract: POST/GET /api/workspaces, 8-char slug, 64KB cap, 201/400/404/413/500 strings

## Tasks & Acceptance

**Execution:**
- [ ] `frontend/src/App.svelte` -- prove DOM-button overlay sync at 60 FPS per spike note then wire tracer Analyze loop with single indigo curve + root/extrema buttons -- riskiest canvas+DOM+query_hover path first
- [ ] `frontend/src/FormulaBar.svelte` + `frontend/src/App.svelte` -- add Analyze button/Enter fire, under-50ms char-exact errors blocking Analyze, clear-to-empty, Try chips fill-and-fire -- R1 validation loop
- [ ] `frontend/src/App.svelte` -- remove g/h demo curves, wire single primary f with param rows only when params exist eval-only -- R2 single-function cut
- [ ] `frontend/src/App.svelte` -- add derivative card with symbolic f' readout, CALCULATED badge on fresh only, always-on toggleable f'' purple dashed -- R3 readout
- [ ] `frontend/src/App.svelte` + `frontend/src/Sliders.svelte` -- add x-domain point slider driving tangent marker with live value/slope-3-decimals/behavior word plus pole em-dash -- R4/R7 readout
- [ ] `frontend/src/Canvas.svelte` + `frontend/src/App.svelte` -- extend tracer mount to full pan-drag/zoom-wheel-buttons-reset with domain label, pole breaks with gray dashed asymptotes, double-failure fallback message -- R7/R8/R9 mount
- [ ] `frontend/src/App.svelte` + `frontend/src/StyleControls.svelte` -- apply fixed DESIGN palette and legend swatches mirroring curves with canvas-only f/f'/f''/tangent/marker toggles -- R10 styling
- [ ] `frontend/src/App.svelte` + `frontend/src/lib/share.js` -- gate share row after first Analyze, align Copy/Copied/retry/offline microcopy, restore expression+sliders+toggles identically -- R11/R12 sharing
- [ ] `frontend/src/App.svelte` + `frontend/src/Canvas.svelte` + `frontend/src/Sliders.svelte` -- full keyboard walkthrough, 44px marker hit padding, aria-live on release only, reduced-motion snap, canvas role=img fallback -- R13 accessibility
- [ ] `frontend/src/app.css` -- adopt DESIGN tokens/typography/radii/spacing across bar/canvas/legend/card/sliders/markers/share -- visual compliance
- [ ] `frontend/tests/` + touched sources -- resolve deferred cleanup with no behavior change, filing pushed-out scope as new stories -- refactor sweep

**Acceptance Criteria:**
- Given a fresh dev server, when a person types e^(0.5x) and hits Analyze, then an indigo solid curve appears with visually distinct root versus extrema buttons showing coordinate tooltips
- Given a mistyped character in the function bar, when the error appears, then its caret span is char-exact within 50ms with Analyze disabled holding the last good canvas
- Given a*x^2+b loaded, when dragging a and b sliders, then the single curve and values reshape live without refiring symbolic analysis, and a parameter-free expression hides the slider row
- Given a fresh Analyze, when the derivative card renders, then it shows the symbolic f' with CALCULATED badge and a toggleable purple dashed f'' line, hiding the badge on stale/error
- Given the x0 point slider, when dragging or pressing arrows, then value rows update live with slope to 3 decimals plus behavior word and tangent line, moving +-0.1 per arrow key at 60 FPS
- Given 1/x at x equals 0, when the point readout renders, then curves break with a gray dashed asymptote and rows show an em-dash never NaN or infinities
- Given pan by drag and zoom by wheel buttons or reset, when the viewport changes, then the domain label updates with analysis intact and reset restores the default domain
- Given legend toggles, when any layer is toggled, then only the canvas re-renders with analysis retained and swatches match indigo green amber purple gray styles
- Given first successful Analyze, when Copy is pressed, then an 8-char shortlink shows inline Copied confirmation and reopening restores expression plus sliders plus toggles identically
- Given keyboard-only operation from input to share copy, when walking the workspace, then focus order holds with slider announcements on release, visible marker focus labels, and no transitions under reduced motion

## Implementation Notes

## Plan Change Log

## Review Triage Log

### 2026-10-19 — Review pass
- verdicts: 13 findings — high 0, medium 8, low 2, false 3, maybe-false 0
- findings:
  - `[medium]` `[patch]` handleClear leaves share/derivative on empty input — fix applied: reset committed/hasAnalyzed/displays/share on clear to cold state
  - `[low]` `[patch]` committed stores original while restore stores normalized — fix applied: store normalized in committed/draft on Analyze
  - `[medium]` `[defer]` tangent/marker toggles never serialize — deferred to epic-workspace-persistence schema; intent forbids ABI change
  - `[medium]` `[patch]` all-off hint ignores tangent/markers and overlaps cold — fix applied: hasAnalyzed prop, allOff includes showTangent/showMarkers, exclusive hints
  - `[medium]` `[patch]` marker buttons have no onclick pin — fix applied: onclick pins x0 via onX0/handleX0 eval-only path
  - `[medium]` `[patch]` tab order toggles-markers-point vs required toggles-point-markers-share — fix applied: point-analysis moved into graph-col with display:contents preserving layout
  - `[medium]` `[patch]` hover tooltip live every mousemove — fix applied: removed role=status, aria-hidden hover, kept release-only sr-live
  - `[false]` `[reject]` fallback default blue violates fixed palette — refutation: Analyze installs DESIGN via setFunctionStyle; fallback defaults are render parity enforced by fallback-style tests
  - `[false]` `[reject]` fallback empty markers/asymptotes breaks acceptance — refutation: Rust bundle provides overlays; fallback empty by design documented/tested; acceptance surface is deployed bundle
  - `[medium]` `[defer]` JS symbolic mirror bypasses Rust ownership — deferred to epic-symbolic-calculus endpoint; intent forbids new ABI exports; sampling stays WASM with parity tests
  - `[medium]` `[patch]` share first Copy shows Ready not Copied — fix applied: auto-copy on create with inline Copied confirmation
  - `[low]` `[patch]` backend span indexes normalized not draft — fix applied: omit span when normalized differs from input
  - `[false]` `[reject]` no keyboard pan equivalent — refutation: R9 specifies pan by drag; zoom/reset are keyboard accessible; banned list covers hover-only/reorder not pan

## Design Notes

DOM-overlay sync reuses Canvas toScreen/xFrac positioning with dirty-flag rAF; markers as absolutely positioned buttons updated in the same frame as curves, never layout-thrash. Eval-only loop is updateParam then refresh with samplePointsFor, never setFunction. Fixed palette wins over fallback blue #2563eb.
Example marker sync:
```svelte
{#each markers as m}
<button class="marker {m.kind}" style="left:{x(m.x)}px;top:{y(m.y)}px" aria-label="{m.kind} at x={m.x}">{m.x},{m.y}</button>
{/each}
```

## Verification

**Commands:**
- `cargo test` -- expected: all crates pass including wasm_bridge contract tests
- `npm test --prefix frontend` -- expected: share/overlays/fallback-style/mount tests pass
- `cargo build` -- expected: workspace builds clean
- `npm run build --prefix frontend` -- expected: static bundle builds with base './'

## Auto Run Result

Summary: Built epic 6 Function explorer frontend as frontend-only Svelte 5 shell contract-only via wasm_bridge ABI: under-50ms Analyze loop with Try chips, single primary f plus auto fp/fpp, DOM marker buttons synced via toScreen, fixed DESIGN palette with canvas-only toggles, point slider with pole em-dash, full mount with pan/zoom/domain label/double-failure fallback, gated share row with restore, keyboard/reduced-motion pass, DESIGN tokens. Reviewed with quick lens, 8 patches applied, 2 deferred.

Files changed:
- `frontend/src/App.svelte` -- single-f shell, Analyze/normalized store, derivative/point/share cards, toggles, keyboard order
- `frontend/src/Canvas.svelte` -- DOM marker buttons with click-pin, zoom/reset/domain label, exclusive empty/all-off hints, role=img fallback
- `frontend/src/FormulaBar.svelte` -- sync validation, Analyze/Enter, clear, Try chips
- `frontend/src/Sliders.svelte` -- param rows hidden when free, eval-only with release commit
- `frontend/src/StyleControls.svelte` -- fixed-palette legend, no color picker
- `frontend/src/Tooltip.svelte` -- 3-decimal em-dash formatter, hover aria-hidden
- `frontend/src/app.css` -- DESIGN tokens, 1120px max, reduced-motion
- `frontend/src/lib/derive.js` -- NEW: e^/implicit-star normalize, JS symbolic mirror, presets, format3/behavior
- `frontend/src/lib/fallback.js` -- e^ normalize, numeric tangent with pole null, silent markers
- `frontend/tests/share.test.js` -- share-row/Copied/offline/retry microcopy asserts
- `frontend/tests/derive.test.js` -- NEW: 6 normalize/validate/derive/pole tests
- `frontend/tests/explorer.test.js` -- NEW: 10 shell compliance tests incl. matrix all-off/empty-silent

Review breakdown: 13 findings — patches applied 8 (medium 6, low 2), deferred 2, rejected false 3.
- Patched: handleClear cold reset; normalized store; all-off hint; marker click-pin; tab order; hover live removal; share auto-Copied; backend span omit.
- Deferred: tangent/marker toggle persistence (schema owned by epic 5, intent forbids ABI change); JS symbolic mirror (missing Rust endpoint owner epic 2, intent forbids new exports).
- Rejected false: fallback blue (DESIGN applied on Analyze; defaults are render parity); fallback empty overlays (Rust path provides data; fallback dev-only by design); keyboard pan (R9 pan-by-drag specified; zoom/reset keyboard accessible).

Follow-up review recommended: true. Patched counts by verdict: high 0, medium 6, low 2. Unverified risk: point-analysis moved into graph-col with display:contents to fix tab order — layout.png visual parity unverified without browser screenshot; 60 FPS DOM-sync relies on dirty-rAF meter, not lab-measured.

Verification: cargo test 13+50+52+36 pass; npm test 48 pass; cargo build clean; npm run build 125 modules built.

Residual risks: e^(0.5x) marker-free silent correct per matrix but AC wording implies buttons (quadratic/cubic show buttons in Rust; fallback markers empty by design); tangent/marker toggles default on after restore (schema fixed); derivative strings equivalent not mockup-identical; e^(0.5x missing-paren span points at paren not end (char-exact, blocks correctly).
