# Adversarial Review: GeoGebra RS Architecture Spine

**Document Under Review:** `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`  
**Reviewer:** Adversarial Architecture Critic  
**Date:** 2026-10-18  
**Verdict:** **REVISE BEFORE BUILD (4 Critical Seam Divergences, 2 Operational Gaps)**

---

## 1. Executive Summary & Verdict

The GeoGebra RS Architecture Spine sets a strong high-level direction with clear design paradigms (Client-Centric Reactive Compute / Thin Persistence) and commendable boundaries (`core_math` as pure `no_std`/`alloc`).

However, **an adversarial breakdown of one-level-down subsystem boundaries reveals severe contract ambiguities**. Two independent engineers building adjacent crates according to the letter of the current ADs will produce completely incompatible subsystems. Specifically:
1. **The Tessellation Pipeline has contradictory owners and conflicting data flows** (`render` sampling `core_math::Expr` vs `wasm_bridge` pushing sampled vertex points).
2. **Dual-mutation conflict on Viewport state** between Svelte 5 Runes ($state) and the `wgpu` canvas event loop.
3. **AD-5's literal ABI contract breaks R10 (Multi-Curve Display)** by defining single-function primitives (`set_function`).
4. **The Persistence schema is undefined**, causing a fatal contract mismatch between the Svelte frontend and the Axum API.

---

## 2. Adversarial Attacks: Four Incompatible Unit Constructions

### Attack 1: The Tessellation Tug-of-War (`render` vs `wasm_bridge` & `core_math`)

* **Conflict:** Who samples the mathematical curve, and how do vertices reach the GPU?
* **Evidence in Spine:**
  - Architecture Diagram (lines 51–53): `BRIDGE -->|"Parse & Differentiate"| CORE`, `CORE -->|"Sampled Points & Roots"| BRIDGE`, `BRIDGE -->|"Vertex Data & Uniforms"| RENDER`.
  - Invariants Diagram (line 75): `RENDER --> MATH`.
  - Structural Seed (line 170): `crates/render/src/tessellator.rs` — *"Adaptive curve sampling and vertex ribbon expansion"*.
  - Capability Map (line 214): `Adaptive Curve Tessellation (R8)` lives in `crates/render/src/tessellator.rs` governed by `AD-4`.
* **Adversarial Construction:**
  - **Engineer A (`wasm_bridge`):** Follows the primary architecture diagram. Expects `core_math::eval` or `core_math::solver` to output a `Vec<(f64, f64)>` of sampled curve coordinates. Passes raw vertex data (`&[f32]`) to `crates/render`, treating `render` as a dumb GPU pipeline that knows nothing about mathematical expressions.
  - **Engineer B (`render`):** Follows `RENDER --> MATH`, `tessellator.rs`, and AD-4. Implements adaptive recursive subdivision (sampling more densely where second derivatives / curvature are high) inside `render::tessellator`. Expects an API signature like `render.tessellate(&core_math::Expr, &Viewport)`.
* **Consequence:** 
  The crates fail to compile or link. If `render` samples `Expr`, passing `Sampled Points` through `wasm_bridge` is redundant and violates `render`'s interface. If `core_math` or `wasm_bridge` samples points, `tessellator.rs` in `render` cannot perform adaptive tessellation because it doesn't own the evaluation step.

---

### Attack 2: Dual State Ownership & 60 FPS Viewport Event Loop (`frontend` vs `wasm_bridge`)

* **Conflict:** Who owns viewport pan/zoom mutation and canvas event dispatch?
* **Evidence in Spine:**
  - AD-5: *"Svelte 5 Runes (`$state`) are the single source of truth for user inputs (raw formula strings, slider values, active viewports). State changes flow strictly unidirectionally into WASM via discrete exported functions..."*
  - AD-4 & R9: 60 FPS viewport navigation with dynamic grid lines.
  - Structural Seed (lines 179, 192): `wasm_bridge/src/session.rs` (*"wgpu canvas hook"*), `frontend/src/components/CanvasView.svelte` (*"HTML <canvas> mount for wgpu surface"*).
* **Adversarial Construction:**
  - **Engineer A (Svelte 5 UI):** Takes AD-5 literally. Declares `viewport = $state({ xmin, xmax, ymin, ymax })` in Svelte. Attaches `onpointerdown`, `onpointermove`, and `onwheel` to `<canvas>`. On every pixel mouse drag, mutates Svelte state, triggering a reactive `$effect` that calls `wasm_bridge.set_viewport(xmin, xmax, ymin, ymax)`.
  - **Engineer B (`wasm_bridge` / `render`):** Knows that routing 120Hz high-frequency mouse/touch pointer events through the JS event loop, Svelte rune dependency tracking, and wasm-bindgen serialization will drop frames below the 60 FPS budget (`NFR-PERF-02`). In `session.rs`, uses `web-sys` to attach pointer and wheel event listeners directly to the `<canvas>` DOM element in Rust/WASM, updating viewport uniforms and dispatching `requestAnimationFrame` render calls directly inside WASM.
* **Consequence:**
  - **Two competing event listeners** fight on the same canvas element.
  - Svelte Runes become desynchronized: when the user drags the canvas, the wgpu grid pans at 60 FPS in WASM, but Svelte's `$state` does not update, freezing the Svelte DOM/SVG coordinate labels and inspection tooltips in place (violating AD-4's rule that DOM overlays are driven by WASM queries).
  - If Svelte is the sole listener, cross-boundary overhead creates jank.

---

### Attack 3: The Single-Curve ABI Trap Breaking Multi-Curve Display (`frontend` vs `wasm_bridge`)

* **Conflict:** ABI function signatures in AD-5 prohibit multi-curve workspaces (R10).
* **Evidence in Spine:**
  - AD-5: *"State changes flow strictly unidirectionally into WASM via discrete exported functions (`parse_and_validate`, `set_function`, `update_param`, `query_hover`)."*
  - R10: *"Multi-Curve Display: Multi-curve display MUST support distinct configurable color palettes, line styles (solid, dashed), and visibility toggles for derivatives."*
  - PRD Story 5: *"Given a configured workspace with multiple functions..."*
* **Adversarial Construction:**
  - **Engineer A (`wasm_bridge`):** Adheres strictly to AD-5's named exported ABI:
    ```rust
    #[wasm_bindgen]
    pub fn set_function(formula: &str) -> JsValue;
    ```
    Stores a single `Option<CompiledCurve>` in `Session`. Has no parameters for function IDs, curve styling (colors, thickness), or toggling derivative visibility.
  - **Engineer B (Svelte UI):** Implements an expression list store:
    ```typescript
    let functions = $state([
      { id: "f1", expr: "x^3 - 3*x", color: "#3b82f6", showDerivative: true },
      { id: "f2", expr: "cos(x)", color: "#10b981", showDerivative: false }
    ]);
    ```
    Attempts to call `wasm.set_functions(...)` or `wasm.add_function(id, expr, style)`. Neither exists in the AD-5 ABI.
* **Consequence:**
  R10 cannot be built. The ABI in AD-5 actively prevents multi-curve support by hardcoding a 1-function scalar interface. Furthermore, the return shape of `query_hover` is left unspecified (does it return a JSON object or a `Float64Array`?), guaranteeing boundary deserialization failure.

---

### Attack 4: The Persistence Contract Vacuum (`frontend` vs `crates/api`)

* **Conflict:** Unspecified JSON schema and identifier keys for workspace persistence.
* **Evidence in Spine:**
  - AD-6: *"The Axum backend acts strictly as an unauthenticated JSON document persistence store with zero mathematical computation. Payloads are hard-limited to 64 KB, and serialized workspace JSON documents must remain under 10 KB. Endpoints `POST /api/workspaces` and `GET /api/workspaces/:slug` must respond within 50 ms using 8-character URL-safe nanoid slugs."*
  - PRD Story 5 & FR-12: Refer to `:id` and *"returns a short ID"*, while the spine refers to `:slug`.
  - PRD Line 170: *"Returns serialized expression tree & viewport"*, whereas Svelte holds raw strings.
* **Adversarial Construction:**
  - **Engineer A (`frontend`):** Serializes user input strings and Svelte state:
    ```json
    {
      "functions": [{ "raw": "x^2 + a", "color": "#ff0000" }],
      "parameters": { "a": 2.5 },
      "viewport": { "xmin": -10, "xmax": 10, "ymin": -5, "ymax": 5 }
    }
    ```
    Expects response: `{ "id": "k8X1z9_a" }`.
  - **Engineer B (`crates/api`):** Follows PRD line 170 and spine line 116:
    ```rust
    #[derive(Deserialize, Serialize)]
    pub struct WorkspaceDocument {
        pub slug: String,
        pub ast: Vec<serde_json::Value>,
        pub bounds: [f64; 4],
    }
    ```
    Expects response: `{ "slug": "k8X1z9_a" }`.
* **Consequence:**
  Zero interoperability. If the API returns `{ "slug": ... }`, the UI looking for `{ "id": ... }` shows an empty share dialog. If the frontend submits raw formula strings but the backend schema validates AST nodes (or vice versa), all save operations fail.

---

## 3. Good-Spine Checklist Audit

| Checklist Criterion | Status | Finding & Evidence |
| :--- | :---: | :--- |
| **1. Enforceable Rules** | **PARTIAL** | AD-1 (`no_std`) and AD-2 (zero-dep Pratt) are strictly enforceable by Cargo. However, AD-4 leaves the rendering responsibility for tangent lines and critical point markers unassigned (GPU quad ribbon vs SVG overlay). |
| **2. Real Divergences Prevented** | **FAIL** | Major divergences are left wide open: (a) Tessellation data flow, (b) 60 FPS viewport event loop ownership, (c) multi-function ABI signature, and (d) workspace JSON schema. |
| **3. Operational Envelope Covered** | **FAIL** | **Database ambiguity:** Spine states *"PostgreSQL / SQLite via SQLx"* (line 47, 122). SQLite requires a stateful persistent disk volume with a single-writer lock; PostgreSQL is a network service. This leaves deployment architecture unresolved.<br/>**Network Seams:** Missing CORS policy between Cloudflare Pages client and Axum API container. Missing rate limiting on unauthenticated `POST /api/workspaces`. |
| **4. Complete Ticket Coverage (R1–R12)** | **PARTIAL** | R1–R9, R11–R12 are mapped. **R10 is broken by AD-5's singular ABI**. Furthermore, **UC-7 (Vector SVG / Image Export)** from PRD lines 115, 134, and 303 is completely omitted from the spine (neither mapped nor listed in Deferred). |
| **5. Brownfield / Stack Ratification** | **PASS** | Versions pinned correctly (Rust 1.85, wgpu 24.0, Axum 0.8.2, Svelte 5.20.0). |

---

## 4. Prioritized Actionable Recommendations (Autofix Plan for Winston)

To harden the architecture spine into an airtight build substrate, apply the following updates:

### Fix 1: Clarify Tessellation Data Flow (AD-4 & Invariants Diagram)
* Decide firmly: Does `crates/render` perform adaptive sampling of `core_math::ast::Expr`, or does `core_math` provide an adaptive curve sampler yielding `Vec<Point2D>`?
* **Recommendation:** Keep `render` free of AST evaluation logic. Have `core_math::solver` or `core_math::tessellator` adaptively evaluate coordinates $[x, f(x)]$, and pass flat `&[f32]` vertex slices to `render`. Remove the `RENDER --> MATH` dependency edge, or explicitly define `render::tessellator` as an internal polyline-to-quad-ribbon expander.

### Fix 2: Unify Viewport State & Canvas Interaction (AD-5)
* Specify the interaction loop:
  - Canvas pointer events (`wheel`, `pointerdown`, `pointermove`) are handled directly by Svelte 5 or by WASM `session.rs`.
  - If handled in Svelte: Svelte updates `$state(viewport)` and calls `wasm.set_viewport(...)`.
  - If handled in WASM for 60 FPS fluidity: WASM exposes an on-change callback or event batching back to Svelte so DOM/SVG overlays remain synchronized.

### Fix 3: Fix AD-5 Exported ABI for Multi-Curve & Feature Inspection
* Replace the singular `set_function` with a collection-oriented or entity-based ABI:
  ```rust
  pub fn upsert_curve(id: &str, formula: &str, color_rgba: u32, show_derivative: bool) -> JsValue;
  pub fn remove_curve(id: &str);
  pub fn set_viewport(xmin: f64, xmax: f64, ymin: f64, ymax: f64);
  pub fn query_hover(px_x: f64, px_y: f64) -> JsValue; // Serialized HoverResult struct
  ```

### Fix 4: Lock Down Workspace JSON Schema & Endpoint Contract (AD-6 & Conventions)
* Add the explicit JSON document schema to Consistency Conventions:
  ```json
  {
    "version": 1,
    "functions": [{ "id": "f1", "expr": "x^3 - 3*x", "color": "#3b82f6", "derivative": true }],
    "parameters": { "a": 1.0 },
    "viewport": { "xmin": -10.0, "xmax": 10.0, "ymin": -5.0, "ymax": 5.0 }
  }
  ```
* Standardize on `slug` (drop `:id` references): `POST /api/workspaces` returns `{ "slug": "abc12345" }`.

### Fix 5: Resolve Database & Operational Seams (AD-7)
* Choose **PostgreSQL** (or SQLite) decisively rather than `/` slash options. Add CORS rule allowing the edge frontend domain, and mandate Axum rate-limiting middleware (e.g. `tower_governor`).
* Explicitly decide UC-7 (Vector SVG Export): place it under `Deferred` or map it to `crates/core_math`.
