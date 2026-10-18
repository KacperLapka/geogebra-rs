---
title: "Refactor sweep"
ticket: 7
status: in-review
---

## Implementation Notes

Built 2026-10-18 on `main` over `e5a4e00` (1.6 in-review), commit `d34a0bd`,
never pushed. Cleanup only; no behavior change, no new ABI, no scope push.

- `core_math/simplify.rs` -- collapsed `collapsible_if` lint; removed unused
  `Func` import and the `_func_link` placeholder hack (genuinely unused;
  `Func` is owned by `diff.rs`).
- `core_math/eval.rs` -- factored the test case tuple into a `Case` type
  alias (`type_complexity` lint).
- `render/pipeline.rs` -- renamed `GpuStatus::from_str` to `from_wire`
  (was confusable with `std::str::FromStr::from_str`); callers updated
  (`wasm_bridge::set_gpu_status`, wire round-trip test).
- NaN-safe negated comparisons (`!(a < b)` rejects NaN where `a >= b`
  would not) documented with why-comments and function-level
  `#[allow(clippy::neg_cmp_op_on_partial_ord)]`: `render/curve.rs`
  (`sample_curve_adaptive`), `render/grid.rs` (`tick_step`),
  `render/pipeline.rs` (`update_viewport`), `wasm_bridge` (`update_viewport`,
  `query_hover_core`).
- `cargo fmt` across the workspace (api, core_math, render, wasm_bridge).
- `frontend/` untouched: no console noise, no dead code; `Canvas.toScreen`
  stays as the CPU-fallback drawing path per 1.5 design notes.

## Verification

**Commands:**
- `cargo clippy --all-targets` -- expected: zero warnings (was 15)
- `cargo test` -- expected: api 1, core_math 32, render 28, wasm_bridge 12, all pass natively
- `cargo build` -- expected: workspace builds clean
- `npm run build` (in `frontend/`) -- expected: production bundle builds
- `wasm-pack build crates/wasm_bridge` -- expected: WASM bundle (requires wasm-pack + wasm32 target; unavailable in this container, same documented gap as 1.1-1.6)
- Fallback-contract smoke (node): `setFunction f=a*x^2`, slider `a=2.0`,
  `samplePoints(5)` corner `200.0`, `queryHover(400,300)=(0,0)`,
  `serializeWorkspace` version 1, `gpuStatus cpu-fallback rev 1`
