---
name: Function explorer
description: Friendly calculus function explorer — type a function, see f(x), f'(x), and the story they tell.
status: final
updated: 2026-10-19
colors:
  page-bg: '#F7F7FC'
  surface: '#FFFFFF'
  surface-soft: '#F1F0FA'
  ink: '#1E1B33'
  ink-muted: '#6E6B8A'
  ink-faint: '#9C99B8'
  border-hairline: '#E6E5F1'
  primary: '#4F46E5'
  primary-hover: '#4338CA'
  primary-ink-on: '#FFFFFF'
  primary-soft: '#EDECFD'
  curve-f: '#4F46E5'
  curve-fprime: '#1FA36B'
  curve-tangent: '#E8930C'
  curve-fdouble: '#9333EA'
  asymptote: '#B9B8D0'
  success: '#16A34A'
  error: '#DC2626'
  slider-track: '#D9D7EF'
typography:
  hero:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 34px
    fontWeight: '800'
    lineHeight: '1.15'
    letterSpacing: -0.02em
  hero-accent:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 34px
    fontWeight: '800'
    lineHeight: '1.15'
    letterSpacing: -0.02em
  eyebrow:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 11px
    fontWeight: '600'
    lineHeight: '1.4'
    letterSpacing: 0.12em
  card-title:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 16px
    fontWeight: '700'
    lineHeight: '1.3'
  card-label:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 11px
    fontWeight: '600'
    lineHeight: '1.4'
    letterSpacing: 0.08em
  body:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 14px
    fontWeight: '400'
    lineHeight: '1.5'
  mono-input:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace'
    fontSize: 15px
    fontWeight: '500'
    lineHeight: '1.5'
  mono-value:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace'
    fontSize: 13px
    fontWeight: '600'
    lineHeight: '1.4'
  caption:
    fontFamily: 'Inter, system-ui, sans-serif'
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.4'
rounded:
  sm: 6px
  md: 10px
  lg: 14px
  xl: 18px
  full: 9999px
spacing:
  '1': 4px
  '2': 8px
  '3': 12px
  '4': 16px
  '5': 24px
  '6': 32px
  gutter: 24px
  content-max: 1120px
components:
  function-bar:
    background: '{colors.surface}'
    border: '1px solid {colors.border-hairline}'
    radius: '{rounded.lg}'
  analyze-button:
    background: '{colors.primary}'
    foreground: '{colors.primary-ink-on}'
    radius: '{rounded.md}'
  try-chip:
    background: '{colors.surface}'
    active-background: '{colors.primary-soft}'
    foreground: '{colors.ink-muted}'
    radius: '{rounded.sm}'
    border: '1px solid {colors.border-hairline}'
  graph-canvas:
    background: '{colors.surface}'
    border: '1px solid {colors.border-hairline}'
    radius: '{rounded.lg}'
  legend:
    font: '{typography.caption.fontSize}'
    foreground: '{colors.ink-muted}'
  derivative-card:
    background: '{colors.surface}'
    border: '1px solid {colors.border-hairline}'
    radius: '{rounded.lg}'
  point-slider:
    track: '{colors.slider-track}'
    fill: '{colors.primary}'
    thumb: '{colors.primary}'
  value-row:
    label: '{colors.ink-muted}'
    value-font: '{typography.mono-value.fontSize}'
  badge-calculated:
    foreground: '{colors.success}'
    font: '{typography.caption.fontSize}'
  param-slider:
    track: '{colors.slider-track}'
    fill: '{colors.primary}'
    thumb: '{colors.primary}'
  analysis-marker:
    root: '{colors.error}'
    extremum: '{colors.primary}'
    inflection: '{colors.curve-fprime}'
  error-text:
    foreground: '{colors.error}'
    font: '{typography.caption.fontSize}'
  curve-toggle:
    active: '{colors.primary}'
    inactive: '{colors.ink-faint}'
  share-row:
    background: '{colors.surface-soft}'
    radius: '{rounded.md}'
---

## Brand & Style

The function explorer is a friendly calculus companion for students working late and lecturers preparing class — "your friendly calculus tool." The posture is encouraging clarity: one hero line ("Make sense of functions."), one function bar, one graph, and two analysis cards. Nothing shouts; the mathematics is the hero. Light theme only (desktop-web decision); no dark mode exists in this run.

Scope-cut for this run: single primary function + derived curves f/f′/f″/tangent per run; multi-function set (Story 5 plural) deferred. Svelte + wgpu geometry via canvas element (WGSL), WebGL2 fallback — no stack cut, no UI kit, no Figma. WebGL2 is an adapter retry; the canvas-failure message shows only if both fail. Every component below is bespoke and specified here in full.

Visual direction: imports/layout.png — hero, function bar, TRY ONE chips, graph + legend, derivative + point-analysis cards.

Composition references: mockups/key-workspace.html, mockups/key-analysis.html, mockups/key-share.html.

## Colors

- **Page wash `{colors.page-bg}`** — cool lavender-gray canvas behind everything. Keeps the surface feeling like paper, not an IDE.
- **Surface `{colors.surface}`** — pure white cards (function bar, Graph canvas, derivative card, Point analysis card). All content lives on white; the page wash only frames.
- **Surface soft `{colors.surface-soft}`** — muted fill for the share-link row and the x-readout pill. Never for primary actions.
- **Ink `{colors.ink}`** — near-black navy body text and hero. **Ink muted `{colors.ink-muted}`** for captions, legend, axis labels, value-row labels. **Ink faint `{colors.ink-faint}`** for axis ticks and disabled toggles only.
- **Hairline `{colors.border-hairline}`** — 1px card and input borders. Barely-there separation; never heavy outlines.
- **Primary indigo `{colors.primary}`** (hover `{colors.primary-hover}`) — the single brand action color: Analyze button, f(x) curve, slider fill + thumb, extremum markers. **Primary soft `{colors.primary-soft}`** is the active Try chip wash.
- **Curve green `{colors.curve-fprime}`** — f′(x), dashed. **Tangent amber `{colors.curve-tangent}`** — tangent line, dashed. **f″ purple `{colors.curve-fdouble}`** [ASSUMPTION — no f″ curve appears in layout.png; purple chosen to sit apart from indigo/green/amber] — f″(x), dashed. **Asymptote gray `{colors.asymptote}`** — vertical asymptotes, thin dashed; the graph must break the curve at poles, never bridge across them.
- **Success `{colors.success}`** — the "Increasing" behavior word and the CALCULATED badge dot. Means "computed, healthy."
- **Error `{colors.error}`** [ASSUMPTION — no error state appears in layout.png; standard red] — char-exact parse errors and root markers only. Never fills, never backgrounds.
- **Contrast targets:** ink on surface, primary-ink-on on primary, curve-f / curve-fprime / curve-tangent / curve-fdouble on surface — all target WCAG AA (4.5:1 text, 3:1 non-text graphics) [ASSUMPTION — ratios unmeasured; verify before build].

Avoid: dark surfaces, gradients, more than the four curve colors on one graph, red anywhere except errors and root dots.

## Typography

Sans-first (Inter / system-ui); monospace reserved for mathematics.

- **Hero `{typography.hero}`** — "Make sense of functions." The word "functions." renders in `{colors.primary}` (`{typography.hero-accent}`). One line, never wrapped into a paragraph.
- **Eyebrow `{typography.eyebrow}`** — uppercase tracked labels: "YOUR FRIENDLY CALCULUS TOOL", "VISUAL EXPLORER", "FIRST DERIVATIVE", "POINT ANALYSIS". Always `{colors.ink-muted}` except the hero eyebrow accent dash.
- **Card title `{typography.card-title}`** — "Graph canvas", "At a glance". **Card label `{typography.card-label}`** — tracked micro-labels above values.
- **Body `{typography.body}`** — helper lines ("The function's rate of change at every point.", "Move along the curve").
- **Mono input `{typography.mono-input}`** — the function field (`f(x) = e^(0.5x)`), derivative readout, and x-pill. Mathematics is always monospace, never proportional.
- **Mono value `{typography.mono-value}`** — right-aligned analysis numbers (1.568, 0.784). Tabular figures so values don't jitter while dragging.
- **Caption `{typography.caption}`** — legend, axis ticks, "Showing x from −5 to 5", examples line.

## Layout & Spacing

Scale: 4 / 8 / 12 / 16 / 24 / 32. Content max `{spacing.content-max}`, centered, page gutter `{spacing.gutter}`.

Vertical stack: hero block → function-bar card → Try chip row → 2-column workspace (Graph canvas left ~2fr, right rail ~1fr with derivative card above Point analysis card). Param sliders (a/b) live in a collapsible row under the function bar, shown only when the expression contains parameters. The share row docks at the bottom of the right rail after first Analyze.

Desktop web only — single fixed 2-column grid, no breakpoints in this run (tablet/mobile deferred per decision). Minimum legible width 1024px; below that, horizontal scroll rather than a broken reflow [ASSUMPTION — sub-1024 behavior not covered by layout.png].

## Elevation & Depth

Flat cards on the page wash; separation comes from 1px `{colors.border-hairline}` borders, not shadows. One exception: the Analyze button carries a soft indigo shadow (`0 4px 14px rgba(79,70,229,.35)`) as the single "press me" signal. Hover states deepen tone; nothing lifts.

## Shapes

Friendly-rounded: cards `{rounded.lg}` (Graph canvas `{rounded.lg}`, function bar `{rounded.lg}`), inputs and Analyze button `{rounded.md}`, Try chip `{rounded.sm}` with pill option `{rounded.full}` for the x-readout. Slider thumbs are full circles `{rounded.full}`. No sharp corners anywhere; no fully-pill cards.

## Components

- **Function bar** — white card, `{rounded.lg}`, hairline border. Mono input with `f(x) =` prefix chip, inline clear (×), "Use x as your variable" hint top-right, examples caption below ("Examples: sin(x), x^2 e^x, log(x)"). Error state: hairline border turns `{colors.error}`, `{components.error-text}` message appears beneath with character position.
- **Analyze button** — `{colors.primary}` fill, white text + arrow (↗), `{rounded.md}`, indigo shadow. Hover `{colors.primary-hover}`. Disabled (empty input) mutes to `{colors.ink-faint}` fill. Label is always "Analyze function".
- **Try chip** — small bordered chip (`Quadratic`, `Sine wave`, `Cubic`, `Exponential`). Active/hover wash `{colors.primary-soft}` with `{colors.primary}` text. One click fills the function bar and fires Analyze.
- **Graph canvas** — white card `{rounded.lg}`. Curves + grid on canvas/wgpu per AD-4 (wgpu via canvas element, WGSL; WebGL2 adapter retry); marker dots are DOM buttons (AD-4 amendment for keyboard/focus), canvas role=img with text fallback. Light grid, axis lines, tick labels in `{colors.ink-faint}`. Curves: f solid `{colors.curve-f}` 2.5px; f′ dashed `{colors.curve-fprime}`; tangent dashed `{colors.curve-tangent}`; f″ dashed `{colors.curve-fdouble}`; asymptotes thin dashed `{colors.asymptote}` with broken (never bridged) curves at poles. Toolbar top-right: f′(x) toggle pill, zoom in/out, reset.
- **Legend** — caption-size row above the canvas: solid indigo swatch `f(x)`, dashed green `f′(x)`, dashed amber `tangent`. Swatch style always matches curve style.
- **Derivative card** — white card `{rounded.lg}`. Waveform icon, eyebrow "FIRST DERIVATIVE", mono formula readout, body helper line, `{components.badge-calculated}` top-right. Shows f″ line below f′ when second-derivative scope is on.
- **Point slider** — in Point analysis card: indigo track fill on `{colors.slider-track}`, circular `{colors.primary}` thumb, min/max captions (−5 / 5). Dragging re-renders values at 60 FPS; keyboard arrows are the equivalent input (see EXPERIENCE.md).
- **Value row** — in Point analysis card: label left (`{colors.ink-muted}`), mono value right. Rows: Function value, Instantaneous slope, Local behavior, Tangent line. Local behavior renders "Increasing" in `{colors.success}` / "Decreasing" in `{colors.ink-muted}` / "Stationary" in `{colors.curve-tangent}` [ASSUMPTION — only "Increasing" is visible in layout.png].
- **Badge calculated** — green dot + uppercase "CALCULATED" caption. Means WASM returned fresh values; absent while analyzing or on error.
- **Param slider** — same visual language as point slider, one row per parameter (`a`, `b`) with mono value. Appears only when the expression contains parameters (tracer `f(x)=a·x²+b`).
- **Analysis marker** — marker dots are DOM buttons (AD-4 amendment for keyboard/focus), curves + grid on canvas/wgpu per AD-4. Dots on the curve: roots (PRD "zeros" → roots; PRD update next edit) `{colors.error}`, extrema `{colors.primary}`, inflection `{colors.curve-fprime}`. White ring, 44px-equivalent hit area via invisible padding; hover/click opens the value tooltip.
- **Error text** — `{colors.error}` caption with caret position ("Unexpected character '^' at position 7"). Appears under the function bar within 50ms of keystroke; never a modal, never a toast.
- **Curve toggle** — legend-adjacent / toolbar switches for f, f′, f″, tangent, markers. Active swatch in curve color, inactive `{colors.ink-faint}`. Toggles re-render canvas only, never refire analysis.
- **Share row** — `{colors.surface-soft}` inset row with readonly shortlink + Copy button. Appears after first successful Analyze; copy confirms inline ("Copied.").

## Do's and Don'ts

| Do | Don't |
|---|---|
| Light surfaces, hairline borders, indigo reserved for action + f(x) | Dark mode, gradients, indigo body text |
| Dashed green f′, dashed amber tangent, dashed purple f″, gray dashed asymptotes | Solid tangent, bridged curves across poles, a fifth curve color |
| Mono for every formula and value; tabular figures in value rows | Proportional font for mathematics |
| Errors inline under the bar, char-exact, <50ms | Modal or toast errors |
| Legend swatches mirror curve style exactly | Legend that disagrees with the canvas |
| One Analyze button shadow as the single CTA signal | Shadows on cards |
