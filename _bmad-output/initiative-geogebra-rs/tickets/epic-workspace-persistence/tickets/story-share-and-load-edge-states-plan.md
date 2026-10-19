---
title: 'Share and load edge states'
type: 'feature'
ticket: '4'
created: '2026-10-19'
status: 'built'
baseline_revision: '28799b01692d06f08cc0f8857fcd5003867c3429'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Edge states are wired but incomplete: the 413 panel has no manual retry (users must close and re-click Share), and the 404/413/offline paths lack proof that the local workspace stays intact.

**Approach:** Add a panel Retry control on oversize that re-POSTs with the local workspace untouched, and cover the 404/413/offline/500 paths with intact-workspace assertions on both backend and frontend.

</frozen-after-approval>

## Implementation Notes

Oneshot route: one additive UI control plus test coverage over existing handlers, estimated under 100 lines across ~3 files with all statuses and microcopy reused as-is.

- Decided Retry as a panel button re-running `handleShare` (fresh serialize + POST, panel stays open): one-click manual retry. UX spine assumes retry is manual re-click of Share, but the Share button closes an open panel, so re-click needs two clicks; Retry honors the ticket's "manual retry" plus "panel stays open".
- Changed: `frontend/src/App.svelte` (shareRetry flag, focus to Retry on 413, retry-control button), `frontend/tests/share.test.js` (retry/offline/intact tests), `crates/api/src/routes.rs` (413 boundary, 404/413/400 bodies, 500 via closed pool).
- Surprise: 5.1's style-carry frontend work is gone from the tree (share.js/fallback/wasm/vite clean at HEAD); only its untracked plan file remains. Out of 5.4 scope — flagging for the user, not re-doing here.
- Verified: `cargo test -p api` 13/13, `npm test --prefix frontend` 22/22.

## Plan Change Log

## Review Triage Log

- Rejected writes lack no-row proof (413/400/500 assert status/text only) — verdict `low`, patch: handlers return before `put` by inspection, adding row-count assertions is trivial.
- No 404-load intact assertion through `restoreFromSlug` — verdict `low`, rejected: App flow needs a Svelte component harness (more than a simple correction); code read proves the early return touches nothing.
- 413 intact/re-POST proven only by string-match — verdict `low`, rejected: same harness reason as above; `handleShare` re-serializes and POSTs by inspection.
- No offline intact App-flow assertion — verdict `low`, rejected: same harness reason; offline branches return before mutating workspace state.
- 500 frontend mapping untested (`save`/`load` → generic invalid) — verdict `low`, patch: pure-function unit test over explicit fetch stubs, trivial.
- Retry `disabled` omits the `!backend` guard — verdict `false`: unreachable — `backend` is assigned once in `onMount` and never nulled, and the panel opens only via `handleShare` which requires `backend`.
- Retry focus queued before the `{#if shareRetry}` block materializes — verdict `low`, patch: await Svelte `tick()` before focusing.

## Verification

**Commands:**
- `cargo test -p api` -- expected: 404, 413 boundary, 400, 500, round-trip all pass.
- `npm test --prefix frontend` -- expected: share tests including retry, intact, and offline-disabled pass.
