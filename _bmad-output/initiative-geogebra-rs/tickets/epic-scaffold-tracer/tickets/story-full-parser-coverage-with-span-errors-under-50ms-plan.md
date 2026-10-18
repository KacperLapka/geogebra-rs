---
title: 'Full parser coverage with span errors under 50ms'
type: 'feature'
ticket: 3
created: '2026-10-18'
status: done
baseline_revision: 'd331618a7c60d0c8ae78644f44943ec036b19c90'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick-self']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Tracer parser covers only thin forms and misreports error spans when a `f(x)=`/`y=` prefix is present, with no timing guard for the 50 ms formula-bar budget.

**Approach:** Harden `core_math` lexer/parser/eval to full R1 coverage (polynomials, trig, exp, log/ln, sqrt, rationals, params) with input-relative Span errors serialized as `{span,message}` and verified under 50 ms.

## Boundaries & Constraints

**Always:** AD-2 Pratt parser with zero external deps, `Span{start,end}` byte offsets over full input, IEEE 754 f64 eval with NaN for missing params, WASM `ParseErr{span:[s,e],message}` shape unchanged, `cargo test` native only.

**Never:** Implicit multiplication, multi-arg functions, base-selectable log, differentiation/solver changes, render/frontend changes, pushing to remote.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Polynomial + params | `f(x)=a*x^2+2*x+1`, `a=2` | Eval matches f64 reference <1e-12 | No error expected |
| Trig/exp/log/sqrt | `sin(x)+cos(x)+tan(x)+exp(x)+log(x)+ln(x)+sqrt(x)` | Matches `std` f64 <1e-12 | No error expected |
| Rational + precedence | `(x+1)/(x-1)`, `2^3^2=512`, `-x^2=-(x^2)` | Correct assoc/prec | No error expected |
| Prefix span | `f(x)=a**x^` | Span points into full input incl. prefix | `{span,message}` <50ms |
| Bad syntax suite | `""`, `sin()`, `sin(x,y)`, `(x`, `x)`, `2+*3`, `frobnicate(x)`, `x$` | Char-accurate span each | `{span,message}` <50ms, loop keeps running |

</frozen-after-approval>

## Code Map

- `crates/core_math/src/parser.rs` -- owns prefix strip + Pratt parse; BUG: `Parser::new(tokens,_offset)` drops offset so all parser errors and `Expr` spans stay body-relative while only lex errors are offset; fix by offsetting token spans once after lex and threading offset into `end_span`.
- `crates/core_math/src/lexer.rs` -- owns tokenizer; BUG: `bytes[i] as char` and byte-wise ident scan break on multi-byte UTF-8; fix with `input[i..].chars()` iteration keeping byte indices.
- `crates/core_math/src/ast.rs` -- owns `Span/Expr/Func/ParseError`; reuse as-is, no shape change.
- `crates/core_math/src/eval.rs` -- owns tree-walk eval; reuse as-is, extend tests only.
- `crates/wasm_bridge/src/lib.rs` -- owns `ParseErr{span:[s,e],message}` JSON; reuse as-is, add span-accuracy + timing assertions via existing `parse_and_validate`.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/core_math/src/parser.rs` -- offset token spans by prefix length, thread offset into end_span/errors -- makes every span input-relative
- [ ] `crates/core_math/src/lexer.rs` -- char-boundary-safe scan with byte spans -- removes panic/mis-span on unicode input
- [ ] `crates/core_math/src/{parser,lexer,eval}.rs` -- full-coverage tests: all funcs, rationals, params order, precedence/assoc, scientific/whitespace, exact-span errors, <50ms parse timing -- proves R1 verify

**Acceptance Criteria:**
- Given any supported form, when `cargo test -p core_math` runs, then all suites pass natively
- Given bad syntax with or without `f(x)=` prefix, when `parse` fails, then `span` indexes the full input exactly and serializes as `{span:[s,e],message}`
- Given a keystroke parse, when timed, then error path completes in <50ms without stalling the loop

## Implementation Notes

Built 2026-10-18 on `main` over baseline `d331618` (attended build: dirty-tree
gate bypassed per parent authorization; dirty files are `_bmad-output` docs plus
untracked `.opencode/AGENTS.md/opencode.json`, none in `crates/`).

- `crates/core_math/src/parser.rs` -- token spans shifted by prefix offset once
  after `lex`, `Parser` now stores `offset` for `end_span`; comma-arity check
  moved before `expect_rparen` so `sin(x,y)` reports `takes exactly one
  argument` at the comma instead of `expected ')'`.
- `crates/core_math/src/lexer.rs` -- char-boundary-safe scan (`input[i..].chars`)
  with byte spans; multibyte `unexpected character` spans exact, no panic.
- Tests only otherwise: 11 new tests (lexer 3, parser 6, eval 2); 32
  `core_math` suites green, full workspace 57 green (`1 api + 32 core_math +
  19 render + 5 wasm_bridge`), `cargo build` clean.
- Quick self-review (no subagent lenses in this spawned session): diff
  inspected; offset math, char-boundary safety, and comma fix verified by the
  new exact-span assertions; no public shape change (`ParseErr{span,message}`
  untouched).

## Plan Change Log

## Review Triage Log

## Design Notes

Token-span offsetting (shift once after `lex`) keeps `Expr` spans and parser errors input-relative with a one-line change instead of wrapping every `ParseError::new`. Lexer keeps byte spans (AD-2 is byte slices) while iterating `chars()` so multibyte `unexpected character` spans stay exact.

## Verification

**Commands:**
- `cargo test -p core_math` -- expected: all suites pass natively
- `cargo test` -- expected: full workspace green (no regressions in render/wasm_bridge/api)
- `cargo build` -- expected: workspace builds clean
