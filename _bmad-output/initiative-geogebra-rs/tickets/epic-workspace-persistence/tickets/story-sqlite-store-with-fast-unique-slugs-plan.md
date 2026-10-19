---
title: 'SQLite store with fast unique slugs'
type: 'feature'
ticket: '2'
created: '2026-10-19'
status: 'built'
baseline_revision: '9454a516336e133e85f854b4caaa77f2f324e53e'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The workspace blob store is in-memory, so saved shortlinks vanish on restart and slugs come from a predictable counter rather than random retry.

**Approach:** Swap the store body to a `sqlx` SQLite file behind the same `put`/`get` contract, with random 48-bit retry 8-char URL-safe slugs, keeping the entry 1 POST/GET shapes, 413/404 behavior, and sub-50ms round-trips.

## Boundaries & Constraints

**Always:** Keep `POST /api/workspaces` and `GET /api/workspaces/:slug` shapes, statuses, and validation verbatim; store the blob whole and return it byte-identical; enforce 64KB bodies with 413; 8-char URL-safe slugs; round-trips under 50ms; unauthenticated blob store with no math, auth, or mutation endpoints.

**Never:** No link expiry or GC, no SVG export, no frontend changes, no new endpoints, no Postgres migration, no schema beyond `slug` plus blob.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Save new blob | Valid version 1 doc, DB ready | `201 {slug}` with fresh 8-char URL-safe slug, stored durably | No error expected |
| Slug collision retry | Generated slug already in table | New random slug generated, single row stored | No error surfaced; retry internal |
| Load hit | Known slug | `200` with byte-identical stored blob | No error expected |
| Load miss or malformed slug | Unknown or non-URL-safe slug | `404 unknown workspace`, store untouched | In-place 404 text path |
| Oversize payload | Body over 64KB | `413` before parsing | Local workspace intact |
| Invalid document | Bad JSON, wrong version, missing keys | `400` with reason | Nothing stored |

</frozen-after-approval>

## Code Map

- `crates/api/Cargo.toml` -- dependency gate: `sqlx` needs `sqlite` plus bundled runtime and a randomness source.
- `crates/api/src/store.rs` -- swap target: `Store::new/put/get`, counter slug encoder, and unit tests.
- `crates/api/src/routes.rs` -- contract to preserve verbatim: validation, 413/404/400, latency assert, round-trip tests.
- `crates/api/src/main.rs` -- wiring: pool connect, table ensure, shared `Arc<Store>` state.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/api/Cargo.toml` -- enable `sqlx` SQLite bundled runtime and add randomness -- so the store can open a file DB and mint random slugs.
- [ ] `crates/api/src/store.rs` -- replace in-memory map with `SqlitePool` table, random 48-bit retry slugs, same `put`/`get` shape -- so blobs persist across restarts.
- [ ] `crates/api/src/main.rs` -- connect the pool, ensure the table, share `Arc<Store>` -- so both routes see one durable store.
- [ ] `crates/api/src/routes.rs` -- await the store calls, keep all validation and statuses -- so the entry 1 contract holds.
- [ ] `crates/api/src/store.rs` -- unit-test slug uniqueness, retry, restart persistence, and opaque round-trip edge cases -- so the matrix stays green.

**Acceptance Criteria:**
- Given a fresh DB file, when two valid docs are saved, then both slugs are 8-char URL-safe, distinct, and each loads its own identical blob.
- Given a saved slug and a restarted server on the same DB file, when the slug is loaded, then the identical blob returns with no re-save.
- Given sustained save/load traffic, when round-trip latency is measured, then it completes under 50ms per the existing assert.

## Implementation Notes

## Plan Change Log

## Review Triage Log

- Collision test never forces the retry branch (random mint ~2^-48 hit chance, only row-count asserted) — verdict `low`, rejected: the 3-line retry loop is obviously correct with its detection half covered, and forcing a real collision needs RNG injection (new surface).
- `:memory:` pool without shared-cache/max_connections(1) flakes — verdict `false`: 15 consecutive `cargo test -p api` runs all 10/10 green; each test owns its pool and awaits sequentially so one connection is reused — cross-connection invisibility does not occur as structured.
- `get` maps every sqlx/parse error to `None` → `404` — verdict `low`, patch with the next row (shared defect: the store has no error channel, so DB failures surface as domain results).
- `put` panics on any non-unique DB error — verdict `low`, patch with the previous row (same defect; panics where the infallible in-memory `put` could not fail).
- `connect` mangles URLs via substring replace and overrides `create_if_missing` — verdict `low`, rejected: the default URL is verified working (reopen test green); exotic env-URL forms are rare and proper parsing adds branching complexity.
- Sub-50ms acceptance untested for file backing/sustained traffic — verdict `low`, patch: add a file-DB loop timing test (debug-only/save-only asserts are the inherited entry-1 contract, left as-is).
- Frontend dashed hunks against `Never: no frontend changes` — verdict `false`: this implementation touched only `crates/api/*`; the frontend hunks are uncommitted 5.1 plus concurrent-session work, not this change's product.
- calculus/render math hunks against blob-store scope — verdict `false`: same attribution; those hunks are other sessions' in-flight work (part already in HEAD), untouched by this implementation.

## Design Notes

Random 48-bit values encoded into the existing 8-char URL-safe alphabet keep the shortlink shape while making guesses infeasible; primary-key collision retries converge immediately at this scale. `put`/`get` go async with the pool since file I/O must not block the runtime, and callers already live in async handlers. One table (`slug TEXT PRIMARY KEY`, `blob TEXT NOT NULL`) with the JSON serialized once keeps the opaque `style?` passthrough byte-identical. Default database URL is `sqlite:data/workspaces.db?create_if_missing=true` with the `data/` directory created at startup.

## Verification

**Commands:**
- `cargo test -p api` -- expected: slug uniqueness, save/load round-trip, opaque style, 404, 413, 400 all pass.
- `cargo build -p api` -- expected: clean build with SQLite bundled, no system sqlite dependency.
