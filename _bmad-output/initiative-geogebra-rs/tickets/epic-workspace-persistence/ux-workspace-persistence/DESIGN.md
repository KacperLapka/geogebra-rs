---
name: Workspace sharing
description: Share and restore calculus workspaces via shortlink.
status: final
updated: 2026-10-18
colors:
  # No UI system in the app (plain Svelte + native inputs); these tokens cover
  # the share layer only. Canvas, FormulaBar, Sliders, tooltip keep current app visuals.
  surface: '#FFFFFF'
  ink: '#1A1A1A'
  muted: '#5A5A5A'
  border: '#D8D8D8'
  accent: '#0F4C81'
  accent-foreground: '#FFFFFF'
  success: '#1E7E34'
  danger: '#B3261E'
typography:
  # Platform-native web: system stack, browser default ramp. Only slug-display is specified.
  body:
    fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif'
    fontSize: 14px
    fontWeight: '400'
    lineHeight: '1.45'
  label:
    fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif'
    fontSize: 13px
    fontWeight: '600'
    lineHeight: '1.4'
  slug-display:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'
    fontSize: 14px
    fontWeight: '600'
    lineHeight: '1.4'
  status-note:
    fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif'
    fontSize: 13px
    fontWeight: '400'
    lineHeight: '1.4'
rounded:
  # Native-adjacent, minimal: inputs and panel keep the app's current square-ish feel.
  sm: 4px
  md: 6px
  full: 9999px
spacing:
  # Share layer only; app layout spacing inherited as-is.
  '1': 4px
  '2': 8px
  '3': 12px
  '4': 16px
components:
  share-button:
    background: '{colors.accent}'
    foreground: '{colors.accent-foreground}'
    radius: '{rounded.md}'
  share-panel:
    background: '{colors.surface}'
    foreground: '{colors.ink}'
    border: '{colors.border}'
    radius: '{rounded.md}'
  slug-field:
    font: '{typography.slug-display.fontFamily}'
    foreground: '{colors.ink}'
    border: '{colors.border}'
    radius: '{rounded.sm}'
  status-note:
    font: '{typography.status-note.fontFamily}'
    foreground: '{colors.muted}'
    radius: '{rounded.sm}'
  copy-control:
    background: '{colors.surface}'
    foreground: '{colors.ink}'
    border: '{colors.border}'
    radius: '{rounded.sm}'
---

## Brand & Style

Workspace sharing is a classroom utility, not a product surface. The aesthetic posture is *invisible infrastructure*: the existing app (FormulaBar, Canvas, Sliders, hover tooltip) looks exactly as it does today, and the share layer adds one header-adjacent control plus a small panel. Nothing about sharing rebrands the app, introduces a component library, or restyles existing controls. Projector-legible and sober: readable at a distance, quiet up close.

There is no UI system — plain Svelte with native browser inputs. This DESIGN.md specifies only the share-layer delta (Share button, share panel, slug field, status note). Everything else inherits the current app as-is.

## Colors

The share layer uses a minimal token set on top of the app's current unstyled-native look.

- **Surface (`#FFFFFF`)** is the share panel background. Used only for the panel. [ASSUMPTION: matches current page background intent; app currently ships browser-default background.]
- **Ink (`#1A1A1A`)** is text in the panel and slug field. Never used on Canvas curves.
- **Muted (`#5A5A5A`)** is the status-note text (saving / copied / error hints). Never used for the slug itself — the slug must stay full-contrast.
- **Border (`#D8D8D8`)** outlines the panel and slug field. Flat 1px borders only, no shadows as the primary separator (see Elevation & Depth).
- **Accent (`#0F4C81`)** is reserved for the Share button fill. Used nowhere else in the share layer — status, success, and error states use text + `{colors.success}` / `{colors.danger}`, never accent. [ASSUMPTION: accent hue; memlog specifies placement, not color.]
- **Accent-foreground (`#FFFFFF`)** is the Share button label.
- **Success (`#1E7E34`)** marks the copied confirmation only. Never used decoratively.
- **Danger (`#B3261E`)** marks 404 / 413 / offline error text only. Never used for chrome.

Contrast targets: slug `{colors.ink}` on `{colors.surface}` full-contrast (7:1 intent, never muted); status note `{colors.muted}` / `{colors.success}` / `{colors.danger}` meet AA 4.5:1 for normal text; every state also changes the note text, never color alone.

Avoid: new brand hues, gradient fills, colored panel backgrounds, restyling FormulaBar/Sliders/Canvas to "match" sharing.

## Typography

Body, label, and status roles inherit the platform-native web stack (`{typography.body.fontFamily}`) at browser-default sizes. Only one role is specified:

- **Slug display** (`{typography.slug-display.fontFamily}`, 14px semibold) renders the visible slug and full shortlink. Monospace because the slug is a machine token the user must transcribe or visually verify — proportional figures risk `l`/`1` confusion on a projector. [ASSUMPTION: monospace choice; memlog requires the slug be visible, not the typeface.]

Rules: status notes stay in `{typography.status-note.fontSize}` regular — never bold error text to "make it louder"; the `{colors.danger}` color carries the signal. The slug is never truncated or masked.

## Layout & Spacing

The app layout is inherited unchanged. Share-layer rules:

- The Share button sits in the app header adjacent to the FormulaBar (per memlog decision) — same row, never inside the Canvas or the Sliders block.
- The share panel opens anchored to the Share button (popover position), sized to content: slug field + copy control + one status-note line. Single-column, no tabs, no secondary views. Gaps use `{spacing.2}` (8px) between rows, `{spacing.3}` (12px) panel padding. [ASSUMPTION: exact padding values.]
- The shared-link load view (`/w/<slug>`) reuses the main workspace layout identically — same FormulaBar / Canvas / Sliders positions — so a projected shared session matches the professor's screen pixel-for-pixel in structure.
- Breakpoints: none new. Desktop/laptop plus projector is the target; tablet is read-only-acceptable and gets the same layout without touch-specific restyling; no native mobile surface. [ASSUMPTION: tablet keeps desktop layout; memlog allows read-only tablet without specifying layout.]

## Elevation & Depth

No shadow language. The share panel separates from the page with a 1px `{colors.border}` outline on a `{colors.surface}` fill — the same flat vocabulary as the app's current `<details>` workspace block. No drop shadows, no overlays dimming the Canvas, no modal backdrop. Sharing never covers the workspace it describes; the panel is a popover, not a dialog.

## Shapes

`{rounded.sm}` (4px) for the slug field and status note; `{rounded.md}` (6px) for the Share button and share panel. `{rounded.full}` appears nowhere in the share layer — no pill buttons, no pill badges. Corners stay rectilinear to match the app's native-input squareness; sharing reads as "tool chrome," not a marketing moment.

## Components

Existing components (FormulaBar, Canvas, Sliders, hover tooltip) are used as-is, unchanged. The share layer adds four specs:

- **Share button** — `{colors.accent}` fill, `{colors.accent-foreground}` label, `{rounded.md}` corners, `{typography.label.fontSize}` semibold label "Share". Adjacent to FormulaBar. While a save is in flight it shows the saving state (label + disabled) rather than spawning a second element.
- **Share panel** — `{colors.surface}` fill, 1px `{colors.border}` outline, `{rounded.md}` corners. Contains exactly: the slug field, a copy control, and one status-note line. Opens on Share click, closes on `Esc` or outside click; focus moves into the panel on open and returns to the Share button on close.
- **Slug field** — read-only text in `{typography.slug-display.fontFamily}` showing the full shortlink (e.g. `/w/<slug>`); selectable for manual copy. `{colors.ink}` on `{colors.surface}` with a `{colors.border}` outline and `{rounded.sm}` corners. The slug is always fully visible, never ellipsized.
- **Status note** — one line in `{typography.status-note.fontSize}` / `{colors.muted}`: saving progress, the `{colors.success}` copied confirmation, or a `{colors.danger}` error (unknown workspace, payload too large, offline). One state visible at a time; success and error never shown simultaneously.
- **Copy control** — text button in `{components.copy-control}` (`{colors.surface}` fill, 1px `{colors.border}` outline, `{rounded.sm}` corners) placed immediately after the slug field in the share panel; label "Copy". Disabled until the slug field is populated; on click writes the full shortlink to the clipboard.

## Do's and Don'ts

| Do | Don't |
|---|---|
| Keep FormulaBar / Canvas / Sliders / tooltip visuals untouched | Restyle existing controls to "match" the share layer |
| Show the slug fully visible in monospace (`{typography.slug-display.fontFamily}`) | Truncate, mask, or ellipsize the slug |
| Use `{colors.accent}` only for the Share button | Use accent for status, success, or error text |
| One status-note line, one state at a time | Stack saving + copied + error messages together |
| Flat 1px `{colors.border}` panel, no backdrop | Modal dialogs, page dimming, shadow-heavy popovers |
| Popover anchored to Share, `Esc` closes | Full-screen share views or multi-step share wizards |
