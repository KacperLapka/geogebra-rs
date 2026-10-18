---
type: epic
title: "Workspace persistence and sharing"
parent: initiative-geogebra-rs
covers: ["R11", "R12"]
sources: []
companions: ["_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md", "_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/ux-workspace-persistence/DESIGN.md", "_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/ux-workspace-persistence/EXPERIENCE.md"]
after: ["epic-scaffold-tracer"]
risk: low
refined: true
---

# Workspace persistence and sharing

## Description

Implements the full save/share/load cycle: `wasm_bridge` serialises the live workspace state to a JSON document under 10 KB, Svelte POSTs it to Axum, the backend stores it and returns an 8-character URL-safe slug, and the resulting shortlink reconstructs the identical session on any device.

## Outcome

A professor saves their configured demonstration and shares a shortlink; students open it and see the identical functions, viewport, and parameter values — with no accounts, no authentication, and API round-trips under 50 ms (R11, R12, SM-3).

## Requirements

1. R1 (parent R11): `serialize_workspace()` returns a JSON document ≤ 10 KB with `"version": 1`, function expressions, slider parameters, viewport coordinates, and the Epic 4 style map carried opaque; shown by a ≤ 10 KB string covering a styled two-function session.
2. R2 (parent R12): Svelte Share button opens a panel and `POST /api/workspaces` stores the blob and returns `{ "slug": "<8-char URL-safe>" }` in < 50 ms; shown by Saving state then a fully visible `/w/<slug>` with copy control.
3. R3 (parent R12): `GET /api/workspaces/:slug` on the primary `/w/<slug>` view plus the `?s=<slug>` fallback restores the identical functions, slider values, and viewport with no accounts; shown by a second device matching the saved session and resuming live interaction.
4. R4 (parent R11, parent R12): Unknown slug (404), oversize payload (413), and offline states show one status line with the local workspace intact; shown by in-place 404 text, untouched demo on 413, and disabled Share offline.
5. R5 (parent R12): Share and load are keyboard operable with `Esc` close, focus return, `aria-live` status, and load surfaces reusing the identical main layout for projector and tablet read-only; shown by a keyboard-only share-load pass with no login.

## Constraints

- Client compute: 100% of mathematical computation executes client-side in WebAssembly and wgpu with zero server-side math evaluation.
- Payload limit: Axum middleware rejects bodies > 64 KB with `413`, and workspace JSON stays ≤ 10 KB with `"version": 1`.
- Performance budget: API save/load round-trips complete in < 50 ms.
- Architecture decoupling: Axum is an unauthenticated JSON blob store with no accounts, no math evaluation, and no mutation endpoints.

## Done when

- `wasm_bridge` `serialize_workspace()` returns a JSON string ≤ 10 KB covering functions, parameters, and viewport (R11, AD-5, AD-6).
- `POST /api/workspaces` stores the JSON blob and returns `{ "slug": "<8-char>" }` in < 50 ms (R12, SM-3).
- `GET /api/workspaces/:slug` returns the stored blob; Svelte restores the session identically (R12).
- Axum middleware rejects payloads > 64 KB with `413 Payload Too Large` (NFR-SECU-02, AD-6).
- Workspace JSON includes `"version": 1` field (AD-6).
- Share panel is keyboard operable with `Esc` close, focus return, and `aria-live` status, and load views reuse the identical main layout (R5).
- Waits on epic-scaffold-tracer because: workspace JSON schema shape is defined there.

## Boundaries

- Covers: `wasm_bridge` serialisation, `api` Axum routes and `sqlx` store, Svelte Share button.
- Does not cover: link expiry policy, SVG export (both deferred per initiative Notes).
- Can run in parallel with epic-symbolic-calculus, epic-asymptote-guard, and epic-rendering-polish after scaffold.

## References

- parent — `_bmad-output/initiative-geogebra-rs/initiative-geogebra-rs.md`, Requirements R11, R12
- architecture — `_bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md`, AD-5, AD-6
- design — `_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/ux-workspace-persistence/DESIGN.md`, share-layer tokens
- experience — `_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/ux-workspace-persistence/EXPERIENCE.md`, share and load flows

## Notes

- Decision: 2026-10-18 — Adopt `ux-workspace-persistence` DESIGN.md and EXPERIENCE.md final as companions; spine wins on conflict.
- Decision: 2026-10-18 — Tracer is entry 1 full-cycle share-load; least-sure store swap (entry 2) second alongside load wiring (entry 3) over the entry 1 contract; edge states (entry 4) need store+load; panel polish (entry 5) needs schema+load; sweep (entry 6) closes.
- Assumption: Store is `sqlx` SQLite file keeping the `put/get` interface; slug is random 48-bit retry in the 8-char URL-safe alphabet (stories confirm).
- Assumption: Exact microcopy strings, accent hue, panel padding, panel-stays-open copy, manual Share retry, offline disable, tablet same-layout, and `aria-live` polite region follow the UX spines (build carries them).
