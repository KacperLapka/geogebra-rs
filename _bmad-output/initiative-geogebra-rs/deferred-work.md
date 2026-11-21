- source_plan: `_bmad-output/initiative-geogebra-rs/plan-plot-engine-readability.md`
  summary: Make `updateGraph`/`updateGraphWithRange` return `Err` instead of trapping when `math_engine` cannot parse an expression.
  evidence: Browser probe against the built WASM: `updateGraphWithRange("x^2", -10, 10)` succeeds, `updateGraphWithRange("not a function", -10, 10)` throws `unreachable`, and the next `updateGraph("x^2")` also throws `unreachable`. Root cause is `math_engine::compute_given_function` using `Builder::compile().expect(...)`; fixing it requires a `math_engine` change or pre-validation in `plot_engine`.

## Deferred from: code review of plan-plot-engine-readability (2026-11-21)

- Frontend/ADR drift: `frontend/src/routes/+page.svelte` imports a stale `#lib/pkg/wasm.js` path and calls the removed free `updateGraph(Float32Array)`. The plot engine now exposes the `PlotEngine` instance (`new PlotEngine('canvas')`, then `updateGraph(fn)` / `addOffset` / `zoom`), and `run_web` is gone. `ADR.md` still documents removed types and contracts. Fix in a dedicated frontend/docs change.
- Non-empty unparseable expressions still trap the WASM module (see the entry above). Deferred by the review because scope is limited to `plot_engine`; fixing it requires a `math_engine` change or a new pre-validation dependency.
- `add_offset` and `zoom` are not atomic: they mutate the active `Mathematician` before validation and can leave the range advanced when computation fails. Deferred because an atomic fix needs cloning or a `math_engine` API change.
