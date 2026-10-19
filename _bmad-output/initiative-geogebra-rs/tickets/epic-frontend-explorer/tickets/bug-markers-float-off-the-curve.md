---
id: 12
type: bug
title: "Markers float off the curve"
parent: epic-frontend-explorer
covers: [R5, R6]
after: [2]
refined: true
hitl: false
risk: high
severity: P2
---

# Markers float off the curve

## Description

Root and extrema markers appear offset from the drawn curve. Markers should sit centered on the curve with tooltips showing the curve coordinate.

## Reproduction

1. Open explorer, type `x^3-3x+1`, hit Analyze on default domain -5 to 5.
2. Observe root and extrema dots over indigo curve.
3. Actual: dots float visibly off the curve at same x.
4. Expected: each dot centered on the curve, tooltip coordinate agrees with curve value.

## Cause Hypothesis

DOM overlay sync likely diverges from canvas geometry: duplicate viewport mapping, button-box centering versus dot centering, missing clamp for out-of-range y, or async viewport refresh showing stale marker positions. Engine marker values are assumed correct.

## Acceptance Criteria

1. **Markers sit on the curve**
   **Given** analyzed `x^3-3x+1` on default domain
   **When** root and extrema markers are shown
   **Then** each visible marker is centered on the curve and its tooltip coordinate agrees with the curve value
2. **Tests cover the condition found and fixed**
   **Given** the test suite
   **When** it runs
   **Then** a test that fails on the defect and passes on the fix covers the reproduction, and one covers each related case the fix touched
3. **Or: no change is needed, with proof**
   **Given** the reproduction
   **When** it is run on the current code
   **Then** the expected behavior already holds, or the report was mistaken, with the evidence recorded in Notes — this supersedes 1 and 2

## References

- parent — _bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/epic-frontend-explorer.md, Requirements R5, R6
- design — _bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md, Components Analysis marker
- experience — _bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md, Component Patterns Analysis marker
- architecture — _bmad-output/initiative-geogebra-rs/architecture-geogebra-rs/architecture-geogebra-rs.md, AD-4 DOM-buttons amendment, AD-5 eval-only

## Notes

- Decision: 2026-10-19 — Placed in epic-frontend-explorer entry 12, not epic-symbolic-calculus, per markers-float-off-curve with engine values assumed correct.
- Decision: 2026-10-19 — Severity P2, risk high: markers mislead analysis but curve remains readable.
- Decision: 2026-10-19 — Repro defaults confirmed: `x^3-3x+1` on -5 to 5 via Analyze in dev server; after [2]; hitl false.
- Open question: Which markers (roots only, extrema and inflection, all) and which browser and viewport show the offset — builder records on repro.
- Extra check (high risk): manual pan and zoom sweep confirms markers stay pinned during 60 FPS moves.
