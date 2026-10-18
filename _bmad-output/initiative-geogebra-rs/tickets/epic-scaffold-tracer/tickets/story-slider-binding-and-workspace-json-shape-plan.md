---
title: "Slider binding and workspace JSON shape"
ticket: 6
status: in-review
baseline_revision: 2983090f39d1416028f8368dd90f674032fa973c
route: full
route_source: auto
risk: low
---

## Intent

Wire `update_param` HashMap eval path and `Sliders.svelte` for real-time
recalculation (R2); define workspace JSON
`{version:1, functions, params, viewport}` as the owned shape Epic 5 builds
on. After [1, 3], no new WASM ABI (AD-5 intact).

## Code Map

- `crates/wasm_bridge/src/lib.rs` -- `Workspace` field docs + owned-shape
  docs on `serialize_workspace`; NEW `MAX_WORKSPACE_JSON_BYTES = 10*1024`;
  4 NEW tests (sweep, rejection, shape/size, round-trip)
- `frontend/src/Sliders.svelte` -- real-time `oninput` contract + `forward`
  finite-guard + `aria-label`/`title` + `fmt` hardening (R2)
- `frontend/src/App.svelte` -- `handleParam` real-time comment; workspace
  `<details>` drops `(stub` label, documents Epic 5 owned shape + restore
  recipe (`setFunction` + `updateParam` + `updateViewport`)

## Tasks & Acceptance

**Execution:**
- [x] `Workspace`/`serialize_workspace` docs -- owned shape for Epic 5, no new ABI
- [x] `MAX_WORKSPACE_JSON_BYTES` + shape/size/round-trip tests -- R2 + R11 shape
- [x] `Sliders.svelte` finite-guard + a11y + sync hardening -- real-time drag
- [x] `App.svelte` stub-label removal + restore-recipe comment -- Epic 5 handoff

**Acceptance Criteria:**
- Given `f(x)=a*x^2`, when slider `a` sweeps 1/2/0.5/-1, then sampled
  corner `y = a*100` and `eval(2.0) = a*4` with no re-parse
- Given a tracer session, when `serialize_workspace` runs, then JSON has
  `version:1`, `functions/params/viewport` keys and `len < 10KB`
- Given a serialized workspace, when restored via existing ABI only, then
  re-serialize is identical (Epic 5 recipe, AD-5)

## Implementation Notes

Built 2026-10-18 on `main` over `2983090` (1.5 in-review), never pushed.

- `crates/wasm_bridge`: module docs now list `serialize_workspace` as owned
  shape (was "tracer helpers owned by follow-up work"). `Workspace` fields
  documented: `version` forwards as `SaveBody.version` (AD-6);
  `functions` id->formula text; `params` superset (unknown -> NaN, never
  panic); `viewport` session-owned (AD-5). No ABI addition.
- New tests: `slider_sweep_updates_curve_in_real_time` (a sweep, corner +
  direct eval), `slider_rejects_non_finite_and_bad_idents` (NaN/Inf/empty/
  leading-digit/`x` rejected, `a=2.0` retained),
  `workspace_json_shape_version_1_under_10kb` (keys + viewport fields +
  `len < 10240`), `workspace_round_trip_restores_identical_session`
  (serialize -> reset -> set_function/update_param/update_viewport ->
  identical re-serialize).
- `frontend`: `Sliders.svelte` `forward` drops non-finite parses before
  `onParam`; range + number stay in sync via `value={value}`; label uses
  `fmt` (`Number(v).toFixed(2)`). `App.svelte handleParam` is synchronous
  `updateParam` + `refresh` (same-frame resample, <16ms budget).
- `fallback.js` already mirrors the exact shape + `1e6` pole guard;
  unchanged (shape parity confirmed by inspection).

## Verification

**Commands:**
- `cargo test` -- expected: api 1, core_math 32, render 28, wasm_bridge 12, all pass natively
- `cargo build` -- expected: workspace builds clean, zero warnings
- `npm run build` (in `frontend/`) -- expected: production bundle builds
- Manual: `npm run dev`, type `f(x)=a*x^2`, drag slider `a` and curve
  follows in real time; workspace `<details>` shows `version 1` JSON
