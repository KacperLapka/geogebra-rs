---
title: "wgpu pipeline with WebGL2 fallback and viewport loop"
ticket: 2
status: in-review
baseline_revision: 8ae5d3d059e5fe1dd8e4e99a1c75362778c72a63
route: full
route_source: auto
risk: high
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Tracer draws on a 2D fallback with a `CpuFallback` stub; no GPU
device init, no canvas event loop, no measured 60 FPS baseline.

**Approach:** Implement `render/pipeline.rs` device init (`PRIMARY | GL`
with silent WebGL2 retry), the `update_viewport` path with revision counter,
a `FrameStats` 60 FPS ledger, WGSL shaders, bridge status plumbing, and a
rAF viewport loop with FPS meter in the frontend — keeping CPU fallback fully
working on GPU-less hosts.

## Boundaries & Constraints

**Always:** AD-4 GPU-geometry / Svelte-text split, `PRIMARY | GL` request,
silent WebGL2 retry, WGSL only, f64 math, client-side compute only, no `eval()`.

**Never:** Adaptive sampling / quad-ribbon vertex buffers (story 1.4 owns
them); parser completion (1.3); hover/tooltip depth (1.5); slider/JSON shape
(1.6); pushing to remote.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| GPU host init | `init_gpu` with adapter | `WebGpu` (or `WebGl2`), device stored | Device-request failure falls through silently |
| GPU-less host init | `init_gpu` in container | `CpuFallback`, renderer usable | Never panics, never surfaces an error |
| WebGPU unavailable | PRIMARY probe misses | GL probe runs second, status `WebGl2` if found | Silent retry, no user-facing error |
| Drag/zoom | continuous `update_viewport` | Uniform refreshes, revision bumps, loop draws ≤1/frame | Degenerate ranges rejected, old viewport kept |
| Frame budget | 60 FPS ledger | `meets_budget()` true at ≤16 ms mean | Garbage deltas ignored |

</frozen-after-approval>

## Code Map

- `crates/render/Cargo.toml` -- adds `wgpu 30.0.1` (+`webgl` feature), `pollster 1`, dev-dep `naga 30` (`wgsl-in`)
- `crates/render/shaders/{curve,grid}.wgsl` -- NEW viewport-uniform line shaders, naga-validated in tests
- `crates/render/src/pipeline.rs` -- `GpuStatus` + wire names, `requested_backends`/`backend_attempts`/`classify_backend`, `probe_backends`, `ViewportUniform`, `FrameStats` (16 ms budget), `Renderer` (viewport/uniform/revision/device, validated `update_viewport`, `init_gpu` + blocking wrapper)
- `crates/render/src/lib.rs` -- re-exports for the new pipeline surface
- `crates/wasm_bridge/src/lib.rs` -- session gains `gpu` + `viewport_revision`; `update_viewport` bumps revision; NEW tracer helpers `gpu_status` / `set_gpu_status`
- `frontend/src/Canvas.svelte` -- rAF viewport loop (dirty-flag present, ≤1 draw/frame) + FPS meter via `onFps`
- `frontend/src/App.svelte` -- GPU status + FPS + revision banner
- `frontend/src/lib/{wasm,fallback}.js` -- `gpuStatus()` on the backend contract, incl. fallback revision counter

## Tasks & Acceptance

**Execution:**
- [x] `crates/render` deps + WGSL shaders -- device-init substrate
- [x] `pipeline.rs` backend probe order, classification, uniform, frame ledger, renderer -- R4 core
- [x] `wasm_bridge` revision + status path -- loop polling contract
- [x] frontend rAF loop + FPS meter + status banner -- viewport loop UX

**Acceptance Criteria:**
- Given a GPU-less host, when `init_gpu` runs, then status is `CpuFallback` with no panic and the viewport path keeps working
- Given PRIMARY and GL probes, when they run, then PRIMARY is attempted first and GL second (observed in `probe_backends`)
- Given continuous drag/zoom, when viewports stream in, then the loop presents at most one frame each with no blanks
- Given WebGPU unavailable but GL present, when init runs, then status is `WebGl2` (silent retry)

## Implementation Notes

Built 2026-10-18 on `main` over `8ae5d3d` (1.1 in-review), commit `64e021e`,
never pushed.

- `crates/render`: 14 pipeline tests (probe order, classification, wire
  names, uniform NDC corners, 16-byte uniform, vertex layout, WGSL parse,
  FPS/budget math, garbage deltas, viewport path, degenerate rejection,
  resize guard, headless silent-degrade, per-frame CPU cost < 16 ms for
  600 updates). 19/19 suite green.
- `crates/wasm_bridge`: `gpu_status` round-trip + revision test (accepted
  updates bump, rejected do not; unknown backend maps to cpu-fallback). 5/5 green.
- Full `cargo test`: api 1, core_math 21, render 19, wasm_bridge 5 — all pass.
  `cargo build` clean, zero warnings in `render`.
- `frontend`: `npm run build` passes (732 ms); fallback contract smoke-tested
  in node (`cpu-fallback` rev 0 → rev 1 after one accepted viewport).
- Toolchain gap (documented, not blocking): no GPU / `wasm-pack` / wasm32
  target in this container, so live WebGPU acquisition and the linked-bundle
  banner state verify on a GPU dev machine: `wasm-pack build crates/wasm_bridge`
  then load the app and drag — banner should read `webgpu` at ~60 FPS, or
  `webgl2` where WebGPU is unavailable.

## Plan Change Log

## Review Triage Log

## Design Notes

`Backends::GL` covers native GLES and browser WebGL2 through one flag; the
`webgl` cargo feature is enabled so the wasm build can actually acquire the
fallback adapter. Surface attachment (`create_surface` from the canvas) stays
a follow-up for the linked-bundle step — probes run display-less, which is
also what makes them unit-testable without a GPU. Quad-ribbon expansion of
the line strip belongs to story 1.4 with the adaptive sampler.

## Verification

**Commands:**
- `cargo test` -- expected: api 1, core_math 21, render 19, wasm_bridge 5, all pass natively
- `cargo build` -- expected: workspace builds clean
- `npm run build` (in `frontend/`) -- expected: production bundle builds
- `wasm-pack build crates/wasm_bridge` -- expected: WASM bundle (requires wasm-pack + wasm32 target; unavailable in this container, verified by consuming build on GPU dev machine)
