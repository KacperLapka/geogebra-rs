# Gate review — adversarial (inline; subagents depth-limited)

Verdict: PASS after 1 fix; remaining pairs closed by existing Rules.

## Pairs attempted

1. Epic 3 vs epic 4 both emitting `dash_flag 1.0` → HOLE FOUND, FIXED: epic 3
   pinned to `2.0`-only (both directions now closed).
2. Two stories each creating `curve.wgsl` with same attributes but different
   uniform packing (vec4 vs 4 floats) → MITIGATED: contract freezes semantic
   layout + "first writer creates both files"; exact WGSL type signatures are
   story-level detail for the slicing story, not an epic invariant. Noted for
   Sofia's slice (one story must own first-writer).
3. `render` vs `wasm_bridge` both owning style truth → CLOSED: E4-STYLE-STRUCT
   puts the struct in `render`, Session only keys the map.
4. Svelte formatting tick numbers vs computing tick positions → CLOSED:
   positions from Rust, Svelte does string formatting + overlay positioning
   only.
5. `visible=false` as delete vs skip → CLOSED: skip-submission, entry
   retained.
6. WebGPU-only WGSL (subgroups) passing review → CLOSED: E4-GL-PARITY forbids
   non-GLES3-translatable features; Safari WebGL2 run in the harness enforces
   it.
7. Two-vs-one style-fn shape (`set_function_visibility` vs `toggle_visibility`)
   → MITIGATED: "one of the two shapes, not both"; Sofia picks at slice time.
   Recorded here so the slicer doesn't schedule both.
