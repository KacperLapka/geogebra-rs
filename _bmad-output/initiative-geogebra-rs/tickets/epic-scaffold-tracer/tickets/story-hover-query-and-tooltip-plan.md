---
title: "Hover query and tooltip"
ticket: 5
status: done
baseline_revision: 15015e94a68085f0641b498a38742daa96e8c318
route: full
route_source: auto
risk: low
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `query_hover` is a 1.1 tracer stub with a hardcoded pole guard
and no pole/degenerate coverage; the tooltip path needs hardening so the
mousemove tooltip always shows exact Rust-returned math.

**Approach:** Extract a pure, natively tested `query_hover_core`, single-source
the pole guard on `render::curve::POLE_BREAK`, return `math_y: None` near
NaN-breaks, and finish the Svelte tooltip overlay without any Svelte-side
spatial math (AD-4).

## Boundaries & Constraints

**Always:** AD-4 GPU-geometry / Svelte-text split (`query_hover` owns the
pixel→math transform; Svelte displays verbatim); AD-5 named ABI surface;
f64 math; client-side compute only; no `eval()`.

**Never:** Adaptive sampling / grid changes (1.4 owns them); slider/JSON
shape (1.6); pushing to remote.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Hover on curve | mousemove, `f(x)=a*x^2` | HTML tooltip with Rust `(math_x, math_y)` | No error expected |
| Hover near pole | `f=1/x`, pixel at math x=0 | `math_y: None`, tooltip shows `—` | Gap, never a spike value |
| No function set | mousemove, empty session | `math_x` exact, `math_y: None` | No error expected |
| Degenerate input | zero canvas, NaN pixels | `math_x: NaN`, `math_y: None` | Graceful, never panics |
| Calculus marker | (future epics) | `marker: None` until markers land | Tooltip renders marker suffix when present |

</frozen-after-approval>

## Code Map

- `crates/render/src/lib.rs` -- re-exports `POLE_BREAK` (single source)
- `crates/wasm_bridge/src/lib.rs` -- NEW pure `query_hover_core`
  (viewport + canvas + pixel + expr/params in, `HoverResult` out);
  `query_hover` thinned to lock + delegate + JSON; 3 new tests
- `frontend/src/Tooltip.svelte` -- verbatim display of Rust values + `—`
  for null/NaN + marker suffix; no spatial math (AD-4)
- `frontend/src/App.svelte` -- `handleHover` guards non-finite pixels,
  forwards `marker` through hover state

## Tasks & Acceptance

**Execution:**
- [x] `render` re-exports `POLE_BREAK` -- single pole-guard source
- [x] `query_hover_core` pure function + `query_hover` delegation -- AD-4/AD-5
- [x] Pole / no-function / degenerate tests -- R3/R4 hover depth
- [x] Tooltip marker suffix + App hover hygiene -- overlay finish

**Acceptance Criteria:**
- Given the mouse over the curve, when `query_hover` returns, then the
  tooltip shows the exact Rust-returned `(math_x, math_y)`
- Given hover near a NaN-break (pole), when evaluated, then `math_y`
  is `None` and the tooltip shows `—`
- Given any hover, when traced, then no Svelte-side spatial math runs (AD-4)

## Implementation Notes

Built 2026-10-18 on `main` over `15015e9` (1.4 in-review), never pushed.

- `crates/wasm_bridge`: `query_hover` body replaced by
  `query_hover_core(&viewport, w, h, px, py, expr, params)`; pole guard now
  `POLE_BREAK` (was hardcoded `1e6`); degenerate canvas / non-finite pixels
  return `NaN`/`None` instead of dividing by zero.
- New tests: `hover_reports_none_near_pole_break` (`1/x` at center pixel →
  `math_x ≈ 0`, `math_y None`), `hover_without_function_returns_math_x_only`
  (exact `math_x = 5.0`, `math_y None`), `hover_core_rejects_degenerate_input`
  (zero canvas + NaN pixel → `NaN`/`None`).
- `frontend`: `Tooltip.svelte` documents the AD-4 contract and appends
  `· {marker}` when a future epic fills it; `App.svelte handleHover`
  drops non-finite pixels and forwards `marker ?? null`.
- `fallback.js` already mirrors the `1e6` pole guard and the
  `{math_x, math_y, marker: null}` shape; unchanged.

## Plan Change Log

## Review Triage Log

## Design Notes

`Canvas.svelte toScreen` remains: it is the CPU-fallback *drawing* path
(pixels for the 2D line strip), not tooltip math. The tooltip path
(`onHover` → `backend.queryHover` → verbatim display) performs zero
Svelte-side spatial math per AD-4. GPU-ribbon drawing supersedes the 2D
path when the linked bundle acquires a device.

## Verification

**Commands:**
- `cargo test` -- expected: api 1, core_math 32, render 28, wasm_bridge 8, all pass natively
- `cargo build` -- expected: workspace builds clean, zero warnings
- `npm run build` (in `frontend/`) -- expected: production bundle builds
- Manual: `npm run dev`, type `f(x)=a*x^2`, mousemove over curve shows
  `(x, y)` tooltip; type `1/x`, hover at center shows `(0.000, —)`
