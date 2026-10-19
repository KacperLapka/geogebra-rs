---
title: '6.13 Gap between function toggles and plot'
type: 'bugfix'
ticket: '13'
created: '2026-10-19'
status: 'built'
baseline_revision: '92617a2b0efe21df24b28b5e8a8c4561318de04d'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/tickets/bug-gap-between-function-toggles-and-plot.md'
  - '/home/kacper/Development/rs/geogebra-rs-6-explorer/_bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/epic-frontend-explorer.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Legend toggle row sits ~24px above graph canvas, feeling detached, instead of tight caption row directly above canvas.

**Approach:** Fix CSS double-spacing to single DESIGN token 8-12px gap, keep caption muted legend and canvas-only toggles, covered by failing-then-passing test.

</frozen-after-approval>

## Implementation Notes

Oneshot route: layout-only CSS fix in 2 files, under 20 lines plus test. No ABI or engine change.

- Decision: 8px single token (DESIGN '2') — workspace row-gap 16→8 in App.svelte:625, legend margin 8px 0→0 in StyleControls.svelte:41. Total gap 8+0=8px within 8-12 AC. Chose 8 to match mockup key-workspace.html legend bottom 8px.
- Side effect: global row-gap tightens right-rail deriv/point/share gaps 16→8, still valid DESIGN token; accepted as consistent tight rhythm, no functional change.
- Files: frontend/src/App.svelte, frontend/src/StyleControls.svelte, frontend/tests/legend-gap.test.js (new, fails pre-fix 24px, passes post-fix).
- Tests: npm --prefix frontend test 57/57 pass; cargo test pass.

## Follow-up 2026-10-19 — shared-row stretch was the real gap

- Report: gap persisted after row-gap fix. Root cause: shared grid rows coupled
  heights — row 1 forced short legend to tall deriv-card height, leaving
  stretch slack below legend inside row 1. Gap math alone (16+8) was secondary.
- Fix: independent columns. `.graph-col`/`.rail` are real flex columns
  (gap 8px left, 16px right), `.workspace` keeps 2fr/1fr with
  `align-items: start`; removed shared grid-row placements. `point-card`
  moved from graph-col into rail after deriv-card.
- Trade-off (flagged, not hidden): DOM/tab order is now
  toggles → markers → point → share; EXPERIENCE wants
  toggles → point → markers → share. Deriv-card has no focusables.
  Entry 10 keyboard pass should re-verify; change back only via human call.
- Tests rewritten to pin decoupling (no display:contents, no shared grid-row,
  per-column DESIGN-token gaps); 59/59 frontend + cargo + vite build pass.

## Plan Change Log

## Review Triage Log

- vacuous toggle assertion (`|| Visibility` always true) — verdict low, patched: strict `!includes('setFunction(')`; Visibility lacks `(` after prefix so no false positive.
- missing related-case coverage (rail/top) — verdict low, patched: added DESIGN-token rail test and top/bottom margin pins.
- global row-gap tightens rail 16→8 — verdict low, rejected: still valid DESIGN token 8, scoped per-row gap needs restructure/negatives beyond simple correction, rarely hit as bug.
- margin 0 drops top 8px — verdict low, patched: `margin: 8px 0 0` preserves top, bottom 0 keeps 8px total.
- placement strings only, no grid/DOM check — verdict low, patched: assert toggles 1,1 / canvas 1,2 and DOM order.

## Verification

**Commands:**
- `cargo test` -- expected: all pass
- `npm --prefix frontend test` or existing frontend check -- expected: new gap test fails pre-fix, passes post-fix; no toggle behavior regression
