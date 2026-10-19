# Gate review — tech currency (inline; subagents depth-limited, verified against repo)

Verdict: PASS — every committed version reality-checked 2026-10-18.

- `wasm-bindgen 0.2.129`: confirmed in `Cargo.lock` (`name = "wasm-bindgen"`,
  `version = "0.2.129"`).
- `wgpu 30.0.1`: NOT in `Cargo.lock` / not a dependency yet — delta states
  this explicitly ("lands story 1.2; not yet a dependency — confirmed"),
  inheriting the pin from the parent spine. No false claim.
- WGSL→WebGL2 (GLES3-subset, no subgroups, f32-only shaders): conservative and
  correct for `wgpu`'s GL backend; f64-CPU / f32-GPU split matches existing
  `Viewport` (f64) + parent convention.
- Rust 2024 edition, Svelte 5: inherited; frontend dir exists (`Canvas.svelte`,
  `Tooltip.svelte`, `Sliders.svelte` present, `StyleControls.svelte` correctly
  marked NEW).
- No new technology introduced by this delta (no starter, no new crate), so no
  further web verification owed.
