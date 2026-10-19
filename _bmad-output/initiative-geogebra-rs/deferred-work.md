- source_plan: `/home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/tickets/story-tracer-bullet-styled-ribbon-with-shader-contract-plan.md`
  summary: Reconcile Axum flat workspace validation with pre-existing wrapped SaveBody callers.
  evidence: api/routes save now requires flat {version:1,functions,params,viewport} and returns 400 for the prior wrapped {version,workspace} shape; change is sibling 5.1 work in the same diff since baseline, not 4.1, so it defers to the persistence entry.
- source_plan: `/home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/tickets/story-tracer-bullet-share-link-restores-identical-session-plan.md`
  summary: Serve frontend static with SPA fallback in Axum so prod direct open of /w/<slug> works.
  evidence: tracer adds Vite dev SPA fallback plus origin-based pkg URL; Axum stays API-only with no ServeDir fallback, so prod /w/<slug> still 404s while ?s=<slug> works.
- source_plan: `/home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/tickets/story-tracer-bullet-share-link-restores-identical-session-plan.md`
  summary: Add Rust custom-style workspace round-trip test covering styled two-function session.
  evidence: serialize now emits style map and JS round-trip verified equal true; Rust tests still cover default style only, so custom hex/width/dashed/visible round-trip via setters belongs in sweep.
- source_plan: `/home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-workspace-persistence/tickets/story-tracer-bullet-share-link-restores-identical-session-plan.md`
  summary: Decide slug-load precedence over seeded DEMO_CURVES and clear extras on restore.
  evidence: onMount seeds DEMO_CURVES g/h while restoreFromSlug only adds functions, so a stored single-function session restores with extra curves; needs the 4.3 palette owner's decision plus a function-removal path.
