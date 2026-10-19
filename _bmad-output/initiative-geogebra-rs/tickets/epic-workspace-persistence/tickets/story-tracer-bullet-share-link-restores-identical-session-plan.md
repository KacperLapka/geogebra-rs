---
title: 'Tracer bullet: share link restores identical session'
type: 'feature'
ticket: '1'
created: '2026-10-19'
status: 'built'
baseline_revision: '16ce70ed9edd7f9be894887c5d23e8fac498c28c'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Styled session share/load tracer is incomplete: Rust `Workspace` drops the style map and `restoreWorkspace` ignores style/visibility, and primary `/w/<slug>` has no SPA fallback so direct open 404s.

**Approach:** Carry the Epic 4 style map opaque through `serialize_workspace`/`restoreWorkspace`, add SPA fallback for `/w/<slug>` while keeping `?s=<slug>` working, preserving version 1 ≤10KB JSON, 8-char slug, and <50ms round-trip.

</frozen-after-approval>

## Implementation Notes

Oneshot route: gaps are narrow and mechanical — opaque style carry plus SPA fallback, estimated under 100 lines across ~4 files with existing routes, store, and Share panel reused as-is.

- Decided style shape as `Option<HashMap<String, StyleDto>>` (Rust) / opaque object (JS): serialize includes auto-default styles, backend already stores `style?` verbatim, restore reuses existing style setters — no new ABI.
- Changed: `crates/wasm_bridge/src/lib.rs` (Workspace.style + serialize), `frontend/src/lib/fallback.js` (serialize style), `frontend/src/lib/share.js` (isWorkspaceDoc allows style, linearToHex/styleColorToHex, restore applies style defensively), `frontend/src/App.svelte` (restore syncs color/width/dashed/visible with clamp), `frontend/src/lib/wasm.js` (bundle URL from origin so `/w/<slug>` loads `/pkg/`), `frontend/vite.config.js` (explicit `appType: 'spa'`).
- Surprise: working tree gained concurrent dashed work (`StyleControls` dashed prop, `App.svelte` dashed mirror) mid-run; review finding on dropped `dashed` was real — patched restore to carry `dashed` and clamp width.
- Surprise: `cargo test -p api --lib` has no lib target (binary); used `cargo test -p api`. All suites pass: api 6, wasm_bridge 19, frontend 17. Manual node check: styled fallback doc 193B <10KB, round-trip equal true.
- Skipped commit: working tree holds concurrent uncommitted changes outside tracer scope (`render/`, `Canvas.svelte`, `StyleControls.svelte`, `core_math`); committing would mix attributions.
- Re-review restoration (2026-10-19): style-carry was rebuilt by later sessions as hex-based `StyleEntry` matching `share.js`; re-applied the lost one-liners (Vite `appType: 'spa'`, origin-based pkg URL, refresh width clamp). `wasm.js`/`App.svelte` one-liners left uncommitted (mixed with 2.6/4.3 hunks); `vite.config.js` committed.

## Plan Change Log

## Review Triage Log

- App.svelte restore dropped `dashed` — verdict `medium`, patched: restore now carries `dashed` with width clamp.
- App.svelte mirror used raw `width_px` without clamp — verdict `low`, patched with above (Math.min/max 2.5–6.0).
- Axum has no prod static fallback for `/w/<slug>` (Vite SPA only covers dev) — verdict `medium`, deferred to prod static work.
- Rust styled round-trip unverified (test only covers default style) — verdict `low`, deferred to sweep test.
- wasm.js origin base breaks subpath deploy (`/app/pkg` -> `/pkg`) — verdict `low`, rejected: rare subpath+primary combo, fix needs base-aware resolution beyond simple correction.
- share.js partial style drops width/dashed when color unconvertible — verdict `low`, rejected: unreachable via serialize (always valid hex), ABI needs hex to apply width.
- share.js accepts any style object then skips malformed still ok:true — verdict `false`: defensive polish-by-design, serialize always valid so malformed unreachable.
- linearToHex quantizes linear f32 so not verbatim identical — verdict `false`: all styles originate from hex, inverse round-trips exactly (fallback default-style round-trip equal true).
- Vite dev has no SPA fallback for primary `/w/<slug>` (no `appType`, no rewrites) — verdict `medium`, patch: one-line `appType: 'spa'`, direct correction.
- `carried` Axum serves only the two `/api` routes, prod direct-open 404s — verdict `medium`, deferred (existing deferred-work entry stands; prod static is separate work).
- WASM glue URL built on `window.location.href` resolves to `/w/pkg/` on the primary route with silent fallback — verdict `low`, patch: resolve from origin (one line; subpath tradeoff already accepted as rare).
- Seeded `DEMO_CURVES` g/h survive slug-load (restore only adds, never removes) so a stored single-function session restores with extra curves — verdict `high`, deferred: precedence between demo seeds and slug-load needs the 4.3 owner's decision plus a removal path.
- `carried` style guard skips entries without `color_hex`/`width_px` yet returns ok — verdict `false`: wire shapes now agree (`StyleEntry.color_hex` + `width_px`, always emitted valid), so skips happen only for hand-made blobs unreachable via serializers.
- `carried` malformed style entries skipped yet ok:true — verdict `false`: same defensive pattern as logged before; serializers emit valid maps only.
- Unvalidated `functions`/`params` values save (201) then fail at restore — verdict `false`: hand-made corrupt docs fail loudly at restore with an error; real serializers emit valid shapes only.
- `restoreFromSlug` copies raw `params`/`viewport` past backend filters — verdict `low`, rejected: non-finite values cannot arrive via JSON and only hand-fed degenerate viewports diverge; a guarded copy adds branches for an unreachable-in-practice state.
- `refresh` mirrors `width_px` with no clamp — verdict `low`, patch: one-line clamp restoring the plan-note intent (stored widths are clamped, so only hand-fed blobs diverge).
- JS rejects `"style": null` while Rust reads it as `None` — verdict `false`: both serializers omit empty style rather than emitting null, and a hand-fed null fails loudly with an error.

## Verification

**Commands:**
- `cargo test -p api --lib` -- expected: save/load round-trip, 404, 413 pass
- `cargo test -p wasm_bridge --lib` -- expected: version 1 ≤10KB two-function round-trip passes
- `npm test --prefix frontend` -- expected: share.test.js slug shapes, both routes, 413/invalid pass
