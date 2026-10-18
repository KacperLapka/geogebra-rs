# Spine Pair Review — Workspace sharing

## Overall verdict
The pair is a clean, buildable contract for the share-layer delta: flows cover the epic Outcome and Done-when states, tokens resolve fully, and states walk all five IA surfaces. One broken `sources` path and one missing Copy-control visual spec are the only fixes that matter before downstream consumers source-extract.

## 1. Flow coverage — strong
Checked: ticket has no Requirements section, so Done-when bullets (R11/R12, serialize ≤10 KB, POST slug <50 ms, GET restore, 413 >64 KB, `version: 1`) plus Outcome (professor shares / students open identical session) were extracted; each needs a Key Flow with named protagonist, numbered steps, climax, failure.
### Findings
None — Flow 1 (Prof. Lena shares, EXPERIENCE.md:85-93, 5 steps + climax + 413/offline failures) covers save/POST/slug/copy; Flow 2 (student Milo opens, EXPERIENCE.md:95-102, 4 steps + climax + 404/mangled-path/offline failures) covers GET/restore/`?s=<slug>` fallback. Memlog protagonists (Lena/Milo, .memlog.md:7) match verbatim.

## 2. Token completeness — adequate
Checked: all 8 `colors`, 4 `typography` roles, 3 `rounded`, 4 `spacing`, 4 `components` frontmatter tokens plus all 28 `{path.to.token}` refs in DESIGN.md and 8 in EXPERIENCE.md against `references/design-md-spec.md` type rules; hex presence; contrast targets.
### Findings
- **[low]** No numeric contrast ratios for load-bearing pairs (slug `{colors.ink}` on `{colors.surface}`, `{colors.muted}`/`{colors.success}`/`{colors.danger}` note text) (DESIGN.md Colors, EXPERIENCE.md:76-80). *Fix:* state targets (e.g. AA 4.5:1 for note text, 7:1 or "full-contrast, never muted" for slug) or record deliberate exemption for internal-classroom stakes.

## 3. Component coverage — adequate
Checked: every component named anywhere needs a DESIGN.md Components visual row *and* an EXPERIENCE.md Component Patterns behavioral row with real rules.
### Findings
- **[medium]** Copy control has an EXPERIENCE.md behavioral row (EXPERIENCE.md:51) but no DESIGN.md Components visual bullet and no `components.copy-control` frontmatter entry; it is described only inside the Share-panel bullet (DESIGN.md:121) and Layout (DESIGN.md:104). *Fix:* add a `copy-control` frontmatter entry plus one DESIGN.md Components bullet (fill, label, radius, placement relative to slug field).
- Inherited components (FormulaBar, Canvas, Sliders, hover tooltip) listed as used-as-is (DESIGN.md:118) with no EXPERIENCE.md rows — defensible, no finding.

## 4. State coverage — strong
Checked: walked all five IA surfaces (main workspace, share control, share panel, `/w/<slug>`, `?s=<slug>` fallback) against applicable states (saving, copied, saved-slug, 404, 413, offline, loaded-identical).
### Findings
None — State Patterns table (EXPERIENCE.md:56-64) covers saving/copied/saved-slug/404/413/offline/loaded-identical; empty/cold-load N/A justified by <50 ms round-trip with no progress bar (EXPERIENCE.md:60); permission-denied N/A (no accounts, EXPERIENCE.md:13). Each load error renders in place with local workspace intact.

## 5. Visual reference coverage — adequate
Checked: listed `mockups/`, `wireframes/`, `imports/`; verified inline links and spines-win-on-conflict statement.
### Findings
- **[low]** `imports/` is empty (verified), no `mockups/` or `wireframes/` directories exist, so zero orphans — but neither spine states spines-win-on-conflict once, as required even when there is nothing to conflict with (cf. `experience-example-shadcn.md:29`). *Fix:* add one sentence in EXPERIENCE.md Foundation (e.g. "Spine wins on conflict with any future mockup/wireframe/import.").

## 6. Bloat & overspecification — strong
Checked: pixel specs beyond tokens, ticket/source restatement, prose-where-table, unread sections, decorative narrative; DESIGN.md editorial voice vs EXPERIENCE.md plain behavior.
### Findings
None — share-layer-only scoping is enforced (DESIGN.md:72-74, Do's and Don'ts DESIGN.md:125-134); technical facts in Foundation (AD-5/AD-6, `version: 1`, 10 KB / 64 KB / <50 ms, EXPERIENCE.md:13) are load-bearing state-ownership context, not Requirements restatement; `[ASSUMPTION]` tags mark invented strings/values.

## 7. Inheritance discipline — adequate
Checked: `sources` resolution; journey/requirement names verbatim from ticket Done-when/Outcome (no Requirements section); glossary identity; component-name identity; EXPERIENCE.md token refs resolving to DESIGN.md.
### Findings
- **[high]** EXPERIENCE.md `sources` path does not resolve: `../../epic-workspace-persistence.md` (EXPERIENCE.md:5) from `tickets/epic-workspace-persistence/ux-workspace-persistence/` points to `tickets/epic-workspace-persistence.md`, which does not exist; the epic lives at `../epic-workspace-persistence.md` (i.e. `tickets/epic-workspace-persistence/epic-workspace-persistence.md`). *Fix:* change source to `../epic-workspace-persistence.md`.
- **[low]** Surface-vs-component naming wobble: IA surface "Share control" (EXPERIENCE.md:20) vs component "Share button" (EXPERIENCE.md:48, DESIGN.md:120); State Patterns "Share control disabled" (EXPERIENCE.md:63) vs "Button disabled" (EXPERIENCE.md:58). *Fix:* keep "Share control" for the IA surface and "Share button" for the component consistently, or collapse to one term.

## 8. Shape fit — adequate
Checked: DESIGN.md canonical order; EXPERIENCE.md required defaults (Foundation, IA, Voice and Tone, Component Patterns, State Patterns, Interaction Primitives, Accessibility Floor, Key Flows); required-when-applicable (Inspiration, Responsive); invented sections.
### Findings
- **[low]** No dedicated Responsive & Platform section despite three consumption contexts (desktop/laptop + projector + tablet-read-only, EXPERIENCE.md:13, DESIGN.md:106). Coverage exists inline (DESIGN.md:105-106, EXPERIENCE.md:71,81) and the "same layout, no touch restyling" decision is committed, so extraction is possible — but a consumer must hunt across three sections. *Fix:* add a 4-row Responsive & Platform table (desktop / projector / tablet-read-only / no-native-mobile) or record the inline coverage as deliberate.
- Dropped Inspiration & Anti-patterns is defensible (no reference products in memlog; deferred link-expiry/SVG-export captured under Banned, EXPERIENCE.md:72) — no finding.

## Mechanical notes
Frontmatter: DESIGN.md has `name/description/status/colors/typography/rounded/spacing/components` but no `updated` (EXPERIENCE.md has `updated: 2026-10-18`); both `status: draft`. All 36 `{path.to.token}` refs resolve (`{typography.label.fontSize}` → 13px, `{spacing.2}`/`{spacing.3}` → 8/12px defined). No Mermaid. No mockup/wireframe/import links to check. DESIGN.md body order matches canonical spec exactly.
