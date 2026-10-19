# Spine Pair Review — Function explorer

## Overall verdict
The pair is close to a clean downstream contract: tokens resolve, components are double-specified, and `reconcile-layout.md` maps all 21 layout.png ideas with spine-wins stated. Do not build from it yet — three load-bearing deviations from sources (single-function scope, canvas-vs-wgpu stack, DOM-vs-canvas markers) are uncommitted, and Stories 3/4 have no protagonist flow.

## 1. Flow coverage — thin
Checked: Initiative R1–R12, PRD Stories 1–5, tracer (`f(x)=a·x²+b` + slider + critical points + tangent + shortlink) against EXPERIENCE.md Key Flows. Both flows have named protagonist, numbered steps, climax beat, failure path.
### Findings
- **[high]** No dedicated protagonist flow for Story 3 (roots/extrema/inflection discovery, R5–R6) — markers appear only as a set-dressing dot in Flow 2 step 2, never as a climax (EXPERIENCE.md §§ Key Flows). *Fix:* add Flow 3 (e.g. Alex verifies hand-computed extrema/roots against markers + hover coordinates) or record a decision that markers are never a primary journey.
- **[high]** No dedicated protagonist flow for Story 4 / R7 (asymptote encounter: pole break, dashed asymptote, no bridged curve) — State Patterns covers it statically but no flow stages a student meeting `1/x` or `(x²−1)/(x−2)` (EXPERIENCE.md §§ Key Flows, State Patterns). *Fix:* add Flow 4 (lecturer demonstrates pole; climax = broken curve + asymptote line, values show "—" at pole) or scope-cut with decision.
- **[medium]** R9 pan/zoom (Story 1 core: "drags to pan / scrolls to zoom", grid + labels update) is specified in Interaction Primitives but staged in no flow's steps (EXPERIENCE.md §§ Key Flows vs § Interaction Primitives). *Fix:* extend Flow 1 with 1–2 pan/zoom steps and domain-label update, or note it as exercised implicitly.
- **[low]** R8 adaptive sampling / 8 ms budget and NFR-PERF-02 have no observable flow beat (correctly — sub-perceptual), but no flow asserts the negative (no jank during 60 FPS drag) (EXPERIENCE.md § Key Flows). *Fix:* no change needed; note as covered-by-architecture in Foundation.

## 2. Token completeness — strong
Checked: all 19 `colors`, 9 `typography` roles, 5 `rounded`, 8 `spacing`, 14 `components` frontmatter entries in DESIGN.md; every `{path.to.token}` ref in both spines resolved against frontmatter and `design-md-spec.md` type rules.
### Findings
- **[medium]** No numeric contrast ratios for load-bearing combos — only an assertion in EXPERIENCE.md § Accessibility Floor ("indigo on white, white on indigo, and curve colors verified to AA against card white at render"). A downstream implementer cannot verify without numbers (DESIGN.md §§ Colors, EXPERIENCE.md § Accessibility Floor). *Fix:* state pairs + ratios (e.g. ink on surface, primary-ink-on on primary, curve-f/fprime/tangent on surface) or cite measured values.
- **[low]** `value-row.value: '{typography.mono-value.fontSize}'` stores a font-size path in a value slot while its sibling `label` stores a color — resolves but is a type-smell; consumers expecting a color get `13px` (DESIGN.md § frontmatter `components`). *Fix:* rename key to `value-font` or point it at a color token.

## 3. Component coverage — strong
Checked: 14/14 components present with real rules in both DESIGN.md § Components and EXPERIENCE.md § Component Patterns: Function bar, Analyze button, Try chip, Graph canvas, Legend, Derivative card, Point slider, Value row, Badge calculated, Param slider, Analysis marker, Error text, Curve toggle, Share row.
### Findings
- **[low]** Hover crosshair + coordinate tooltip (EXPERIENCE.md § Interaction Primitives "Hover query") and the x-readout pill (DESIGN.md § Shapes) have no component row; both are currently subsumed under Graph canvas / Point slider / Analysis marker rules. *Fix:* keep subsumed, but name the tooltip owner explicitly in the Analysis marker behavioral rule (already implied — one sentence).

## 4. State coverage — strong
Checked: all 8 IA surfaces walked against EXPERIENCE.md § State Patterns (cold load, empty input, analyzing, parse error, pole/asymptote, no-markers, share fail, offline, WebGL2/canvas fallback). Permission-denied correctly absent (auth out of scope per Initiative Boundaries).
### Findings
- **[low]** "Toggling all curves off shows empty-state hint on canvas" is specified in Component Patterns (Curve toggle row) rather than State Patterns, so a state-table extractor misses it (EXPERIENCE.md § Component Patterns vs § State Patterns). *Fix:* duplicate one row in State Patterns (`All curves off | Graph view | hint overlay, analysis retained`).

## 5. Visual reference coverage — adequate
Checked: `imports/` contains one file (`layout.png`); no `mockups/` or `wireframes/` directories exist. EXPERIENCE.md § IA cites `imports/layout.png` with "Spine wins on conflict" stated once; `reconcile-layout.md` maps all 21 layout ideas to spine locations with 0 dropped.
### Findings
- **[low]** DESIGN.md never links `imports/layout.png` inline — the only spine-internal visual citation is EXPERIENCE.md's single generic "Composition reference" line, which does not name what the image illustrates (hero, bar, chips, graph, cards) (DESIGN.md §§ all; EXPERIENCE.md § IA). *Fix:* add one inline pointer in DESIGN.md § Brand & Style (`Visual direction: imports/layout.png — hero, function bar, TRY ONE chips, graph + legend, derivative + point-analysis cards`) or declare `reconcile-layout.md` the canonical map from DESIGN.md too.
- **[low]** No orphans, but EXPERIENCE.md's reference is unspecific standing alone; it is rescued only by `reconcile-layout.md` existing alongside (EXPERIENCE.md § IA). *Fix:* same one-line fix as above; no new artifact needed.

## 6. Bloat & overspecification — strong
Checked: DESIGN.md editorial voice (allowed) vs EXPERIENCE.md tables; pixel values, source restatement, invented prose.
### Findings
No findings. Pixel values present (`2.5px` curves, `0 4px 14px` button shadow, 44px hit padding) have no covering token and are justified instance specs. No Requirements/persona restatement. `[ASSUMPTION]` flags (f″ purple, error red, sub-1024 scroll, "Stationary" color, pole "—" readout, canvas fallback) are committed decisions marked honestly, not bloat.

## 7. Inheritance discipline — thin
Checked: `sources` frontmatter resolution, verbatim names, glossary identity, cross-spine component names, EXPERIENCE.md token refs.
### Findings
- **[critical]** Single-function-per-session ("each Analyze replaces the workspace state", EXPERIENCE.md § Foundation) contradicts PRD Story 5 ("a configured workspace with multiple functions and custom viewports", prd-geogebra-rs.md § Story 5) with no recorded scope-cut decision in `.memlog.md`. A story-dev consumer cannot know whether multi-function is deferred or the PRD is wrong. *Fix:* either restore multi-function handling to the spine or record an explicit decision (memlog + Initiative Boundaries note) cutting Story 5 to single-function for this run.
- **[high]** Marker rendering contradicts AD-4 while citing it: AD-4 places root/extrema marker circles on canvas with Svelte rendering only text overlay (architecture-geogebra-rs.md § AD-4); both spines place "DOM-overlay dots" / "markers are real buttons" and cite "(per AD-4)" (DESIGN.md § Components/Graph canvas + Analysis marker; EXPERIENCE.md § IA + § Accessibility Floor). The DOM-button rationale (keyboard/focus) is sound but unattributed as a deviation. *Fix:* either move markers back to canvas per AD-4 or record an AD-4 amendment (markers are DOM buttons for a11y; canvas exposes `role="img"` + text fallback).
- **[high]** Rendering stack drift: sources mandate wgpu/WebGPU + WebGL2 fallback with WGSL shaders (PRD §§ 5/8, Arch §§ AD-4/Stack); spines specify "Plain Svelte + canvas geometry with DOM overlay" with no wgpu/WebGPU/WGSL mention, demoting WebGL2 to a "DOM/SVG fallback message" row (DESIGN.md § Brand & Style; EXPERIENCE.md §§ Foundation, State Patterns). A consumer would build 2D canvas, not the `render` crate pipeline. *Fix:* name wgpu as the geometry renderer in Foundation (canvas element / WebGL2 fallback preserved) or record a stack-cut decision.
- **[medium]** R-IDs / Story numbers never cited in either spine (except "Story 5" in Flow 2's title); traceability to R1–R12 relies on prose matching (both spines §§ all). *Fix:* append requirement tags to Component Patterns / State Patterns rows (e.g. `R1`, `R5–R6`, `R11–R12`) — one editing pass, no restructuring.
- **[medium]** Component-name drift across sections: IA says "Graph view" / "TRY ONE chips" / "Point analysis" while component rows say "Graph canvas" / "Try chip" / "Point slider + Value row" (EXPERIENCE.md § IA vs § Component Patterns; DESIGN.md § Components). Resolvable by a human, brittle for extraction. *Fix:* pick one name per component and use it verbatim in IA, patterns, and DESIGN.md (recommend: Graph canvas, Try chip, Point analysis card).
- **[medium]** Term drift: PRD Story 3 acceptance says "zeros"; spines say "roots" throughout (EXPERIENCE.md §§ Component Patterns, State Patterns; glossary.md defines neither term, PRD § 6 defines neither). *Fix:* adopt one term in the glossary and use it verbatim (recommend `root`, then update PRD wording at next edit).
- **[low]** Share row omits the AD-6 wire contract it implements (8-char slug, `"version"` field, 64 KB cap, <50 ms target) — analysis state is specified, persistence shape is not (EXPERIENCE.md § Component Patterns/Share row vs architecture-geogebra-rs.md § AD-6). *Fix:* one sentence in the Share row citing slug/version/payload limits.

## 8. Shape fit — strong
Checked: DESIGN.md section order, EXPERIENCE.md required defaults, dropped defaults, required-when-applicable, invented sections.
### Findings
No findings. DESIGN.md follows the canonical order exactly (Brand & Style → Colors → Typography → Layout & Spacing → Elevation & Depth → Shapes → Components → Do's and Don'ts), all present. EXPERIENCE.md carries all eight required defaults in order. Dropped Responsive & Platform is defensible (desktop-only run, tablet/mobile explicitly deferred; min-width + scroll fallback committed in DESIGN.md § Layout & Spacing). Inspiration & Anti-patterns correctly present (layout.png + GeoGebra lifts, three reasoned rejects). No invented sections; `reconcile-layout.md` is a companion file, not a spine section.

## Mechanical notes
- EXPERIENCE.md `sources` (3 paths) all resolve relative to the workspace dir to existing files: `../prd-geogebra-rs/prd-geogebra-rs.md`, `../architecture-geogebra-rs/architecture-geogebra-rs.md`, `../initiative-geogebra-rs.md`. No ticket exists (consumer-stakes coaching path per `.memlog.md`) — sources-frontmatter basis is correct.
- DESIGN.md frontmatter: `name`, `description` present; `status: draft`, `updated: 2026-10-19`. All 19 color values are quoted hex; typography roles carry valid `fontFamily/fontSize/fontWeight/lineHeight[/letterSpacing]` subsets; `rounded`/`spacing` values are CSS dimensions. `components` composite value `'1px solid {colors.border-hairline}'` embeds a ref inside a shorthand — resolvable by convention, flagged here so a strict `{path}`-only resolver is not surprised.
- EXPERIENCE.md token refs (`{colors.primary-soft}`, `{colors.error}`) resolve to DESIGN.md frontmatter. No Mermaid in either spine — nothing to syntax-check. `imports/layout.png` viewed and matches the spine's composition claims (hero, bar + Analyze, 4 TRY ONE chips, graph + 3-item legend, derivative card + CALCULATED badge, point-analysis card with `x = 0.9`, 4 value rows).
