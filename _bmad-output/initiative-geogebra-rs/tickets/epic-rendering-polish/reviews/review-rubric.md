# Gate review — rubric walker (inline; subagents depth-limited)

Verdict: PASS with two notes (both recorded, neither blocking).

## Findings

1. (High — fixed inline) Dash-flag reservation was one-sided: epic 4 was
   forbidden from emitting `2.0` but nothing forbade epic 3 from emitting
   `0.0`/`1.0` and colliding with solid/derivative styles. Fixed: epic 3 MUST
   use only `dash_flag = 2.0` (E4-DASH + E4-SHADER-CONTRACT).
2. (Medium — deferred, correctly) Exact 8-colour hex values unresolved, but the
   order rule is binding regardless, so no two stories can diverge on
   assignment — only on hue. Correctly in Deferred + open question for Sofia.
3. (Low — noted) `lint_spine.py` resolves the workspace to the epic ticket
   file, not the delta (epic-folder layout vs `architecture-{slug}/` run-folder
   expectation). Manual check substituted: AD IDs unique (E4-*), every AD has
   Binds/Prevents/Rule, stack pinned, no placeholders, no template comments.

## Checklist

- Real divergence points fixed: ribbon width, dash encoding, style shape,
  shader layout, viewport ownership, GL parity, perf harness, epic boundary.
- Covers all 6 epic Done-when bullets + R8/R9/R10.
- No parent AD weakened: amendments explicit, original IDs never renumbered.
- Operational envelope (deploy/infra) silent — correctly out of epic altitude;
  owned by parent Deferred.
