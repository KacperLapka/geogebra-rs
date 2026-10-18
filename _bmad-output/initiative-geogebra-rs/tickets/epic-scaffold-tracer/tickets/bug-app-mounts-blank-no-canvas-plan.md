---
title: "App mounts blank, no canvas"
ticket: 8
status: done
---

## Implementation Notes

Built 2026-10-18 on `main` over `dd1bc42` (1.7 in-review), never pushed.
One-line root cause in `frontend/src/main.js`: the entry used the Svelte 4
constructor API (`new App({ target })`) while the installed `svelte@5.57.2`
exposes the function-based mount surface (`mount(App, { target })` from
`'svelte'`). Construction threw before first render, leaving `#app` empty.

- `frontend/src/main.js` -- replaced `new App({ target })` with
  `mount(App, { target: document.getElementById('app') })` imported from
  `'svelte'`; default export of the mounted instance kept for HMR/tests.
  No `compatibility.componentApi` flag; the real API is used.
- `frontend/tests/mount-entry.test.js` -- new `node --test` regression suite
  (7 tests, no new deps): repro asserts `mount(App, ...)` and rejects
  `new App(...)`; related cases cover the `mount` import source, the `#app`
  target, the default export, the bare `#app` shell in `index.html` (no
  static canvas), the tracer wiring in `App.svelte`/`Canvas.svelte`, and the
  absence of a `componentApi` compat workaround.
- `frontend/package.json` -- added `"test": "node --test tests/"`.
- `frontend/`, `index.html`, `App/Canvas/FormulaBar/Tooltip/Sliders.svelte`,
  `lib/wasm.js`, `lib/fallback.js`, `vite.config.js` otherwise untouched;
  canvas still renders via `Canvas.svelte` by design.

## Verification

**Repro (pre-fix, chromium headless against `vite --port 5173`):**

- `npm run build` passed (runtime-only failure confirmed).
- `--dump-dom` showed `<div id="app"></div>` empty, no header/canvas/sliders.
- Console error verbatim:

```text
Uncaught Svelte error: component_api_invalid_new
Attempted to instantiate src/App.svelte with `new App`, which is no longer valid in Svelte 5. If this component is not under your control, set the `compatibility.componentApi` compiler option to `4` to keep it working.
https://svelte.dev/e/component_api_invalid_new
```

**Post-fix:**

- `npm test` (in `frontend/`) -- 7 pass, 0 fail. Pre-fix run: 5 pass, 2 fail
  (the two mount-API assertions), confirming red-to-green.
- `npm run build` -- production bundle builds (116 modules, no warnings).
- `cargo test` -- api 1, core_math 32, render 28, wasm_bridge 12, all pass.
- Chromium headless `--virtual-time-budget=4000` against dev server: zero
  `component_api_invalid_new` / Uncaught errors (only `[vite] connecting` /
  `connected`); `#app` renders `<h1>geogebra-rs — tracer bullet</h1>`,
  `banner rust` (wasm bundle linked), formula bar, `<canvas 731x420>`,
  slider `a = 1.00`, workspace JSON version 1 with `f: a*x^2`.
- Tracer path intact: `App.svelte` wiring (formula/hover/slider/viewport) and
  `Canvas.svelte` draw loop untouched; hover tooltip component still mounted
  via `{@render children}`.

**HITL for PO close:** open dev server URL, confirm header, backend banner,
formula bar, 800x420 canvas with live curve, tooltip on hover, sliders, with
no console mount error (AC1 `hitl: true`).
