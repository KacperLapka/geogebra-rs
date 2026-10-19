---
name: Function explorer
status: final
sources:
  - ../prd-geogebra-rs/prd-geogebra-rs.md
  - ../architecture-geogebra-rs/architecture-geogebra-rs.md
  - ../initiative-geogebra-rs.md
updated: 2026-10-19
---

# Function explorer — Experience Spine

## Foundation

Desktop web only, single 2-column workspace (tablet/mobile deferred per decision). Svelte + wgpu geometry via canvas element (WGSL), WebGL2 fallback — no stack cut; curves + grid on canvas/wgpu per AD-4, marker dots are DOM buttons (AD-4 amendment for keyboard/focus), canvas role=img with text fallback; symbolic/numeric core in WASM, share shortlinks via Axum blob store. No UI kit, no Figma — `DESIGN.md` is the sole visual reference and names every token this spine cites. Scope-cut for this run: single primary function + derived curves f/f′/f″/tangent per run; multi-function set (Story 5 plural) deferred, share covers single-function state. Each Analyze replaces the workspace state.

## Information Architecture

| Surface | Reached from | Purpose |
|---|---|---|
| Function bar | Top of workspace, always visible | Type/edit expression, char-exact errors, Analyze trigger, examples |
| Try chip | Below function bar | One-click preset functions (Quadratic, Sine wave, Cubic, Exponential) |
| Param sliders | Under function bar, only when expression has parameters | Tracer controls for `a`/`b` in `f(x)=a·x²+b` |
| Graph canvas | Left column | Canvas/wgpu curves (f, f′, f″, tangent, asymptotes) + DOM marker dots, pan/zoom, legend + toggles |
| Derivative card | Right rail, top | Symbolic f′ (and f″) readout with CALCULATED badge |
| Point analysis card | Right rail, bottom | x-slider + value rows (function value, slope, behavior, tangent line) |
| Analysis marker | DOM overlay on Graph canvas | Roots (PRD "zeros" → roots; PRD update next edit) / extrema / inflection dots with value tooltip |
| Share row | Right rail bottom, after first Analyze | Shortlink display + copy (lecturer flow, Story 5 single-function scope-cut) |

→ Composition references: Function bar / Graph canvas / Derivative card / Point analysis card → `mockups/key-workspace.html`; Param sliders / Analysis markers / Asymptote / Error state → `mockups/key-analysis.html`; Share row → `mockups/key-share.html` (base layout: `imports/layout.png`). Spine wins on conflict.

## Voice and Tone

Microcopy. Brand voice and aesthetic posture live in `DESIGN.md`.

| Do | Don't |
|---|---|
| "Make sense of functions." | "Unleash powerful math analytics!" |
| "Use x as your variable" | "ERROR: invalid variable binding" |
| "Unexpected character '^' at position 7" | "Syntax error" |
| "Move along the curve" | "Drag the slider to interpolate values" |
| "Copied." | "Shortlink copied successfully ✓" |
| "Couldn't create the link. Try again." | "Axum blob store 500" |

Plain words, character positions, no exclamation marks, no internals leaking.

## Component Patterns

Behavioral. Visual specs live in `DESIGN.md.Components`; token refs below resolve to `DESIGN.md` frontmatter.

| Component | Use | Behavioral rules |
|---|---|---|
| Function bar | Workspace top | Enter or Analyze fires analysis. Typing revalidates <50ms; error text shows char position and blocks Analyze. Clear (×) empties input, returns workspace to empty state. (R1) |
| Analyze button | Function bar right | Fires WASM analysis; loading label "Analyzing…" (button stays, no spinner overlay). Disabled on empty input or live parse error. (R1) |
| Try chip | Below function bar | Click fills bar + fires Analyze immediately. No confirmation. Active chip (matching current expression) shows `{colors.primary-soft}` wash. |
| Graph canvas | Left column | Renders on Analyze, slider drag, toggle, pan/zoom. Poles break curves — never bridge asymptotes. Pan (drag) + zoom (wheel/buttons/reset) preserve analysis state. Curves + grid on canvas/wgpu per AD-4. (R7 asymptote break, R8 canvas geometry, R9 pan/zoom) |
| Legend | Above canvas | Mirrors visible curves. Clicking a swatch toggles that curve (same as curve-toggle). (R10) |
| Derivative card | Right rail top | Shows symbolic f′ after Analyze; f″ line appears when second-derivative scope applies. Badge shows CALCULATED only on fresh WASM values. (R3) |
| Point slider | Point analysis card | Range = current x-domain (−5…5 default). Drag updates marker, tangent, and all value rows live at 60 FPS. Arrow keys move ±0.1 when focused. (R4) |
| Value row | Point analysis card | Updates live with slider. Slope shown to 3 decimals. Behavior derives from sign of f′ (Increasing / Decreasing / Stationary). (R4) |
| Badge calculated | Derivative card | Hidden while analyzing or on error. Green dot + "CALCULATED". Not a button. (R3) |
| Param slider | Under function bar | One row per detected parameter (`a`, `b`). Drag re-renders curves + values without refiring symbolic analysis. Hidden when expression has no parameters. (R2) |
| Analysis marker | DOM overlay on Graph canvas | DOM-button dots for roots (PRD "zeros" → roots; PRD update next edit) (`{colors.error}`), extrema (`{colors.primary}`), inflection (`{colors.curve-fprime}`). Click/hover shows tooltip with coordinates. Hover crosshair + coordinate tooltip is owned by Analysis marker, not Graph canvas. Toggleable as a layer. (R5–R6) |
| Error text | Under function bar | Appears <50ms after keystroke with char-exact message. Clears on next valid keystroke. Never modal, never toast. (R1) |
| Curve toggle | Graph toolbar | Independent toggles for f / f′ / f″ / tangent / markers. Canvas-only re-render; analysis state untouched. f toggle always present; toggling all off shows empty-state hint on canvas). (R10) |
| Share row | Right rail bottom | Appears after first successful Analyze. Copy writes shortlink to clipboard, confirms inline "Copied.". Opening a shortlink restores expression + x-position + toggles. Wire per AD-6: 8-char slug, version field, 64KB cap, <50ms target. (R11–R12) |

## State Patterns

| State | Surface | Treatment |
|---|---|---|
| Cold load | Workspace | Hero + empty function bar + Try chip row; Graph canvas shows faint grid with "Type a function and hit Analyze." overlay. Derivative/Point analysis cards show em-dash placeholders, no badge. |
| Empty input | Function bar | Analyze disabled; examples caption visible. Clearing input returns here. (R1) |
| Analyzing | Workspace | Button reads "Analyzing…"; cards show skeleton shimmer; badge hidden. Previous curves stay visible until new values land (no flash to empty). (R1, R8) |
| Parse error (char-exact) | Function bar | Hairline turns `{colors.error}`; `{colors.error}` caption with character + position; Analyze disabled. Graph canvas and cards keep last good state. (R1) |
| Pole / asymptote | Graph canvas | Curve breaks at pole; gray dashed asymptote line; point slider skips across the pole (values show "—" exactly at the pole, never ±∞ as a finite number) [ASSUMPTION — pole readout treatment not in layout.png]. (R7) |
| No markers | Graph canvas | Expression with no roots/extrema/inflection in domain → markers layer silently empty; no "none found" banner. (R5–R6) |
| All curves off | Graph canvas | Hint overlay on empty grid, analysis retained; toggling any curve back on restores canvas without refiring analysis. (R10) |
| Share fail | Share row | Inline "Couldn't create the link. Try again." Retry button; analysis state untouched. (R11–R12) |
| Offline | Global | Analyze on cached WASM still works; share shows "You're offline — sharing needs a connection." No blocking modal. (R11–R12) |
| WebGL2 / canvas fallback | Graph canvas | wgpu/WebGPU first, WebGL2 adapter retry; message "Your browser can't draw the graph. Try a recent Chrome, Edge, or Firefox." shows only if both fail. Rest of workspace stays usable [ASSUMPTION — fallback path not in sources]. (R8) |

## Interaction Primitives

- **Type + Analyze:** the core loop. Type expression → <50ms validation → Enter/Analyze → curves + cards + badge. Presets skip typing.
- **Drag slider, 60 FPS:** point slider and param sliders update canvas + values continuously; no debounce visible to the user; values formatted to 3 decimals.
- **Pan / zoom:** drag pans, wheel or ± buttons zoom, reset restores −5…5. Domain label ("Showing x from −5 to 5") updates; curves re-render for the visible domain without refiring analysis.
- **Hover query:** hovering the Graph canvas shows a crosshair + coordinate tooltip (owned by Analysis marker); clicking pins the point slider to that x.
- **Toggles:** legend swatches, toolbar pill, and marker-layer switch all flip instantly with canvas-only re-render.
- **Banned:** page reload to re-analyze, modal dialogs for errors, drag-to-reorder anything, hover-only controls without keyboard equivalent.

## Accessibility Floor

Behavioral. Visual contrast lives in `DESIGN.md` (indigo on white, white on indigo, and curve colors verified to AA against card white at render).

- Every slider has full keyboard operation (arrows ±0.1, PageUp/Down ±1, Home/End to domain edges); the point slider is focusable in tab order right after the Analyze button.
- Value rows and derivative readout sit in an `aria-live="polite"` region; screen readers announce updated slope/value on slider release, not on every frame.
- Focus order: function input → Analyze → Try chip row → param sliders → graph toggles → point slider → markers → share copy.
- Markers are DOM buttons for keyboard/focus (AD-4 amendment — curves + grid on canvas/wgpu per AD-4; canvas `role="img"` with text fallback); each marker is a real button (role + label: "Root at x = 1.00").
- All interactive targets ≥ 44px (markers via invisible hit padding).
- `prefers-reduced-motion`: slider and pan updates snap without transition; skeleton shimmer becomes static placeholder.

## Inspiration & Anti-patterns

- **Lifted from layout.png:** the clarity contract — one hero line, one bar, graph left / insight right, legend that mirrors the canvas. This spine preserves that composition exactly and extends it (params, markers, share) in the same visual language.
- **Lifted from GeoGebra:** the point-on-curve probe — drag x, everything follows. The point slider + live value rows are this product's version.
- **Rejected — 3D graph view:** depth adds nothing to f/f′ comprehension at this scope; flat canvas/wgpu geometry per AD-4.
- **Rejected — auth / accounts:** sharing works via anonymous shortlinks (Axum blob store); no login wall before Analyze or Share.
- **Rejected — SVG-everything rendering:** geometry stays on canvas/wgpu (wgpu via canvas element, WGSL) for 60 FPS drags; marker dots are DOM buttons (AD-4 amendment for keyboard/focus), readouts are DOM.

## Key Flows

### Flow 1 — Maya's 11pm breakthrough (second-year student, exponential)

1. Maya opens the explorer at 11pm, stuck on `e^(0.5x)` and its derivative.
2. Cold workspace: hero, empty function bar, Try chip row, Graph canvas faint grid.
3. She types `e^(0.5x)`; validation stays silent (valid so far). She hits Enter.
4. Button reads "Analyzing…"; previous empty grid holds; then curves land — indigo f rising, dashed green f′, amber tangent.
5. Derivative card shows `f′(x) = e^(x/2)/2` with CALCULATED badge. **Climax:** she drags the x-slider in the Point analysis card to 0.9 — Function value 1.568, slope 0.784, "Increasing" in green. The number she half-guessed on paper clicks into place against the curve.
6. She drags the Graph canvas to pan right; the domain label updates from "Showing x from −5 to 5" to the new range, analysis follows without refiring.
7. She scrolls to zoom out, spots the flat tail near −5, then resets to −5…5. She hovers the tangent, toggles f″ on to check concavity, then copies nothing — understanding was the deliverable.

Failure: she mistypes `e^(0.5x` → inline "Missing ')' at position 7" in `{colors.error}`; Analyze disables; Graph canvas keeps last good state; closing the paren clears the error in <50ms.

### Flow 2 — Lecturer share (Dr. Nowak, morning class prep — Story 5)

1. Dr. Nowak analyzes `a·x²+b` with `a=1, b=0`, drags param sliders to the shape she wants to demonstrate.
2. She sets the point slider to the vertex, toggles markers on (extremum dot visible).
3. Share row is live after Analyze; she hits Copy → "Copied."
4. **Climax:** in class, students open the shortlink — same expression, same slider position, same toggles. The room sees her exact setup, no dictation of formulas.
5. A student drags `a` negative on their own copy; the parabola flips; the lesson lands.

Failure: campus Wi-Fi drops during copy → inline "Couldn't create the link. Try again." Analysis and slider state are untouched; retry succeeds when the network returns. Offline openers of an existing link see cached expression with share disabled and the offline note.

### Flow 3 — Alex verifies his paper (student, cubic roots/extrema/inflection — Story 3, R5–R6)

1. Alex (student) hand-computed roots, extrema, and inflection for `x^3-3x+1` in exercise class and wants to check his paper.
2. He types `x^3-3x+1` in the function bar and hits Analyze; Graph canvas renders indigo f with DOM-button Analysis marker dots for roots, extrema, inflection.
3. He hovers each marker dot; the coordinate tooltip shows exact values while the crosshair tracks the curve.
4. He drags the point slider in the Point analysis card across the domain; value rows update live and agree with the marker positions.
5. **Climax:** the root/extremum marker tooltip matches his hand-computed numbers — paper verified against the graph, error hunt over.

Failure: he pans to a domain slice with no roots/extrema/inflection in view → markers layer stays silently empty, no "none found" banner; analysis values keep working.

### Flow 4 — The pole encounter (lecturer demo, `1/x` singularity — Story 4, R7)

1. Dr. Nowak (lecturer) types `1/x` to show why a pole is not a steep curve.
2. Graph canvas breaks the indigo curve at `x=0` and draws a gray dashed asymptote — never bridged.
3. She drags the point slider toward 0; value rows track, then show "—" exactly at the pole.
4. **Climax:** the room sees broken curve + asymptote + "—" readout together — singularity clicks as different from steep-but-finite slope.

Failure: NaN / ±∞ is never shown as a finite number; at the pole the readout holds "—" and the curve stays broken.
