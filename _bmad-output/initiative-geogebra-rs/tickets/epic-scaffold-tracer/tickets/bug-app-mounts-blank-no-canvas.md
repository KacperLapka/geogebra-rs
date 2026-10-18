---
id: 8
type: bug
title: "App mounts blank, no canvas"
parent: epic-scaffold-tracer
covers: [R3, R4]
after: [7]
refined: true
hitl: true
risk: high
severity: P1
---

# App mounts blank, no canvas

## Description

Opening the dev server shows a blank page with no formula bar, canvas curve, or sliders. The Svelte app never mounts into `#app`, so the tracer demo cannot run. The canvas element itself is rendered by Svelte (`Canvas.svelte`), not present in `index.html` by design.

## Reproduction

1. In `frontend/`, run `npm run dev`.
2. Open `http://localhost:5173`.
3. Actual: blank page under `#app`, no formula bar, no canvas, no sliders; console shows a mount/constructor error.
4. Expected: header, backend banner, formula bar, 800x420 canvas with live curve, tooltip on hover, sliders.
5. Environment: `svelte@5.57.2`, `@sveltejs/vite-plugin-svelte@5.1.1`, node v26.10.0. `npm run build` passes; failure is runtime-only.

## Cause Hypothesis

The defect likely lives in the app entry mount path: the entry uses a component-construction call while the installed Svelte major version exposes a function-based mount surface, so construction throws before first render.

## Acceptance Criteria

1. **App mounts with canvas and tracer controls**
   **Given** the dev server running with the Rust bundle linked or fallback active
   **When** a person opens the app
   **Then** the header, backend banner, formula bar, canvas with live curve, hover tooltip, and sliders all render with no console mount error
2. **Tests cover the condition found and fixed**
   **Given** the test suite
   **When** it runs
   **Then** a test that fails on the defect and passes on the fix covers the reproduction, and one covers each related case the fix touched
3. **Or: no change is needed, with proof**
   **Given** the reproduction
   **When** it is run on the current code
   **Then** the expected behavior already holds, or the report was mistaken, with the evidence recorded in Notes — this supersedes 1 and 2

## References

- parent — _bmad-output/initiative-geogebra-rs/tickets/epic-scaffold-tracer/epic-scaffold-tracer.md
- svelte version — frontend/package.json, devDependencies

## Notes

- Decision: 2026-10-18 — User confirmed blank app on dev server and approved filing as bug against 1.1 with immediate builder handoff.
- Open question: console error verbatim from the user's browser (builder to record on repro).
