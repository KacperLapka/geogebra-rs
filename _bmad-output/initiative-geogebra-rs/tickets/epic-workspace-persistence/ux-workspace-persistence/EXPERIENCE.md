---
name: Workspace sharing
status: final
sources:
  - ../epic-workspace-persistence.md
updated: 2026-10-18
---

# Workspace sharing — Experience Spine

## Foundation

Web desktop/laptop plus projector; Svelte 5 frontend with plain native inputs — no UI system, platform conventions native (browser form controls, system type stack). This spine is the behavior; `DESIGN.md` is the visual identity reference (share-layer tokens: `{colors.accent}`, `{typography.slug-display.fontFamily}`, `{rounded.md}`, `{spacing.2}`). Spine wins on conflict with any future mockup, wireframe, or import. State ownership follows AD-5/AD-6: Rust `wasm_bridge` owns math state and workspace serialisation (`version: 1`, ≤ 10 KB), Axum is an unauthenticated JSON blob store (8-char URL-safe slug, 64 KB middleware cap, < 50 ms), Svelte owns UI state and rendering. No accounts, no authentication. Stakes are internal classroom use; tablet read-only acceptable, no native mobile surface.

## Information Architecture

| Surface | Reached from | Purpose |
|---|---|---|
| Main workspace | App open (`/`) | Existing surface: FormulaBar, Canvas, Sliders, hover tooltip; where sharing starts |
| Share control | App header, adjacent to FormulaBar | Single Share button; entry point to saving + shortlink |
| Share panel | Share control click | Shows shortlink slug visibly, copy-to-clipboard, one status-note line |
| Shared link view | `/w/<slug>` (primary) | Reconstructs the identical session: functions, params, viewport |
| Fallback load | `?s=<slug>` query | Same reconstruction when the path form is unavailable (e.g. mangled link) |

Load surfaces reuse the main workspace layout identically so a projected shared session matches the professor's screen in structure.

## Voice and Tone

Microcopy. Brand voice and aesthetic posture live in `DESIGN.md`.

| Do | Don't |
|---|---|
| "Share" (button label) | "Save & Publish 🚀" |
| "Saving…" (in-flight) [ASSUMPTION: exact string] | "Please wait while we process your request…" |
| "Link copied." (confirmation) [ASSUMPTION: exact string] | "Workspace persisted successfully ✓" |
| "Unknown workspace. Check the link and try again." (404) [ASSUMPTION: exact string] | "404 — resource not found" |
| "Workspace too large to share." (413) [ASSUMPTION: exact string] | "413 Payload Too Large" |
| "You're offline. Showing your local workspace." (offline) [ASSUMPTION: exact string] | "Network error. Retry connection." |

Classroom register: plain verbs, no jargon, no error codes in user-facing text, no celebratory copy. Prof. Lena and student Milo get the same tone.

## Component Patterns

Behavioral. Visual specs live in `DESIGN.md` (Components) and the current app otherwise.

| Component | Use | Behavioral rules |
|---|---|---|
| Share button | App header near FormulaBar | Click opens the share panel and starts the save POST. Disabled with "Saving…" while in flight; re-enabled on slug return or error. Never duplicated elsewhere on the surface. |
| Share panel | Anchored to Share button | Contains slug field + copy control + one status-note line. `Esc` or outside click closes. Focus moves into the panel on open, returns to Share button on close. |
| Slug field | Share panel | Read-only, fully visible shortlink (`/w/<slug>`), selectable for manual copy. Never truncated, masked, or ellipsized. |
| Copy control | Share panel | Copies the full shortlink via clipboard. Success swaps the status note to the copied confirmation; the panel stays open so the slug remains visible. [ASSUMPTION: panel stays open after copy; clipboard failure falls back to manual selection of the slug field.] |
| Status note | Share panel; load surfaces for errors | Exactly one state visible at a time: saving, copied, 404, 413, offline. Load-surface errors render in place where the workspace would appear, with the local workspace left intact. |

## State Patterns

| State | Surface | Treatment |
|---|---|---|
| Saving | Share panel | Button disabled, status note "Saving…" (`{colors.muted}`). Panel stays open; no second indicator. |
| Copied | Share panel | Status note flips to `{colors.success}` "Link copied." Slug remains visible and selected-reachable. |
| Saved slug returned | Share panel | Slug field populated with `/w/<slug>`; copy control enabled. API round-trip < 50 ms, so no progress bar. |
| Unknown workspace (404) | Shared link view / fallback load | In-place note: "Unknown workspace. Check the link and try again." (`{colors.danger}`). No redirect, no empty Canvas. |
| Payload too large (413) | Share panel | Status note: "Workspace too large to share." (`{colors.danger}`). Existing workspace untouched; user can simplify and retry. [ASSUMPTION: retry is manual re-click of Share.] |
| Offline | Main workspace / shared link view | Note: "You're offline. Showing your local workspace." (`{colors.muted}`); local workspace intact and read-only until reconnect. Share button disabled while offline. [ASSUMPTION: Share disabled offline rather than queueing.] |
| Loaded identical | Shared link view / fallback load | Functions, params (slider values), and viewport match the saved session exactly; normal workspace interaction resumes. |

## Interaction Primitives

- **Click** Share → panel opens + POST fires. Click copy control → clipboard write + copied confirmation. Click outside / `Esc` → panel closes, focus returns to Share.
- **Copy-to-clipboard** is the primary handoff; the visible slug plus manual text selection is the fallback path (projector rooms, locked-down browsers). [ASSUMPTION: manual selection as the designated fallback.]
- **Keyboard:** Share button is `Tab`-reachable; `Enter`/`Space` activates. `Esc` always closes the panel. No custom shortcuts, no vim-style bindings — the audience is classroom users, not power users.
- **No drag, no hover-only affordances:** every share action is a visible button; nothing is revealed on hover (projector and tablet use forbid it).
- **Banned everywhere:** multi-step share wizards, modal backdrops over the Canvas, link-expiry or SVG-export UI (both deferred per epic boundaries), authentication prompts.

## Accessibility Floor

Behavioral. Visual contrast lives in `DESIGN.md` (slug at full `{colors.ink}` on `{colors.surface}`; status never carried by color alone — every state also changes the note text).

- Keyboard-only path complete: `Tab` to Share, `Enter` to open, `Tab` to copy control, `Esc` to close. Focus trapped neither in nor out — focus moves in on open, returns on close.
- Screen reader announces surface and state via `aria-live` on the status note: "Saving…", "Link copied.", error texts verbatim. Slug field exposed as read-only text, not a disabled control (disabled inputs are skipped by some readers). [ASSUMPTION: `aria-live` polite region; no live-region pattern in current app.]
- Projector legibility: slug in `{typography.slug-display.fontFamily}` at full contrast, fully visible — transcribable from the back row. Status text never relies on `{colors.success}` / `{colors.danger}` alone.
- `Tab` order matches reading order: Share button → panel (slug → copy) → rest of header. Touch targets reuse the desktop layout on tablet; no hover-dependent action exists to lose.

## Responsive & Platform

| Context | Behavior |
|---|---|
| Desktop / laptop | Full workspace + Share control + panel; primary authoring and loading surface. |
| Projector | Same layout as desktop; slug monospace fully visible for back-row transcription. |
| Tablet | Same layout, read-only acceptable; no touch-specific restyling, no hover affordances. |
| Native mobile | No dedicated surface. |

## Key Flows

### Flow 1 — Lena shares the demo (Prof. Lena, classroom, 8:50am Monday)

1. Lena opens the main workspace and configures `f(x) = a*x^2 + b`: two Sliders for `a` and `b`, viewport framed on the Canvas.
2. She clicks the Share control in the app header, adjacent to the FormulaBar. The share panel opens; status note shows "Saving…" and the button disables.
3. The app serialises the workspace (`version: 1`, functions, params, viewport) and POSTs it; Axum returns the 8-char slug in < 50 ms. The slug field populates with `/w/<slug>`, fully visible.
4. Lena clicks the copy control. Status note flips to "Link copied." The panel stays open with the slug visible so she can verify it before pasting into the class chat.
5. **Climax:** Lena pastes the link into the projector-room chat without leaving her configured workspace — the Canvas behind the panel still shows her exact demo, untouched. Sharing added a shortlink; it changed nothing she built.

Failure (413): workspace exceeds the cap → status note "Workspace too large to share." Panel stays open, demo intact; she simplifies (fewer functions) and clicks Share again. Failure (offline): Share control disabled with "You're offline. Showing your local workspace." — nothing half-saved, nothing lost.

### Flow 2 — Milo opens the link (student Milo, laptop, 9:02am Monday)

1. Milo clicks Lena's link and lands on the shared link view `/w/<slug>`.
2. The app GETs the blob and reconstructs the session: identical functions, slider values for `a` and `b`, identical viewport — the same layout Lena projected.
3. Milo drags a slider; the curve follows (normal workspace interaction resumes — he holds Lena's demo, live).
4. **Climax:** Milo compares his screen to the projector and they match — functions, params, viewport. No login stood between him and the demo; the link *was* the access.

Failure (404): mistyped slug → in-place "Unknown workspace. Check the link and try again." — no blank Canvas, no redirect. Failure (mangled path, e.g. LMS link-rewriter strips it): Milo reloads via the `?s=<slug>` fallback form and lands identically. Failure (offline): "You're offline. Showing your local workspace." with his local state intact.
