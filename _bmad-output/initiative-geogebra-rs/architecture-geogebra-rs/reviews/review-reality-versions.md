# Architecture Review: Reality-Check & Version Audit

**Target Document:** [`architecture-geogebra-rs.md`](file:///home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md)  
**Reviewer:** Reality-Check & Version Reviewer (Subagent)  
**Date:** 2026-10-18  
**Verdict:** **CONDITIONAL PASS (Requires 2 Critical/High Fixes)**

---

## 1. Executive Summary

A comprehensive reality check and web verification was performed on every committed technology decision and version specified in the GeoGebra RS Architecture Spine. 

All selected libraries, frameworks, and tools are active, production-grade, and well-aligned with the client-centric reactive compute paradigm. However, two critical/high issues were discovered:
1. **`axum = "0.8.2"` is YANKED on crates.io**: Fresh dependency resolution will skip or fail on this exact version. The stack should specify `0.8.9` (or `0.8`).
2. **Axum 0.8 Routing Syntax Change**: Axum 0.8 migrated from `:param` to `{param}` (via `matchit 0.8`). The spine's repeated specification of `GET /api/workspaces/:slug` contradicts Axum 0.8 runtime routing rules and will cause runtime registration panics or routing errors if implemented as written.

---

## 2. Technology & Version Audit Table

| Technology / Crate | Spine Version | Verification Status | Real-World Status / Findings |
| :--- | :--- | :--- | :--- |
| **Rust Toolchain & Edition** | `1.85 (edition 2024)` | **VERIFIED** | Rust 1.85.0 stabilized Edition 2024 on Feb 20, 2025. Host compiler is `rustc 1.98.1`. Fully supported. |
| **axum** | `0.8.2` | **CRITICAL ISSUE** | **Version 0.8.2 is YANKED on crates.io.** Must update to `0.8.9` (latest patch in `0.8` track). |
| **Axum Routing Syntax** | `:slug` syntax | **HIGH COMPATIBILITY RISK** | Axum 0.8 breaking change: route syntax is `{slug}`, not `:slug`. Routes specified as `/api/workspaces/:slug` must be `/api/workspaces/{slug}`. |
| **wgpu** | `24.0.0` | **VERIFIED** | Released January 15, 2025. Fits requirements. Requires `webgl` feature and `wgpu::Limits::downlevel_defaults()` for WebGL2 fallback. |
| **wasm-bindgen** | `0.2.100` | **VERIFIED (With Gotcha)** | Exists on crates.io. Strict 1:1 schema version match required between `wasm-bindgen` dependency and `wasm-bindgen-cli`. |
| **Svelte** | `5.20.0` | **VERIFIED** | Svelte 5 Rune-based release (`$state`, `$derived`, `$props`). Fits frontend reactive requirements cleanly. |
| **Vite** | `6.2.0` | **VERIFIED** | Released February 25, 2025. Compatible with Svelte 5 and `@sveltejs/vite-plugin-svelte`. |
| **SQLx** | `0.8.3` | **VERIFIED** | Released January 4, 2025. Fully compatible with Tokio 1.x and Axum 0.8. |
| **serde** | `1.0.218` | **VERIFIED** | Released February 20, 2025. Standard for Rust serialization. |
| **serde_json** | `1.0.139` | **VERIFIED** | Released February 27, 2025. |
| **nanoid** | `0.4.0` | **VERIFIED** | Released and stable on crates.io for generating URL-safe 8-char IDs. |

---

## 3. Deep-Dive Findings & Reality Checks

### Finding 1 [CRITICAL]: `axum 0.8.2` is yanked on crates.io
- **Location:** Line 145 (Stack table)
- **Detail:** Crate version `0.8.2` was yanked from crates.io shortly after release due to a regression. While existing lockfiles can download yanked crates, new `Cargo.toml` definitions specifying `axum = "0.8.2"` will fail during cargo dependency resolution on clean environments.
- **Remediation:** Update stack version to `0.8.9` (or declare `0.8`).

### Finding 2 [HIGH]: Route parameter syntax mismatch in Axum 0.8 (`:slug` vs `{slug}`)
- **Location:** Lines 46, 60, 116, 129, 185
- **Detail:** In Axum 0.7 and earlier, path parameters used colon syntax: `/api/workspaces/:slug`. In Axum 0.8.0, routing was overhauled with `matchit 0.8`, changing route parameter syntax to curly braces: `/api/workspaces/{slug}`. Using `:slug` in Axum 0.8 results in a compile/runtime panic:
  `Invalid route: path parameters must start with '{' and end with '}'`.
- **Remediation:** Update all mentions of `GET /api/workspaces/:slug` to `GET /api/workspaces/{slug}` across the spine (diagrams, AD-6, Consistency Conventions, and Structural Seed).

### Finding 3 [MEDIUM]: `wgpu` WebGL2 fallback requires explicit downlevel limits & features
- **Location:** Line 42, AD-4 (lines 100-104), and Stack table
- **Detail:** `wgpu` 24.0.0 supports WebGL2 fallback on `wasm32-unknown-unknown`, but WebGL2 has strict hardware limitations:
  1. It requires `wgpu = { version = "24.0.0", features = ["webgl", "wgsl"] }`.
  2. Device request must configure `wgpu::Limits::downlevel_defaults()` or `downlevel_webgl2_defaults()` rather than `Limits::default()`, or context creation fails on mobile browsers and older GPUs.
  3. WebGL2 does not support WebGPU compute shaders. AD-4 correctly specifies quad ribbons via vertex/fragment shaders, which is compatible, but this restriction should be explicitly noted to avoid future developers attempting compute passes on the fallback path.
- **Remediation:** Add a brief implementation note in AD-4 regarding `downlevel_defaults()` and feature flags.

### Finding 4 [MEDIUM]: `wasm-bindgen` schema version synchronization
- **Location:** Line 142 (Stack table)
- **Detail:** `wasm-bindgen 0.2.100` enforces an exact version match with `wasm-bindgen-cli`. A mismatch between local/CI tooling (e.g. CLI 0.2.99 or 0.2.102) and crate dependency causes build failure:
  `rust Wasm file schema version (0.2.100) does not match binary schema version`.
- **Remediation:** Note in the stack or seed instructions that `wasm-bindgen-cli` version should be pinned to `=0.2.100` in CI/build scripts.

### Finding 5 [LOW]: Vite WASM integration plugin
- **Location:** AD-7 and Structural Seed (`frontend/vite.config.ts`)
- **Detail:** Vite 6 handles WASM, but depending on whether `wasm-pack` outputs `web` target or `bundler` target, `vite-plugin-wasm` and `vite-plugin-top-level-await` may be needed in `vite.config.ts` for synchronous-like import semantics in Svelte 5 components.
- **Remediation:** Ensure `frontend/vite.config.ts` mentions or configures appropriate WASM plugin handling.

---

## 4. Concrete Remediation Diff

```diff
--- a/architecture-geogebra-rs.md
+++ b/architecture-geogebra-rs.md
@@ -46,3 +46,3 @@
-        API["crates/api (Axum 0.8)<br/>REST Endpoints: /api/workspaces"]
+        API["crates/api (Axum 0.8.9)<br/>REST Endpoints: /api/workspaces"]
@@ -60,1 +60,1 @@
-    API -->|"GET /api/workspaces/:slug"| UI
+    API -->|"GET /api/workspaces/{slug}"| UI
@@ -104,1 +104,1 @@
-`crates/render` must attempt WebGPU initialization first and automatically fall back to WebGL2 without crashing if WebGPU is unsupported.
+`crates/render` must attempt WebGPU initialization first and automatically fall back to WebGL2 without crashing if WebGPU is unsupported, using `wgpu::Limits::downlevel_defaults()` and the `webgl` feature flag.
@@ -116,1 +116,1 @@
-Endpoints `POST /api/workspaces` and `GET /api/workspaces/:slug` must respond within 50 ms using 8-character URL-safe nanoid slugs.
+Endpoints `POST /api/workspaces` and `GET /api/workspaces/{slug}` must respond within 50 ms using 8-character URL-safe nanoid slugs.
@@ -129,1 +129,1 @@
-| Naming (API routes) | RESTful kebab-case: `POST /api/workspaces`, `GET /api/workspaces/:slug` |
+| Naming (API routes) | RESTful kebab-case: `POST /api/workspaces`, `GET /api/workspaces/{slug}` |
@@ -145,1 +145,1 @@
-| Axum | 0.8.2 |
+| Axum | 0.8.9 |
@@ -185,1 +185,1 @@
-        routes.rs               # POST /api/workspaces, GET /api/workspaces/:slug
+        routes.rs               # POST /api/workspaces, GET /api/workspaces/{slug}
```

---

## 5. Review Conclusion

The architecture spine is mathematically sound and technologically viable. With the two recommended fixes (bumping yanked `axum 0.8.2` to `0.8.9` and updating route path parameters from `:slug` to `{slug}` for Axum 0.8), the spine is ready for build execution.
