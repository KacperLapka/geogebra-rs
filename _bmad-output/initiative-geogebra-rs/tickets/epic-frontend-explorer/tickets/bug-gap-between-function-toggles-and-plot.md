---
id: 13
type: bug
title: "Gap between function toggles and plot"
parent: epic-frontend-explorer
covers: [R10]
after: [8]
refined: true
hitl: false
risk: low
severity: P3
---

# Gap between function toggles and plot

## Description

The legend toggle row sits visibly detached above the graph canvas with a large vertical whitespace. The legend should read as a tight caption-size row directly above the canvas per DESIGN, with toggles flipping instantly canvas-only.

## Reproduction

1. Open explorer in desktop Chrome dev (Vite dev server), type `e^(0.5x)`, hit Analyze on default domain -5 to 5.
2. Observe legend toggle row (f/f'/f''/tangent/markers) above the white graph card.
3. Actual: ~24px vertical gap, toggle row feels detached from canvas card.
4. Expected: tight caption row 8-12px above canvas, single DESIGN spacing token, legend still caption-size muted above canvas.

## Cause Hypothesis

Layout stacking likely doubles spacing: code shows workspace grid 16px row-gap between toggle row (grid 1,1) and canvas row (grid 1,2) plus legend 8px top/bottom margin, summing to ~24px. Wrappers place-toggles / place-canvas carry no own spacing and display:contents placement keeps the two rows separate.

## Acceptance Criteria

1. **Toggles sit tight above plot**
   **Given** analyzed `e^(0.5x)` on default domain
   **When** legend toggles and graph canvas are shown
   **Then** vertical gap between toggle row and canvas card is a single DESIGN token (8-12px), legend remains caption-size muted directly above canvas
2. **Tests cover the condition found and fixed**
   **Given** the test suite
   **When** it runs
   **Then** a test that fails on the defect and passes on the fix covers the reproduction, and one covers each related case the fix touched
3. **Or: no change is needed, with proof**
   **Given** the reproduction
   **When** it is run on the current code
   **Then** the expected behavior already holds, or the report was mistaken, with the evidence recorded in Notes — this supersedes 1 and 2

## References

- parent — _bmad-output/initiative-geogebra-rs/tickets/epic-frontend-explorer/epic-frontend-explorer.md, Requirements R10
- design — _bmad-output/initiative-geogebra-rs/ux-function-explorer/DESIGN.md, Components Legend, Curve toggle and Layout & Spacing
- experience — _bmad-output/initiative-geogebra-rs/ux-function-explorer/EXPERIENCE.md, Component Patterns Legend, Curve toggle

## Notes

- Decision: 2026-10-19 — Placed in epic-frontend-explorer entry 13 (next_id 13), after [8] Legend with fixed palette, not epic-rendering-polish, per legend-above-canvas ownership.
- Decision: 2026-10-19 — Severity P3, risk low: cosmetic spacing only, toggles and canvas function remain correct.
- Decision: 2026-10-19 — Repro defaults confirmed: `e^(0.5x)` on -5 to 5 via Analyze in desktop Chrome dev; hitl false; toggles must stay canvas-only without refiring analysis.
- Open question: Exact token — 8px vs 12px vs 16px row only — builder confirms against DESIGN scale 4/8/12/16/24/32 and records measured gap.
- Assumption: Engine marker values and toggle canvas-only behavior are correct; defect is layout-only — builder confirms on repro.
